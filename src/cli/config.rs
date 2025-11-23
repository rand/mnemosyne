//! Configuration management command

use clap::Subcommand;
use mnemosyne_core::{error::Result, orchestration::events::AgentEvent, ConfigManager};

use super::event_helpers;

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

/// Mask an API key for safe logging/display
fn mask_api_key(key: &str) -> String {
    if key.len() > 12 {
        format!("{}...{}", &key[..8], &key[key.len() - 4..])
    } else {
        "***".to_string()
    }
}

/// Handle configuration management command
pub async fn handle(action: ConfigAction) -> Result<()> {
    let config_manager = ConfigManager::new()?;

    match action {
        ConfigAction::SetKey { key } => {
            event_helpers::with_event_lifecycle(
                "config set-key",
                vec![],  // Never log the actual key value
                async move {
                    #[cfg(feature = "keyring-fallback")]
                    {
                        let old_value = if config_manager.has_api_key() {
                            config_manager.get_api_key().ok().map(|k| mask_api_key(&k))
                        } else {
                            None
                        };

                        if let Some(key_value) = key {
                            // API key provided as argument
                            config_manager.set_api_key(&key_value)?;
                            println!(" API key securely saved to OS keychain");

                            // Emit config changed event with obfuscated values
                            event_helpers::emit_domain_event(AgentEvent::ConfigChanged {
                                setting: "anthropic_api_key".to_string(),
                                old_value,
                                new_value: Some("***REDACTED***".to_string()),  // Never log actual key
                            }).await;
                        } else {
                            // Interactive prompt
                            config_manager.prompt_and_set_api_key()?;

                            // Emit config changed event with obfuscated values
                            event_helpers::emit_domain_event(AgentEvent::ConfigChanged {
                                setting: "anthropic_api_key".to_string(),
                                old_value,
                                new_value: Some("***REDACTED***".to_string()),  // Never log actual key
                            }).await;
                        }
                    }
                    #[cfg(not(feature = "keyring-fallback"))]
                    {
                        eprintln!(
                            "Config set-key is deprecated. Use: mnemosyne secrets set ANTHROPIC_API_KEY"
                        );

                        let old_value = if config_manager.has_api_key() {
                            Some("***REDACTED***".to_string())
                        } else {
                            None
                        };

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

                        // Emit config changed event with obfuscated values
                        event_helpers::emit_domain_event(AgentEvent::ConfigChanged {
                            setting: "anthropic_api_key".to_string(),
                            old_value,
                            new_value: Some("***REDACTED***".to_string()),  // Never log actual key
                        }).await;
                    }
                    Ok(())
                }
            ).await
        }
        ConfigAction::ShowKey => {
            event_helpers::with_event_lifecycle("config show-key", vec![], async move {
                if config_manager.has_api_key() {
                    match config_manager.get_api_key() {
                        Ok(key) => {
                            // Show only first and last 4 characters
                            let masked = mask_api_key(&key);
                            println!(" API key configured: {}", masked);

                            // Show source
                            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                                println!("  Source: ANTHROPIC_API_KEY environment variable");
                            } else {
                                println!("  Source: OS keychain");
                            }
                        }
                        Err(e) => {
                            eprintln!(" Error retrieving API key: {}", e);
                        }
                    }
                } else {
                    println!(" No API key configured");
                    println!("\nTo set your API key:");
                    println!("  mnemosyne config set-key");
                    println!("or");
                    println!("  export ANTHROPIC_API_KEY=sk-ant-...");
                }
                Ok(())
            })
            .await
        }
        ConfigAction::DeleteKey => {
            event_helpers::with_event_lifecycle("config delete-key", vec![], async move {
                #[cfg(feature = "keyring-fallback")]
                {
                    let old_value = if config_manager.has_api_key() {
                        Some("***REDACTED***".to_string())
                    } else {
                        None
                    };

                    config_manager.delete_api_key()?;
                    println!(" API key deleted from OS keychain");
                    println!("\nNote: If ANTHROPIC_API_KEY environment variable is set,");
                    println!("      it will still be used. Unset it with:");
                    println!("      unset ANTHROPIC_API_KEY");

                    // Emit config changed event
                    event_helpers::emit_domain_event(AgentEvent::ConfigChanged {
                        setting: "anthropic_api_key".to_string(),
                        old_value,
                        new_value: None,
                    })
                    .await;
                }
                #[cfg(not(feature = "keyring-fallback"))]
                {
                    eprintln!(
                        "Config delete-key is deprecated. Secrets are managed in encrypted config."
                    );
                    eprintln!(
                        "To reset, delete: {}",
                        config_manager.secrets().secrets_file().display()
                    );
                }
                Ok(())
            })
            .await
        }
    }
}
