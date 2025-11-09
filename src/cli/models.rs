//! Embedding model management command

use mnemosyne_core::{error::Result, EmbeddingConfig};
use mnemosyne_core::orchestration::events::AgentEvent;
use clap::Subcommand;

use super::event_helpers;

#[derive(Subcommand)]
pub enum ModelsAction {
    /// List available embedding models
    List,

    /// Show model cache information
    Info,

    /// Clear model cache
    Clear {
        /// Confirm deletion without prompting
        #[arg(long)]
        yes: bool,
    },
}

/// Handle model management command
pub async fn handle(action: ModelsAction) -> Result<()> {
    let config = EmbeddingConfig::default();
    let cache_dir = &config.cache_dir;

    match action {
        ModelsAction::List => {
            event_helpers::with_event_lifecycle("models-list", vec![], async {
                println!("Available embedding models:");
                println!();
                println!("  nomic-embed-text-v1.5  (768 dims, recommended)");
                println!("  nomic-embed-text-v1    (768 dims)");
                println!("  all-MiniLM-L6-v2       (384 dims)");
                println!("  all-MiniLM-L12-v2      (384 dims)");
                println!("  bge-small-en-v1.5      (384 dims)");
                println!("  bge-base-en-v1.5       (768 dims)");
                println!("  bge-large-en-v1.5      (1024 dims)");
                println!();
                println!("Set model in config or use EmbeddingConfig::default()");

                // Emit domain event
                event_helpers::emit_domain_event(AgentEvent::ModelOperationCompleted {
                    operation: "list".to_string(),
                    model_name: None,
                    result_summary: "7 models available".to_string(),
                }).await;

                Ok(())
            }).await
        }
        ModelsAction::Info => {
            event_helpers::with_event_lifecycle("models-info", vec![], async {
                println!("Model cache directory: {}", cache_dir.display());
                println!();

                let mut result_summary = String::new();

                if cache_dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(cache_dir) {
                        let mut found_models = Vec::new();
                        for entry in entries.flatten() {
                            if entry.file_type().ok().map(|t| t.is_dir()).unwrap_or(false) {
                                found_models.push(entry.file_name());
                            }
                        }

                        if found_models.is_empty() {
                            println!("No models cached yet.");
                            println!("Models will be downloaded automatically on first use.");
                            result_summary = "No models cached".to_string();
                        } else {
                            println!("Cached models:");
                            for model in &found_models {
                                println!("  - {}", model.to_string_lossy());
                            }

                            // Calculate total size
                            if let Ok(metadata) = std::fs::metadata(cache_dir) {
                                println!();
                                println!("Total cache size: {} bytes", metadata.len());
                            }
                            result_summary = format!("{} models cached", found_models.len());
                        }
                    }
                } else {
                    println!("Cache directory does not exist yet.");
                    println!("It will be created on first model download.");
                    result_summary = "Cache directory not yet created".to_string();
                }

                // Emit domain event
                event_helpers::emit_domain_event(AgentEvent::ModelOperationCompleted {
                    operation: "info".to_string(),
                    model_name: None,
                    result_summary,
                }).await;

                Ok(())
            }).await
        }
        ModelsAction::Clear { yes } => {
            event_helpers::with_event_lifecycle("models-clear", vec![], async {
                if !cache_dir.exists() {
                    println!("Cache directory does not exist.");

                    // Emit domain event for no-op
                    event_helpers::emit_domain_event(AgentEvent::ModelOperationCompleted {
                        operation: "clear".to_string(),
                        model_name: None,
                        result_summary: "Cache directory does not exist".to_string(),
                    }).await;

                    return Ok(());
                }

                let confirm = if yes {
                    true
                } else {
                    use std::io::{self, Write};
                    print!("Clear model cache at {}? (y/N): ", cache_dir.display());
                    io::stdout().flush()?;

                    let mut response = String::new();
                    io::stdin().read_line(&mut response)?;
                    response.trim().to_lowercase() == "y"
                };

                let result_summary = if confirm {
                    std::fs::remove_dir_all(cache_dir)?;
                    println!("Model cache cleared successfully.");
                    println!("Models will be re-downloaded on next use.");
                    "Cache cleared successfully".to_string()
                } else {
                    println!("Cancelled.");
                    "Cancelled by user".to_string()
                };

                // Emit domain event
                event_helpers::emit_domain_event(AgentEvent::ModelOperationCompleted {
                    operation: "clear".to_string(),
                    model_name: None,
                    result_summary,
                }).await;

                Ok(())
            }).await
        }
    }
}
