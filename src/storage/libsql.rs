//! LibSQL storage backend implementation
//!
//! Provides persistent storage using Turso/libSQL with native vector search,
//! FTS5 for keyword search, and efficient indexing for graph traversal.

use crate::error::{MnemosyneError, Result};
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote, Namespace, SearchResult};
use async_trait::async_trait;
use chrono::Utc;
use libsql::{params, Builder, Connection, Database};
use tracing::{debug, info, warn};

/// LibSQL storage backend
pub struct LibsqlStorage {
    db: Database,
}

/// Database connection mode
#[derive(Debug, Clone)]
pub enum ConnectionMode {
    /// Local file-based database
    Local(String),
    /// In-memory database (for testing)
    InMemory,
    /// Remote database (Turso Cloud)
    Remote { url: String, token: String },
    /// Embedded replica with sync
    EmbeddedReplica {
        path: String,
        url: String,
        token: String,
    },
}

impl LibsqlStorage {
    /// Create a new LibSQL storage backend
    ///
    /// # Arguments
    /// * `mode` - Connection mode (local, in-memory, remote, or replica)
    ///
    /// # Example
    /// ```ignore
    /// // Local file
    /// let storage = LibsqlStorage::new(ConnectionMode::Local("mnemosyne.db".into())).await?;
    ///
    /// // In-memory (testing)
    /// let storage = LibsqlStorage::new(ConnectionMode::InMemory).await?;
    ///
    /// // Remote (Turso Cloud)
    /// let storage = LibsqlStorage::new(ConnectionMode::Remote {
    ///     url: "libsql://your-db.turso.io".into(),
    ///     token: "your-token".into(),
    /// }).await?;
    /// ```
    pub async fn new(mode: ConnectionMode) -> Result<Self> {
        info!("Connecting to LibSQL database: {:?}", mode);

        let db = match mode {
            ConnectionMode::Local(path) => {
                Builder::new_local(&path)
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create local database: {}", e)))?
            }
            ConnectionMode::InMemory => {
                Builder::new_local(":memory:")
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create in-memory database: {}", e)))?
            }
            ConnectionMode::Remote { url, token } => {
                Builder::new_remote(url, token)
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create remote database: {}", e)))?
            }
            ConnectionMode::EmbeddedReplica { path, url, token } => {
                Builder::new_remote_replica(&path, url, token)
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create embedded replica: {}", e)))?
            }
        };

        info!("LibSQL database connection established");

        let storage = Self { db };

        // Run migrations
        storage.run_migrations().await?;

        Ok(storage)
    }

