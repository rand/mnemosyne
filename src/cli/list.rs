//! Memory listing command
//!
//! Lists stored memories with optional filtering and sorting.
//! Useful for personal agents to browse their memory history and
//! review what the system has retained.

use mnemosyne_core::{
    icons, orchestration::events::AgentEvent, ConnectionMode, LibsqlStorage, Namespace,
    StorageBackend,
};
use std::sync::Arc;
use tracing::debug;

use super::event_bridge;
use super::helpers::get_db_path;

/// Handle memory listing command
pub async fn handle(
    namespace_str: Option<String>,
    limit: usize,
    sort_by: String,
    format: String,
    tags: Option<String>,
    global_db_path: Option<String>,
) -> mnemosyne_core::error::Result<()> {
    let start_time = std::time::Instant::now();

    event_bridge::emit_command_started(
        "list",
        vec![
            format!("--limit={}", limit),
            format!("--sort-by={}", sort_by),
        ],
    )
    .await;

    let db_path = get_db_path(global_db_path);
    let storage =
        Arc::new(LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?);

    // Parse namespace
    let namespace = if let Some(ns_str) = &namespace_str {
        Some(parse_namespace(ns_str))
    } else {
        None
    };

    // Parse sort order
    let sort = match sort_by.as_str() {
        "importance" => mnemosyne_core::storage::MemorySortOrder::Importance,
        "access_count" => mnemosyne_core::storage::MemorySortOrder::AccessCount,
        _ => mnemosyne_core::storage::MemorySortOrder::Recent,
    };

    // Parse tag filters
    let tag_filters: Vec<String> = tags
        .as_ref()
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut memories = storage.list_memories(namespace, limit, sort).await?;

    // Apply tag filters client-side
    if !tag_filters.is_empty() {
        memories.retain(|m| {
            m.tags
                .iter()
                .any(|t| tag_filters.iter().any(|f| t.eq_ignore_ascii_case(f)))
        });
    }

    let count = memories.len();

    // Emit search performed event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    let _ = event_bridge::emit_event(AgentEvent::SearchPerformed {
        query: format!("list:sort={} tags={:?}", sort_by, tag_filters),
        search_type: "browse".to_string(),
        result_count: count,
        duration_ms,
    })
    .await;

    // Output
    if format == "json" {
        let json_results: Vec<_> = memories
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id.to_string(),
                    "summary": m.summary,
                    "importance": m.importance,
                    "tags": m.tags,
                    "memory_type": format!("{:?}", m.memory_type),
                    "namespace": serde_json::to_string(&m.namespace).unwrap_or_default(),
                    "created_at": m.created_at.to_rfc3339(),
                    "access_count": m.access_count,
                })
            })
            .collect();

        println!(
            "{}",
            serde_json::json!({ "results": json_results, "count": count })
        );
    } else if memories.is_empty() {
        eprintln!("{} No memories found.", icons::status::info());
    } else {
        eprintln!(
            "Found {} memor{}:\n",
            count,
            if count == 1 { "y" } else { "ies" }
        );
        for (i, mem) in memories.iter().enumerate() {
            println!(
                "{}. {}\n   ID: {}\n   Importance: {}/10\n   Tags: {}\n   Type: {}\n   Accessed: {}\n",
                i + 1,
                mem.summary,
                mem.id,
                mem.importance,
                if mem.tags.is_empty() { "<none>".to_string() } else { mem.tags.join(", ") },
                format!("{:?}", mem.memory_type),
                mem.access_count,
            );
        }
    }

    let _ = event_bridge::emit_command_completed(
        "list",
        duration_ms,
        format!("Listed {} memories", count),
    )
    .await;

    Ok(())
}

/// Parse a namespace string into a Namespace
fn parse_namespace(ns_str: &str) -> Namespace {
    if let Some(agent_id) = ns_str.strip_prefix("agent:") {
        Namespace::Agent {
            agent_id: agent_id.to_string(),
        }
    } else if ns_str.starts_with("project:") {
        let project = ns_str.strip_prefix("project:").unwrap();
        Namespace::Project {
            name: project.to_string(),
        }
    } else if ns_str.starts_with("session:") {
        let parts: Vec<&str> = ns_str
            .strip_prefix("session:")
            .unwrap()
            .splitn(2, ':')
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
}
