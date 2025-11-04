//! Memory entry creation for artifacts
//!
//! Phase 1: Basic memory creation without complex linking.
//! Link functionality will be added in Phase 2 when implementing slash commands.

use crate::error::Result;
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote, MemoryType, Namespace};
use std::sync::Arc;

/// Memory linker for creating artifact memory entries
pub struct MemoryLinker {
    storage: Arc<dyn StorageBackend>,
}

impl MemoryLinker {
    /// Create a new memory linker
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    /// Create a memory entry for an artifact
    pub async fn create_artifact_memory(
        &self,
        memory_type: MemoryType,
        content: String,
        namespace: Namespace,
        artifact_path: String,
        importance: u8,
        tags: Vec<String>,
    ) -> Result<MemoryId> {
        let now = chrono::Utc::now();
        let memory = MemoryNote {
            id: MemoryId::new(),
            namespace,
            created_at: now,
            updated_at: now,
            content: content.clone(),
            summary: format!("Artifact: {}", artifact_path),
            keywords: Vec::new(),
            tags,
            context: format!("Specification artifact stored at {}", artifact_path),
            memory_type,
            importance,
            confidence: 1.0,
            links: Vec::new(),
            related_files: Vec::new(),
            related_entities: Vec::new(),
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "none".to_string(),
        };

        self.storage.store_memory(&memory).await?;
        Ok(memory.id)
    }

    // TODO Phase 2: Add link_artifacts, create_workflow_links, etc.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_linker_creation() {
        // Basic struct creation test
        // Full integration tests will be added when StorageBackend mock is available
    }
}
