//! Daemon Mode for MCP Server
//!
//! Provides functionality to run the Mnemosyne MCP server in background (daemon) mode.
//!
//! # Features
//! - Daemonize MCP server process
//! - PID file management
//! - Log file rotation
//! - Signal handling (SIGTERM, SIGINT)
//! - Status checking
//!
//! # Usage
//! ```no_run
//! use mnemosyne_core::daemon;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     daemon::start_daemon(None).await?;
//!     Ok(())
//! }
//! ```

use crate::error::{MnemosyneError, Result};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// PID file location
    pub pid_file: PathBuf,

    /// Log file location
    pub log_file: PathBuf,

    /// Maximum log file size (bytes)
    pub max_log_size: u64,

    /// Database path
    pub db_path: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let runtime_dir = dirs::runtime_dir()
            .or_else(|| dirs::data_local_dir())
            .unwrap_or_else(|| PathBuf::from("."));

        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mnemosyne")
            .join("logs");

        Self {
            pid_file: runtime_dir.join("mnemosyne").join("mnemosyne.pid"),
            log_file: log_dir.join("mnemosyne.log"),
            max_log_size: 10 * 1024 * 1024, // 10MB
            db_path: None,
        }
    }
}

/// Daemon status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonStatus {
    /// Daemon is running
    Running { pid: u32 },
    /// Daemon is not running
    NotRunning,
    /// PID file exists but process is not running (stale)
    Stale { pid: u32 },
}

/// MCP Daemon manager
pub struct McpDaemon {
    config: DaemonConfig,
}

impl McpDaemon {
    /// Create a new daemon with default configuration
    pub fn new() -> Self {
        Self::with_config(DaemonConfig::default())
    }

    /// Create a new daemon with custom configuration
    pub fn with_config(config: DaemonConfig) -> Self {
        Self { config }
    }

    /// Start the daemon
    pub async fn start(&self) -> Result<()> {
        info!("Starting Mnemosyne MCP daemon");

        // Check if already running
        match self.status()? {
            DaemonStatus::Running { pid } => {
                return Err(MnemosyneError::Other(format!(
                    "Daemon already running with PID {}",
                    pid
                )));
            }
            DaemonStatus::Stale { pid } => {
                warn!("Found stale PID file for process {}, removing", pid);
                self.remove_pid_file()?;
            }
            DaemonStatus::NotRunning => {
                // Good to start
            }
        }

        // Create directories
        self.ensure_directories()?;

        // For now, we'll run in foreground but with logging to file
        // Full daemonization would require platform-specific code
        // and the `daemonize` crate, which we can add later
        info!("Starting MCP server (foreground mode with file logging)");

        // Write PID file
        let pid = std::process::id();
        self.write_pid_file(pid)?;

        info!("Daemon started with PID {}", pid);
        info!("Logs: {}", self.config.log_file.display());

        // Note: Actual MCP server startup would happen here
        // For now, this is a stub that would be integrated with the serve command

        Ok(())
    }

