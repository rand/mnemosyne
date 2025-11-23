//! Secrets management command

use clap::Subcommand;
use mnemosyne_core::{
    error::Result, icons, orchestration::events::AgentEvent, secrets::SecretsManager,
};
use secrecy::ExposeSecret;

use super::event_helpers;

#[derive(Subcommand)]
pub enum SecretsCommand {
    /// Initialize secrets (first-time setup)
    Init,

    /// Set a secret value
    Set {
        /// Secret name (e.g., ANTHROPIC_API_KEY)
        name: String,

        /// Value (optional, will prompt if not provided)
        #[arg(short, long)]
        value: Option<String>,
    },

    /// Get a secret value (for testing)
    Get {
        /// Secret name
        name: String,
    },

    /// List configured secrets (names only)
    List,

    /// Show where secrets are stored
    Info,
}

/// Handle secrets management command
pub async fn handle(command: SecretsCommand) -> Result<()> {
    let secrets = SecretsManager::new()?;

    match command {
        SecretsCommand::Init => {
            event_helpers::with_event_lifecycle("secrets init", vec![], async move {
                if secrets.is_initialized() {
                    println!(
                        "{}  Secrets already initialized at: {}",
                        icons::status::warning(),
                        secrets.secrets_file().display()
                    );
                    print!("Reinitialize? This will overwrite existing secrets. [y/N]: ");
                    std::io::Write::flush(&mut std::io::stdout())?;

                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Cancelled.");
                        return Ok(());
                    }
                }
                secrets.initialize_interactive()?;

                // Emit domain event - initialization is essentially a "rotate" operation
                event_helpers::emit_domain_event(AgentEvent::SecretsModified {
                    operation: "initialize".to_string(),
                    secret_name: "encryption_key".to_string(),
                })
                .await;

                Ok(())
            })
            .await
        }
        SecretsCommand::Set { name, value } => {
            event_helpers::with_event_lifecycle(
                "secrets set",
                vec![format!("--name={}", name)], // Only log name, never value
                async move {
                    let val = if let Some(v) = value {
                        v
                    } else {
                        print!("Enter value for {}: ", name);
                        std::io::Write::flush(&mut std::io::stdout())?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        input.trim().to_string()
                    };
                    secrets.set_secret(&name, &val)?;

                    // SECURITY: Emit domain event with NO secret value
                    event_helpers::emit_domain_event(AgentEvent::SecretsModified {
                        operation: "set".to_string(),
                        secret_name: name.clone(),
                    })
                    .await;

                    Ok(())
                },
            )
            .await
        }
        SecretsCommand::Get { name } => {
            event_helpers::with_event_lifecycle(
                "secrets get",
                vec![format!("--name={}", name)],
                async move {
                    match secrets.get_secret(&name) {
                        Ok(secret) => {
                            println!("{}", secret.expose_secret());
                        }
                        Err(e) => {
                            eprintln!(" {}", e);
                        }
                    }
                    Ok(())
                },
            )
            .await
        }
        SecretsCommand::List => {
            event_helpers::with_event_lifecycle("secrets list", vec![], async move {
                if !secrets.is_initialized() {
                    println!("No secrets configured. Run: mnemosyne secrets init");
                    return Ok(());
                }

                let names = secrets.list_secrets()?;
                if names.is_empty() {
                    println!("No secrets configured. Run: mnemosyne secrets init");
                } else {
                    println!("Configured secrets:");
                    for name in names {
                        // Check if available via environment variable
                        let source = if std::env::var(&name).is_ok() {
                            " (from environment)"
                        } else {
                            ""
                        };
                        println!("  - {}{}", name, source);
                    }
                }
                Ok(())
            })
            .await
        }
        SecretsCommand::Info => {
            event_helpers::with_event_lifecycle("secrets info", vec![], async move {
                println!("Secrets Configuration");
                println!();
                println!("Config dir:     {}", secrets.config_dir().display());
                println!("Secrets file:   {}", secrets.secrets_file().display());
                println!("Identity key:   {}", secrets.identity_file().display());
                println!(
                    "Initialized:    {}",
                    if secrets.is_initialized() {
                        "yes"
                    } else {
                        "no"
                    }
                );
                println!();

                if secrets.is_initialized() {
                    let names = secrets.list_secrets()?;
                    println!("Configured secrets: {}", names.len());
                    for name in names {
                        let _available = secrets.get_secret(&name).is_ok();
                        let status = "";
                        let source = if std::env::var(&name).is_ok() {
                            " (env)"
                        } else {
                            " (file)"
                        };
                        println!("  {} {}{}", status, name, source);
                    }
                } else {
                    println!("Run 'mnemosyne secrets init' to set up encrypted secrets.");
                }
                Ok(())
            })
            .await
        }
    }
}
