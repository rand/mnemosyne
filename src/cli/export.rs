//! Memory export command

use mnemosyne_core::{
    error::{MnemosyneError, Result},
    storage::MemorySortOrder,
    ConnectionMode, LibsqlStorage, Namespace, StorageBackend,
};
use std::{io::Write, path::PathBuf};
use tracing::debug;

use super::helpers::get_db_path;

/// Handle memory export command
pub async fn handle(
    output: Option<String>,
    namespace: Option<String>,
    global_db_path: Option<String>,
) -> Result<()> {
    if let Some(ref out_path) = output {
        debug!("Exporting memories to {}...", out_path);
    } else {
        debug!("Exporting memories to stdout...");
    }

    // Initialize storage (read-only)
    let db_path = get_db_path(global_db_path);
    let storage =
        LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), false).await?;

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

    // Helper closure to write formatted output
    let write_output = |writer: &mut dyn Write| -> Result<()> {
        match format {
            "json" => {
                // Pretty-printed JSON
                let json = serde_json::to_string_pretty(&memories)?;
                writer.write_all(json.as_bytes())?;
                writer.write_all(b"\n")?;
            }
            "jsonl" => {
                // Newline-delimited JSON (one object per line)
                for memory in &memories {
                    let json = serde_json::to_string(memory)?;
                    writeln!(writer, "{}", json)?;
                }
            }
            "markdown" => {
                // Human-readable Markdown
                writeln!(writer, "# Memory Export\n")?;
                writeln!(writer, "Exported {} memories\n", memories.len())?;
                writeln!(writer, "---\n")?;

                for (i, memory) in memories.iter().enumerate() {
                    writeln!(writer, "## {}. {}\n", i + 1, memory.summary)?;
                    writeln!(writer, "**ID**: {}", memory.id)?;
                    writeln!(
                        writer,
                        "**Namespace**: {}",
                        serde_json::to_string(&memory.namespace)?
                    )?;
                    writeln!(writer, "**Importance**: {}/10", memory.importance)?;
                    writeln!(writer, "**Type**: {:?}", memory.memory_type)?;
                    writeln!(
                        writer,
                        "**Created**: {}",
                        memory.created_at.format("%Y-%m-%d %H:%M:%S")
                    )?;
                    if !memory.tags.is_empty() {
                        writeln!(writer, "**Tags**: {}", memory.tags.join(", "))?;
                    }
                    if !memory.keywords.is_empty() {
                        writeln!(writer, "**Keywords**: {}", memory.keywords.join(", "))?;
                    }
                    writeln!(writer, "\n### Content\n")?;
                    writeln!(writer, "{}\n", memory.content)?;
                    writeln!(writer, "---\n")?;
                }
            }
            _ => {
                return Err(MnemosyneError::ValidationError(format!(
                    "Unsupported export format: {}",
                    format
                ))
                .into());
            }
        }
        Ok(())
    };

    // Write to stdout or file
    if use_stdout {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        write_output(&mut handle)?;
    } else {
        use std::fs::File;
        let output_path = PathBuf::from(output.as_ref().unwrap());
        let mut file = File::create(&output_path)?;
        write_output(&mut file)?;
        eprintln!(
            " Exported {} memories to {}",
            memories.len(),
            output_path.display()
        );
    }

    Ok(())
}
