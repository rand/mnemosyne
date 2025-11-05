//! Configuration and secure credential management for Mnemosyne
//!
//! Provides secure storage of API keys using multiple methods:
//! 1. Environment variables (highest priority)
//! 2. Age-encrypted config file (main method)
//! 3. OS-level keychains (fallback with keyring-fallback feature):
//!    - macOS: Keychain
//!    - Windows: Credential Manager
//!    - Linux: Secret Service (libsecret)

use crate::error::{MnemosyneError, Result};
use crate::secrets::SecretsManager;
#[cfg(feature = "keyring-fallback")]
use keyring::Entry;
use secrecy::ExposeSecret;
use std::env;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Configuration for local embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Enable or disable embeddings globally
    pub enabled: bool,

    /// Model to use for embeddings
    /// Options: "nomic-embed-text-v1.5", "all-MiniLM-L6-v2", "bge-small-en-v1.5"
    pub model: String,

    /// Device to run on ("cpu" or "cuda")
    pub device: String,

    /// Batch size for embedding generation
    pub batch_size: usize,

    /// Cache directory for downloaded models
    pub cache_dir: PathBuf,

    /// Show download progress for models
    pub show_download_progress: bool,
}

/// Configuration for hybrid search scoring weights
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Weight for vector similarity (0.0-1.0)
    pub vector_weight: f32,

    /// Weight for keyword matching (0.0-1.0)
    pub keyword_weight: f32,

    /// Weight for graph connections (0.0-1.0)
    pub graph_weight: f32,

    /// Weight for importance score (0.0-1.0)
    pub importance_weight: f32,

    /// Weight for recency (0.0-1.0)
    pub recency_weight: f32,

    /// Enable vector search (requires embedding service)
    pub enable_vector_search: bool,

    /// Enable graph expansion in hybrid search
    pub enable_graph_expansion: bool,

    /// Maximum graph traversal depth
    pub max_graph_depth: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            // Balanced hybrid search weights (sum to 1.0)
            vector_weight: 0.35,     // Vector similarity is primary
            keyword_weight: 0.30,    // Keyword matching is secondary
            graph_weight: 0.20,      // Graph connections for context
            importance_weight: 0.10, // Importance as tie-breaker
            recency_weight: 0.05,    // Slight recency bias

            enable_vector_search: true,
            enable_graph_expansion: true,
            max_graph_depth: 2,
        }
    }
}

impl SearchConfig {
    /// Validate search configuration
    pub fn validate(&self) -> Result<()> {
        // Check weight ranges
        let weights = [
            ("vector_weight", self.vector_weight),
            ("keyword_weight", self.keyword_weight),
            ("graph_weight", self.graph_weight),
            ("importance_weight", self.importance_weight),
            ("recency_weight", self.recency_weight),
        ];

        for (name, weight) in &weights {
            if *weight < 0.0 || *weight > 1.0 {
                return Err(MnemosyneError::Config(config::ConfigError::Message(
                    format!("{} must be between 0.0 and 1.0, got {}", name, weight),
                )));
            }
        }

        // Warn if weights don't sum to 1.0 (but allow it)
        let sum: f32 = weights.iter().map(|(_, w)| w).sum();
        if (sum - 1.0).abs() > 0.01 {
            warn!(
                "Search weights sum to {}, not 1.0. Results may be scaled unexpectedly.",
                sum
            );
        }

        // Check graph depth
        if self.max_graph_depth == 0 {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                "max_graph_depth must be at least 1".to_string(),
            )));
        }

        Ok(())
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        // Use default HuggingFace cache directory
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache")
            .join("mnemosyne")
            .join("models");

        Self {
            enabled: true,
            model: "nomic-embed-text-v1.5".to_string(),
            device: "cpu".to_string(),
            batch_size: 32,
            cache_dir,
            show_download_progress: true,
        }
    }
}

