//! Orchestration Daemon
//!
//! Manages the lifecycle of the 4-agent orchestration system:
//! - Orchestrator: Central coordinator
//! - Optimizer: Context and resource optimization
//! - Reviewer: Quality assurance and gating
//! - Executor: Work execution (wraps Claude Code sessions)
//!
//! # Architecture
//! - Rust supervision tree with Ractor actors
//! - Python DSPy agents for intelligence
//! - PyO3 bindings for low-latency communication
//! - Shared Rust state (WorkQueue, Context) for coordination
//!
//! # Usage
//! ```no_run
//! use mnemosyne_core::daemon::orchestration::OrchestrationDaemon;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let daemon = OrchestrationDaemon::new();
//!     daemon.start().await?;
//!     Ok(())
//! }
//! ```

use crate::api::{EventBroadcaster, StateManager};
use crate::error::{MnemosyneError, Result};
use crate::orchestration::network;
use crate::orchestration::supervision::{SupervisionConfig, SupervisionTree};
use crate::storage::StorageBackend;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::ipc::{self, IpcMessage};

/// Orchestration daemon configuration
#[derive(Debug, Clone)]
pub struct OrchestrationDaemonConfig {
    /// PID file location
    pub pid_file: PathBuf,

    /// Log file location
    pub log_file: PathBuf,

    /// Unix socket for IPC
    pub socket_path: PathBuf,

    /// Database path
    pub db_path: Option<String>,

    /// Supervision configuration
    pub supervision_config: SupervisionConfig,
}

impl Default for OrchestrationDaemonConfig {
    fn default() -> Self {
        let runtime_dir = dirs::runtime_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mnemosyne")
            .join("logs");

        Self {
            pid_file: runtime_dir.join("mnemosyne-orchestration.pid"),
            log_file: log_dir.join("orchestration.log"),
            socket_path: runtime_dir.join("mnemosyne-orchestration.sock"),
            db_path: None,
            supervision_config: SupervisionConfig::default(),
        }
    }
}

/// Orchestration daemon status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestrationStatus {
    /// All 4 agents running
    Running {
        pid: u32,
        orchestrator: bool,
        optimizer: bool,
        reviewer: bool,
        executor: bool,
    },
    /// Daemon not running
    NotRunning,
    /// PID file exists but process is not running (stale)
    Stale { pid: u32 },
    /// Daemon running but some agents failed
    Degraded {
        pid: u32,
        failed_agents: Vec<String>,
    },
}

/// Orchestration daemon manager
pub struct OrchestrationDaemon {
    config: OrchestrationDaemonConfig,
}

impl Default for OrchestrationDaemon {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrationDaemon {
    /// Create a new daemon with default configuration
    pub fn new() -> Self {
        Self::with_config(OrchestrationDaemonConfig::default())
    }

    /// Create a new daemon with custom configuration
    pub fn with_config(config: OrchestrationDaemonConfig) -> Self {
        Self { config }
    }

    /// Start the orchestration daemon
    ///
    /// This spawns:
    /// 1. SupervisionTree with 4 Ractor actors
    /// 2. Python agent processes (Orchestrator, Optimizer, Reviewer, Executor)
    /// 3. IPC server (Unix socket) for coordination
    /// 4. Health monitoring loop
    pub async fn start(&self) -> Result<()> {
        info!("Starting Mnemosyne orchestration daemon");

        // Check if already running
        match self.status().await? {
            OrchestrationStatus::Running { pid, .. } => {
                return Err(MnemosyneError::Other(format!(
                    "Orchestration daemon already running with PID {}",
                    pid
                )));
            }
            OrchestrationStatus::Stale { pid } => {
                warn!("Found stale PID file for process {}, removing", pid);
                self.remove_pid_file()?;
            }
            OrchestrationStatus::Degraded { pid, .. } => {
                warn!("Found degraded daemon with PID {}, stopping", pid);
                self.stop().await?;
            }
            OrchestrationStatus::NotRunning => {
                // Good to start
            }
        }

        // Create directories
        self.ensure_directories()?;

        info!("Spawning orchestration daemon process");

        // Get the path to the current executable
        let current_exe = std::env::current_exe().map_err(|e| {
            MnemosyneError::Other(format!("Failed to get current executable path: {}", e))
        })?;

        // Open log file for stdout/stderr
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file)
            .map_err(|e| MnemosyneError::Other(format!("Failed to open log file: {}", e)))?;

