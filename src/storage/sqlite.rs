//! SQLite storage backend implementation
//!
//! Provides persistent storage using SQLite with FTS5 for keyword search
//! and efficient indexing for graph traversal and filtering.

use crate::error::{MnemosyneError, Result};
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote, Namespace, SearchResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqliteRow};
use sqlx::{ConnectOptions, Row};
use std::str::FromStr;
use tracing::{debug, info, warn};

/// SQLite storage backend
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Create a new SQLite storage backend
    ///
    /// # Arguments
    /// * `database_url` - Path to SQLite database file (e.g., "sqlite:///path/to/db.sqlite")
    ///
    /// # Example
    /// ```ignore
    /// let storage = SqliteStorage::new("sqlite://mnemosyne.db").await?;
    /// ```
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to SQLite database: {}", database_url);

        // Parse connection options
        let mut options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(30));

        // Disable logging for queries (too verbose)
        options = options.disable_statement_logging();

        // Create connection pool
        let pool = SqlitePool::connect_with(options).await?;

        info!("SQLite connection established");

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // Run migrations from embedded SQL files
        sqlx::migrate!("./migrations/sqlite")
            .run(&self.pool)
            .await?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Serialize f32 vector to bytes
    fn serialize_embedding(embedding: &[f32]) -> Vec<u8> {
        embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }

    /// Deserialize bytes to f32 vector
    fn deserialize_embedding(bytes: &[u8]) -> Result<Vec<f32>> {
        if bytes.len() % 4 != 0 {
            return Err(MnemosyneError::Other(
                "Invalid embedding byte length".to_string(),
            ));
        }

        Ok(bytes
            .chunks_exact(4)
            .map(|chunk| {
                let arr: [u8; 4] = chunk.try_into().unwrap();
                f32::from_le_bytes(arr)
            })
            .collect())
    }

    /// Convert database row to MemoryNote
    async fn row_to_memory(&self, row: SqliteRow) -> Result<MemoryNote> {
        let id_str: String = row.try_get("id")?;
        let id = MemoryId::from_string(&id_str)?;

        let namespace_str: String = row.try_get("namespace")?;
        let namespace: Namespace = serde_json::from_str(&namespace_str)?;

        let memory_type_str: String = row.try_get("memory_type")?;
        let memory_type = serde_json::from_str(&format!("\"{}\"", memory_type_str))?;

        let keywords_str: String = row.try_get("keywords")?;
        let keywords: Vec<String> = serde_json::from_str(&keywords_str)?;

        let tags_str: String = row.try_get("tags")?;
        let tags: Vec<String> = serde_json::from_str(&tags_str)?;

        let related_files_str: String = row.try_get("related_files")?;
        let related_files: Vec<String> = serde_json::from_str(&related_files_str)?;

        let related_entities_str: String = row.try_get("related_entities")?;
        let related_entities: Vec<String> = serde_json::from_str(&related_entities_str)?;

        let superseded_by: Option<String> = row.try_get("superseded_by")?;
        let superseded_by_id = superseded_by.and_then(|s| MemoryId::from_string(&s).ok());

        // Fetch embedding if exists
        let embedding = self.get_embedding(id).await.ok();

        Ok(MemoryNote {
            id,
            namespace,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            content: row.try_get("content")?,
            summary: row.try_get("summary")?,
            keywords,
            tags,
            context: row.try_get("context")?,
            memory_type,
            importance: row.try_get::<i32, _>("importance")? as u8,
            confidence: row.try_get("confidence")?,
            links: vec![], // Loaded separately if needed
            related_files,
            related_entities,
            access_count: row.try_get::<i32, _>("access_count")? as u32,
            last_accessed_at: row.try_get("last_accessed_at")?,
            expires_at: row.try_get("expires_at").ok(),
            is_archived: row.try_get::<i32, _>("is_archived")? != 0,
            superseded_by: superseded_by_id,
            embedding,
            embedding_model: row.try_get("embedding_model")?,
        })
    }

    /// Get embedding for a memory
    async fn get_embedding(&self, memory_id: MemoryId) -> Result<Vec<f32>> {
        let row = sqlx::query(
            "SELECT embedding FROM memory_embeddings WHERE memory_id = ?",
        )
        .bind(memory_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        let bytes: Vec<u8> = row.try_get("embedding")?;
        Self::deserialize_embedding(&bytes)
    }

    /// Store embedding for a memory
    async fn store_embedding(&self, memory_id: MemoryId, embedding: &[f32]) -> Result<()> {
        let bytes = Self::serialize_embedding(embedding);
        let dimension = embedding.len() as i32;

        sqlx::query(
            r#"
            INSERT INTO memory_embeddings (memory_id, embedding, dimension)
            VALUES (?, ?, ?)
            ON CONFLICT(memory_id) DO UPDATE SET
                embedding = excluded.embedding,
                dimension = excluded.dimension,
                created_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(memory_id.to_string())
        .bind(bytes)
        .bind(dimension)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Log audit entry
    async fn log_audit(
        &self,
        operation: &str,
        memory_id: Option<MemoryId>,
        details: serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_log (operation, memory_id, details)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(operation)
        .bind(memory_id.map(|id| id.to_string()))
        .bind(serde_json::to_string(&details)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Storing memory: {}", memory.id);

        let mut tx = self.pool.begin().await?;

        // Insert memory metadata
        sqlx::query(
            r#"
            INSERT INTO memories (
                id, namespace, created_at, updated_at,
                content, summary, keywords, tags, context,
                memory_type, importance, confidence,
                related_files, related_entities,
                access_count, last_accessed_at, expires_at,
                is_archived, superseded_by, embedding_model
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(memory.id.to_string())
        .bind(serde_json::to_string(&memory.namespace)?)
        .bind(memory.created_at)
        .bind(memory.updated_at)
        .bind(&memory.content)
        .bind(&memory.summary)
        .bind(serde_json::to_string(&memory.keywords)?)
        .bind(serde_json::to_string(&memory.tags)?)
        .bind(&memory.context)
        .bind(serde_json::to_value(&memory.memory_type)?.as_str().unwrap())
        .bind(memory.importance as i32)
        .bind(memory.confidence)
        .bind(serde_json::to_string(&memory.related_files)?)
        .bind(serde_json::to_string(&memory.related_entities)?)
        .bind(memory.access_count as i32)
        .bind(memory.last_accessed_at)
        .bind(memory.expires_at)
        .bind(memory.is_archived as i32)
        .bind(memory.superseded_by.map(|id| id.to_string()))
        .bind(&memory.embedding_model)
        .execute(&mut *tx)
        .await?;

        // Store embedding if present
        if let Some(ref embedding) = memory.embedding {
            let bytes = Self::serialize_embedding(embedding);
            let dimension = embedding.len() as i32;

            sqlx::query(
                r#"
                INSERT INTO memory_embeddings (memory_id, embedding, dimension)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(memory.id.to_string())
            .bind(bytes)
            .bind(dimension)
            .execute(&mut *tx)
            .await?;
        }

        // Store links
        for link in &memory.links {
            sqlx::query(
                r#"
                INSERT INTO memory_links (source_id, target_id, link_type, strength, reason, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(memory.id.to_string())
            .bind(link.target_id.to_string())
            .bind(serde_json::to_value(&link.link_type)?.as_str().unwrap())
            .bind(link.strength)
            .bind(&link.reason)
            .bind(link.created_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Log audit entry
        self.log_audit(
            "create",
            Some(memory.id),
            serde_json::json!({
                "namespace": memory.namespace,
                "memory_type": memory.memory_type,
                "importance": memory.importance,
            }),
        )
        .await?;

        debug!("Memory stored successfully: {}", memory.id);
        Ok(())
    }

    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
        debug!("Fetching memory: {}", id);

        let row = sqlx::query("SELECT * FROM memories WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| MnemosyneError::MemoryNotFound(id.to_string()))?;

        self.row_to_memory(row).await
    }

    async fn update_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Updating memory: {}", memory.id);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE memories SET
                updated_at = ?,
                content = ?,
                summary = ?,
                keywords = ?,
                tags = ?,
                context = ?,
                importance = ?,
                confidence = ?,
                related_files = ?,
                related_entities = ?,
                is_archived = ?,
                superseded_by = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now())
        .bind(&memory.content)
        .bind(&memory.summary)
        .bind(serde_json::to_string(&memory.keywords)?)
        .bind(serde_json::to_string(&memory.tags)?)
        .bind(&memory.context)
        .bind(memory.importance as i32)
        .bind(memory.confidence)
        .bind(serde_json::to_string(&memory.related_files)?)
        .bind(serde_json::to_string(&memory.related_entities)?)
        .bind(memory.is_archived as i32)
        .bind(memory.superseded_by.map(|id| id.to_string()))
        .bind(memory.id.to_string())
        .execute(&mut *tx)
        .await?;

        // Update embedding if present
        if let Some(ref embedding) = memory.embedding {
            self.store_embedding(memory.id, embedding).await?;
        }

        tx.commit().await?;

        self.log_audit(
            "update",
            Some(memory.id),
            serde_json::json!({"importance": memory.importance}),
        )
        .await?;

        Ok(())
    }

    async fn archive_memory(&self, id: MemoryId) -> Result<()> {
        debug!("Archiving memory: {}", id);

        sqlx::query(
            r#"
            UPDATE memories
            SET is_archived = 1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        self.log_audit("archive", Some(id), serde_json::json!({}))
            .await?;

        Ok(())
    }

    async fn vector_search(
        &self,
        _embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Vector search (limit: {}, namespace: {:?})", limit, namespace);

        // For now, return empty results
        // TODO: Implement proper vector similarity search
        // This requires either:
        // 1. sqlite-vec extension
        // 2. Manual cosine similarity calculation
        // 3. External vector database

        warn!("Vector search not yet fully implemented");
        Ok(vec![])
    }

    async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Keyword search: {} (namespace: {:?})", query, namespace);

        let namespace_filter = namespace.map(|ns| serde_json::to_string(&ns).unwrap());

        let sql = if namespace_filter.is_some() {
            r#"
            SELECT m.* FROM memories m
            JOIN memories_fts fts ON m.id = fts.memory_id
            WHERE memories_fts MATCH ? AND m.namespace = ? AND m.is_archived = 0
            ORDER BY rank
            LIMIT 20
            "#
        } else {
            r#"
            SELECT m.* FROM memories m
            JOIN memories_fts fts ON m.id = fts.memory_id
            WHERE memories_fts MATCH ? AND m.is_archived = 0
            ORDER BY rank
            LIMIT 20
            "#
        };

        let mut query_builder = sqlx::query(sql).bind(query);

        if let Some(ns) = namespace_filter {
            query_builder = query_builder.bind(ns);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let memory = self.row_to_memory(row).await?;
            results.push(SearchResult {
                memory,
                score: 0.8, // FTS5 rank would be better
                match_reason: "keyword_match".to_string(),
            });
        }

        Ok(results)
    }

    async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
    ) -> Result<Vec<MemoryNote>> {
        debug!("Graph traverse from {} seeds, max {} hops", seed_ids.len(), max_hops);

        // TODO: Implement recursive CTE for graph traversal
        warn!("Graph traversal not yet implemented");
        Ok(vec![])
    }

    async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>> {
        debug!("Finding consolidation candidates (namespace: {:?})", namespace);

        // TODO: Implement similarity-based candidate finding
        warn!("Consolidation candidate search not yet implemented");
        Ok(vec![])
    }

    async fn increment_access(&self, id: MemoryId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE memories
            SET access_count = access_count + 1,
                last_accessed_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn count_memories(&self, namespace: Option<Namespace>) -> Result<usize> {
        let count: i64 = if let Some(ns) = namespace {
            let ns_str = serde_json::to_string(&ns)?;
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM memories WHERE namespace = ? AND is_archived = 0",
            )
            .bind(ns_str)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar("SELECT COUNT(*) FROM memories WHERE is_archived = 0")
                .fetch_one(&self.pool)
                .await?
        };

        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemoryType;

    #[tokio::test]
    async fn test_sqlite_storage_lifecycle() {
        // Create in-memory database for testing
        let storage = SqliteStorage::new("sqlite::memory:").await.unwrap();
        storage.run_migrations().await.unwrap();

        // Create test memory
        let memory = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "Test memory content".to_string(),
            summary: "Test summary".to_string(),
            keywords: vec!["test".to_string()],
            tags: vec!["testing".to_string()],
            context: "Test context".to_string(),
            memory_type: MemoryType::CodePattern,
            importance: 5,
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
            embedding_model: "test-model".to_string(),
        };

        // Store memory
        storage.store_memory(&memory).await.unwrap();

        // Retrieve memory
        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert_eq!(retrieved.id, memory.id);
        assert_eq!(retrieved.content, memory.content);

        // Update memory
        let mut updated = retrieved.clone();
        updated.importance = 8;
        storage.update_memory(&updated).await.unwrap();

        // Verify update
        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert_eq!(retrieved.importance, 8);

        // Archive memory
        storage.archive_memory(memory.id).await.unwrap();
        let retrieved = storage.get_memory(memory.id).await.unwrap();
        assert!(retrieved.is_archived);

        // Count memories
        let count = storage.count_memories(None).await.unwrap();
        assert_eq!(count, 0); // Archived memories don't count
    }
}
