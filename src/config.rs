//! Configuration and secure credential management for Mnemosyne
//!
//! Provides secure storage of API keys using OS-level keychains:
//! - macOS: Keychain
//! - Windows: Credential Manager
//! - Linux: Secret Service (libsecret)

use crate::error::{MnemosyneError, Result};
use keyring::Entry;
use std::env;
use tracing::{debug, info, warn};

/// Service name for keyring storage
const KEYRING_SERVICE: &str = "mnemosyne-memory-system";
const KEYRING_USER: &str = "anthropic-api-key";

/// Configuration manager for Mnemosyne
pub struct ConfigManager {
    keyring_entry: Entry,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self> {
        let keyring_entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| MnemosyneError::Config(
                config::ConfigError::Message(format!("Failed to access keyring: {}", e))
            ))?;

        Ok(Self { keyring_entry })
    }

    /// Get the Anthropic API key from:
    /// 1. Environment variable ANTHROPIC_API_KEY
    /// 2. OS keychain
    /// 3. Prompt user (if interactive)
    pub fn get_api_key(&self) -> Result<String> {
        // Try environment variable first
        if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                debug!("Using API key from ANTHROPIC_API_KEY environment variable");
                return Ok(key);
            }
        }

        // Try keychain
        match self.keyring_entry.get_password() {
            Ok(key) => {
                debug!("Retrieved API key from OS keychain");
                Ok(key)
            }
            Err(keyring::Error::NoEntry) => {
                warn!("No API key found in keychain");
                Err(MnemosyneError::Config(
                    config::ConfigError::Message(
                        "Anthropic API key not found. Set ANTHROPIC_API_KEY environment variable or use 'mnemosyne config set-key'".to_string()
                    )
                ))
            }
            Err(e) => {
                Err(MnemosyneError::Config(
                    config::ConfigError::Message(format!("Failed to retrieve API key: {}", e))
                ))
            }
        }
    }

    /// Store the API key securely in OS keychain
    pub fn set_api_key(&self, key: &str) -> Result<()> {
        if key.is_empty() {
            return Err(MnemosyneError::Config(
                config::ConfigError::Message("API key cannot be empty".to_string())
            ));
        }

        // Validate key format (basic check)
        if !key.starts_with("sk-ant-") {
            warn!("API key doesn't match expected Anthropic format (sk-ant-...)");
        }

        self.keyring_entry
            .set_password(key)
            .map_err(|e| MnemosyneError::Config(
                config::ConfigError::Message(format!("Failed to store API key: {}", e))
            ))?;

        info!("API key securely stored in OS keychain");
        Ok(())
    }

    /// Delete the API key from keychain
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
            Err(e) => {
                Err(MnemosyneError::Config(
                    config::ConfigError::Message(format!("Failed to delete API key: {}", e))
                ))
            }
        }
    }

    /// Check if an API key is configured
    pub fn has_api_key(&self) -> bool {
        env::var("ANTHROPIC_API_KEY").is_ok() || self.keyring_entry.get_password().is_ok()
    }

    /// Interactive prompt to set API key (for CLI use)
    pub fn prompt_and_set_api_key(&self) -> Result<()> {
        println!("\n🔑 Mnemosyne API Key Setup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nMnemosyne uses Claude Haiku for memory intelligence.");
        println!("You need an Anthropic API key to use this feature.\n");
        println!("Get your API key from: https://console.anthropic.com/settings/keys\n");
        println!("The key will be securely stored in your OS keychain.");
        println!("You can also set the ANTHROPIC_API_KEY environment variable.\n");

        print!("Enter your Anthropic API key (starts with sk-ant-): ");
        std::io::Write::flush(&mut std::io::stdout())
            .map_err(|e| MnemosyneError::Io(e))?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| MnemosyneError::Io(e))?;

        let key = input.trim();

        if key.is_empty() {
            return Err(MnemosyneError::Config(
                config::ConfigError::Message("No API key provided".to_string())
            ));
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

    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_set_and_get_api_key() {
        let manager = ConfigManager::new().unwrap();

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
    fn test_env_var_takes_precedence() {
        // Clean up first to avoid interference from other tests
        env::remove_var("ANTHROPIC_API_KEY");

        let manager = ConfigManager::new().unwrap();
        let _ = manager.delete_api_key(); // Clean keychain too

        // Set both env var and keychain
        env::set_var("ANTHROPIC_API_KEY", "env-key");
        let _ = manager.set_api_key("keychain-key");

        // Env var should win
        let retrieved = manager.get_api_key().unwrap();
        assert_eq!(retrieved, "env-key");

        // Clean up
        env::remove_var("ANTHROPIC_API_KEY");
        let _ = manager.delete_api_key();
    }

    #[test]
    fn test_has_api_key() {
        let manager = ConfigManager::new().unwrap();
        let _ = manager.delete_api_key();
        env::remove_var("ANTHROPIC_API_KEY");

        // Should be false initially
        assert!(!manager.has_api_key());

        // Set key
        manager.set_api_key("sk-ant-test").unwrap();
        assert!(manager.has_api_key());

        // Clean up
        manager.delete_api_key().unwrap();
    }
}
