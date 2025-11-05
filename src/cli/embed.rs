//! Embedding generation command

use mnemosyne_core::{
    error::Result, ConnectionMode, EmbeddingConfig, LibsqlStorage, LocalEmbeddingService,
    MemoryId, Namespace, StorageBackend,
};
use std::sync::Arc;
use uuid::Uuid;

use super::helpers::get_db_path;

/// Handle embedding generation command
pub async fn handle(
    all: bool,
    memory_id: Option<String>,
    namespace: Option<String>,
    batch_size: usize,
    progress: bool,
    global_db_path: Option<String>,
) -> Result<()> {
    // Initialize embedding service
    println!("Initializing local embedding service...");
    let embedding_config = EmbeddingConfig::default();
    let embedding_service = Arc::new(LocalEmbeddingService::new(embedding_config).await?);

    // Initialize storage
    let db_path = get_db_path(global_db_path);
    let mut storage = LibsqlStorage::new(ConnectionMode::Local(db_path.clone())).await?;

    // Set embedding service on storage
    storage.set_embedding_service(embedding_service.clone());

    // Determine which memories to embed
    let memories = if let Some(id_str) = memory_id {
        // Single memory
        let uuid =
            Uuid::parse_str(&id_str).map_err(|e| anyhow::anyhow!("Invalid memory ID: {}", e))?;
        let id = MemoryId(uuid);
        vec![storage.get_memory(id).await?]
    } else {
        // Fetch all memories using search with empty query
        let ns = if let Some(ns_str) = namespace {
            println!("Fetching memories in namespace '{}'...", ns_str);
            Some(if ns_str.starts_with("project:") {
                let project = ns_str.strip_prefix("project:").unwrap();
                Namespace::Project {
                    name: project.to_string(),
                }
            } else if ns_str.starts_with("session:") {
                let parts: Vec<&str> = ns_str
                    .strip_prefix("session:")
                    .unwrap()
                    .split(':')
                    .collect();
                if parts.len() == 2 {
                    Namespace::Session {
                        project: parts[0].to_string(),
                        session_id: parts[1].to_string(),
                    }
                } else {
                    Namespace::Global
                }
            } else {
                Namespace::Global
            })
        } else if all {
            println!("Fetching all memories...");
            None
        } else {
            eprintln!("Error: Must specify --all, --memory-id, or --namespace");
            std::process::exit(1);
        };

        // Use hybrid_search with empty query to get all memories
        let results = storage.hybrid_search("", ns, 10000, false).await?;
        results.into_iter().map(|r| r.memory).collect()
    };

    let total = memories.len();
    println!("Generating embeddings for {} memories...", total);

    // Process memories in batches
    let mut processed = 0;
    let mut succeeded = 0;
    let mut failed = 0;

    for chunk in memories.chunks(batch_size) {
        for memory in chunk {
            processed += 1;

            if progress {
                print!("\rProgress: {}/{} ", processed, total);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }

            match storage
                .generate_and_store_embedding(&memory.id, &memory.content)
                .await
            {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    if progress {
                        eprintln!("\nFailed to embed memory {}: {}", memory.id, e);
                    }
                    failed += 1;
                }
            }
        }
    }

    if progress {
        println!();
    }

    println!("Embedding generation complete!");
    println!("  Total: {}", total);
    println!("  Succeeded: {}", succeeded);
    println!("  Failed: {}", failed);

    Ok(())
}
