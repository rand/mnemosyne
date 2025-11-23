use crate::cli::helpers::get_db_path;
use mnemosyne_core::{
    error::Result,
    storage::{MemorySortOrder, StorageBackend},
    types::{MemoryNote, Namespace},
    ConnectionMode, LibsqlStorage,
};
use std::fs::File;
use std::io::Write;

pub async fn handle(
    format: String,
    depth: usize,
    output: Option<String>,
    query: Option<String>,
    namespace_str: Option<String>,
    db_path: Option<String>,
) -> Result<()> {
    let db_path = get_db_path(db_path);
    let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?;

    // Parse namespace
    let namespace = if let Some(ns) = namespace_str {
        if ns == "global" {
            Some(Namespace::Global)
        } else if let Some(project) = ns.strip_prefix("project:") {
            Some(Namespace::Project {
                name: project.to_string(),
            })
        } else if let Some(session) = ns.strip_prefix("session:") {
            // Session format is session:project:id
            let parts: Vec<&str> = session.split(':').collect();
            if parts.len() == 2 {
                Some(Namespace::Session {
                    project: parts[0].to_string(),
                    session_id: parts[1].to_string(),
                })
            } else {
                eprintln!(
                    "Invalid session namespace format. Expected 'session:project:id', got '{}'",
                    ns
                );
                return Ok(());
            }
        } else {
            // Default to project if just a name provided
            Some(Namespace::Project { name: ns })
        }
    } else {
        None
    };

    let memories = if let Some(q) = query {
        let seeds = storage.keyword_search(&q, namespace.clone()).await?;
        if seeds.is_empty() {
            eprintln!("No memories found matching query '{}'", q);
            return Ok(());
        }
        let seed_ids: Vec<_> = seeds.iter().map(|s| s.memory.id).collect();
        storage
            .graph_traverse(&seed_ids, depth, namespace)
            .await?
    } else {
        // No query, list top memories (limit 100 to avoid huge graphs)
        storage
            .list_memories(namespace, 100, MemorySortOrder::Importance)
            .await?
    };

    let output_content = match format.as_str() {
        "dot" => generate_dot(&memories),
        "mermaid" => generate_mermaid(&memories),
        "json" => serde_json::to_string_pretty(&memories).unwrap(),
        _ => {
            eprintln!(
                "Unknown format: {}. Supported formats: dot, mermaid, json",
                format
            );
            return Ok(());
        }
    };

    if let Some(path) = output {
        let mut file = File::create(path)?;
        file.write_all(output_content.as_bytes())?;
    } else {
        println!("{}", output_content);
    }

    Ok(())
}

fn generate_dot(memories: &[MemoryNote]) -> String {
    let mut dot = String::from(
        "digraph G {\n  rankdir=LR;\n  node [shape=box style=filled fillcolor=\"#f0f0f0\"];\n",
    );

    for memory in memories {
        let short_id: String = memory.id.to_string().chars().take(8).collect();
        let label = format!("{}|{:?}", short_id, memory.memory_type);
        
        dot.push_str(&format!(
            "  \"{}\" [label=\"{}\" tooltip=\"{}\"];\n",
            memory.id,
            label,
            escape_dot_string(&memory.summary)
        ));

        for link in &memory.links {
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{:?}\"];\n",
                memory.id, link.target_id, link.link_type
            ));
        }
    }
    dot.push_str("}\n");
    dot
}

fn generate_mermaid(memories: &[MemoryNote]) -> String {
    let mut mm = String::from("graph TD\n");

    for memory in memories {
        let id_clean = memory.id.to_string().replace("-", "");
        let label = format!("{:?}: {}", memory.memory_type, escape_mermaid_string(&memory.summary));

        mm.push_str(&format!("  {}[{:?}]\n", id_clean, label));

        for link in &memory.links {
            let target_clean = link.target_id.to_string().replace("-", "");
            let link_label = format!("{:?}", link.link_type);
            mm.push_str(&format!(
                "  {} -->|{}| {}\n",
                id_clean, link_label, target_clean
            ));
        }
    }
    mm
}

fn escape_dot_string(s: &str) -> String {
    s.replace("\"", "\\\"").replace("\n", " ")
}

fn escape_mermaid_string(s: &str) -> String {
    s.replace("\"", "'")
        .replace("\n", " ")
        .replace("[", "(")
        .replace("]", ")")
}
