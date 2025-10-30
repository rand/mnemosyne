//! Agent-specific memory views with role-based filtering
//!
//! This module implements typed hole #6 (AgentRole) and typed hole #7 (AgentMemoryView)
//! from the v2.0 specification, providing role-specific memory access for the
//! multi-agent architecture.

use crate::error::{MnemosyneError, Result};
use crate::storage::{MemorySortOrder, StorageBackend};
use crate::types::{MemoryNote, MemoryType, Namespace};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Agent roles in the multi-agent system
///
/// Each role has specific memory type interests and access patterns:
/// - **Orchestrator**: Coordinates execution, focuses on decisions and architecture
/// - **Optimizer**: Manages context and skills, focuses on patterns and decisions
/// - **Reviewer**: Quality assurance, focuses on bugs, tests, and quality standards
/// - **Executor**: Primary implementation, focuses on code patterns and bug fixes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Central coordinator and state manager
    Orchestrator,

    /// Context and resource optimization specialist
    Optimizer,

    /// Quality assurance and validation specialist
    Reviewer,

    /// Primary work agent and sub-agent manager
    Executor,
}

impl AgentRole {
    /// Get memory types relevant to this agent role
    ///
    /// This implements the role-to-memory-type mapping from the specification:
    /// - Orchestrator sees: Decisions, Architecture, Coordination
    /// - Optimizer sees: Decisions, Patterns, Skills
    /// - Reviewer sees: Bugs, Tests, Quality Standards
    /// - Executor sees: Implementation, Patterns, Bug Fixes
    pub fn memory_types(&self) -> Vec<MemoryType> {
        match self {
            AgentRole::Orchestrator => vec![
                MemoryType::ArchitectureDecision,
                // Coordination type will be added in v2.0
                MemoryType::Constraint,
            ],
            AgentRole::Optimizer => vec![
                MemoryType::ArchitectureDecision,
                MemoryType::CodePattern,
                MemoryType::Insight,
            ],
            AgentRole::Reviewer => vec![
                MemoryType::BugFix,
                MemoryType::Constraint,
                MemoryType::ArchitectureDecision,
            ],
            AgentRole::Executor => vec![
                MemoryType::CodePattern,
                MemoryType::BugFix,
                MemoryType::Entity,
            ],
        }
    }

    /// Get default visibility for memories created by this agent
    ///
    /// Determines which other agents can see memories created by this role:
    /// - Orchestrator memories visible to: Orchestrator, Optimizer
    /// - Optimizer memories visible to: Optimizer, Executor
    /// - Reviewer memories visible to: Reviewer, Executor
    /// - Executor memories visible to: Executor, Reviewer
    pub fn default_visibility(&self) -> Vec<AgentRole> {
        match self {
            AgentRole::Orchestrator => vec![AgentRole::Orchestrator, AgentRole::Optimizer],
            AgentRole::Optimizer => vec![AgentRole::Optimizer, AgentRole::Executor],
            AgentRole::Reviewer => vec![AgentRole::Reviewer, AgentRole::Executor],
            AgentRole::Executor => vec![AgentRole::Executor, AgentRole::Reviewer],
        }
    }
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Orchestrator => write!(f, "orchestrator"),
            AgentRole::Optimizer => write!(f, "optimizer"),
            AgentRole::Reviewer => write!(f, "reviewer"),
            AgentRole::Executor => write!(f, "executor"),
        }
    }
}

impl std::str::FromStr for AgentRole {
    type Err = MnemosyneError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "orchestrator" => Ok(AgentRole::Orchestrator),
            "optimizer" => Ok(AgentRole::Optimizer),
            "reviewer" => Ok(AgentRole::Reviewer),
            "executor" => Ok(AgentRole::Executor),
            _ => Err(MnemosyneError::InvalidAgentRole(s.to_string())),
        }
    }
}

/// Agent-specific memory view with role-based filtering
///
/// Implements typed hole #7 from the specification. Provides filtered access
/// to memories based on agent role, restricting results to relevant memory types.
///
/// # Example
///
/// ```ignore
/// let view = AgentMemoryView::new(AgentRole::Executor, storage);
///
/// // Only returns Implementation, Pattern, and BugFix memories
/// let memories = view.search("error handling pattern", 10).await?;
///
/// // Only returns recently accessed memories of relevant types
/// let recent = view.list_recent(20).await?;
/// ```
pub struct AgentMemoryView<S: StorageBackend> {
    /// Agent role for this view
    role: AgentRole,

    /// Storage backend
    storage: Arc<S>,
}

impl<S: StorageBackend> AgentMemoryView<S> {
    /// Create a new agent memory view
    pub fn new(role: AgentRole, storage: Arc<S>) -> Self {
        Self { role, storage }
    }

    /// Get the agent role for this view
    pub fn role(&self) -> AgentRole {
        self.role
    }

    /// Search memories with role-based type filtering
    ///
    /// Automatically injects memory type filters based on agent role,
    /// ensuring agents only see relevant memories.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryNote>> {
        // Use hybrid search from storage backend
        let results = self
            .storage
            .hybrid_search(query, None, limit, false)
            .await?;

        // Filter by role-specific memory types
        let relevant_types = self.role.memory_types();
        let filtered = results
            .into_iter()
            .map(|r| r.memory)
            .filter(|m| relevant_types.contains(&m.memory_type))
            .take(limit)
            .collect();

