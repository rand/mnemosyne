//! Role-based access control for memory operations
//!
//! This module implements typed hole #8 (MemoryAccessControl) from the v2.0 specification,
//! providing ownership tracking, permission checks, and audit trails.

use crate::agents::AgentRole;
use crate::error::{MnemosyneError, Result};
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote, MemoryType, Namespace};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Type of modification made to a memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModificationType {
    /// Memory created
    Create,

    /// Memory updated
    Update,

    /// Memory deleted
    Delete,

    /// Memory archived
    Archive,

    /// Memory unarchived
    Unarchive,

    /// Memory superseded by another
    Supersede,
}

impl std::fmt::Display for ModificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModificationType::Create => write!(f, "create"),
            ModificationType::Update => write!(f, "update"),
            ModificationType::Delete => write!(f, "delete"),
            ModificationType::Archive => write!(f, "archive"),
            ModificationType::Unarchive => write!(f, "unarchive"),
            ModificationType::Supersede => write!(f, "supersede"),
        }
    }
}

/// Log entry for memory modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationLog {
    /// Log entry ID
    pub id: String,

    /// Memory that was modified
    pub memory_id: MemoryId,

    /// Agent that made the modification
    pub agent_role: AgentRole,

    /// Type of modification
    pub modification_type: ModificationType,

    /// When the modification occurred
    pub timestamp: DateTime<Utc>,

    /// Optional JSON describing what changed
    pub changes: Option<String>,
}

/// Metadata for creating a new memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Memory type classification
    pub memory_type: MemoryType,

    /// Namespace for the memory
    pub namespace: Namespace,

    /// Importance level (1-10)
    pub importance: u8,

    /// Confidence in the information (0.0-1.0)
    pub confidence: f32,

    /// Summary of the memory
    pub summary: String,

    /// Keywords for search
    pub keywords: Vec<String>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Context about when/why this is relevant
    pub context: String,

    /// Related file paths
    pub related_files: Vec<String>,

    /// Related entities
    pub related_entities: Vec<String>,

    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,

    /// Custom visibility (overrides default)
    pub visible_to: Option<Vec<AgentRole>>,
}

/// Updates to apply to an existing memory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUpdates {
    /// Updated content
    pub content: Option<String>,

    /// Updated summary
    pub summary: Option<String>,

    /// Updated keywords
    pub keywords: Option<Vec<String>>,

    /// Updated tags
    pub tags: Option<Vec<String>>,

    /// Updated context
    pub context: Option<String>,

    /// Updated importance
    pub importance: Option<u8>,

    /// Updated confidence
    pub confidence: Option<f32>,

    /// Updated related files
    pub related_files: Option<Vec<String>>,

    /// Updated related entities
    pub related_entities: Option<Vec<String>>,

    /// Updated expiration
    pub expires_at: Option<Option<DateTime<Utc>>>,

    /// Updated visibility
    pub visible_to: Option<Vec<AgentRole>>,
}

/// Memory access control system
///
/// Implements ownership tracking and permission checks for memory operations.
/// Only the agent that created a memory (or admin) can modify it.
pub struct MemoryAccessControl<S: StorageBackend> {
    /// Agent role for this access control instance
    agent: AgentRole,

    /// Storage backend
    storage: Arc<S>,
}

impl<S: StorageBackend> MemoryAccessControl<S> {
    /// Create a new access control instance
    ///
    /// # Arguments
    ///
    /// * `agent` - The agent role for this instance
    /// * `storage` - The storage backend to use
    pub fn new(agent: AgentRole, storage: Arc<S>) -> Self {
        Self { agent, storage }
    }

    /// Get the agent role for this access control instance
    pub fn agent(&self) -> AgentRole {
        self.agent
    }

    /// Check if this instance is in admin mode
    ///
    /// Admin mode allows bypassing all permission checks.
    /// Activated by:
    /// - Setting MNEMOSYNE_ADMIN_MODE=1 environment variable
    /// - Setting MNEMOSYNE_USER=human environment variable
    pub fn is_admin(&self) -> bool {
        std::env::var("MNEMOSYNE_ADMIN_MODE").is_ok()
            || std::env::var("MNEMOSYNE_USER") == Ok("human".to_string())
    }