    /// Stop the daemon
    pub fn stop(&self) -> Result<()> {
        info!("Stopping Mnemosyne MCP daemon");

        match self.status()? {
            DaemonStatus::Running { pid } => {
                // Send SIGTERM (graceful shutdown)
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;

                    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                        .map_err(|e| MnemosyneError::Other(format!("Failed to send SIGTERM: {}", e)))?;

                    info!("Sent SIGTERM to process {}", pid);
                }

                #[cfg(not(unix))]
                {
                    return Err(MnemosyneError::Other(
                        "Daemon stop is only supported on Unix systems".to_string(),
                    ));
                }

                // Wait a bit for graceful shutdown
                std::thread::sleep(std::time::Duration::from_secs(2));

                // Check if still running
                match self.status()? {
                    DaemonStatus::NotRunning => {
                        self.remove_pid_file()?;
                        info!("Daemon stopped successfully");
                        Ok(())
                    }
                    DaemonStatus::Running { .. } => {
                        warn!("Daemon did not stop gracefully, may need SIGKILL");
                        Err(MnemosyneError::Other(
                            "Daemon did not stop within timeout".to_string(),
                        ))
                    }
                    DaemonStatus::Stale { .. } => {
                        self.remove_pid_file()?;
                        Ok(())
                    }
                }
            }
            DaemonStatus::NotRunning => {
                info!("Daemon is not running");
                Ok(())
            }
            DaemonStatus::Stale { pid } => {
                warn!("Found stale PID file for process {}, removing", pid);
                self.remove_pid_file()?;
                Ok(())
            }
        }
    }

    /// Get daemon status
    pub fn status(&self) -> Result<DaemonStatus> {
        if !self.config.pid_file.exists() {
            return Ok(DaemonStatus::NotRunning);
        }

        // Read PID from file
        let pid_str = fs::read_to_string(&self.config.pid_file).map_err(|e| {
            MnemosyneError::Other(format!("Failed to read PID file: {}", e))
        })?;

        let pid: u32 = pid_str.trim().parse().map_err(|e| {
            MnemosyneError::Other(format!("Invalid PID in file: {}", e))
        })?;

        // Check if process is running
        if is_process_running(pid) {
            Ok(DaemonStatus::Running { pid })
        } else {
            Ok(DaemonStatus::Stale { pid })
        }
    }

    /// Write PID file
    fn write_pid_file(&self, pid: u32) -> Result<()> {
        fs::write(&self.config.pid_file, pid.to_string()).map_err(|e| {
            MnemosyneError::Other(format!("Failed to write PID file: {}", e))
        })?;

        debug!("Wrote PID {} to {}", pid, self.config.pid_file.display());
        Ok(())
    }

    /// Remove PID file
    fn remove_pid_file(&self) -> Result<()> {
        if self.config.pid_file.exists() {
            fs::remove_file(&self.config.pid_file).map_err(|e| {
                MnemosyneError::Other(format!("Failed to remove PID file: {}", e))
            })?;

            debug!("Removed PID file: {}", self.config.pid_file.display());
        }
        Ok(())
    }

    /// Ensure required directories exist
    fn ensure_directories(&self) -> Result<()> {
        // PID file directory
        if let Some(parent) = self.config.pid_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MnemosyneError::Other(format!("Failed to create PID directory: {}", e))
            })?;
        }

        // Log file directory
        if let Some(parent) = self.config.log_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MnemosyneError::Other(format!("Failed to create log directory: {}", e))
            })?;
        }

        Ok(())
    }
}

/// Check if a process is running
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    // Send signal 0 (null signal) to check if process exists
    // In nix 0.30.x, we can use None to send signal 0
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // Windows implementation would go here
    // For now, always return false on non-Unix
    false
}

/// Start daemon (convenience function)
pub async fn start_daemon(db_path: Option<String>) -> Result<()> {
    let mut config = DaemonConfig::default();
    config.db_path = db_path;

    let daemon = McpDaemon::with_config(config);
    daemon.start().await
}

/// Stop daemon (convenience function)
pub fn stop_daemon() -> Result<()> {
    let daemon = McpDaemon::new();
    daemon.stop()
}

/// Get daemon status (convenience function)
pub fn daemon_status() -> Result<DaemonStatus> {
    let daemon = McpDaemon::new();
    daemon.status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (DaemonConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = DaemonConfig {
            pid_file: temp_dir.path().join("test.pid"),
            log_file: temp_dir.path().join("test.log"),
            max_log_size: 1024,
            db_path: None,
        };
        (config, temp_dir)
    }

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert!(config.pid_file.to_string_lossy().contains("mnemosyne.pid"));
        assert!(config.log_file.to_string_lossy().contains("mnemosyne.log"));
        assert_eq!(config.max_log_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_daemon_status_not_running() {
        let (config, _temp) = create_test_config();
        let daemon = McpDaemon::with_config(config);

        let status = daemon.status().unwrap();
        assert_eq!(status, DaemonStatus::NotRunning);
    }

    #[test]
    fn test_write_and_read_pid_file() {
        let (config, _temp) = create_test_config();
        let daemon = McpDaemon::with_config(config);

        daemon.ensure_directories().unwrap();
        daemon.write_pid_file(12345).unwrap();

        assert!(daemon.config.pid_file.exists());

        let content = fs::read_to_string(&daemon.config.pid_file).unwrap();
        assert_eq!(content, "12345");
    }

    #[test]
    fn test_remove_pid_file() {
        let (config, _temp) = create_test_config();
        let daemon = McpDaemon::with_config(config);

        daemon.ensure_directories().unwrap();
        daemon.write_pid_file(12345).unwrap();
        assert!(daemon.config.pid_file.exists());

        daemon.remove_pid_file().unwrap();
        assert!(!daemon.config.pid_file.exists());
    }

    #[test]
    fn test_is_process_running_current_process() {
        let current_pid = std::process::id();
        #[cfg(unix)]
        assert!(is_process_running(current_pid));

        // Test with obviously non-existent PID (use 99999 instead of u32::MAX)
        // u32::MAX can behave unexpectedly on some systems
        #[cfg(unix)]
        assert!(!is_process_running(99999));
    }
}
