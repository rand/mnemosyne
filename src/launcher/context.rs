//! Context Loading for Session Startup
//!
//! Generates initial context prompts by loading high-importance memories
//! from the Mnemosyne storage system.
//!
//! # Features
//! - Load top memories by importance
//! - Filter by namespace
//! - Format as natural language summary
//! - Include semantic links and relationships
//! - Respect context budget allocation (20%)
//! - Graceful degradation on errors

use crate::error::Result;
use crate::storage::{MemorySortOrder, StorageBackend};
use crate::types::{MemoryNote, Namespace};
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Configuration for context loading
#[derive(Debug, Clone)]
pub struct ContextLoadConfig {
    /// Maximum number of memories to load
    pub max_memories: usize,

    /// Minimum importance threshold (1-10)
    pub min_importance: u8,

    /// Maximum context size in bytes
    pub max_size_bytes: usize,

    /// Include knowledge graph metadata
    pub include_metadata: bool,
}

impl Default for ContextLoadConfig {
    fn default() -> Self {
        Self {
            max_memories: 10,
            min_importance: 7,
            max_size_bytes: 10 * 1024, // 10KB (20% of ~50KB context budget)
            include_metadata: true,
        }
    }
}

/// Context loader for session startup
pub struct ContextLoader {
    storage: Arc<dyn StorageBackend>,
}

impl ContextLoader {
    /// Create a new context loader with storage backend
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    /// Generate startup prompt with project context
    ///
    /// This loads high-importance memories and formats them as a natural
    /// language summary to be included in the session's initial context.
    ///
    /// # Arguments
    /// * `namespace` - Memory namespace to load from
    /// * `config` - Context loading configuration
    ///
    /// # Returns
    /// Formatted context string for --append-system-prompt
    pub async fn generate_startup_prompt(
        &self,
        namespace: &str,
        config: &ContextLoadConfig,
    ) -> Result<String> {
        // Parse namespace
        let ns = parse_namespace(namespace);

        // Query memories (over-fetch to ensure we have enough after filtering)
        let memories = self
            .storage
            .list_memories(
                Some(ns),
                config.max_memories * 2,
                MemorySortOrder::Importance,
            )
            .await?;

        // Filter by importance threshold
        let filtered: Vec<_> = memories
            .into_iter()
            .filter(|m| m.importance >= config.min_importance)
            .take(config.max_memories)
            .collect();

        // Format context
        self.format_context(&filtered, namespace, config)
    }

    /// Generate compact context for quick session startup
    ///
    /// Returns only the most critical memories (importance >= 8)
    pub async fn generate_compact_prompt(&self, namespace: &str) -> Result<String> {
        let compact_config = ContextLoadConfig {
            max_memories: 5,
            min_importance: 8,
            max_size_bytes: 5 * 1024, // 5KB
            include_metadata: false,
        };

        self.generate_startup_prompt(namespace, &compact_config)
            .await
    }

    /// Format memories as natural language context
    fn format_context(
        &self,
        memories: &[MemoryNote],
        namespace: &str,
        config: &ContextLoadConfig,
    ) -> Result<String> {
        if memories.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();

        // Header
        output.push_str(&format!("# Project Context: {}\n\n", namespace));

        // Group by importance tier
        let critical: Vec<_> = memories.iter().filter(|m| m.importance >= 8).collect();
        let important: Vec<_> = memories.iter().filter(|m| m.importance == 7).collect();

        // Critical memories (detailed)
        if !critical.is_empty() {
            output.push_str("## Critical Memories (Importance ≥ 8)\n\n");
            for mem in critical {
                output.push_str(&format_memory(mem, true));
            }
            output.push('\n');
        }

        // Important memories (compact)
        if !important.is_empty() {
            output.push_str("## Important Memories (Importance 7)\n\n");
            for mem in important {
                output.push_str(&format_memory(mem, false));
            }
            output.push('\n');
        }

        // Knowledge graph summary
        if config.include_metadata {
            let total_links: usize = memories.iter().map(|m| m.links.len()).sum();
            output.push_str(&format!(
                "## Knowledge Graph\n{} semantic connections across {} memories\n\n",
                total_links,
                memories.len()
            ));
        }

        // Footer
        output.push_str("---\n");
        output.push_str(&format!(
            "*Context from Mnemosyne • {} memories loaded*\n",
            memories.len()
        ));

        // Enforce size limit (hard cap)
        if output.len() > config.max_size_bytes {
            output.truncate(config.max_size_bytes - 50); // Leave room for truncation notice
            output.push_str("\n\n[Context truncated to fit budget]");
        }

        Ok(output)
    }
}

/// Parse namespace string to Namespace enum
fn parse_namespace(s: &str) -> Namespace {
    if s == "global" {
        Namespace::Global
    } else if let Some(name) = s.strip_prefix("project:") {
        Namespace::Project {
            name: name.to_string(),
        }
    } else if let Some(rest) = s.strip_prefix("session:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            Namespace::Session {
                project: parts[0].to_string(),
                session_id: parts[1].to_string(),
            }
        } else {
            // Malformed session namespace, default to global
            Namespace::Global
        }
    } else {
        // Unknown format, default to global
        Namespace::Global
    }
}

/// Format a single memory for context display
fn format_memory(mem: &MemoryNote, detailed: bool) -> String {
    if detailed {
        // Detailed format for critical memories
        format!(
            "**{}** — {} — {}\n{}\n\n",
            mem.summary,
            format_memory_type(&mem.memory_type),
            relative_time(&mem.updated_at),
            truncate_content(&mem.content, 200)
        )
    } else {
        // Compact format for important memories
        format!(
            "- {} — {}\n",
            mem.summary,
            format_memory_type(&mem.memory_type)
        )
    }
}