impl EmbeddingConfig {
    /// Model download strategy:
    /// - Models are automatically downloaded by fastembed on first use
    /// - Downloaded to `cache_dir` (defaults to ~/.cache/mnemosyne/models/)
    /// - Progress displayed if `show_download_progress` is true
    /// - Subsequent runs use cached models (no re-download)
    /// - Models are typically 90-200MB depending on selection
    ///
    /// Supported models and sizes:
    /// - nomic-embed-text-v1.5: ~140MB, 768 dimensions (recommended)
    /// - all-MiniLM-L6-v2: ~90MB, 384 dimensions (fastest)
    /// - bge-small-en-v1.5: ~130MB, 384 dimensions
    /// - bge-base-en-v1.5: ~440MB, 768 dimensions
    /// - bge-large-en-v1.5: ~1.3GB, 1024 dimensions
    ///
    /// Get the embedding dimensions for the configured model
    pub fn dimensions(&self) -> usize {
        match self.model.as_str() {
            "nomic-embed-text-v1.5" | "nomic-embed-text-v1" => 768,
            "all-MiniLM-L6-v2" => 384,
            "all-MiniLM-L12-v2" => 384,
            "bge-small-en-v1.5" => 384,
            "bge-base-en-v1.5" => 768,
            "bge-large-en-v1.5" => 1024,
            _ => {
                warn!(
                    "Unknown model '{}', defaulting to 768 dimensions",
                    self.model
                );
                768
            }
        }
    }

    /// Check if GPU (CUDA) is requested
    pub fn use_gpu(&self) -> bool {
        self.device == "cuda"
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Check if model is supported
        let supported_models = ["nomic-embed-text-v1.5",
            "nomic-embed-text-v1",
            "all-MiniLM-L6-v2",
            "all-MiniLM-L12-v2",
            "bge-small-en-v1.5",
            "bge-base-en-v1.5",
            "bge-large-en-v1.5"];

        if !supported_models.contains(&self.model.as_str()) {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                format!(
                    "Unsupported embedding model: '{}'. Supported models: {}",
                    self.model,
                    supported_models.join(", ")
                ),
            )));
        }

        // Check device
        if self.device != "cpu" && self.device != "cuda" {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                format!("Invalid device '{}'. Must be 'cpu' or 'cuda'", self.device),
            )));
        }

        // Check batch size
        if self.batch_size == 0 || self.batch_size > 1000 {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                format!(
                    "Batch size {} out of range. Must be between 1 and 1000",
                    self.batch_size
                ),
            )));
        }

        Ok(())
    }
}

/// Service name for keyring storage
const KEYRING_SERVICE: &str = "mnemosyne-memory-system";
#[cfg(test)]
const KEYRING_SERVICE_TEST: &str = "mnemosyne-memory-system-test";
const KEYRING_USER: &str = "anthropic-api-key";

/// Configuration manager for Mnemosyne
pub struct ConfigManager {
    secrets: SecretsManager,
    #[cfg(feature = "keyring-fallback")]
    keyring_entry: Entry,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self> {
        Self::new_with_service(KEYRING_SERVICE)
    }

    /// Create a new configuration manager for testing (uses separate keyring entry and temp secrets)
    #[cfg(test)]
    pub fn new_for_testing() -> Result<Self> {
        // Use a temporary directory for test secrets (isolated from real user config)
        let test_config_dir = std::env::temp_dir().join("mnemosyne-test-secrets");
        let secrets = SecretsManager::new_with_config_dir(test_config_dir).map_err(|e| {
            MnemosyneError::Config(config::ConfigError::Message(format!(
                "Failed to initialize test secrets manager: {}",
                e
            )))
        })?;

        #[cfg(feature = "keyring-fallback")]
        let keyring_entry = Entry::new(KEYRING_SERVICE_TEST, KEYRING_USER).map_err(|e| {
            MnemosyneError::Config(config::ConfigError::Message(format!(
                "Failed to access keyring: {}",
                e
            )))
        })?;

        Ok(Self {
            secrets,
            #[cfg(feature = "keyring-fallback")]
            keyring_entry,
        })
    }

    /// Create a new configuration manager with custom service name
    fn new_with_service(service_name: &str) -> Result<Self> {
        let secrets = SecretsManager::new().map_err(|e| {
            MnemosyneError::Config(config::ConfigError::Message(format!(
                "Failed to initialize secrets manager: {}",
                e
            )))
        })?;

        #[cfg(feature = "keyring-fallback")]
        let keyring_entry = Entry::new(service_name, KEYRING_USER).map_err(|e| {
            MnemosyneError::Config(config::ConfigError::Message(format!(
                "Failed to access keyring: {}",
                e
            )))
        })?;

        Ok(Self {
            secrets,
            #[cfg(feature = "keyring-fallback")]
            keyring_entry,
        })
    }

