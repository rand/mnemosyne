//! Memory creation command

use mnemosyne_core::{
    error::Result, icons, ConnectionMode, EmbeddingService, LibsqlStorage, LlmConfig, LlmService,
    MemoryNote, Namespace, RemoteEmbeddingService, StorageBackend,
    orchestration::events::AgentEvent,
};
use tracing::{debug, warn};

use super::helpers::{get_db_path, parse_memory_type};
use super::event_bridge;

/// Handle memory creation command
#[allow(clippy::too_many_arguments)]
pub async fn handle(
    content: String,
    namespace: String,
    importance: u8,
    context: Option<String>,
    tags: Option<String>,
    memory_type: Option<String>,
    format: String,
    global_db_path: Option<String>,
) -> Result<()> {
    let start_time = std::time::Instant::now();

    // Emit CLI command started event
    event_bridge::emit_command_started(
        "remember",
        vec![
            format!("--content={}", content.chars().take(50).collect::<String>()),
            format!("--importance={}", importance),
            format!("--namespace={}", namespace),
        ],
    )
    .await;

    // Initialize storage and services
    let db_path = get_db_path(global_db_path);
    // Remember command creates database if it doesn't exist (write implies initialize)
    let storage =
        LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path.clone()), true).await?;

    // Check if API key is available for LLM enrichment
    let llm_config = LlmConfig::default();
    let has_api_key = !llm_config.api_key.is_empty();

    // Parse namespace
    let ns = if namespace.starts_with("project:") {
        let project = namespace.strip_prefix("project:").unwrap();
        Namespace::Project {
            name: project.to_string(),
        }
    } else if namespace.starts_with("session:") {
        let parts: Vec<&str> = namespace
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
    };

    // Create or enrich memory
    let mut memory = if has_api_key {
        // Try to enrich memory with LLM, but fall back if it fails
        let llm = LlmService::new(llm_config.clone())?;
        let ctx = context.unwrap_or_else(|| "CLI input".to_string());

        match llm.enrich_memory(&content, &ctx).await {
            Ok(enriched_memory) => {
                debug!("Memory enriched successfully with LLM");
                enriched_memory
            }
            Err(e) => {
                // LLM enrichment failed - fall back to basic memory
                // Log specific error type for better debugging
                match &e {
                    mnemosyne_core::MnemosyneError::AuthenticationError(_) => {
                        warn!("LLM enrichment failed (invalid API key): {}, storing memory without enrichment", e);
                    }
                    mnemosyne_core::MnemosyneError::RateLimitExceeded(_) => {
                        warn!("LLM enrichment failed (rate limit): {}, storing memory without enrichment", e);
                    }
                    mnemosyne_core::MnemosyneError::NetworkError(_) => {
                        warn!("LLM enrichment failed (network error): {}, storing memory without enrichment", e);
                    }
                    _ => {
                        warn!(
                            "LLM enrichment failed: {}, storing memory without enrichment",
                            e
                        );
                    }
                }

                use mnemosyne_core::types::MemoryId;

                let now = chrono::Utc::now();

                MemoryNote {
                    id: MemoryId::new(),
                    namespace: ns.clone(),
                    created_at: now,
                    updated_at: now,
                    content: content.clone(),
                    summary: content.chars().take(100).collect::<String>(),
                    keywords: Vec::new(),
                    tags: Vec::new(),
                    context: ctx.clone(),
                    memory_type: memory_type
                        .as_deref()
                        .map(parse_memory_type)
                        .unwrap_or(mnemosyne_core::MemoryType::Insight),
                    importance: importance.clamp(1, 10),
                    confidence: 0.5,
                    links: Vec::new(),
                    related_files: Vec::new(),
                    related_entities: Vec::new(),
                    access_count: 0,
                    last_accessed_at: now,
                    expires_at: None,
                    is_archived: false,
                    superseded_by: None,
                    embedding: None,
                    embedding_model: String::new(),
                }
            }
        }
    } else {
        // Create basic memory without LLM enrichment
        debug!("Creating basic memory without LLM enrichment - no API key");
        use mnemosyne_core::types::MemoryId;

        let now = chrono::Utc::now();
        let ctx = context.unwrap_or_else(|| "CLI input".to_string());

        MemoryNote {
            id: MemoryId::new(),
            namespace: ns.clone(),
            created_at: now,
            updated_at: now,
            content: content.clone(),
            summary: content.chars().take(100).collect::<String>(),
            keywords: Vec::new(),
            tags: Vec::new(),
            context: ctx,
            memory_type: memory_type
                .as_deref()
                .map(parse_memory_type)
                .unwrap_or(mnemosyne_core::MemoryType::Insight),
            importance: importance.clamp(1, 10),
            confidence: 0.5,
            links: Vec::new(),
            related_files: Vec::new(),
            related_entities: Vec::new(),
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        }
    };

    // Override with CLI parameters (in case LLM set different values)
    memory.namespace = ns;
    memory.importance = importance.clamp(1, 10);
    if let Some(ref type_str) = memory_type {
        memory.memory_type = parse_memory_type(type_str);
    }

    // Add custom tags if provided
    if let Some(tag_str) = tags {
        let custom_tags: Vec<String> = tag_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        memory.tags.extend(custom_tags);
    }

    // Generate embedding if API key available
    if has_api_key {
        match RemoteEmbeddingService::new(llm_config.api_key.clone(), None, None) {
            Ok(embedding_service) => match embedding_service.embed(&memory.content).await {
                Ok(embedding) => memory.embedding = Some(embedding),
                Err(_) => {
                    debug!("Failed to generate embedding, storing without it");
                }
            },
            Err(_) => {
                debug!("Failed to create embedding service, storing without embedding");
            }
        }
    }

    // Store memory
    storage.store_memory(&memory).await?;

    // Emit memory stored event
    let remember_event = AgentEvent::RememberExecuted {
        content_preview: memory.summary.chars().take(100).collect(),
        memory_id: memory.id.clone(),
        importance: memory.importance,
    };
    let _ = event_bridge::emit_event(remember_event).await;

    // Output result
    if format == "json" {
        println!(
            "{}",
            serde_json::json!({
                "id": memory.id.to_string(),
                "summary": memory.summary,
                "importance": memory.importance,
                "tags": memory.tags,
                "namespace": serde_json::to_string(&memory.namespace).unwrap_or_default()
            })
        );
    } else {
        println!("{} Memory saved", icons::status::success());
        println!("ID: {}", memory.id);
        println!("Summary: {}", memory.summary);
        println!("Importance: {}/10", memory.importance);
        println!("Tags: {}", memory.tags.join(", "));
    }

    // Emit command completed event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    event_bridge::emit_command_completed(
        "remember",
        duration_ms,
        format!("Stored memory {} (importance {})", memory.id, memory.importance),
    )
    .await;

    Ok(())
}