    /// Create from string path (backward compatibility)
    ///
    /// Parses database path and creates appropriate connection mode:
    /// - ":memory:" → InMemory
    /// - "libsql://..." → Remote (requires token in environment)
    /// - Other → Local file path
    pub async fn from_path(database_url: &str) -> Result<Self> {
        let mode = if database_url == ":memory:" {
            ConnectionMode::InMemory
        } else if database_url.starts_with("libsql://") {
            let token = std::env::var("TURSO_AUTH_TOKEN")
                .map_err(|_| MnemosyneError::Other("TURSO_AUTH_TOKEN not found".into()))?;
            ConnectionMode::Remote {
                url: database_url.to_string(),
                token,
            }
        } else {
            ConnectionMode::Local(database_url.to_string())
        };

        Self::new(mode).await
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // Get a connection for migrations
        let conn = self.get_conn()?;

        // Use libsql_migration with dir feature
        let migrations_path = std::path::PathBuf::from("./migrations/libsql");
        libsql_migration::dir::migrate(&conn, migrations_path)
            .await
            .map_err(|e| MnemosyneError::Migration(format!("Failed to apply migrations: {}", e)))?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Get a connection from the database
    fn get_conn(&self) -> Result<Connection> {
        self.db
            .connect()
            .map_err(|e| MnemosyneError::Database(format!("Failed to get connection: {}", e)))
    }

    /// Convert a libsql row to a MemoryNote
    async fn row_to_memory(&self, row: &libsql::Row) -> Result<MemoryNote> {
        // Extract all fields from row
        let id_str: String = row.get(0)?;
        let id = MemoryId::from_string(&id_str)?;

        let namespace_json: String = row.get(1)?;
        let namespace: Namespace = serde_json::from_str(&namespace_json)?;

        let created_at: String = row.get(2)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let updated_at: String = row.get(3)?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let content: String = row.get(4)?;
        let summary: String = row.get(5)?;

        let keywords_json: String = row.get(6)?;
        let keywords: Vec<String> = serde_json::from_str(&keywords_json)?;

        let tags_json: String = row.get(7)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;

        let context: String = row.get(8)?;

        let memory_type_str: String = row.get(9)?;
        let memory_type = match memory_type_str.as_str() {
            "architecture_decision" => crate::types::MemoryType::ArchitectureDecision,
            "code_pattern" => crate::types::MemoryType::CodePattern,
            "bug_fix" => crate::types::MemoryType::BugFix,
            "configuration" => crate::types::MemoryType::Configuration,
            "constraint" => crate::types::MemoryType::Constraint,
            "entity" => crate::types::MemoryType::Entity,
            "insight" => crate::types::MemoryType::Insight,
            "reference" => crate::types::MemoryType::Reference,
            "preference" => crate::types::MemoryType::Preference,
            _ => return Err(MnemosyneError::Other(format!("Unknown memory type: {}", memory_type_str))),
        };

        let importance: i64 = row.get(10)?;
        let confidence: f64 = row.get(11)?;

        let related_files_json: String = row.get(12)?;
        let related_files: Vec<String> = serde_json::from_str(&related_files_json)?;

        let related_entities_json: String = row.get(13)?;
        let related_entities: Vec<String> = serde_json::from_str(&related_entities_json)?;

        let access_count: i64 = row.get(14)?;

        let last_accessed_str: String = row.get(15)?;
        let last_accessed_at = chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let expires_at: Option<String> = row.get(16)?;
        let expires_at = expires_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let is_archived: i64 = row.get(17)?;
        let is_archived = is_archived != 0;

        let superseded_by: Option<String> = row.get(18)?;
        let superseded_by = superseded_by.and_then(|s| MemoryId::from_string(&s).ok());

        let embedding_model: String = row.get(19)?;

        // Try to get embedding from column 20 if it exists (F32_BLOB)
        // Note: We'll handle embedding parsing in Phase 3 when implementing vector search
        let embedding: Option<Vec<f32>> = None; // Placeholder for now

        Ok(MemoryNote {
            id,
            namespace,
            created_at,
            updated_at,
            content,
            summary,
            keywords,
            tags,
            context,
            memory_type,
            importance: importance as u8,
            confidence: confidence as f32,
            links: Vec::new(), // Will be populated separately via graph traversal
            related_files,
            related_entities,
            access_count: access_count as u32,
            last_accessed_at,
            expires_at,
            is_archived,
            superseded_by,
            embedding_model,
            embedding,
        })
    }

    /// Log an audit event
    async fn log_audit(
        &self,
        operation: &str,
        memory_id: Option<MemoryId>,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let conn = self.get_conn()?;

        let memory_id_str = memory_id.map(|id| id.to_string());
        let metadata_json = metadata.to_string();

        conn.execute(
            "INSERT INTO audit_log (operation, memory_id, metadata) VALUES (?, ?, ?)",
            params![operation, memory_id_str, metadata_json],
        )
        .await?;

        Ok(())
    }
}

// Placeholder - will implement all StorageBackend methods in Phase 2
#[async_trait]
impl StorageBackend for LibsqlStorage {
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        todo!("Implement in Phase 2")
    }

    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
        todo!("Implement in Phase 2")
    }

    async fn update_memory(&self, memory: &MemoryNote) -> Result<()> {
        todo!("Implement in Phase 2")
    }

    async fn archive_memory(&self, id: MemoryId) -> Result<()> {
        todo!("Implement in Phase 2")
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        todo!("Implement in Phase 3")
    }

    async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        todo!("Implement in Phase 2")
    }

    async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
    ) -> Result<Vec<MemoryNote>> {
        todo!("Implement in Phase 2")
    }

    async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>> {
        todo!("Implement in Phase 2")
    }

    async fn increment_access(&self, id: MemoryId) -> Result<()> {
        todo!("Implement in Phase 2")
    }

    async fn count_memories(&self, namespace: Option<Namespace>) -> Result<usize> {
        todo!("Implement in Phase 2")
    }

    async fn hybrid_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
        max_results: usize,
        expand_graph: bool,
    ) -> Result<Vec<SearchResult>> {
        todo!("Implement in Phase 2")
    }

    async fn list_memories(
        &self,
        namespace: Option<Namespace>,
        limit: usize,
        sort_by: crate::storage::MemorySortOrder,
    ) -> Result<Vec<MemoryNote>> {
        todo!("Implement in Phase 2")
    }
}