        // Spawn orchestration daemon process
        let mut cmd = std::process::Command::new(&current_exe);
        cmd.arg("orchestrate")
            .arg("--daemon")
            .arg("--socket")
            .arg(&self.config.socket_path);

        // Add database path if configured
        if let Some(db_path) = &self.config.db_path {
            cmd.arg("--db-path").arg(db_path);
        }

        // Set environment variables
        cmd.env("RUST_LOG", "info,mnemosyne=debug");
        cmd.env("MNEMOSYNE_ORCHESTRATION_DAEMON", "true");

        // Redirect stdout and stderr to log file
        let log_file_stdout = log_file.try_clone().map_err(|e| {
            MnemosyneError::Other(format!("Failed to clone log file handle: {}", e))
        })?;
        let log_file_stderr = log_file;

        cmd.stdout(log_file_stdout);
        cmd.stderr(log_file_stderr);

        // On Unix, use double-fork technique to properly daemonize
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;

            // Create new process group to detach from parent
            unsafe {
                cmd.pre_exec(|| {
                    // Create new session
                    nix::unistd::setsid()
                        .map_err(|e| std::io::Error::other(format!("setsid failed: {}", e)))?;
                    Ok(())
                });
            }
        }

        // Spawn the process
        let child = cmd.spawn().map_err(|e| {
            MnemosyneError::Other(format!("Failed to spawn orchestration daemon: {}", e))
        })?;

        let pid = child.id();

        // Write PID file
        self.write_pid_file(pid)?;

        info!("Orchestration daemon started with PID {}", pid);
        info!("Logs: {}", self.config.log_file.display());
        info!("Socket: {}", self.config.socket_path.display());

        // Wait a moment to check if process is still running
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        if !is_process_running(pid) {
            self.remove_pid_file()?;
            return Err(MnemosyneError::Other(
                "Orchestration daemon exited immediately after startup. Check logs.".to_string(),
            ));
        }

        info!("Orchestration daemon started successfully");

