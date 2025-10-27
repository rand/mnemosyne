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

#[async_trait]
impl StorageBackend for LibsqlStorage {
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Storing memory: {}", memory.id);

        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        // Insert memory metadata including embedding column
        tx.execute(
            r#"
            INSERT INTO memories (
                id, namespace, created_at, updated_at,
                content, summary, keywords, tags, context,
                memory_type, importance, confidence,
                related_files, related_entities,
                access_count, last_accessed_at, expires_at,
                is_archived, superseded_by, embedding_model, embedding
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                memory.id.to_string(),
                serde_json::to_string(&memory.namespace)?,
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
                memory.content.clone(),
                memory.summary.clone(),
                serde_json::to_string(&memory.keywords)?,
                serde_json::to_string(&memory.tags)?,
                memory.context.clone(),
                serde_json::to_value(&memory.memory_type)?.as_str().unwrap(),
                memory.importance as i64,
                memory.confidence as f64,
                serde_json::to_string(&memory.related_files)?,
                serde_json::to_string(&memory.related_entities)?,
                memory.access_count as i64,
                memory.last_accessed_at.to_rfc3339(),
                memory.expires_at.map(|dt| dt.to_rfc3339()),
                if memory.is_archived { 1i64 } else { 0i64 },
                memory.superseded_by.map(|id| id.to_string()),
                memory.embedding_model.clone(),
                libsql::Value::Null // Embedding storage in Phase 3
            ],
        )
        .await?;

        // Store links
        for link in &memory.links {
            tx.execute(
                r#"
                INSERT INTO memory_links (source_id, target_id, link_type, strength, reason, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                params![
                    memory.id.to_string(),
                    link.target_id.to_string(),
                    serde_json::to_value(&link.link_type)?.as_str().unwrap(),
                    link.strength as f64,
                    link.reason.clone(),
                    link.created_at.to_rfc3339(),
                ],
            )
            .await?;
        }

        tx.commit().await?;

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

        let conn = self.get_conn()?;
        let mut rows = conn
            .query("SELECT * FROM memories WHERE id = ?", params![id.to_string()])
            .await?;

        let row = rows
            .next()
            .await?
            .ok_or_else(|| MnemosyneError::MemoryNotFound(id.to_string()))?;

        let mut memory = self.row_to_memory(&row).await?;

        // Fetch associated links
        let mut link_rows = conn
            .query(
                "SELECT target_id, link_type, strength, reason, created_at FROM memory_links WHERE source_id = ?",
                params![id.to_string()],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(link_row) = link_rows.next().await? {
            let target_id_str: String = link_row.get(0)?;
            let target_id = MemoryId::from_string(&target_id_str)?;

            let link_type_str: String = link_row.get(1)?;
            let link_type = match link_type_str.as_str() {
                "extends" => crate::types::LinkType::Extends,
                "contradicts" => crate::types::LinkType::Contradicts,
                "implements" => crate::types::LinkType::Implements,
                "references" => crate::types::LinkType::References,
                "supersedes" => crate::types::LinkType::Supersedes,
                _ => continue,
            };

            let strength: f64 = link_row.get(2)?;
            let reason: String = link_row.get(3)?;
            let created_at_str: String = link_row.get(4)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc);

            links.push(crate::types::MemoryLink {
                target_id,
                link_type,
                strength: strength as f32,
                reason,
                created_at,
            });
        }

        memory.links = links;
        Ok(memory)
    }

    async fn update_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Updating memory: {}", memory.id);

        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        tx.execute(
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
                superseded_by = ?,
                embedding = ?
            WHERE id = ?
            "#,
            params![
                Utc::now().to_rfc3339(),
                memory.content.clone(),
                memory.summary.clone(),
                serde_json::to_string(&memory.keywords)?,
                serde_json::to_string(&memory.tags)?,
                memory.context.clone(),
                memory.importance as i64,
                memory.confidence as f64,
                serde_json::to_string(&memory.related_files)?,
                serde_json::to_string(&memory.related_entities)?,
                if memory.is_archived { 1i64 } else { 0i64 },
                memory.superseded_by.map(|id| id.to_string()),
                libsql::Value::Null, // Embedding update in Phase 3
                memory.id.to_string(),
            ],
        )
        .await?;

