//! Handoff coordination between Claude Code and ICS
//!
//! File-based protocol for seamless context editing integration.

use anyhow::{Context as AnyhowContext, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

/// Edit intent - what Claude Code wants ICS to do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditIntent {
    /// Unique session ID
    pub session_id: String,

    /// Timestamp when intent was created
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Action to perform (always "edit" for now)
    pub action: String,

    /// File path to edit
    pub file_path: PathBuf,

    /// Optional template to use (api, architecture, bugfix, feature, refactor)
    pub template: Option<String>,

    /// Read-only mode
    pub readonly: bool,

    /// Panel to open (memory, diagnostics, proposals, holes)
    pub panel: Option<String>,

    /// Context from conversation
    pub context: EditContext,
}

/// Context from the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditContext {
    /// Summary of conversation so far
    pub conversation_summary: String,

    /// Relevant memory IDs
    pub relevant_memories: Vec<String>,

    /// Related files mentioned in conversation
    pub related_files: Vec<String>,
}

/// Edit result - what ICS produced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    /// Session ID (matches intent)
    pub session_id: String,

    /// Timestamp when editing finished
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Status: "completed", "cancelled", "error"
    pub status: String,

    /// File path that was edited
    pub file_path: PathBuf,

    /// Were changes made?
    pub changes_made: bool,

    /// Why did editing end?
    pub exit_reason: ExitReason,

    /// Optional semantic analysis summary
    pub analysis: Option<SemanticAnalysisSummary>,

    /// Optional error message
    pub error: Option<String>,
}

/// Reason for exiting ICS
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    /// User saved and quit
    UserSaved,
    /// User quit without saving
    UserCancelled,
    /// Error occurred
    Error,
    /// Timeout
    Timeout,
}

/// Summary of semantic analysis performed in ICS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysisSummary {
    /// Number of typed holes filled
    pub holes_filled: usize,

    /// Number of memories referenced
    pub memories_referenced: usize,

    /// Number of diagnostics resolved
    pub diagnostics_resolved: usize,

    /// Entities extracted
    pub entities: Vec<String>,

    /// Relationships discovered
    pub relationships: Vec<String>,
}

/// Handoff coordinator for file-based protocol
pub struct HandoffCoordinator {
    /// Session directory (.claude/sessions/)
    session_dir: PathBuf,
}

impl HandoffCoordinator {
    /// Create new coordinator with session directory
    pub fn new(session_dir: PathBuf) -> Result<Self> {
        // Ensure session directory exists
        if !session_dir.exists() {
            std::fs::create_dir_all(&session_dir)
                .with_context(|| format!("Failed to create session directory: {:?}", session_dir))?;
        }

        Ok(Self { session_dir })
    }

    /// Write edit intent for ICS to read
    pub fn write_intent(&self, intent: &EditIntent) -> Result<PathBuf> {
        let intent_path = self.session_dir.join("edit-intent.json");

        let json = serde_json::to_string_pretty(intent)
            .context("Failed to serialize edit intent")?;

        std::fs::write(&intent_path, json)
            .with_context(|| format!("Failed to write intent to {:?}", intent_path))?;

        Ok(intent_path)
    }

