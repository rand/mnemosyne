//! Embedding model management command

use mnemosyne_core::{error::Result, EmbeddingConfig};
use clap::Subcommand;

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
            Ok(())
        }
        ModelsAction::Info => {
            println!("Model cache directory: {}", cache_dir.display());
            println!();

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
                    } else {
                        println!("Cached models:");
                        for model in found_models {
                            println!("  - {}", model.to_string_lossy());
                        }

                        // Calculate total size
                        if let Ok(metadata) = std::fs::metadata(cache_dir) {
                            println!();
                            println!("Total cache size: {} bytes", metadata.len());
                        }
                    }
                }
            } else {
                println!("Cache directory does not exist yet.");
                println!("It will be created on first model download.");
            }

            Ok(())
        }
        ModelsAction::Clear { yes } => {
            if !cache_dir.exists() {
                println!("Cache directory does not exist.");
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

            if confirm {
                std::fs::remove_dir_all(cache_dir)?;
                println!("Model cache cleared successfully.");
                println!("Models will be re-downloaded on next use.");
            } else {
                println!("Cancelled.");
            }

            Ok(())
        }
    }
}
