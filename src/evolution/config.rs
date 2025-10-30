// Evolution Configuration
//
// Defines configuration for background evolution jobs including
// scheduling intervals, batch sizes, and job-specific settings.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

/// Main evolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// Enable/disable all evolution jobs
    pub enabled: bool,

    /// Consolidation job configuration
    pub consolidation: JobConfig,

    /// Consolidation-specific settings (optional for backward compatibility)
    #[serde(default)]
    pub consolidation_config: ConsolidationConfig,

    /// Importance recalibration job configuration
    pub importance: JobConfig,

    /// Link decay job configuration
    pub link_decay: JobConfig,

    /// Archival job configuration
    pub archival: JobConfig,
}

/// Configuration for individual evolution jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Enable/disable this specific job
    pub enabled: bool,

    /// Interval between job runs (in seconds)
    #[serde(with = "serde_duration")]
    pub interval: Duration,

    /// Maximum number of memories to process per run
    pub batch_size: usize,

    /// Maximum duration for job execution (in seconds)
    #[serde(with = "serde_duration")]
    pub max_duration: Duration,
}

/// Decision mode for consolidation job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DecisionMode {
    /// Use heuristics only (fast, free, less accurate)
    #[default]
    Heuristic,

    /// Use LLM for all decisions (slow, costs money, most accurate)
    LlmAlways,

    /// Use LLM selectively based on similarity range
    LlmSelective {
        /// Only use LLM for similarity in this range
        llm_range: (f32, f32), // e.g., (0.80, 0.95)

        /// Use heuristics outside range
        heuristic_fallback: bool,
    },

    /// Try LLM, fall back to heuristics on error
    LlmWithFallback,
}

/// Consolidation-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    /// Decision mode for consolidation
    pub decision_mode: DecisionMode,

    /// Maximum cost per consolidation run (in USD)
    pub max_cost_per_run_usd: f32,
}


impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            decision_mode: DecisionMode::Heuristic,
            max_cost_per_run_usd: 0.50, // Default budget: 50 cents per run
        }
    }
}

// Custom serde module for Duration (serialize/deserialize as seconds)
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            consolidation: JobConfig {
                enabled: true,
                interval: Duration::from_secs(86400), // 24 hours
                batch_size: 100,
                max_duration: Duration::from_secs(300), // 5 minutes
            },
            consolidation_config: ConsolidationConfig::default(),
            importance: JobConfig {
                enabled: true,
                interval: Duration::from_secs(604800), // 7 days
                batch_size: 1000,
                max_duration: Duration::from_secs(300), // 5 minutes
            },
            link_decay: JobConfig {
                enabled: true,
                interval: Duration::from_secs(604800), // 7 days
                batch_size: 1000,
                max_duration: Duration::from_secs(300), // 5 minutes
            },
            archival: JobConfig {
                enabled: true,
                interval: Duration::from_secs(2592000), // 30 days
                batch_size: 500,
                max_duration: Duration::from_secs(300), // 5 minutes
            },
        }
    }
}

impl EvolutionConfig {
    /// Load configuration from TOML file
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: EvolutionConfig = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let config: EvolutionConfig = toml::from_str(toml_str)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate consolidation config
        self.validate_job_config("consolidation", &self.consolidation)?;

        // Validate importance config
        self.validate_job_config("importance", &self.importance)?;

        // Validate link_decay config
        self.validate_job_config("link_decay", &self.link_decay)?;

        // Validate archival config
        self.validate_job_config("archival", &self.archival)?;

        Ok(())
    }

    fn validate_job_config(&self, name: &str, config: &JobConfig) -> Result<(), ConfigError> {
        // Interval must be at least 1 hour
        if config.interval < Duration::from_secs(3600) {
            return Err(ConfigError::ValidationError(format!(
                "{}: interval must be at least 1 hour",
                name
            )));
        }

        // Batch size must be reasonable (1-10000)
        if config.batch_size == 0 || config.batch_size > 10000 {
            return Err(ConfigError::ValidationError(format!(
                "{}: batch_size must be between 1 and 10000",
                name
            )));
        }

        // Max duration must be at least 1 minute and at most 30 minutes
        if config.max_duration < Duration::from_secs(60)
            || config.max_duration > Duration::from_secs(1800)
        {
            return Err(ConfigError::ValidationError(format!(
                "{}: max_duration must be between 1 and 30 minutes",
                name
            )));
        }

        Ok(())
    }

    /// Save configuration to TOML file
    pub fn to_file(&self, path: &Path) -> Result<(), ConfigError> {
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_default_config_is_valid() {
        let config = EvolutionConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_interval_too_short() {
        let mut config = EvolutionConfig::default();
        config.consolidation.interval = Duration::from_secs(60); // 1 minute (too short)

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interval must be at least 1 hour"));
    }

    #[test]
    fn test_validate_batch_size_zero() {
        let mut config = EvolutionConfig::default();
        config.importance.batch_size = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("batch_size must be between"));
    }

    #[test]
    fn test_validate_batch_size_too_large() {
        let mut config = EvolutionConfig::default();
        config.link_decay.batch_size = 20000;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("batch_size must be between"));
    }

    #[test]
    fn test_validate_max_duration_too_short() {
        let mut config = EvolutionConfig::default();
        config.archival.max_duration = Duration::from_secs(30); // 30 seconds (too short)

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_duration must be between"));
    }

    #[test]
    fn test_validate_max_duration_too_long() {
        let mut config = EvolutionConfig::default();
        config.consolidation.max_duration = Duration::from_secs(3600); // 1 hour (too long)

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("max_duration must be between"));
    }

    #[test]
    fn test_from_toml() {
        let toml_str = r#"
            enabled = true

            [consolidation]
            enabled = true
            interval = 86400
            batch_size = 100
            max_duration = 300

            [importance]
            enabled = true
            interval = 604800
            batch_size = 1000
            max_duration = 300

            [link_decay]
            enabled = true
            interval = 604800
            batch_size = 1000
            max_duration = 300

            [archival]
            enabled = false
            interval = 2592000
            batch_size = 500
            max_duration = 300
        "#;

        let config = EvolutionConfig::from_toml(toml_str).unwrap();
        assert!(config.enabled);
        assert!(config.consolidation.enabled);
        assert!(!config.archival.enabled);
        assert_eq!(config.importance.batch_size, 1000);
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = EvolutionConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: EvolutionConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(
            config.consolidation.batch_size,
            deserialized.consolidation.batch_size
        );
    }
}