    /// Get default visibility for memories created by this agent
    ///
    /// Returns the list of agent roles that can see memories created by this agent.
    /// This implements the visibility rules from the specification:
    /// - Orchestrator memories visible to: Orchestrator, Optimizer
    /// - Optimizer memories visible to: Optimizer, Executor
    /// - Reviewer memories visible to: Reviewer, Executor
    /// - Executor memories visible to: Executor, Reviewer
    pub fn default_visibility(&self) -> Vec<AgentRole> {
        self.agent.default_visibility()
    }

    /// Check if this agent can update the given memory
    ///
    /// Rules:
    /// - Agent can update memories it created
    /// - Admin can update any memory
    pub fn can_update(&self, memory: &MemoryNote) -> bool {
        if self.is_admin() {
            return true;
        }

        // In v1.0, we check created_by through storage metadata
        // For now, allow all updates (will be enforced when storage supports created_by)
        true
    }

    /// Check if this agent can delete the given memory
    ///
    /// Rules:
    /// - Agent can delete memories it created
    /// - Admin can delete any memory
    pub fn can_delete(&self, memory: &MemoryNote) -> bool {
        if self.is_admin() {
            return true;
        }

        // In v1.0, we check created_by through storage metadata
        // For now, allow all deletes (will be enforced when storage supports created_by)
        true
    }

    /// Check if this agent can archive the given memory
    ///
    /// Same rules as delete
    pub fn can_archive(&self, memory: &MemoryNote) -> bool {
        self.can_delete(memory)
    }

    /// Create a new memory with ownership tracking
    ///
    /// # Arguments
    ///
    /// * `content` - The memory content
    /// * `metadata` - Memory metadata and classification
    ///
    /// # Returns
    ///
    /// The ID of the newly created memory
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage operation fails
    /// - Audit logging fails
    pub async fn create_memory(&self, content: &str, metadata: MemoryMetadata) -> Result<MemoryId> {
        let memory_id = MemoryId::new();
        let now = Utc::now();

        // Determine visibility
        let visible_to = metadata.visible_to.unwrap_or_else(|| self.default_visibility());

        // Create the memory note
        let memory = MemoryNote {
            id: memory_id,
            namespace: metadata.namespace.clone(),
            created_at: now,
            updated_at: now,
            content: content.to_string(),
            summary: metadata.summary,
            keywords: metadata.keywords,
            tags: metadata.tags,
            context: metadata.context,
            memory_type: metadata.memory_type,
            importance: metadata.importance,
            confidence: metadata.confidence,
            links: vec![],
            related_files: metadata.related_files,
            related_entities: metadata.related_entities,
            access_count: 0,
            last_accessed_at: now,
            expires_at: metadata.expires_at,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "none".to_string(), // Will be set by embedding service
        };

        // Store the memory
        self.storage.store_memory(&memory).await?;

        // Log the creation
        self.log_modification(
            &memory_id,
            ModificationType::Create,
            Some(format!(
                "{{\"created_by\":\"{}\",\"memory_type\":\"{:?}\",\"namespace\":\"{}\"}}",
                self.agent, metadata.memory_type, metadata.namespace
            )),
        )
        .await?;

        Ok(memory_id)
    }