/// Format memory type for display
fn format_memory_type(mt: &crate::types::MemoryType) -> &'static str {
    use crate::types::MemoryType;
    match mt {
        MemoryType::ArchitectureDecision => "Architecture",
        MemoryType::CodePattern => "Pattern",
        MemoryType::BugFix => "Bug Fix",
        MemoryType::Configuration => "Config",
        MemoryType::Constraint => "Constraint",
        MemoryType::Entity => "Entity",
        MemoryType::Insight => "Insight",
        MemoryType::Reference => "Reference",
        MemoryType::Preference => "Preference",
        MemoryType::AgentEvent => "Agent Event",
    }
}

/// Truncate content to max length, adding ellipsis
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        format!("{}...", &content[..max_len])
    }
}

/// Format relative time (e.g., "2 days ago", "3 hours ago")
fn relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_days() > 0 {
        let days = duration.num_days();
        if days == 1 {
            "1 day ago".to_string()
        } else if days < 7 {
            format!("{} days ago", days)
        } else if days < 30 {
            let weeks = days / 7;
            if weeks == 1 {
                "1 week ago".to_string()
            } else {
                format!("{} weeks ago", weeks)
            }
        } else if days < 365 {
            let months = days / 30;
            if months == 1 {
                "1 month ago".to_string()
            } else {
                format!("{} months ago", months)
            }
        } else {
            let years = days / 365;
            if years == 1 {
                "1 year ago".to_string()
            } else {
                format!("{} years ago", years)
            }
        }
    } else if duration.num_hours() > 0 {
        let hours = duration.num_hours();
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if duration.num_minutes() > 0 {
        let minutes = duration.num_minutes();
        if minutes == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", minutes)
        }
    } else {
        "just now".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryId, MemoryLink, MemoryType};
    use chrono::Utc;

    #[test]
    fn test_parse_namespace_global() {
        let ns = parse_namespace("global");
        assert_eq!(ns, Namespace::Global);
    }

    #[test]
    fn test_parse_namespace_project() {
        let ns = parse_namespace("project:myapp");
        assert_eq!(
            ns,
            Namespace::Project {
                name: "myapp".to_string()
            }
        );
    }

    #[test]
    fn test_parse_namespace_session() {
        let ns = parse_namespace("session:myapp:abc123");
        assert!(matches!(ns, Namespace::Session { .. }));
        if let Namespace::Session { project, session_id } = ns {
            assert_eq!(project, "myapp");
            assert_eq!(session_id, "abc123");
        }
    }

    #[test]
    fn test_parse_namespace_unknown_defaults_to_global() {
        let ns = parse_namespace("unknown:format");
        assert_eq!(ns, Namespace::Global);
    }

    #[test]
    fn test_format_memory_type() {
        assert_eq!(
            format_memory_type(&MemoryType::ArchitectureDecision),
            "Architecture"
        );
        assert_eq!(format_memory_type(&MemoryType::CodePattern), "Pattern");
        assert_eq!(format_memory_type(&MemoryType::BugFix), "Bug Fix");
        assert_eq!(format_memory_type(&MemoryType::Insight), "Insight");
    }

    #[test]
    fn test_truncate_content() {
        let content = "This is a long piece of content that should be truncated";
        let truncated = truncate_content(content, 20);
        assert_eq!(truncated, "This is a long piece...");
        assert!(truncated.len() <= 23); // 20 + "..."
    }

    #[test]
    fn test_truncate_content_shorter_than_max() {
        let content = "Short";
        let result = truncate_content(content, 20);
        assert_eq!(result, "Short");
    }

    #[test]
    fn test_relative_time_just_now() {
        let now = Utc::now();
        assert_eq!(relative_time(&now), "just now");
    }

    #[test]
    fn test_relative_time_hours() {
        let dt = Utc::now() - chrono::Duration::hours(3);
        assert_eq!(relative_time(&dt), "3 hours ago");
    }

    #[test]
    fn test_relative_time_days() {
        let dt = Utc::now() - chrono::Duration::days(5);
        assert_eq!(relative_time(&dt), "5 days ago");
    }

    #[test]
    fn test_format_memory_detailed() {
        let memory = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "Test content for memory".to_string(),
            summary: "Test summary".to_string(),
            keywords: vec![],
            tags: vec![],
            context: "".to_string(),
            memory_type: MemoryType::Insight,
            importance: 8,
            confidence: 0.9,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "voyage-2".to_string(),
        };

        let formatted = format_memory(&memory, true);
        assert!(formatted.contains("Test summary"));
        assert!(formatted.contains("Insight"));
        assert!(formatted.contains("Test content"));
    }

    #[test]
    fn test_format_memory_compact() {
        let memory = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "Test content".to_string(),
            summary: "Test summary".to_string(),
            keywords: vec![],
            tags: vec![],
            context: "".to_string(),
            memory_type: MemoryType::CodePattern,
            importance: 7,
            confidence: 0.9,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "voyage-2".to_string(),
        };

        let formatted = format_memory(&memory, false);
        assert!(formatted.contains("Test summary"));
        assert!(formatted.contains("Pattern"));
        assert!(!formatted.contains("Test content")); // Compact mode doesn't include content
    }

    #[test]
    fn test_context_load_config_default() {
        let config = ContextLoadConfig::default();
        assert_eq!(config.max_memories, 10);
        assert_eq!(config.min_importance, 7);
        assert_eq!(config.max_size_bytes, 10 * 1024);
        assert!(config.include_metadata);
    }
}