    /// Read edit result from ICS (with timeout)
    pub async fn read_result(&self, max_wait: Duration) -> Result<EditResult> {
        let result_path = self.session_dir.join("edit-result.json");

        // Poll for result file with timeout
        let result = timeout(max_wait, async {
            loop {
                if result_path.exists() {
                    // File exists, try to read it
                    match std::fs::read_to_string(&result_path) {
                        Ok(json) => {
                            // Try to parse
                            match serde_json::from_str::<EditResult>(&json) {
                                Ok(result) => return Ok(result),
                                Err(e) => {
                                    // Invalid JSON, wait a bit and retry (file might be being written)
                                    tracing::debug!("Failed to parse result JSON, retrying: {}", e);
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            // Failed to read, wait and retry
                            tracing::debug!("Failed to read result file, retrying: {}", e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    }
                }

                // File doesn't exist yet, wait a bit
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .context("Timeout waiting for ICS result")?;

        result
    }

    /// Write edit result (called by ICS)
    pub fn write_result(&self, result: &EditResult) -> Result<PathBuf> {
        let result_path = self.session_dir.join("edit-result.json");

        let json = serde_json::to_string_pretty(result)
            .context("Failed to serialize edit result")?;

        std::fs::write(&result_path, json)
            .with_context(|| format!("Failed to write result to {:?}", result_path))?;

        Ok(result_path)
    }

    /// Read edit intent (called by ICS)
    pub fn read_intent(&self) -> Result<EditIntent> {
        let intent_path = self.session_dir.join("edit-intent.json");

        let json = std::fs::read_to_string(&intent_path)
            .with_context(|| format!("Failed to read intent from {:?}", intent_path))?;

        serde_json::from_str(&json)
            .context("Failed to parse edit intent JSON")
    }

    /// Clean up coordination files
    pub fn cleanup(&self) -> Result<()> {
        let intent_path = self.session_dir.join("edit-intent.json");
        let result_path = self.session_dir.join("edit-result.json");

        if intent_path.exists() {
            std::fs::remove_file(&intent_path)
                .context("Failed to remove intent file")?;
        }

        if result_path.exists() {
            std::fs::remove_file(&result_path)
                .context("Failed to remove result file")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_read_intent() {
        let temp_dir = TempDir::new().unwrap();
        let coordinator = HandoffCoordinator::new(temp_dir.path().to_path_buf()).unwrap();

        let intent = EditIntent {
            session_id: "test-123".to_string(),
            timestamp: chrono::Utc::now(),
            action: "edit".to_string(),
            file_path: PathBuf::from("/tmp/test.md"),
            template: Some("api".to_string()),
            readonly: false,
            panel: Some("memory".to_string()),
            context: EditContext {
                conversation_summary: "User wants to design an API".to_string(),
                relevant_memories: vec!["mem_123".to_string()],
                related_files: vec!["src/api.rs".to_string()],
            },
        };

        // Write intent
        coordinator.write_intent(&intent).unwrap();

        // Read it back
        let read_intent = coordinator.read_intent().unwrap();

        assert_eq!(read_intent.session_id, "test-123");
        assert_eq!(read_intent.template, Some("api".to_string()));
        assert_eq!(read_intent.panel, Some("memory".to_string()));
    }

    #[test]
    fn test_write_read_result() {
        let temp_dir = TempDir::new().unwrap();
        let coordinator = HandoffCoordinator::new(temp_dir.path().to_path_buf()).unwrap();

        let result = EditResult {
            session_id: "test-123".to_string(),
            timestamp: chrono::Utc::now(),
            status: "completed".to_string(),
            file_path: PathBuf::from("/tmp/test.md"),
            changes_made: true,
            exit_reason: ExitReason::UserSaved,
            analysis: Some(SemanticAnalysisSummary {
                holes_filled: 3,
                memories_referenced: 2,
                diagnostics_resolved: 1,
                entities: vec!["User".to_string(), "API".to_string()],
                relationships: vec!["User -> API".to_string()],
            }),
            error: None,
        };

        // Write result
        coordinator.write_result(&result).unwrap();

        // Read it back (using a small timeout for testing)
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let read_result = runtime.block_on(async {
            coordinator.read_result(Duration::from_secs(1)).await
        }).unwrap();

        assert_eq!(read_result.session_id, "test-123");
        assert_eq!(read_result.status, "completed");
        assert!(read_result.changes_made);
        assert!(read_result.analysis.is_some());
    }

    #[test]
    fn test_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let coordinator = HandoffCoordinator::new(temp_dir.path().to_path_buf()).unwrap();

        // Create both files
        let intent = EditIntent {
            session_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            action: "edit".to_string(),
            file_path: PathBuf::from("/tmp/test.md"),
            template: None,
            readonly: false,
            panel: None,
            context: EditContext {
                conversation_summary: "Test".to_string(),
                relevant_memories: vec![],
                related_files: vec![],
            },
        };

        let result = EditResult {
            session_id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            status: "completed".to_string(),
            file_path: PathBuf::from("/tmp/test.md"),
            changes_made: false,
            exit_reason: ExitReason::UserCancelled,
            analysis: None,
            error: None,
        };

        coordinator.write_intent(&intent).unwrap();
        coordinator.write_result(&result).unwrap();

        // Verify files exist
        assert!(temp_dir.path().join("edit-intent.json").exists());
        assert!(temp_dir.path().join("edit-result.json").exists());

        // Cleanup
        coordinator.cleanup().unwrap();

        // Verify files are gone
        assert!(!temp_dir.path().join("edit-intent.json").exists());
        assert!(!temp_dir.path().join("edit-result.json").exists());
    }
}
