//! Built-in encrypted secrets management using age encryption
//!
//! Provides zero-dependency secrets management with age encryption.
//! Secrets stored in ~/.config/mnemosyne/secrets.age
//!
//! Priority order:
//! 1. Environment variables
//! 2. Encrypted config file (~/.config/mnemosyne/secrets.age)
//! 3. OS Keychain (fallback, requires keyring-fallback feature)

use age::{
    armor::{ArmoredReader, ArmoredWriter, Format},
    Decryptor, Encryptor,
};
use anyhow::{Context, Result};
use directories::ProjectDirs;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{debug, info};

/// Required secrets (always needed)
pub const REQUIRED_SECRETS: &[&str] = &["ANTHROPIC_API_KEY"];

/// Optional secrets (for Turso Cloud deployments)
pub const OPTIONAL_SECRETS: &[&str] = &["TURSO_AUTH_TOKEN"];

/// Secrets manager with age encryption
pub struct SecretsManager {
    config_dir: PathBuf,
    identity_file: PathBuf,
    secrets_file: PathBuf,
}

impl SecretsManager {
    /// Initialize secrets manager with standard config directory
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "mnemosyne", "mnemosyne")
            .context("Failed to determine config directory")?;

        let config_dir = dirs.config_dir().to_path_buf();
        Self::new_with_config_dir(config_dir)
    }

    /// Initialize secrets manager with custom config directory (for testing)
    pub fn new_with_config_dir(config_dir: PathBuf) -> Result<Self> {
        let identity_file = config_dir.join("identity.key");
        let secrets_file = config_dir.join("secrets.age");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;

        debug!(
            "Secrets manager initialized (config_dir: {})",
            config_dir.display()
        );

        Ok(Self {
            config_dir,
            identity_file,
            secrets_file,
        })
    }

    /// Check if secrets are initialized
    pub fn is_initialized(&self) -> bool {
        self.identity_file.exists() && self.secrets_file.exists()
    }

    /// Initialize on first run (generates keypair, prompts for secrets)
    pub fn initialize_interactive(&self) -> Result<()> {
        println!("\n🔐 Mnemosyne Secrets Setup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("\nGenerating encryption key...");

        // Generate age keypair
        let key = age::x25519::Identity::generate();
        let recipient = key.to_public();

        // Save identity (private key) with secure permissions
        let key_str = key.to_string();
        fs::write(&self.identity_file, key_str.expose_secret())
            .context("Failed to write identity file")?;

        // Set file permissions to 0600 (owner read/write only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.identity_file)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.identity_file, perms)?;
        }

        info!("Encryption key generated and saved");
        println!(
            "{} Encryption key saved to: {}",
            crate::icons::status::success(),
            self.identity_file.display()
        );

        // Prompt for required secrets
        let mut secrets = HashMap::new();

        for secret_name in REQUIRED_SECRETS {
            println!("\n{} (required)", secret_name);
            if *secret_name == "ANTHROPIC_API_KEY" {
                println!("Get your key from: https://console.anthropic.com/settings/keys");
            }
            print!("Enter value: ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let value = input.trim();

            if value.is_empty() {
                anyhow::bail!("Required secret '{}' cannot be empty", secret_name);
            }

            secrets.insert(secret_name.to_string(), value.to_string());
        }

        // Prompt for optional secrets
        println!("\n📦 Optional Secrets (for Turso Cloud)");
        println!("Press Enter to skip if not using Turso Cloud\n");

        for secret_name in OPTIONAL_SECRETS {
            print!("{} (optional): ", secret_name);
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let value = input.trim();

            if !value.is_empty() {
                secrets.insert(secret_name.to_string(), value.to_string());
                info!("Optional secret '{}' configured", secret_name);
            }
        }

        // Encrypt and save secrets
        self.save_secrets(&secrets, &recipient)?;

        println!(
            "\n{} Secrets encrypted and saved!",
            crate::icons::status::success()
        );
        println!(
            "{} Location: {}",
            crate::icons::status::success(),
            self.secrets_file.display()
        );
        println!("\nYou can update secrets anytime with: mnemosyne secrets set <KEY>");

        info!("Secrets initialization complete");
        Ok(())
    }

    /// Get a secret by name (decrypts on-demand)
    pub fn get_secret(&self, name: &str) -> Result<SecretString> {
        // 1. Check environment variable first (highest priority)
        if let Ok(val) = std::env::var(name) {
            if !val.is_empty() {
                debug!("Retrieved secret '{}' from environment variable", name);
                return Ok(SecretString::new(val.into()));
            }
        }

        // 2. Decrypt from age file
        if self.secrets_file.exists() {
            let secrets = self.load_secrets()?;
            if let Some(value) = secrets.get(name) {
                debug!("Retrieved secret '{}' from encrypted config", name);
                return Ok(SecretString::new(value.clone().into()));
            }
        }

        anyhow::bail!(
            "Secret '{}' not found. Set it with: mnemosyne secrets set {}",
            name,
            name
        )
    }

    /// Set a secret (encrypt and save)
    pub fn set_secret(&self, name: &str, value: &str) -> Result<()> {
        if value.is_empty() {
            anyhow::bail!("Secret value cannot be empty");
        }

        // Ensure initialized
        if !self.identity_file.exists() {
            anyhow::bail!("Secrets not initialized. Run: mnemosyne secrets init");
        }

        let mut secrets = if self.secrets_file.exists() {
            self.load_secrets()?
        } else {
            HashMap::new()
        };

        secrets.insert(name.to_string(), value.to_string());

        // Load recipient from identity file
        let identity_str =
            fs::read_to_string(&self.identity_file).context("Failed to read identity file")?;
        let identity = identity_str
            .parse::<age::x25519::Identity>()
            .map_err(|e| anyhow::anyhow!("Failed to parse identity: {}", e))?;
        let recipient = identity.to_public();

        self.save_secrets(&secrets, &recipient)?;

        info!("Secret '{}' updated", name);
        println!(
            "{} Secret '{}' updated",
            crate::icons::status::success(),
            name
        );

        Ok(())
    }

    /// Load and decrypt secrets
    fn load_secrets(&self) -> Result<HashMap<String, String>> {
        let identity_str =
            fs::read_to_string(&self.identity_file).context("Failed to read identity file")?;
        let identity = identity_str
            .parse::<age::x25519::Identity>()
            .map_err(|e| anyhow::anyhow!("Failed to parse identity: {}", e))?;

        let encrypted = fs::read(&self.secrets_file).context("Failed to read secrets file")?;

        let decryptor = Decryptor::new(ArmoredReader::new(&encrypted[..]))
            .map_err(|e| anyhow::anyhow!("Failed to create decryptor: {}", e))?;

        let mut decrypted = vec![];
        let mut reader = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .context("Failed to decrypt secrets (wrong key?)")?;
        reader
            .read_to_end(&mut decrypted)
            .context("Failed to read decrypted data")?;

        let secrets_str =
            String::from_utf8(decrypted).context("Decrypted data is not valid UTF-8")?;
        let secrets: HashMap<String, String> =
            serde_json::from_str(&secrets_str).context("Failed to parse secrets JSON")?;

        debug!("Loaded {} secrets from encrypted file", secrets.len());
        Ok(secrets)
    }

    /// Encrypt and save secrets
    fn save_secrets(
        &self,
        secrets: &HashMap<String, String>,
        recipient: &age::x25519::Recipient,
    ) -> Result<()> {
        let secrets_json =
            serde_json::to_string_pretty(secrets).context("Failed to serialize secrets")?;

        let recipient_box: Box<dyn age::Recipient + Send> = Box::new(recipient.clone());
        let encryptor =
            Encryptor::with_recipients(std::iter::once(&*recipient_box as &dyn age::Recipient))
                .context("Failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor
            .wrap_output(
                ArmoredWriter::wrap_output(&mut encrypted, Format::AsciiArmor)
                    .context("Failed to create armored writer")?,
            )
            .context("Failed to wrap encryptor")?;

        writer
            .write_all(secrets_json.as_bytes())
            .context("Failed to write encrypted data")?;
        writer.finish().and_then(|armor| armor.finish())?;

        fs::write(&self.secrets_file, encrypted).context("Failed to write secrets file")?;

        // Set file permissions to 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.secrets_file)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.secrets_file, perms)?;
        }

        debug!("Saved {} secrets to encrypted file", secrets.len());
        Ok(())
    }

    /// List configured secrets (shows names only, not values)
    pub fn list_secrets(&self) -> Result<Vec<String>> {
        if !self.secrets_file.exists() {
            return Ok(vec![]);
        }

        let secrets = self.load_secrets()?;
        let mut names: Vec<String> = secrets.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Get config directory path
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    /// Get identity file path
    pub fn identity_file(&self) -> &PathBuf {
        &self.identity_file
    }

    /// Get secrets file path
    pub fn secrets_file(&self) -> &PathBuf {
        &self.secrets_file
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize secrets manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (SecretsManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().to_path_buf();

        let manager = SecretsManager {
            config_dir: config_dir.clone(),
            identity_file: config_dir.join("identity.key"),
            secrets_file: config_dir.join("secrets.age"),
        };

        (manager, temp_dir)
    }

    #[test]
    fn test_is_not_initialized_initially() {
        let (manager, _temp) = create_test_manager();
        assert!(!manager.is_initialized());
    }

    #[test]
    fn test_set_and_get_secret() {
        let (manager, _temp) = create_test_manager();

        // Generate and save identity
        let key = age::x25519::Identity::generate();
        fs::write(&manager.identity_file, key.to_string().expose_secret()).unwrap();

        // Set a secret
        manager.set_secret("TEST_KEY", "test_value").unwrap();

        // Get the secret
        let retrieved = manager.get_secret("TEST_KEY").unwrap();
        assert_eq!(retrieved.expose_secret(), "test_value");
    }

    #[test]
    fn test_environment_variable_takes_precedence() {
        let (manager, _temp) = create_test_manager();

        // Generate and save identity
        let key = age::x25519::Identity::generate();
        fs::write(&manager.identity_file, key.to_string().expose_secret()).unwrap();

        // Set a secret in file
        manager.set_secret("TEST_KEY", "file_value").unwrap();

        // Set environment variable
        std::env::set_var("TEST_KEY", "env_value");

        // Environment variable should win
        let retrieved = manager.get_secret("TEST_KEY").unwrap();
        assert_eq!(retrieved.expose_secret(), "env_value");

        // Cleanup
        std::env::remove_var("TEST_KEY");
    }

    #[test]
    fn test_list_secrets() {
        let (manager, _temp) = create_test_manager();

        // Generate and save identity
        let key = age::x25519::Identity::generate();
        fs::write(&manager.identity_file, key.to_string().expose_secret()).unwrap();

        // Initially empty
        assert_eq!(manager.list_secrets().unwrap().len(), 0);

        // Add some secrets
        manager.set_secret("KEY1", "value1").unwrap();
        manager.set_secret("KEY2", "value2").unwrap();

        // Should list both
        let secrets = manager.list_secrets().unwrap();
        assert_eq!(secrets.len(), 2);
        assert!(secrets.contains(&"KEY1".to_string()));
        assert!(secrets.contains(&"KEY2".to_string()));
    }
}