        Ok(())
    }

    /// Stop the orchestration daemon
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Mnemosyne orchestration daemon");

        match self.status().await? {
            OrchestrationStatus::Running { pid, .. }
            | OrchestrationStatus::Degraded { pid, .. } => {
                // Send SIGTERM (graceful shutdown)
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;

                    kill(Pid::from_raw(pid as i32), Signal::SIGTERM).map_err(|e| {
                        MnemosyneError::Other(format!("Failed to send SIGTERM: {}", e))
                    })?;

                    info!("Sent SIGTERM to orchestration daemon {}", pid);
                }

                #[cfg(not(unix))]
                {
                    return Err(MnemosyneError::Other(
                        "Daemon stop is only supported on Unix systems".to_string(),
                    ));
                }

                // Wait for graceful shutdown
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                // Check if still running
                match self.status().await? {
                    OrchestrationStatus::NotRunning => {
                        self.remove_pid_file()?;
                        self.remove_socket()?;
                        info!("Orchestration daemon stopped successfully");
                        Ok(())
                    }
                    OrchestrationStatus::Running { .. } | OrchestrationStatus::Degraded { .. } => {
                        warn!("Daemon did not stop gracefully, may need SIGKILL");
                        Err(MnemosyneError::Other(
                            "Orchestration daemon did not stop within timeout".to_string(),
                        ))
                    }
                    OrchestrationStatus::Stale { .. } => {
                        self.remove_pid_file()?;
                        self.remove_socket()?;
                        Ok(())
                    }
                }
            }
            OrchestrationStatus::NotRunning => {
                info!("Orchestration daemon is not running");
                Ok(())
            }
            OrchestrationStatus::Stale { pid } => {
                warn!("Found stale PID file for process {}, removing", pid);
                self.remove_pid_file()?;
                self.remove_socket()?;
                Ok(())
            }
        }
    }

    /// Get orchestration daemon status
    pub async fn status(&self) -> Result<OrchestrationStatus> {
        // Try connecting to the Unix socket first
        if self.config.socket_path.exists() {
            debug!(
                "Querying status via IPC: {}",
                self.config.socket_path.display()
            );
            match ipc::query_status(&self.config.socket_path).await {
                Ok(status) => return Ok(status),
                Err(e) => {
                    debug!("Failed to query status via IPC: {}", e);
                    // Fallback to PID file check if IPC fails
                }
            }
        }

        if !self.config.pid_file.exists() {
            return Ok(OrchestrationStatus::NotRunning);
        }

        // Read PID from file
        let pid_str = fs::read_to_string(&self.config.pid_file)
            .map_err(|e| MnemosyneError::Other(format!("Failed to read PID file: {}", e)))?;

        let pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|e| MnemosyneError::Other(format!("Invalid PID in file: {}", e)))?;

        // Check if process is running
        if !is_process_running(pid) {
            return Ok(OrchestrationStatus::Stale { pid });
        }

        // Process is running but IPC failed or socket missing
        // We can't know detailed status, so assume running but maybe degraded IPC
        Ok(OrchestrationStatus::Running {
            pid,
            orchestrator: true,
            optimizer: true,
            reviewer: true,
            executor: true,
        })
    }

    /// Health check - verify daemon and all agents are running
    pub async fn health_check(&self) -> Result<bool> {
        match self.status().await? {
            OrchestrationStatus::Running { .. } => Ok(true),
            OrchestrationStatus::Degraded { .. } => Ok(false),
            _ => Ok(false),
        }
    }

    /// Run the orchestration engine (called by daemon process)
    ///
    /// This is invoked by the `mnemosyne orchestrate --daemon` command
    /// and runs in the background process.
    pub async fn run_engine(
        &self,
        storage: Arc<dyn StorageBackend>,
        network: Arc<network::NetworkLayer>,
    ) -> Result<()> {
        info!("Starting orchestration engine");

        // Create event broadcaster and state manager
        let event_broadcaster = EventBroadcaster::new(1000);
        let state_manager = Arc::new(StateManager::new());

        // Subscribe state manager to events
        state_manager.subscribe_to_events(event_broadcaster.subscribe());

        // Create supervision tree with all 4 agents
        let mut supervision_tree = SupervisionTree::new_with_state(
            self.config.supervision_config.clone(),
            storage.clone(),
            network.clone(),
            Some(event_broadcaster.clone()),
            Some(state_manager.clone()),
        )
        .await?;

        // Spawn all 4 agents (Orchestrator, Optimizer, Reviewer, Executor)
        info!("Spawning agent actors...");
        supervision_tree.spawn_agents().await?;

        info!("All agents started successfully");

        // Start IPC server
        let (ipc_tx, mut ipc_rx) = mpsc::channel(32);
        ipc::start_ipc_server(self.config.socket_path.clone(), ipc_tx).await?;

        // Start network monitoring task
        let network_monitor = network.clone();
        // Initialize event persistence for network events
        // We use Global namespace as network state is system-wide
        let persistence = Arc::new(
            crate::orchestration::events::EventPersistence::new_with_broadcaster(
                storage.clone(),
                crate::types::Namespace::Global,
                Some(event_broadcaster.clone()),
            ),
        );
        let persistence_monitor = persistence.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;

                let agents = network_monitor.router().list_agents().await;

                // Extract unique known nodes (remote)
                let mut known_nodes = std::collections::HashSet::new();
                for (_, loc) in &agents {
                    if let crate::orchestration::network::router::AgentLocation::Remote(node_id) =
                        loc
                    {
                        known_nodes.insert(node_id.clone());
                    }
                }

                let known_nodes_vec: Vec<String> = known_nodes.into_iter().collect();
                let connected_peers = known_nodes_vec.len();

                let event = crate::orchestration::events::AgentEvent::NetworkStateUpdate {
                    connected_peers,
                    known_nodes: known_nodes_vec,
                };

                if let Err(e) = persistence_monitor.persist(event).await {
                    tracing::warn!("Failed to persist network state update: {}", e);
                }
            }
        });

        // Health check ticker
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        // Keep daemon running
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Check agent health
                    if !supervision_tree.is_healthy().await {
                        warn!("Agent health check failed, restarting agents");
                        supervision_tree.restart_failed_agents().await?;
                    }
                }
                Some(msg) = ipc_rx.recv() => {
                    match msg {
                        IpcMessage::GetStatus(reply_tx) => {
                            // Construct status based on supervision tree
                            // For now, we use a simple check.
                            // TODO: Implement granular status in SupervisionTree
                            let is_healthy = supervision_tree.is_healthy().await;

                            let status = if is_healthy {
                                OrchestrationStatus::Running {
                                    pid: std::process::id(),
                                    orchestrator: true,
                                    optimizer: true,
                                    reviewer: true,
                                    executor: true,
                                }
                            } else {
                                OrchestrationStatus::Degraded {
                                    pid: std::process::id(),
                                    failed_agents: vec!["unknown".to_string()],
                                }
                            };

                            if let Err(e) = reply_tx.send(status) {
                                warn!("Failed to send status reply: {:?}", e);
                            }
                        }
                        IpcMessage::CreateInvite(reply_tx) => {
                            let result = network.create_invite().await;
                            if let Err(e) = reply_tx.send(result) {
                                warn!("Failed to send create invite reply: {:?}", e);
                            }
                        }
                        IpcMessage::JoinPeer(ticket, reply_tx) => {
                            let result = network.join_peer(&ticket).await;
                            if let Err(e) = reply_tx.send(result) {
                                warn!("Failed to send join peer reply: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Write PID file
    fn write_pid_file(&self, pid: u32) -> Result<()> {
        fs::write(&self.config.pid_file, pid.to_string())
            .map_err(|e| MnemosyneError::Other(format!("Failed to write PID file: {}", e)))?;

        debug!("Wrote PID {} to {}", pid, self.config.pid_file.display());
        Ok(())
    }

    /// Remove PID file
    fn remove_pid_file(&self) -> Result<()> {
        if self.config.pid_file.exists() {
            fs::remove_file(&self.config.pid_file)
                .map_err(|e| MnemosyneError::Other(format!("Failed to remove PID file: {}", e)))?;

            debug!("Removed PID file: {}", self.config.pid_file.display());
        }
        Ok(())
    }

    /// Remove Unix socket
    fn remove_socket(&self) -> Result<()> {
        if self.config.socket_path.exists() {
            fs::remove_file(&self.config.socket_path)
                .map_err(|e| MnemosyneError::Other(format!("Failed to remove socket: {}", e)))?;

            debug!("Removed socket: {}", self.config.socket_path.display());
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

        // Socket directory
        if let Some(parent) = self.config.socket_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                MnemosyneError::Other(format!("Failed to create socket directory: {}", e))
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
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // Windows implementation would go here
    // For now, always return false on non-Unix
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (OrchestrationDaemonConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = OrchestrationDaemonConfig {
            pid_file: temp_dir.path().join("orchestration.pid"),
            log_file: temp_dir.path().join("orchestration.log"),
            socket_path: temp_dir.path().join("orchestration.sock"),
            db_path: None,
            supervision_config: SupervisionConfig::default(),
        };
        (config, temp_dir)
    }

    #[test]
    fn test_config_default() {
        let config = OrchestrationDaemonConfig::default();
        assert!(config
            .pid_file
            .to_string_lossy()
            .contains("mnemosyne-orchestration.pid"));
        assert!(config
            .log_file
            .to_string_lossy()
            .contains("orchestration.log"));
        assert!(config
            .socket_path
            .to_string_lossy()
            .contains("mnemosyne-orchestration.sock"));
    }

    #[tokio::test]
    async fn test_status_not_running() {
        let (config, _temp) = create_test_config();
        let daemon = OrchestrationDaemon::with_config(config);

        let status = daemon.status().await.unwrap();
        assert_eq!(status, OrchestrationStatus::NotRunning);
    }

    #[test]
    fn test_write_and_read_pid_file() {
        let (config, _temp) = create_test_config();
        let daemon = OrchestrationDaemon::with_config(config);

        daemon.ensure_directories().unwrap();
        daemon.write_pid_file(12345).unwrap();

        assert!(daemon.config.pid_file.exists());

        let content = fs::read_to_string(&daemon.config.pid_file).unwrap();
        assert_eq!(content, "12345");
    }

    #[test]
    fn test_remove_pid_file() {
        let (config, _temp) = create_test_config();
        let daemon = OrchestrationDaemon::with_config(config);

        daemon.ensure_directories().unwrap();
        daemon.write_pid_file(12345).unwrap();
        assert!(daemon.config.pid_file.exists());

        daemon.remove_pid_file().unwrap();
        assert!(!daemon.config.pid_file.exists());
    }

    #[tokio::test]
    async fn test_health_check_not_running() {
        let (config, _temp) = create_test_config();
        let daemon = OrchestrationDaemon::with_config(config);

        // No daemon running, health check should return false
        let healthy = daemon.health_check().await.unwrap();
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_health_check_with_running_process() {
        let (config, _temp) = create_test_config();
        let daemon = OrchestrationDaemon::with_config(config);

        daemon.ensure_directories().unwrap();

        // Write PID of current process
        let current_pid = std::process::id();
        daemon.write_pid_file(current_pid).unwrap();

        // Health check should succeed since current process is running
        let healthy = daemon.health_check().await.unwrap();
        assert!(healthy);
    }
}
