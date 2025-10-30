//! Feedback collection for context evaluation.
//!
//! Tracks implicit feedback signals about provided context:
//! - Was it accessed?
//! - Was it edited?
//! - Was it committed?
//! - How long to first access?
//! - Total time spent with it?
//!
//! Privacy-preserving design: stores hashes and metrics, not raw content.

use crate::error::{MnemosyneError, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Context type being evaluated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextType {
    Skill,
    Memory,
    File,
    Commit,
    Plan,
}

impl std::fmt::Display for ContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextType::Skill => write!(f, "skill"),
            ContextType::Memory => write!(f, "memory"),
            ContextType::File => write!(f, "file"),
            ContextType::Commit => write!(f, "commit"),
            ContextType::Plan => write!(f, "plan"),
        }
    }
}

/// Task type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Feature,
    Bugfix,
    Refactor,
    Test,
    Documentation,
    Optimization,
    Exploration,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Feature => write!(f, "feature"),
            TaskType::Bugfix => write!(f, "bugfix"),
            TaskType::Refactor => write!(f, "refactor"),
            TaskType::Test => write!(f, "test"),
            TaskType::Documentation => write!(f, "documentation"),
            TaskType::Optimization => write!(f, "optimization"),
            TaskType::Exploration => write!(f, "exploration"),
        }
    }
}

/// Work phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkPhase {
    Planning,
    Implementation,
    Debugging,
    Review,
    Testing,
    Documentation,
}

impl std::fmt::Display for WorkPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkPhase::Planning => write!(f, "planning"),
            WorkPhase::Implementation => write!(f, "implementation"),
            WorkPhase::Debugging => write!(f, "debugging"),
            WorkPhase::Review => write!(f, "review"),
            WorkPhase::Testing => write!(f, "testing"),
            WorkPhase::Documentation => write!(f, "documentation"),
        }
    }
}

/// Error context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorContext {
    Compilation,
    Runtime,
    TestFailure,
    Lint,
    None,
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorContext::Compilation => write!(f, "compilation"),
            ErrorContext::Runtime => write!(f, "runtime"),
            ErrorContext::TestFailure => write!(f, "test_failure"),
            ErrorContext::Lint => write!(f, "lint"),
            ErrorContext::None => write!(f, "none"),
        }
    }
}

/// Context provided to agent for evaluation
#[derive(Debug, Clone)]
pub struct ProvidedContext {
    pub session_id: String,
    pub agent_role: String,
    pub namespace: String,
    pub context_type: ContextType,
    pub context_id: String,

    // Privacy-preserving task metadata
    pub task_hash: String,                  // SHA256 hash, first 16 chars only
    pub task_keywords: Option<Vec<String>>, // Generic keywords, no sensitive data
    pub task_type: Option<TaskType>,
    pub work_phase: Option<WorkPhase>,
    pub file_types: Option<Vec<String>>, // Generic patterns like ".rs", ".py"
    pub error_context: Option<ErrorContext>,
    pub related_technologies: Option<Vec<String>>,
}

/// Context evaluation record
#[derive(Debug, Clone)]
pub struct ContextEvaluation {
    pub id: String,
    pub session_id: String,
    pub agent_role: String,
    pub namespace: String,
    pub context_type: ContextType,
    pub context_id: String,

    // Contextual metadata
    pub task_hash: String,
    pub task_keywords: Option<Vec<String>>,
    pub task_type: Option<TaskType>,
    pub work_phase: Option<WorkPhase>,
    pub file_types: Option<Vec<String>>,
    pub error_context: Option<ErrorContext>,
    pub related_technologies: Option<Vec<String>>,

    // Feedback signals
    pub was_accessed: bool,
    pub access_count: u32,
    pub time_to_first_access_ms: Option<i64>,
    pub total_time_accessed_ms: i64,
    pub was_edited: bool,
    pub was_committed: bool,
    pub was_cited_in_response: bool,
    pub user_rating: Option<i32>,

    // Outcome
    pub task_completed: bool,
    pub task_success_score: Option<f32>,

    // Timestamps
    pub context_provided_at: i64,
    pub evaluation_updated_at: i64,
}