    /// Get the Anthropic API key from:
    /// 1. Environment variable ANTHROPIC_API_KEY
    /// 2. Age-encrypted config file
    /// 3. OS keychain (fallback with keyring-fallback feature)
    pub fn get_api_key(&self) -> Result<String> {
        // Try SecretsManager (checks env var, then encrypted file)
        match self.secrets.get_secret("ANTHROPIC_API_KEY") {
            Ok(secret) => {
                debug!("Retrieved API key from secrets manager");
                return Ok(secret.expose_secret().to_string());
            }
            Err(e) => {
                debug!("SecretsManager couldn't retrieve key: {}", e);
            }
        }

        // Fallback to keychain for backward compatibility
        #[cfg(feature = "keyring-fallback")]
        {
            match self.keyring_entry.get_password() {
                Ok(key) => {
                    debug!("Retrieved API key from OS keychain (fallback)");
                    return Ok(key);
                }
                Err(keyring::Error::NoEntry) => {
                    debug!("No API key found in keychain");
                }
                Err(e) => {
                    warn!("Keychain error: {}", e);
                }
            }
        }

        // No key found anywhere
        Err(MnemosyneError::Config(config::ConfigError::Message(
            "ANTHROPIC_API_KEY not found. Options:\n\
                 1. export ANTHROPIC_API_KEY=sk-ant-...\n\
                 2. mnemosyne secrets set ANTHROPIC_API_KEY\n\
                 3. mnemosyne secrets init (first-time setup)"
                .to_string(),
        )))
    }

