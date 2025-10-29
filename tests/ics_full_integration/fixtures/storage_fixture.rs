//! Storage fixture for integration tests
//!
//! Provides real LibsqlStorage instances with test isolation

use mnemosyne_core::{
    ConnectionMode, LibsqlStorage, MemoryNote, MemoryType, Namespace, Result, StorageBackend,
};
use std::sync::Arc;
use tempfile::TempDir;

/// Test storage fixture with automatic cleanup
pub struct StorageFixture {
    /// Temporary directory for database
    pub _temp_dir: TempDir,
    /// Storage backend instance
    pub storage: Arc<LibsqlStorage>,
    /// Database path for reference
    pub db_path: String,
}

impl StorageFixture {
    /// Create new storage fixture with isolated database
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_string_lossy().to_string();

        let storage = LibsqlStorage::new(ConnectionMode::Local(db_path_str.clone())).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            storage: Arc::new(storage),
            db_path: db_path_str,
        })
    }

    /// Create storage fixture with pre-populated memories
    pub async fn with_memories(count: usize, namespace: Namespace) -> Result<Self> {
        let fixture = Self::new().await?;

        for i in 0..count {
            let memory = create_test_memory(
                &format!("Test memory {}", i + 1),
                MemoryType::CodePattern,
                namespace.clone(),
                7,
            );
            fixture.storage.store_memory(&memory).await?;
        }

        Ok(fixture)
    }

    /// Get storage backend
    pub fn storage(&self) -> Arc<LibsqlStorage> {
        self.storage.clone()
    }

    /// Clear all memories from storage
    pub async fn clear(&self) -> Result<()> {
        let results = self
            .storage
            .keyword_search("", Some(Namespace::Global))
            .await?;

        for result in results {
            self.storage.archive_memory(result.memory.id).await?;
        }

        Ok(())
    }

    /// Get memory count
    pub async fn memory_count(&self) -> Result<usize> {
        let results = self
            .storage
            .keyword_search("", Some(Namespace::Global))
            .await?;
        Ok(results.len())
    }
}

/// Create a test memory with standard fields
pub fn create_test_memory(
    content: &str,
    memory_type: MemoryType,
    namespace: Namespace,
    importance: u8,
) -> MemoryNote {
    use mnemosyne_core::types::MemoryId;
    use chrono::Utc;

    MemoryNote {
        id: MemoryId::new(),
        namespace,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        content: content.to_string(),
        summary: format!("Summary: {}", content),
        keywords: vec!["test".to_string(), "integration".to_string()],
        tags: vec!["Test".to_string()],
        context: "Test context".to_string(),
        memory_type,
        importance,
        confidence: 0.8,
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
