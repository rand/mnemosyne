//! Memory export command

use mnemosyne_core::{
    error::{MnemosyneError, Result},
    orchestration::events::AgentEvent,
    storage::MemorySortOrder,
    ConnectionMode, LibsqlStorage, Namespace, StorageBackend,
};
use std::{io::Write, path::PathBuf};
use tracing::debug;

use super::event_helpers;
use super::helpers::get_db_path;

/// Handle memory export command
pub async fn handle(
    output: Option<String>,
    namespace: Option<String>,
    global_db_path: Option<String>,
) -> Result<()> {
    let start_time = std::time::Instant::now();

    if let Some(ref out_path) = output {
        debug!("Exporting memories to {}...", out_path);
    } else {
        debug!("Exporting memories to stdout...");
    }

    // Emit ExportStarted event
    event_helpers::emit_domain_event(AgentEvent::ExportStarted {
        output_path: output.clone(),
        namespace_filter: namespace.clone(),
    })
    .await;

    // Initialize storage (read-only)
    let db_path = get_db_path(global_db_path);
    let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), false).await?;

    // Parse namespace if provided
    let ns = namespace.map(|ns_str| {
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

    // Query all memories (or filtered by namespace)
    let memories = storage
        .list_memories(ns, 10000, MemorySortOrder::Recent)
        .await?;

    // Determine output format and destination
    let (format, use_stdout) = if let Some(ref path) = output {
        let fmt = if path.ends_with(".jsonl") {
            "jsonl"
        } else if path.ends_with(".md") || path.ends_with(".markdown") {
            "markdown"
        } else {
            "json" // default
        };
        (fmt, false)
    } else {
        // Default to JSON for stdout
        ("json", true)
    };

    // Track output size for event emission
    let mut output_size_bytes = 0usize;

    // Helper closure to write formatted output
    let write_output = |writer: &mut dyn Write| -> Result<(usize,)> {
        let mut bytes_written = 0usize;
        match format {
            "json" => {
                // Pretty-printed JSON
                let json = serde_json::to_string_pretty(&memories)?;
                bytes_written += json.len();
                writer.write_all(json.as_bytes())?;
                bytes_written += 1;
                writer.write_all(b"\n")?;
            }
            "jsonl" => {
                // Newline-delimited JSON (one object per line)
                for memory in &memories {
                    let json = serde_json::to_string(memory)?;
                    bytes_written += json.len() + 1; // +1 for newline
                    writeln!(writer, "{}", json)?;
                }
            }
            "markdown" => {
                // Human-readable Markdown
                let line = "# Memory Export\n".to_string();
                bytes_written += line.len();
                writer.write_all(line.as_bytes())?;

                let line = format!("Exported {} memories\n", memories.len());
                bytes_written += line.len();
                writer.write_all(line.as_bytes())?;

                let line = "---\n\n";
                bytes_written += line.len();
                writer.write_all(line.as_bytes())?;

                for (i, memory) in memories.iter().enumerate() {
                    let lines = vec![
                        format!("## {}. {}\n\n", i + 1, memory.summary),
                        format!("**ID**: {}\n", memory.id),
                        format!(
                            "**Namespace**: {}\n",
                            serde_json::to_string(&memory.namespace)?
                        ),
                        format!("**Importance**: {}/10\n", memory.importance),
                        format!("**Type**: {:?}\n", memory.memory_type),
                        format!(
                            "**Created**: {}\n",
                            memory.created_at.format("%Y-%m-%d %H:%M:%S")
                        ),
                    ];
                    for line in lines {
                        bytes_written += line.len();
                        writer.write_all(line.as_bytes())?;
                    }

                    if !memory.tags.is_empty() {
                        let line = format!("**Tags**: {}\n", memory.tags.join(", "));
                        bytes_written += line.len();
                        writer.write_all(line.as_bytes())?;
                    }
                    if !memory.keywords.is_empty() {
                        let line = format!("**Keywords**: {}\n", memory.keywords.join(", "));
                        bytes_written += line.len();
                        writer.write_all(line.as_bytes())?;
                    }

                    let lines = vec![
                        "\n### Content\n\n".to_string(),
                        format!("{}\n\n", memory.content),
                        "---\n\n".to_string(),
                    ];
                    for line in lines {
                        bytes_written += line.len();
                        writer.write_all(line.as_bytes())?;
                    }
                }
            }
            _ => {
                return Err(MnemosyneError::ValidationError(format!(
                    "Unsupported export format: {}",
                    format
                )));
            }
        }
        Ok((bytes_written,))
    };

    // Write to stdout or file
    if use_stdout {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let (size,) = write_output(&mut handle)?;
        output_size_bytes = size;
    } else {
        use std::fs::File;
        let output_path = PathBuf::from(output.as_ref().unwrap());
        let mut file = File::create(&output_path)?;
        let (size,) = write_output(&mut file)?;
        output_size_bytes = size;
        eprintln!(
            " Exported {} memories to {}",
            memories.len(),
            output_path.display()
        );
    }

    // Emit ExportCompleted event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    event_helpers::emit_domain_event(AgentEvent::ExportCompleted {
        memories_exported: memories.len(),
        output_size_bytes,
        duration_ms,
    })
    .await;

    Ok(())
}