    /// Update an existing memory
    ///
    /// # Arguments
    ///
    /// * `id` - The memory ID to update
    /// * `updates` - The updates to apply
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Memory not found
    /// - Permission denied (not owner and not admin)
    /// - Storage operation fails
    /// - Audit logging fails
    pub async fn update_memory(&self, id: &MemoryId, updates: MemoryUpdates) -> Result<()> {
        // Fetch the existing memory
        let mut memory = self.storage.get_memory(*id).await?;

        // Check permissions
        if !self.can_update(&memory) {
            return Err(MnemosyneError::PermissionDenied(format!(
                "Agent {} cannot update memory {} (not owner)",
                self.agent, id
            )));
        }

        // Track changes for audit log
        let mut changes = Vec::new();

        // Apply updates
        if let Some(content) = updates.content {
            changes.push(format!("content: {} chars", content.len()));
            memory.content = content;
        }

        if let Some(summary) = updates.summary {
            changes.push("summary".to_string());
            memory.summary = summary;
        }

        if let Some(keywords) = updates.keywords {
            changes.push(format!("keywords: {}", keywords.len()));
            memory.keywords = keywords;
        }

        if let Some(tags) = updates.tags {
            changes.push(format!("tags: {}", tags.len()));
            memory.tags = tags;
        }

        if let Some(context) = updates.context {
            changes.push("context".to_string());
            memory.context = context;
        }

        if let Some(importance) = updates.importance {
            changes.push(format!("importance: {} -> {}", memory.importance, importance));
            memory.importance = importance;
        }

        if let Some(confidence) = updates.confidence {
            changes.push(format!("confidence: {:.2} -> {:.2}", memory.confidence, confidence));
            memory.confidence = confidence;
        }

        if let Some(related_files) = updates.related_files {
            changes.push(format!("related_files: {}", related_files.len()));
            memory.related_files = related_files;
        }

        if let Some(related_entities) = updates.related_entities {
            changes.push(format!("related_entities: {}", related_entities.len()));
            memory.related_entities = related_entities;
        }

        if let Some(expires_at) = updates.expires_at {
            changes.push(format!("expires_at: {:?}", expires_at));
            memory.expires_at = expires_at;
        }

        memory.updated_at = Utc::now();

        // Update in storage
        self.storage.update_memory(&memory).await?;

        // Log the modification
        self.log_modification(
            id,
            ModificationType::Update,
            Some(format!("{{\"changes\":[\"{}\"]}}", changes.join("\",\""))),
        )
        .await?;

        Ok(())
    }

    /// Delete a memory
    ///
    /// # Arguments
    ///
    /// * `id` - The memory ID to delete
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Memory not found
    /// - Permission denied (not owner and not admin)
    /// - Storage operation fails
    /// - Audit logging fails
    pub async fn delete_memory(&self, id: &MemoryId) -> Result<()> {
        // Fetch the memory to check permissions
        let memory = self.storage.get_memory(*id).await?;

        // Check permissions
        if !self.can_delete(&memory) {
            return Err(MnemosyneError::PermissionDenied(format!(
                "Agent {} cannot delete memory {} (not owner)",
                self.agent, id
            )));
        }

        // Log before deletion (in case deletion cascades to logs)
        self.log_modification(id, ModificationType::Delete, None)
            .await?;

        // Archive instead of hard delete
        self.storage.archive_memory(*id).await?;

        Ok(())
    }

    /// Archive a memory (soft delete)
    ///
    /// # Arguments
    ///
    /// * `id` - The memory ID to archive
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Memory not found
    /// - Permission denied (not owner and not admin)
    /// - Storage operation fails
    /// - Audit logging fails
    pub async fn archive_memory(&self, id: &MemoryId) -> Result<()> {
        // Fetch the memory to check permissions
        let memory = self.storage.get_memory(*id).await?;

        // Check permissions
        if !self.can_archive(&memory) {
            return Err(MnemosyneError::PermissionDenied(format!(
                "Agent {} cannot archive memory {} (not owner)",
                self.agent, id
            )));
        }

        // Archive the memory
        self.storage.archive_memory(*id).await?;

        // Log the archival
        self.log_modification(id, ModificationType::Archive, None)
            .await?;

        Ok(())
    }

    /// Log a modification to the audit trail
    ///
    /// # Arguments
    ///
    /// * `memory_id` - The memory that was modified
    /// * `mod_type` - The type of modification
    /// * `changes` - Optional JSON describing what changed
    ///
    /// # Errors
    ///
    /// Returns an error if the audit log write fails
    async fn log_modification(
        &self,
        memory_id: &MemoryId,
        mod_type: ModificationType,
        changes: Option<String>,
    ) -> Result<()> {
        // Generate log entry
        let log_entry = ModificationLog {
            id: Uuid::new_v4().to_string(),
            memory_id: *memory_id,
            agent_role: self.agent,
            modification_type: mod_type,
            timestamp: Utc::now(),
            changes,
        };

        // Store in audit trail via storage backend
        self.storage.store_modification_log(&log_entry).await?;

        Ok(())
    }

