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
use tracing::{debug, info, warn};

/// Service name for keyring storage
const KEYRING_SERVICE: &str = "mnemosyne-memory-system";
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
        let secrets = SecretsManager::new()
            .map_err(|e| MnemosyneError::Config(
                config::ConfigError::Message(format!("Failed to initialize secrets manager: {}", e))
            ))?;

        #[cfg(feature = "keyring-fallback")]
        let keyring_entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| MnemosyneError::Config(
                config::ConfigError::Message(format!("Failed to access keyring: {}", e))
            ))?;

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
        Err(MnemosyneError::Config(
            config::ConfigError::Message(
                "ANTHROPIC_API_KEY not found. Options:\n\
                 1. export ANTHROPIC_API_KEY=sk-ant-...\n\
                 2. mnemosyne secrets set ANTHROPIC_API_KEY\n\
                 3. mnemosyne secrets init (first-time setup)".to_string()
            )
        ))
    }

    /// Store the API key securely (keyring only, use `mnemosyne secrets set` for encrypted config)
    #[cfg(feature = "keyring-fallback")]
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

        debug!("Attempting to store API key in keychain (service={}, user={})", KEYRING_SERVICE, KEYRING_USER);

        self.keyring_entry
            .set_password(key)
            .map_err(|e| MnemosyneError::Config(
                config::ConfigError::Message(format!("Failed to store API key: {}", e))
            ))?;

        info!("API key securely stored in OS keychain");

        // Immediately verify storage
        match self.keyring_entry.get_password() {
            Ok(_) => debug!("Verified: API key successfully stored and retrievable"),
            Err(e) => {
                warn!("WARNING: API key was stored but immediate retrieval failed: {}", e);
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
            Err(e) => {
                Err(MnemosyneError::Config(
                    config::ConfigError::Message(format!("Failed to delete API key: {}", e))
                ))
            }
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
    #[serial]
    fn test_env_var_takes_precedence() {
        // Clean up first to avoid interference from other tests
        env::remove_var("ANTHROPIC_API_KEY");

        let manager = ConfigManager::new().unwrap();

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

        let manager = ConfigManager::new().unwrap();

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
}
