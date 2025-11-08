//! Memory recall/query command

use mnemosyne_core::{
    utils::string::truncate_at_char_boundary, ConnectionMode, EmbeddingService, LibsqlStorage,
    LlmConfig, Namespace, RemoteEmbeddingService, StorageBackend,
    orchestration::events::AgentEvent,
};
use std::collections::HashMap;
use tracing::debug;

use super::helpers::get_db_path;
use super::event_bridge;

/// Handle memory recall command
pub async fn handle(
    query: String,
    namespace: Option<String>,
    limit: usize,
    min_importance: Option<u8>,
    format: String,
    global_db_path: Option<String>,
) -> mnemosyne_core::error::Result<()> {
    let start_time = std::time::Instant::now();

    // Emit CLI command started event
    event_bridge::emit_command_started(
        "recall",
        vec![
            format!("--query={}", query),
            format!("--limit={}", limit),
        ],
    )
    .await;

    // Initialize storage and services
    let db_path = get_db_path(global_db_path);
    let storage = LibsqlStorage::new(ConnectionMode::Local(db_path.clone())).await?;

    // Check if API key is available for vector search
    let embedding_service_config = LlmConfig::default();
    let has_api_key = !embedding_service_config.api_key.is_empty();

    // Parse namespace
    let ns = namespace.as_ref().map(|ns_str| {
        if ns_str.starts_with("project:") {
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
        }
    });

    // Perform hybrid search (keyword + vector + graph)
    let keyword_results = storage
        .hybrid_search(&query, ns.clone(), limit * 2, true)
        .await?;

    // Vector search (optional - only if API key available)
    let vector_results = if has_api_key {
        match RemoteEmbeddingService::new(
            embedding_service_config.api_key.clone(),
            None, // Use default model
            None, // Use default base URL
        ) {
            Ok(embedding_service) => {
                match embedding_service.embed(&query).await {
                    Ok(query_embedding) => storage
                        .vector_search(&query_embedding, limit * 2, ns.clone())
                        .await
                        .unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            }
            Err(_) => Vec::new(),
        }
    } else {
        debug!("Skipping vector search - no API key configured");
        Vec::new()
    };

    // Merge results
    let mut memory_scores = HashMap::new();

    for result in keyword_results {
        memory_scores
            .entry(result.memory.id)
            .or_insert((result.memory.clone(), vec![]))
            .1
            .push(result.score * 0.4);
    }

    for (memory_id, similarity) in vector_results {
        // Fetch the memory for this ID
        if let Ok(memory) = storage.get_memory(memory_id).await {
            memory_scores
                .entry(memory_id)
                .or_insert((memory, vec![]))
                .1
                .push(similarity * 0.3);
        }
    }

    let mut results: Vec<_> = memory_scores
        .into_iter()
        .map(|(_, (memory, scores))| {
            let total_score: f32 = scores.iter().sum();
            (memory, total_score)
        })
        .collect();

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);

    // Filter by importance if specified
    if let Some(min_imp) = min_importance {
        results.retain(|(m, _)| m.importance >= min_imp);
    }

    let result_count = results.len();

    // Output results
    if format == "json" {
        let json_results: Vec<_> = results
            .iter()
            .map(|(m, score)| {
                serde_json::json!({
                    "id": m.id.to_string(),
                    "summary": m.summary,
                    "content": m.content,
                    "importance": m.importance,
                    "tags": m.tags,
                    "memory_type": format!("{:?}", m.memory_type),
                    "score": score,
                    "namespace": serde_json::to_string(&m.namespace).unwrap_or_default()
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::json!({
                "results": json_results,
                "count": json_results.len()
            })
        );
    } else if results.is_empty() {
        println!("No memories found matching '{}'", query);
    } else {
        println!("Found {} memories:\n", results.len());
        for (i, (memory, score)) in results.iter().enumerate() {
            println!(
                "{}. {} (score: {:.2}, importance: {}/10)",
                i + 1,
                memory.summary,
                score,
                memory.importance
            );
            println!("   ID: {}", memory.id);
            println!("   Tags: {}", memory.tags.join(", "));
            println!(
                "   Content: {}\n",
                truncate_at_char_boundary(&memory.content, 100)
            );
        }
    }

    // Emit recall executed event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    let recall_event = AgentEvent::RecallExecuted {
        query: query.clone(),
        result_count,
        duration_ms,
    };
    let _ = event_bridge::emit_event(recall_event).await;

    // Emit search performed event
    let search_event = AgentEvent::SearchPerformed {
        query: query.clone(),
        search_type: "hybrid".to_string(), // keyword + vector search
        result_count,
        duration_ms,
    };
    let _ = event_bridge::emit_event(search_event).await;

    // Emit command completed event
    event_bridge::emit_command_completed(
        "recall",
        duration_ms,
        format!("Found {} results for query '{}'", result_count, query),
    )
    .await;

    Ok(())
}