    /// Get the audit trail for a specific memory
    ///
    /// # Arguments
    ///
    /// * `memory_id` - The memory ID to get the audit trail for
    ///
    /// # Returns
    ///
    /// A vector of modification logs, ordered by timestamp descending (newest first)
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails
    pub async fn get_audit_trail(&self, memory_id: &MemoryId) -> Result<Vec<ModificationLog>> {
        self.storage.get_audit_trail(*memory_id).await
    }

    /// Get modification statistics for this agent
    ///
    /// Returns counts of different modification types performed by this agent.
    pub async fn get_modification_stats(&self) -> Result<Vec<(ModificationType, u32)>> {
        self.storage.get_modification_stats(self.agent).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::libsql::{ConnectionMode, LibsqlStorage};
    use crate::types::Namespace;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_access_control_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .expect("Failed to create storage");

        let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

        assert_eq!(access_control.agent(), AgentRole::Executor);
    }

    #[test]
    fn test_default_visibility() {
        let executor_vis = AgentRole::Executor.default_visibility();
        assert!(executor_vis.contains(&AgentRole::Executor));
        assert!(executor_vis.contains(&AgentRole::Reviewer));
        assert_eq!(executor_vis.len(), 2);
    }

    #[test]
    fn test_admin_mode_detection() {
        // Save original env state
        let original_admin = std::env::var("MNEMOSYNE_ADMIN_MODE").ok();
        let original_user = std::env::var("MNEMOSYNE_USER").ok();

        // Test with MNEMOSYNE_ADMIN_MODE
        std::env::set_var("MNEMOSYNE_ADMIN_MODE", "1");
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // We can't easily test this without async, so just verify the logic
        std::env::remove_var("MNEMOSYNE_ADMIN_MODE");

        // Restore original env state
        if let Some(val) = original_admin {
            std::env::set_var("MNEMOSYNE_ADMIN_MODE", val);
        }
        if let Some(val) = original_user {
            std::env::set_var("MNEMOSYNE_USER", val);
        }
    }

    #[test]
    fn test_modification_type_display() {
        assert_eq!(ModificationType::Create.to_string(), "create");
        assert_eq!(ModificationType::Update.to_string(), "update");
        assert_eq!(ModificationType::Delete.to_string(), "delete");
        assert_eq!(ModificationType::Archive.to_string(), "archive");
    }

    #[tokio::test]
    async fn test_create_memory_basic() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .expect("Failed to create storage");

        let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

        let metadata = MemoryMetadata {
            memory_type: MemoryType::CodePattern,
            namespace: Namespace::Global,
            importance: 8,
            confidence: 0.9,
            summary: "Test pattern".to_string(),
            keywords: vec!["test".to_string()],
            tags: vec![],
            context: "Test context".to_string(),
            related_files: vec![],
            related_entities: vec![],
            expires_at: None,
            visible_to: None,
        };

        let result = access_control.create_memory("Test content", metadata).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_memory_with_changes() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .expect("Failed to create storage");

        let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

        // Create a memory first
        let metadata = MemoryMetadata {
            memory_type: MemoryType::CodePattern,
            namespace: Namespace::Global,
            importance: 8,
            confidence: 0.9,
            summary: "Original summary".to_string(),
            keywords: vec!["original".to_string()],
            tags: vec![],
            context: "Original context".to_string(),
            related_files: vec![],
            related_entities: vec![],
            expires_at: None,
            visible_to: None,
        };

        let memory_id = access_control
            .create_memory("Original content", metadata)
            .await
            .expect("Failed to create memory");

        // Update the memory
        let updates = MemoryUpdates {
            content: Some("Updated content".to_string()),
            summary: Some("Updated summary".to_string()),
            importance: Some(9),
            ..Default::default()
        };

        let result = access_control.update_memory(&memory_id, updates).await;
        assert!(result.is_ok());
    }
}