        // Delete and re-insert links
        tx.execute(
            "DELETE FROM memory_links WHERE source_id = ?",
            params![memory.id.to_string()],
        )
        .await?;

        for link in &memory.links {
            tx.execute(
                r#"
                INSERT INTO memory_links (source_id, target_id, link_type, strength, reason, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                params![
                    memory.id.to_string(),
                    link.target_id.to_string(),
                    serde_json::to_value(&link.link_type)?.as_str().unwrap(),
                    link.strength as f64,
                    link.reason.clone(),
                    link.created_at.to_rfc3339(),
                ],
            )
            .await?;
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

        let conn = self.get_conn()?;

        conn.execute(
            r#"
            UPDATE memories
            SET is_archived = 1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
            params![id.to_string()],
        )
        .await?;

        self.log_audit("archive", Some(id), serde_json::json!({}))
            .await?;

        Ok(())
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Vector search (limit: {}, namespace: {:?})", limit, namespace);

        let conn = self.get_conn()?;
        let query_embedding = serde_json::to_string(embedding)?;

        let sql = if namespace.is_some() {
            format!(
                r#"
                SELECT
                    id, namespace, created_at, updated_at, content, summary,
                    keywords, tags, context, memory_type, importance, confidence,
                    related_files, related_entities, access_count, last_accessed_at,
                    expires_at, is_archived, superseded_by, embedding_model,
                    vector_distance(embedding, vector(?), 'cosine') as distance
                FROM memories
                WHERE embedding IS NOT NULL
                  AND is_archived = 0
                  AND namespace = ?
                ORDER BY distance ASC
                LIMIT {}
                "#,
                limit
            )
        } else {
            format!(
                r#"
                SELECT
                    id, namespace, created_at, updated_at, content, summary,
                    keywords, tags, context, memory_type, importance, confidence,
                    related_files, related_entities, access_count, last_accessed_at,
                    expires_at, is_archived, superseded_by, embedding_model,
                    vector_distance(embedding, vector(?), 'cosine') as distance
                FROM memories
                WHERE embedding IS NOT NULL
                  AND is_archived = 0
                ORDER BY distance ASC
                LIMIT {}
                "#,
                limit
            )
        };

        let mut rows = if let Some(ref ns) = namespace {
            let ns_json = serde_json::to_string(ns)?;
            conn.query(&sql, params![query_embedding, ns_json]).await?
        } else {
            conn.query(&sql, params![query_embedding]).await?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let distance: f64 = row.get(20)?;
            let memory = self.row_to_memory(&row).await?;
            let similarity = (1.0 - (distance as f32 / 2.0)).max(0.0).min(1.0);

            results.push(SearchResult {
                memory,
                score: similarity,
                match_reason: format!("Vector similarity: {:.2}", similarity),
            });
        }

        debug!("Vector search returned {} results", results.len());
        Ok(results)
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
            WHERE m.rowid IN (
                SELECT rowid FROM memories_fts WHERE memories_fts MATCH ?
            )
            AND m.namespace = ?
            AND m.is_archived = 0
            LIMIT 20
            "#
        } else {
            r#"
            SELECT m.* FROM memories m
            WHERE m.rowid IN (
                SELECT rowid FROM memories_fts WHERE memories_fts MATCH ?
            )
            AND m.is_archived = 0
            LIMIT 20
            "#
        };

        let conn = self.get_conn()?;
        let mut rows = if let Some(ns) = namespace_filter {
            conn.query(sql, params![query, ns]).await?
        } else {
            conn.query(sql, params![query]).await?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let memory = self.row_to_memory(&row).await?;
            results.push(SearchResult {
                memory,
                score: 0.8,
                match_reason: "keyword_match".to_string(),
            });
        }

        debug!("Keyword search found {} results", results.len());
        Ok(results)
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