    /// Store the API key securely (keyring only, use `mnemosyne secrets set` for encrypted config)
    #[cfg(feature = "keyring-fallback")]
    pub fn set_api_key(&self, key: &str) -> Result<()> {
        if key.is_empty() {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                "API key cannot be empty".to_string(),
            )));
        }

        // Validate key format (basic check)
        if !key.starts_with("sk-ant-") {
            warn!("API key doesn't match expected Anthropic format (sk-ant-...)");
        }

        debug!(
            "Attempting to store API key in keychain (service={}, user={})",
            KEYRING_SERVICE, KEYRING_USER
        );

        self.keyring_entry.set_password(key).map_err(|e| {
            MnemosyneError::Config(config::ConfigError::Message(format!(
                "Failed to store API key: {}",
                e
            )))
        })?;

        info!("API key securely stored in OS keychain");

        // Immediately verify storage
        match self.keyring_entry.get_password() {
            Ok(_) => debug!("Verified: API key successfully stored and retrievable"),
            Err(e) => {
                warn!(
                    "WARNING: API key was stored but immediate retrieval failed: {}",
                    e
                );
                warn!("This may indicate a keychain access permission issue");
            }
        }

        Ok(())
    }

    /// Delete the API key from keychain (keyring only)
    #[cfg(feature = "keyring-fallback")]
    pub fn delete_api_key(&self) -> Result<()> {
        match self.keyring_entry.delete_credential() {
            Ok(_) => {
                info!("API key deleted from OS keychain");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                warn!("No API key found to delete");
                Ok(())
            }
            Err(e) => Err(MnemosyneError::Config(config::ConfigError::Message(
                format!("Failed to delete API key: {}", e),
            ))),
        }
    }

    /// Get the secrets manager reference
    pub fn secrets(&self) -> &SecretsManager {
        &self.secrets
    }

    /// Check if an API key is configured
    pub fn has_api_key(&self) -> bool {
        // Check environment variable
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            return true;
        }

        // Check secrets manager
        if self.secrets.get_secret("ANTHROPIC_API_KEY").is_ok() {
            return true;
        }

        // Check keychain fallback
        #[cfg(feature = "keyring-fallback")]
        if self.keyring_entry.get_password().is_ok() {
            return true;
        }

        false
    }

    /// Interactive prompt to set API key (for CLI use)
    #[cfg(feature = "keyring-fallback")]
    pub fn prompt_and_set_api_key(&self) -> Result<()> {
        println!("\n🔑 Mnemosyne API Key Setup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nMnemosyne uses Claude Haiku for memory intelligence.");
        println!("You need an Anthropic API key to use this feature.\n");
        println!("Get your API key from: https://console.anthropic.com/settings/keys\n");
        println!("The key will be securely stored in your OS keychain.");
        println!("You can also set the ANTHROPIC_API_KEY environment variable.\n");

        print!("Enter your Anthropic API key (starts with sk-ant-): ");
        std::io::Write::flush(&mut std::io::stdout()).map_err(MnemosyneError::Io)?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(MnemosyneError::Io)?;

        let key = input.trim();

        if key.is_empty() {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                "No API key provided".to_string(),
            )));
        }

        self.set_api_key(key)?;
        println!("✓ API key securely saved!\n");

        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize config manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    #[serial]
    #[cfg(feature = "keyring-fallback")]
    fn test_set_and_get_api_key() {
        let manager = ConfigManager::new_for_testing().unwrap();

        // Clean up first
        let _ = manager.delete_api_key();

        // Set a test key
        let test_key = "sk-ant-test-key-123456789";
        manager.set_api_key(test_key).unwrap();

        // Verify it was stored (without env var)
        env::remove_var("ANTHROPIC_API_KEY");
        let retrieved = manager.get_api_key().unwrap();
        assert_eq!(retrieved, test_key);

        // Clean up
        manager.delete_api_key().unwrap();
    }

    #[test]
    #[serial]
    fn test_env_var_takes_precedence() {
        // Clean up first to avoid interference from other tests
        env::remove_var("ANTHROPIC_API_KEY");

        let manager = ConfigManager::new_for_testing().unwrap();

        #[cfg(feature = "keyring-fallback")]
        {
            let _ = manager.delete_api_key(); // Clean keychain too
            let _ = manager.set_api_key("keychain-key");
        }

        // Set env var
        env::set_var("ANTHROPIC_API_KEY", "env-key");

        // Env var should win
        let retrieved = manager.get_api_key().unwrap();
        assert_eq!(retrieved, "env-key");

        // Clean up
        env::remove_var("ANTHROPIC_API_KEY");

        #[cfg(feature = "keyring-fallback")]
        {
            let _ = manager.delete_api_key();
        }
    }

    #[test]
    #[serial]
    fn test_has_api_key() {
        env::remove_var("ANTHROPIC_API_KEY");

        let manager = ConfigManager::new_for_testing().unwrap();

        #[cfg(feature = "keyring-fallback")]
        {
            let _ = manager.delete_api_key();

            // Should be false initially
            assert!(!manager.has_api_key());

            // Set key in keychain
            manager.set_api_key("sk-ant-test").unwrap();
            assert!(manager.has_api_key());

            // Clean up
            manager.delete_api_key().unwrap();
        }

        // Test with environment variable
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test");
        assert!(manager.has_api_key());
        env::remove_var("ANTHROPIC_API_KEY");
    }

    // EmbeddingConfig tests
    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.model, "nomic-embed-text-v1.5");
        assert_eq!(config.device, "cpu");
        assert_eq!(config.batch_size, 32);
        assert!(config.show_download_progress);
    }

    #[test]
    fn test_embedding_dimensions() {
        let mut config = EmbeddingConfig::default();

        config.model = "nomic-embed-text-v1.5".to_string();
        assert_eq!(config.dimensions(), 768);

        config.model = "all-MiniLM-L6-v2".to_string();
        assert_eq!(config.dimensions(), 384);

        config.model = "bge-base-en-v1.5".to_string();
        assert_eq!(config.dimensions(), 768);

        config.model = "bge-large-en-v1.5".to_string();
        assert_eq!(config.dimensions(), 1024);

        // Unknown model defaults to 768
        config.model = "unknown-model".to_string();
        assert_eq!(config.dimensions(), 768);
    }

    #[test]
    fn test_embedding_use_gpu() {
        let mut config = EmbeddingConfig::default();

        config.device = "cpu".to_string();
        assert!(!config.use_gpu());

        config.device = "cuda".to_string();
        assert!(config.use_gpu());
    }

    #[test]
    fn test_embedding_config_validation() {
        let mut config = EmbeddingConfig::default();

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid model
        config.model = "invalid-model".to_string();
        assert!(config.validate().is_err());
        config.model = "nomic-embed-text-v1.5".to_string();

        // Invalid device
        config.device = "gpu".to_string();
        assert!(config.validate().is_err());
        config.device = "cpu".to_string();

        // Invalid batch size (too small)
        config.batch_size = 0;
        assert!(config.validate().is_err());

        // Invalid batch size (too large)
        config.batch_size = 1001;
        assert!(config.validate().is_err());

        // Valid batch size
        config.batch_size = 32;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_embedding_config_all_supported_models() {
        let supported_models = vec![
            "nomic-embed-text-v1.5",
            "nomic-embed-text-v1",
            "all-MiniLM-L6-v2",
            "all-MiniLM-L12-v2",
            "bge-small-en-v1.5",
            "bge-base-en-v1.5",
            "bge-large-en-v1.5",
        ];

        for model in supported_models {
            let mut config = EmbeddingConfig::default();
            config.model = model.to_string();
            assert!(config.validate().is_ok(), "Model {} should be valid", model);
        }
    }
}
