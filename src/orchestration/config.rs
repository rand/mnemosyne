//! Configuration for Branch Isolation System
//!
//! Provides configuration parsing and management for branch coordination,
//! conflict detection, and notification settings.
//!
//! # Configuration File Format
//!
//! TOML format in `.mnemosyne/config.toml`:
//!
//! ```toml
//! [branch_isolation]
//! enabled = true
//! default_mode = "isolated"
//! auto_approve_readonly = true
//! orchestrator_bypass = true
//!
//! [conflict_detection]
//! enabled = true
//! critical_paths = ["migrations/**", "schema/**", "**/.env"]
//!
//! [notifications]
//! enabled = true
//! on_save = true
//! periodic_interval_minutes = 20
//! session_end_summary = true
//!
//! [cross_process]
//! enabled = true
//! mnemosyne_dir = ".mnemosyne"
//! poll_interval_seconds = 2
//! heartbeat_timeout_seconds = 30
//! ```

use crate::error::{MnemosyneError, Result};
use crate::orchestration::branch_coordinator::BranchCoordinatorConfig;
use crate::orchestration::branch_guard::BranchGuardConfig;
use crate::orchestration::branch_registry::CoordinationMode;
use crate::orchestration::conflict_notifier::NotificationConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Complete configuration for branch isolation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchIsolationConfig {
    /// Branch isolation settings
    #[serde(default)]
    pub branch_isolation: BranchIsolationSettings,

    /// Conflict detection settings
    #[serde(default)]
    pub conflict_detection: ConflictDetectionSettings,

    /// Notification settings
    #[serde(default)]
    pub notifications: NotificationSettings,

    /// Cross-process coordination settings
    #[serde(default)]
    pub cross_process: CrossProcessSettings,
}

impl Default for BranchIsolationConfig {
    fn default() -> Self {
        Self {
            branch_isolation: BranchIsolationSettings::default(),
            conflict_detection: ConflictDetectionSettings::default(),
            notifications: NotificationSettings::default(),
            cross_process: CrossProcessSettings::default(),
        }
    }
}

/// Branch isolation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchIsolationSettings {
    /// Enable branch isolation
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Default coordination mode
    #[serde(default = "default_isolated_mode")]
    pub default_mode: String,

    /// Auto-approve read-only access
    #[serde(default = "default_true")]
    pub auto_approve_readonly: bool,

    /// Allow orchestrator bypass
    #[serde(default = "default_true")]
    pub orchestrator_bypass: bool,

    /// Timeout multipliers by phase
    #[serde(default)]
    pub timeout_multipliers: TimeoutMultipliers,
}

impl Default for BranchIsolationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            default_mode: "isolated".to_string(),
            auto_approve_readonly: true,
            orchestrator_bypass: true,
            timeout_multipliers: TimeoutMultipliers::default(),
        }
    }
}

/// Timeout multipliers for different work phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutMultipliers {
    pub prompt_to_spec: f64,
    pub spec_to_full_spec: f64,
    pub full_spec_to_plan: f64,
    pub plan_to_artifacts: f64,
}

impl Default for TimeoutMultipliers {
    fn default() -> Self {
        Self {
            prompt_to_spec: 0.5,
            spec_to_full_spec: 1.0,
            full_spec_to_plan: 0.5,
            plan_to_artifacts: 2.0,
        }
    }
}

/// Conflict detection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetectionSettings {
    /// Enable conflict detection
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Critical file patterns (glob patterns)
    #[serde(default = "default_critical_paths")]
    pub critical_paths: Vec<String>,

    /// Test isolation (allow test files to not conflict)
    #[serde(default = "default_true")]
    pub test_isolation: bool,
}

impl Default for ConflictDetectionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            critical_paths: default_critical_paths(),
            test_isolation: true,
        }
    }
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Enable notifications
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Notify on save
    #[serde(default = "default_true")]
    pub on_save: bool,

    /// Periodic interval in minutes
    #[serde(default = "default_periodic_interval")]
    pub periodic_interval_minutes: i64,

    /// Session end summary
    #[serde(default = "default_true")]
    pub session_end_summary: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            on_save: true,
            periodic_interval_minutes: 20, // Per user requirement
            session_end_summary: true,
        }
    }
}

/// Cross-process coordination settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossProcessSettings {
    /// Enable cross-process coordination
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Mnemosyne directory for shared state
    #[serde(default = "default_mnemosyne_dir")]
    pub mnemosyne_dir: String,

    /// Poll interval in seconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,

    /// Heartbeat timeout in seconds
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_seconds: i64,
}

impl Default for CrossProcessSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mnemosyne_dir: ".mnemosyne".to_string(),
            poll_interval_seconds: 2,
            heartbeat_timeout_seconds: 30,
        }
    }
}

