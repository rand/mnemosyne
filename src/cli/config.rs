//! Configuration management command

use clap::Subcommand;
use mnemosyne_core::{error::Result, ConfigManager};

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum ConfigAction {
    /// Set Anthropic API key (stores securely in OS keychain)
    SetKey {
        /// API key (if not provided, will prompt interactively)
        key: Option<String>,
    },

    /// Show API key configuration status
    ShowKey,

    /// Delete stored API key
    DeleteKey,
}

/// Handle configuration management command
pub async fn handle(action: ConfigAction) -> Result<()> {
    let config_manager = ConfigManager::new()?;

    match action {
        ConfigAction::SetKey { key } => {
            #[cfg(feature = "keyring-fallback")]
            {
                if let Some(key_value) = key {
                    // API key provided as argument
                    config_manager.set_api_key(&key_value)?;
                    println!(" API key securely saved to OS keychain");
                } else {
                    // Interactive prompt
                    config_manager.prompt_and_set_api_key()?;
                }
            }
            #[cfg(not(feature = "keyring-fallback"))]
            {
                eprintln!(
                    "Config set-key is deprecated. Use: mnemosyne secrets set ANTHROPIC_API_KEY"
                );
                if let Some(key_value) = key {
                    config_manager
                        .secrets()
                        .set_secret("ANTHROPIC_API_KEY", &key_value)?;
                } else {
                    print!("Enter API key: ");
                    std::io::Write::flush(&mut std::io::stdout())?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    config_manager
                        .secrets()
                        .set_secret("ANTHROPIC_API_KEY", input.trim())?;
                }
            }
            Ok(())
        }
        ConfigAction::ShowKey => {
            if config_manager.has_api_key() {
                match config_manager.get_api_key() {
                    Ok(key) => {
                        // Show only first and last 4 characters
                        let masked = if key.len() > 12 {
                            format!("{}...{}", &key[..8], &key[key.len() - 4..])
                        } else {
                            "***".to_string()
                        };
                        println!(" API key configured: {}", masked);

                        // Show source
                        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                            println!("  Source: ANTHROPIC_API_KEY environment variable");
                        } else {
                            println!("  Source: OS keychain");
                        }
                    }
                    Err(e) => {
                        eprintln!(" Error retrieving API key: {}", e);
                    }
                }
            } else {
                println!(" No API key configured");
                println!("\nTo set your API key:");
                println!("  mnemosyne config set-key");
                println!("or");
                println!("  export ANTHROPIC_API_KEY=sk-ant-...");
            }
            Ok(())
        }
        ConfigAction::DeleteKey => {
            #[cfg(feature = "keyring-fallback")]
            {
                config_manager.delete_api_key()?;
                println!(" API key deleted from OS keychain");
                println!("\nNote: If ANTHROPIC_API_KEY environment variable is set,");
                println!("      it will still be used. Unset it with:");
                println!("      unset ANTHROPIC_API_KEY");
            }
            #[cfg(not(feature = "keyring-fallback"))]
            {
                eprintln!("Config delete-key is deprecated. Secrets are managed in encrypted config.");
                eprintln!(
                    "To reset, delete: {}",
                    config_manager.secrets().secrets_file().display()
                );
            }
            Ok(())
        }
    }
}