        Ok(filtered)
    }

    /// Search with additional filters
    ///
    /// Allows specifying namespace, tags, etc. while maintaining role-based type filtering.
    pub async fn search_with_filters(
        &self,
        query: &str,
        namespace: Option<Namespace>,
        min_importance: Option<u8>,
        limit: usize,
    ) -> Result<Vec<MemoryNote>> {
        // Use hybrid search from storage backend
        let results = self
            .storage
            .hybrid_search(query, namespace, limit * 2, false)
            .await?;

        // Filter by role-specific memory types and importance
        let relevant_types = self.role.memory_types();
        let filtered = results
            .into_iter()
            .map(|r| r.memory)
            .filter(|m| {
                relevant_types.contains(&m.memory_type)
                    && min_importance.map_or(true, |min| m.importance >= min)
            })
            .take(limit)
            .collect();

        Ok(filtered)
    }

    /// List recent memories of relevant types
    ///
    /// Returns recently accessed memories filtered by role-specific types,
    /// ordered by last_accessed_at descending.
    pub async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryNote>> {
        // Get recent memories from storage
        let memories = self
            .storage
            .list_memories(None, limit * 2, MemorySortOrder::Recent)
            .await?;

        // Filter by role-specific memory types
        let relevant_types = self.role.memory_types();
        let filtered = memories
            .into_iter()
            .filter(|m| relevant_types.contains(&m.memory_type))
            .take(limit)
            .collect();

        Ok(filtered)
    }

    /// List high-importance memories
    ///
    /// Returns memories with importance >= threshold, filtered by role-specific types.
    pub async fn list_high_importance(
        &self,
        threshold: u8,
        limit: usize,
    ) -> Result<Vec<MemoryNote>> {
        // Get important memories sorted by importance
        let memories = self
            .storage
            .list_memories(None, limit * 2, MemorySortOrder::Importance)
            .await?;

        // Filter by role-specific memory types and importance threshold
        let relevant_types = self.role.memory_types();
        let filtered = memories
            .into_iter()
            .filter(|m| relevant_types.contains(&m.memory_type) && m.importance >= threshold)
            .take(limit)
            .collect();

        Ok(filtered)
    }

    /// Check if a memory is visible to this agent
    ///
    /// Determines visibility based on:
    /// 1. Memory type matches role interests
    /// 2. Visible_to field includes this role (checked in v2.0)
    pub fn is_visible(&self, memory: &MemoryNote) -> bool {
        self.role.memory_types().contains(&memory.memory_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::libsql::LibsqlStorage;
    use crate::types::{MemoryId, Namespace};
    use chrono::Utc;
    use tempfile::tempdir;

    fn create_test_memory(memory_type: MemoryType, importance: u8) -> MemoryNote {
        MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "Test memory content".to_string(),
            summary: "Test summary".to_string(),
            keywords: vec!["test".to_string()],
            tags: vec![],
            context: "Test context".to_string(),
            memory_type,
            importance,
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
            embedding_model: "test".to_string(),
        }
    }

    #[test]
    fn test_agent_role_memory_types() {
        assert_eq!(
            AgentRole::Orchestrator.memory_types(),
            vec![MemoryType::ArchitectureDecision, MemoryType::Constraint]
        );

        assert_eq!(
            AgentRole::Executor.memory_types(),
            vec![
                MemoryType::CodePattern,
                MemoryType::BugFix,
                MemoryType::Entity
            ]
        );
    }

    #[test]
    fn test_agent_role_default_visibility() {
        let orchestrator_vis = AgentRole::Orchestrator.default_visibility();
        assert!(orchestrator_vis.contains(&AgentRole::Orchestrator));
        assert!(orchestrator_vis.contains(&AgentRole::Optimizer));

        let executor_vis = AgentRole::Executor.default_visibility();
        assert!(executor_vis.contains(&AgentRole::Executor));
        assert!(executor_vis.contains(&AgentRole::Reviewer));
    }

    #[test]
    fn test_agent_role_from_str() {
        use std::str::FromStr;

        assert_eq!(
            AgentRole::from_str("orchestrator").unwrap(),
            AgentRole::Orchestrator
        );
        assert_eq!(
            AgentRole::from_str("Executor").unwrap(),
            AgentRole::Executor
        );
        assert!(AgentRole::from_str("invalid").is_err());
    }

    #[test]
    fn test_is_visible() {
        let executor = AgentRole::Executor;

        let pattern_memory = create_test_memory(MemoryType::CodePattern, 8);
        let decision_memory = create_test_memory(MemoryType::ArchitectureDecision, 9);

        // Mock storage for testing visibility
        // In real usage, we'd check visible_to field from database
        assert!(executor
            .memory_types()
            .contains(&pattern_memory.memory_type));
        assert!(!executor
            .memory_types()
            .contains(&decision_memory.memory_type));
    }

    #[tokio::test]
    async fn test_agent_memory_view_search() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = LibsqlStorage::new_with_validation(
            crate::storage::libsql::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .expect("Failed to create storage");

        let view = AgentMemoryView::new(AgentRole::Executor, Arc::new(storage));

        // Search should automatically filter by executor-relevant types
        let results = view.search("test", 10).await;
        assert!(results.is_ok());
    }
}