// Default value helpers
fn default_true() -> bool {
    true
}

fn default_isolated_mode() -> String {
    "isolated".to_string()
}

fn default_critical_paths() -> Vec<String> {
    vec![
        "migrations/**".to_string(),
        "schema/**".to_string(),
        "**/.env".to_string(),
        "**/credentials.json".to_string(),
    ]
}

fn default_periodic_interval() -> i64 {
    20 // Per user requirement
}

fn default_mnemosyne_dir() -> String {
    ".mnemosyne".to_string()
}

fn default_poll_interval() -> u64 {
    2
}

fn default_heartbeat_timeout() -> i64 {
    30
}

impl BranchIsolationConfig {
    /// Load configuration from file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::info!("Config file not found, using defaults: {:?}", path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read config file: {}", e),
            ))
        })?;

        let config: BranchIsolationConfig = toml::from_str(&content)
            .map_err(|e| MnemosyneError::Other(format!("Failed to parse config file: {}", e)))?;

        tracing::info!("Loaded configuration from {:?}", path);
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize config: {}", e)))?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create config directory: {}", e),
                ))
            })?;
        }

        std::fs::write(path, content).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write config file: {}", e),
            ))
        })?;

        tracing::info!("Saved configuration to {:?}", path);
        Ok(())
    }

    /// Convert to BranchCoordinatorConfig
    pub fn to_coordinator_config(&self) -> BranchCoordinatorConfig {
        let default_mode = match self.branch_isolation.default_mode.as_str() {
            "coordinated" => CoordinationMode::Coordinated,
            _ => CoordinationMode::Isolated,
        };

        BranchCoordinatorConfig {
            enable_cross_process: self.cross_process.enabled,
            auto_approve_readonly: self.branch_isolation.auto_approve_readonly,
            default_mode,
            mnemosyne_dir: Some(PathBuf::from(&self.cross_process.mnemosyne_dir)),
        }
    }

    /// Convert to BranchGuardConfig
    pub fn to_guard_config(&self) -> BranchGuardConfig {
        BranchGuardConfig {
            enabled: self.branch_isolation.enabled,
            orchestrator_bypass: self.branch_isolation.orchestrator_bypass,
            auto_approve_readonly: self.branch_isolation.auto_approve_readonly,
            conflict_detection: self.conflict_detection.enabled,
        }
    }

    /// Convert to NotificationConfig
    pub fn to_notification_config(&self) -> NotificationConfig {
        NotificationConfig {
            enabled: self.notifications.enabled,
            notify_on_save: self.notifications.on_save,
            periodic_interval_minutes: self.notifications.periodic_interval_minutes,
            session_end_summary: self.notifications.session_end_summary,
        }
    }

    /// Get default config path for a project
    pub fn default_path() -> PathBuf {
        PathBuf::from(".mnemosyne/config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = BranchIsolationConfig::default();

        assert!(config.branch_isolation.enabled);
        assert_eq!(config.branch_isolation.default_mode, "isolated");
        assert!(config.branch_isolation.auto_approve_readonly);
        assert!(config.branch_isolation.orchestrator_bypass);

        assert!(config.conflict_detection.enabled);
        assert!(!config.conflict_detection.critical_paths.is_empty());

        assert!(config.notifications.enabled);
        assert_eq!(config.notifications.periodic_interval_minutes, 20);

        assert!(config.cross_process.enabled);
        assert_eq!(config.cross_process.poll_interval_seconds, 2);
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = BranchIsolationConfig::default();
        config.save(&config_path).unwrap();

        assert!(config_path.exists());

        let loaded = BranchIsolationConfig::load(&config_path).unwrap();

        assert_eq!(
            loaded.branch_isolation.enabled,
            config.branch_isolation.enabled
        );
        assert_eq!(
            loaded.notifications.periodic_interval_minutes,
            config.notifications.periodic_interval_minutes
        );
    }

    #[test]
    fn test_to_coordinator_config() {
        let config = BranchIsolationConfig::default();
        let coordinator_config = config.to_coordinator_config();

        assert_eq!(coordinator_config.auto_approve_readonly, true);
        assert_eq!(coordinator_config.default_mode, CoordinationMode::Isolated);
    }

    #[test]
    fn test_coordinated_mode_parsing() {
        let mut config = BranchIsolationConfig::default();
        config.branch_isolation.default_mode = "coordinated".to_string();

        let coordinator_config = config.to_coordinator_config();
        assert_eq!(
            coordinator_config.default_mode,
            CoordinationMode::Coordinated
        );
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let config = BranchIsolationConfig::load(Path::new("/nonexistent/config.toml")).unwrap();
        assert!(config.branch_isolation.enabled);
    }
}