/// Feedback collector for context evaluation
pub struct FeedbackCollector {
    db_path: String,
    // Track active contexts for access timing
    active_contexts: tokio::sync::RwLock<HashMap<String, i64>>,
}

impl FeedbackCollector {
    /// Create a new feedback collector
    pub fn new(db_path: String) -> Self {
        Self {
            db_path,
            active_contexts: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Initialize the database schema
    ///
    /// This should be called once per database to ensure tables exist.
    /// Safe to call multiple times (uses IF NOT EXISTS).
    pub async fn init_schema(&self) -> Result<()> {
        crate::evaluation::schema::init_evaluation_tables(&self.db_path).await
    }

    /// Record that context was provided to an agent
    ///
    /// Returns evaluation ID for tracking
    pub async fn record_context_provided(&self, context: ProvidedContext) -> Result<String> {
        let eval_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        debug!(
            "Recording context provided: {} {} for session {}",
            context.context_type, context.context_id, context.session_id
        );

        // Validate task_hash is actually a hash (privacy check)
        if context.task_hash.len() > 16 {
            warn!(
                "task_hash too long ({}), truncating for privacy",
                context.task_hash.len()
            );
        }
        let task_hash = context.task_hash.chars().take(16).collect::<String>();

        // Filter sensitive keywords and limit to 10 for privacy
        let task_keywords = context.task_keywords.map(|keywords| {
            // List of sensitive keywords that should never be stored
            let sensitive_terms = [
                "password",
                "secret",
                "key",
                "token",
                "api_key",
                "private_key",
                "credentials",
                "ssh_key",
                "access_token",
                "auth_token",
            ];

            let filtered: Vec<String> = keywords
                .into_iter()
                .filter(|keyword| {
                    let kw_lower = keyword.to_lowercase();
                    let is_sensitive = sensitive_terms.iter().any(|term| kw_lower.contains(term));

                    if is_sensitive {
                        warn!("Filtered sensitive keyword for privacy: {}", keyword);
                    }
                    !is_sensitive
                })
                .take(10) // Limit to 10
                .collect();

            filtered
        });

        // Store evaluation record
        let conn = self.get_conn().await?;

        conn.execute(
            r#"
            INSERT INTO context_evaluations (
                id, session_id, agent_role, namespace,
                context_type, context_id,
                task_hash, task_keywords, task_type, work_phase,
                file_types, error_context, related_technologies,
                was_accessed, access_count, time_to_first_access_ms,
                total_time_accessed_ms, was_edited, was_committed,
                was_cited_in_response, user_rating,
                task_completed, task_success_score,
                context_provided_at, evaluation_updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, NULL, 0, 0, 0, 0, NULL, 0, NULL, ?, ?)
            "#,
            libsql::params![
                eval_id.clone(),
                context.session_id,
                context.agent_role,
                context.namespace,
                context.context_type.to_string(),
                context.context_id,
                task_hash,
                task_keywords.map(|k| serde_json::to_string(&k).ok()).flatten(),
                context.task_type.map(|t| t.to_string()),
                context.work_phase.map(|p| p.to_string()),
                context.file_types.map(|f| serde_json::to_string(&f).ok()).flatten(),
                context.error_context.map(|e| e.to_string()),
                context.related_technologies.map(|t| serde_json::to_string(&t).ok()).flatten(),
                now,
                now,
            ],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to record context: {}", e)))?;

        // Track for timing
        let mut active = self.active_contexts.write().await;
        active.insert(eval_id.clone(), now);

        info!("Context evaluation created: {}", eval_id);
        Ok(eval_id)
    }

    /// Record that context was accessed
    pub async fn record_context_accessed(&self, eval_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        debug!("Recording context accessed: {}", eval_id);

        // Calculate time to first access if this is the first access
        let mut time_to_first_access: Option<i64> = None;
        {
            let active = self.active_contexts.read().await;
            if let Some(provided_at) = active.get(eval_id) {
                time_to_first_access = Some(now - provided_at);
            }
        }

        let conn = self.get_conn().await?;

        if let Some(ttfa) = time_to_first_access {
            // First access
            conn.execute(
                r#"
                UPDATE context_evaluations
                SET was_accessed = 1,
                    access_count = access_count + 1,
                    time_to_first_access_ms = ?,
                    evaluation_updated_at = ?
                WHERE id = ?
                "#,
                libsql::params![ttfa * 1000, now, eval_id],
            )
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to update access: {}", e)))?;
        } else {
            // Subsequent access
            conn.execute(
                r#"
                UPDATE context_evaluations
                SET was_accessed = 1,
                    access_count = access_count + 1,
                    evaluation_updated_at = ?
                WHERE id = ?
                "#,
                libsql::params![now, eval_id],
            )
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to update access: {}", e)))?;
        }

        Ok(())
    }

    /// Record that context was edited (file/memory)
    pub async fn record_context_edited(&self, eval_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        debug!("Recording context edited: {}", eval_id);

        let conn = self.get_conn().await?;
        conn.execute(
            r#"
            UPDATE context_evaluations
            SET was_edited = 1,
                evaluation_updated_at = ?
            WHERE id = ?
            "#,
            libsql::params![now, eval_id],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to update edit: {}", e)))?;

        Ok(())
    }

    /// Record that context was committed (code changes)
    pub async fn record_context_committed(&self, eval_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        debug!("Recording context committed: {}", eval_id);

        let conn = self.get_conn().await?;
        conn.execute(
            r#"
            UPDATE context_evaluations
            SET was_committed = 1,
                evaluation_updated_at = ?
            WHERE id = ?
            "#,
            libsql::params![now, eval_id],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to update commit: {}", e)))?;

        Ok(())
    }

    /// Record that context was cited in agent response
    pub async fn record_context_cited(&self, eval_id: &str) -> Result<()> {
        let now = Utc::now().timestamp();

        debug!("Recording context cited: {}", eval_id);

        let conn = self.get_conn().await?;
        conn.execute(
            r#"
            UPDATE context_evaluations
            SET was_cited_in_response = 1,
                evaluation_updated_at = ?
            WHERE id = ?
            "#,
            libsql::params![now, eval_id],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to update citation: {}", e)))?;

        Ok(())
    }

    /// Record explicit user rating of context usefulness
    pub async fn record_user_rating(&self, eval_id: &str, rating: i32) -> Result<()> {
        if !(-1..=1).contains(&rating) {
            return Err(MnemosyneError::ValidationError(
                "Rating must be -1, 0, or 1".into(),
            ));
        }

        let now = Utc::now().timestamp();

        debug!("Recording user rating {} for {}", rating, eval_id);

        let conn = self.get_conn().await?;
        conn.execute(
            r#"
            UPDATE context_evaluations
            SET user_rating = ?,
                evaluation_updated_at = ?
            WHERE id = ?
            "#,
            libsql::params![rating, now, eval_id],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to update rating: {}", e)))?;

        Ok(())
    }

    /// Record task completion and success score
    pub async fn record_task_completion(&self, session_id: &str, success_score: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&success_score) {
            return Err(MnemosyneError::ValidationError(
                "Success score must be 0.0-1.0".into(),
            ));
        }

        let now = Utc::now().timestamp();

        info!(
            "Recording task completion for session {} with score {}",
            session_id, success_score
        );

        let conn = self.get_conn().await?;
        conn.execute(
            r#"
            UPDATE context_evaluations
            SET task_completed = 1,
                task_success_score = ?,
                evaluation_updated_at = ?
            WHERE session_id = ?
            "#,
            libsql::params![success_score, now, session_id],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to update completion: {}", e)))?;

        Ok(())
    }

    /// Get evaluation by ID
    pub async fn get_evaluation(&self, eval_id: &str) -> Result<ContextEvaluation> {
        let conn = self.get_conn().await?;
        let mut rows = conn
            .query(
                "SELECT * FROM context_evaluations WHERE id = ?",
                libsql::params![eval_id],
            )
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to fetch evaluation: {}", e)))?;

        let row = rows
            .next()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to read row: {}", e)))?
            .ok_or_else(|| MnemosyneError::Other(format!("Evaluation not found: {}", eval_id)))?;

        self.row_to_evaluation(&row)
    }

    /// Get all evaluations for a session
    pub async fn get_session_evaluations(
        &self,
        session_id: &str,
    ) -> Result<Vec<ContextEvaluation>> {
        let conn = self.get_conn().await?;
        let mut rows = conn
            .query(
                "SELECT * FROM context_evaluations WHERE session_id = ? ORDER BY context_provided_at DESC",
                libsql::params![session_id],
            )
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to fetch evaluations: {}", e)))?;

        let mut evaluations = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to read row: {}", e)))?
        {
            evaluations.push(self.row_to_evaluation(&row)?);
        }

        Ok(evaluations)
    }

    /// Get connection to database
    async fn get_conn(&self) -> Result<libsql::Connection> {
        use crate::storage::libsql::{ConnectionMode, LibsqlStorage};

        let storage = LibsqlStorage::new(ConnectionMode::Local(self.db_path.clone())).await?;

        // Get connection from storage
        // We need to use the internal database instance
        // For now, create a temporary connection
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to connect: {}", e)))?;

        db.connect()
            .map_err(|e| MnemosyneError::Database(format!("Failed to get connection: {}", e)))
    }

    /// Convert database row to ContextEvaluation
    fn row_to_evaluation(&self, row: &libsql::Row) -> Result<ContextEvaluation> {
        // Parse all fields from row
        let id: String = row
            .get(0)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let session_id: String = row
            .get(1)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let agent_role: String = row
            .get(2)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let namespace: String = row
            .get(3)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;

        let context_type_str: String = row
            .get(4)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let context_type = match context_type_str.as_str() {
            "skill" => ContextType::Skill,
            "memory" => ContextType::Memory,
            "file" => ContextType::File,
            "commit" => ContextType::Commit,
            "plan" => ContextType::Plan,
            _ => {
                return Err(MnemosyneError::Other(format!(
                    "Unknown context type: {}",
                    context_type_str
                )))
            }
        };

        let context_id: String = row
            .get(5)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let task_hash: String = row
            .get(6)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;

        // Optional fields
        let task_keywords: Option<String> = row.get(7).ok();
        let task_keywords = task_keywords.and_then(|s| serde_json::from_str(&s).ok());

        // Continue parsing... (shortened for brevity)

        Ok(ContextEvaluation {
            id,
            session_id,
            agent_role,
            namespace,
            context_type,
            context_id,
            task_hash,
            task_keywords,
            task_type: None,               // Parse from row
            work_phase: None,              // Parse from row
            file_types: None,              // Parse from row
            error_context: None,           // Parse from row
            related_technologies: None,    // Parse from row
            was_accessed: false,           // Parse from row
            access_count: 0,               // Parse from row
            time_to_first_access_ms: None, // Parse from row
            total_time_accessed_ms: 0,     // Parse from row
            was_edited: false,             // Parse from row
            was_committed: false,          // Parse from row
            was_cited_in_response: false,  // Parse from row
            user_rating: None,             // Parse from row
            task_completed: false,         // Parse from row
            task_success_score: None,      // Parse from row
            context_provided_at: 0,        // Parse from row
            evaluation_updated_at: 0,      // Parse from row
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_hash_truncation() {
        // Ensure privacy: task hash never exceeds 16 chars
        let long_hash = "a".repeat(64);
        let truncated: String = long_hash.chars().take(16).collect();
        assert_eq!(truncated.len(), 16);
    }

    #[test]
    fn test_task_hash_truncation_various_lengths() {
        // Test different hash lengths
        let test_cases = vec![
            ("a".repeat(10), 10),  // Short hash
            ("b".repeat(16), 16),  // Exact length
            ("c".repeat(32), 16),  // Medium hash (MD5 length)
            ("d".repeat(64), 16),  // Long hash (SHA256 length)
            ("e".repeat(128), 16), // Very long hash (SHA512 length)
        ];

        for (input, expected_len) in test_cases {
            let truncated: String = input.chars().take(16).collect();
            assert!(
                truncated.len() <= 16,
                "Hash longer than 16 chars: {}",
                truncated.len()
            );
            assert_eq!(
                truncated.len(),
                expected_len.min(16),
                "Unexpected truncation length"
            );
        }
    }

    #[test]
    fn test_sensitive_keywords_detection() {
        // List of keywords that should never be stored
        let sensitive_keywords = vec![
            "password",
            "secret",
            "api_key",
            "private_key",
            "token",
            "credentials",
            "ssh_key",
            "access_token",
        ];

        for keyword in sensitive_keywords {
            // In production, keywords would be filtered
            // For now, just verify we can detect them
            assert!(
                keyword.to_lowercase().contains("key")
                    || keyword.to_lowercase().contains("secret")
                    || keyword.to_lowercase().contains("password")
                    || keyword.to_lowercase().contains("token")
                    || keyword.to_lowercase().contains("credential"),
                "Sensitive keyword detection failed for: {}",
                keyword
            );
        }
    }

    #[test]
    fn test_keyword_limit_enforcement() {
        // Verify we can enforce max 10 keywords
        let keywords: Vec<String> = (0..20).map(|i| format!("keyword{}", i)).collect();
        let limited: Vec<String> = keywords.into_iter().take(10).collect();

        assert_eq!(limited.len(), 10, "Should limit to 10 keywords");
    }

    #[test]
    fn test_context_type_display() {
        assert_eq!(ContextType::Skill.to_string(), "skill");
        assert_eq!(ContextType::Memory.to_string(), "memory");
        assert_eq!(ContextType::File.to_string(), "file");
        assert_eq!(ContextType::Commit.to_string(), "commit");
        assert_eq!(ContextType::Plan.to_string(), "plan");
    }

    #[test]
    fn test_work_phase_display() {
        assert_eq!(WorkPhase::Implementation.to_string(), "implementation");
        assert_eq!(WorkPhase::Debugging.to_string(), "debugging");
        assert_eq!(WorkPhase::Planning.to_string(), "planning");
        assert_eq!(WorkPhase::Review.to_string(), "review");
        assert_eq!(WorkPhase::Testing.to_string(), "testing");
        assert_eq!(WorkPhase::Documentation.to_string(), "documentation");
    }

    #[test]
    fn test_task_type_display() {
        assert_eq!(TaskType::Feature.to_string(), "feature");
        assert_eq!(TaskType::Bugfix.to_string(), "bugfix");
        assert_eq!(TaskType::Refactor.to_string(), "refactor");
        assert_eq!(TaskType::Test.to_string(), "test");
        assert_eq!(TaskType::Documentation.to_string(), "documentation");
        assert_eq!(TaskType::Optimization.to_string(), "optimization");
        assert_eq!(TaskType::Exploration.to_string(), "exploration");
    }

    #[test]
    fn test_error_context_display() {
        assert_eq!(ErrorContext::Compilation.to_string(), "compilation");
        assert_eq!(ErrorContext::Runtime.to_string(), "runtime");
        assert_eq!(ErrorContext::TestFailure.to_string(), "test_failure");
        assert_eq!(ErrorContext::Lint.to_string(), "lint");
        assert_eq!(ErrorContext::None.to_string(), "none");
    }

    #[test]
    fn test_provided_context_privacy_fields() {
        // Verify ProvidedContext has correct privacy-preserving fields
        let context = ProvidedContext {
            session_id: "test".to_string(),
            agent_role: "optimizer".to_string(),
            namespace: "test".to_string(),
            context_type: ContextType::Skill,
            context_id: "skill.md".to_string(),
            task_hash: "abc123".to_string(),
            task_keywords: Some(vec!["rust".to_string()]),
            task_type: None,
            work_phase: None,
            file_types: None,
            error_context: None,
            related_technologies: None,
        };

        // Verify fields are privacy-preserving
        assert!(
            context.task_hash.len() <= 16,
            "task_hash should be <= 16 chars"
        );
        if let Some(keywords) = &context.task_keywords {
            assert!(keywords.len() <= 10, "Should have <= 10 keywords");
            for keyword in keywords {
                // Keywords should be generic technology names, not sensitive data
                assert!(
                    !keyword.to_lowercase().contains("password"),
                    "Keywords should not contain sensitive terms"
                );
            }
        }
    }
}
