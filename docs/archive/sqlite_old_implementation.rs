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

        let storage = Self { pool };

        // Load sqlite-vec extension for vector similarity search
        storage.load_vector_extension().await?;

        // Automatically run migrations on new database
        storage.run_migrations().await?;

        Ok(storage)
    }

    /// Load sqlite-vec extension for vector similarity search
    async fn load_vector_extension(&self) -> Result<()> {
        // Note: sqlite-vec should be installed and accessible
        // On macOS with Homebrew: brew install sqlite-vec
        // The extension is typically at /opt/homebrew/lib/vec0.dylib

        // Try to load the extension - it's optional for basic functionality
        let result = sqlx::query("SELECT load_extension('vec0')")
            .execute(&self.pool)
            .await;

        match result {
            Ok(_) => {
                info!("sqlite-vec extension loaded successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to load sqlite-vec extension: {}", e);
                warn!("Vector search will not be available");
                warn!("Install with: brew install sqlite-vec (macOS) or build from source");
                // Don't fail - allow basic functionality without vector search
                Ok(())
            }
        }
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

    /// Get embedding for a memory from vec0 virtual table
    async fn get_embedding(&self, memory_id: MemoryId) -> Result<Vec<f32>> {
        let row = sqlx::query(
            "SELECT embedding FROM vec_memories WHERE memory_id = ?",
        )
        .bind(memory_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        let embedding_json: String = row.try_get("embedding")?;
        let embedding: Vec<f32> = serde_json::from_str(&embedding_json)?;
        Ok(embedding)
    }

    /// Store embedding for a memory using vec0 virtual table
    async fn store_embedding(&self, memory_id: MemoryId, embedding: &[f32]) -> Result<()> {
        // Serialize embedding as JSON array for vec0
        let embedding_json = serde_json::to_string(embedding)?;

        sqlx::query(
            r#"
            INSERT INTO vec_memories (memory_id, embedding)
            VALUES (?, ?)
            ON CONFLICT(memory_id) DO UPDATE SET
                embedding = excluded.embedding
            "#,
        )
        .bind(memory_id.to_string())
        .bind(embedding_json)
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
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Vector search (limit: {}, namespace: {:?})", limit, namespace);

        // Serialize query embedding as JSON array
        let query_embedding = serde_json::to_string(embedding)?;

        // Build query with optional namespace filtering
        let (sql, namespace_json) = if let Some(ref ns) = namespace {
            let ns_json = serde_json::to_string(ns)?;
            (
                format!(
                    r#"
                    SELECT
                        m.*,
                        vec_distance_cosine(v.embedding, ?) as distance
                    FROM vec_memories v
                    JOIN memories m ON m.id = v.memory_id
                    WHERE m.namespace = ?
                      AND m.is_archived = 0
                    ORDER BY distance ASC
                    LIMIT {}
                    "#,
                    limit
                ),
                Some(ns_json),
            )
        } else {
            (
                format!(
                    r#"
                    SELECT
                        m.*,
                        vec_distance_cosine(v.embedding, ?) as distance
                    FROM vec_memories v
                    JOIN memories m ON m.id = v.memory_id
                    WHERE m.is_archived = 0
                    ORDER BY distance ASC
                    LIMIT {}
                    "#,
                    limit
                ),
                None,
            )
        };

        let mut query = sqlx::query(&sql).bind(&query_embedding);

        if let Some(ns_json) = namespace_json {
            query = query.bind(ns_json);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let distance: f32 = row.try_get("distance").unwrap_or(1.0);
            let memory = self.row_to_memory(row).await?;

            // Convert distance to similarity score (1.0 - distance)
            // Cosine distance is [0, 2], so normalize to [0, 1]
            let similarity = (1.0 - (distance / 2.0)).max(0.0).min(1.0);

            results.push(SearchResult {
                memory,
                score: similarity,
                match_reason: format!("Vector similarity: {:.2}", similarity),
            });
        }

        Ok(results)
    }

    async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Keyword search: {} (namespace: {:?})", query, namespace);

        let namespace_filter = namespace.map(|ns| serde_json::to_string(&ns).unwrap());

        // FTS5 with content table - join on rowid
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

        if seed_ids.is_empty() || max_hops == 0 {
            return Ok(vec![]);
        }

        // Convert seed IDs to strings for SQL IN clause
        let seed_strings: Vec<String> = seed_ids.iter().map(|id| id.to_string()).collect();
        let placeholders = seed_strings.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        // Recursive CTE to traverse the graph bidirectionally
        let sql = format!(
            r#"
            WITH RECURSIVE graph_walk(memory_id, depth) AS (
                -- Base case: start with seed nodes at depth 0
                SELECT id, 0 FROM memories WHERE id IN ({placeholders})

                UNION

                -- Recursive case: follow links bidirectionally
                SELECT
                    CASE
                        WHEN ml.source_id = gw.memory_id THEN ml.target_id
                        ELSE ml.source_id
                    END as memory_id,
                    gw.depth + 1
                FROM graph_walk gw
                JOIN memory_links ml ON (
                    ml.source_id = gw.memory_id OR ml.target_id = gw.memory_id
                )
                WHERE gw.depth < ?
            )
            SELECT DISTINCT m.*
            FROM memories m
            JOIN graph_walk gw ON m.id = gw.memory_id
            WHERE m.is_archived = 0
            ORDER BY gw.depth, m.importance DESC
            "#,
            placeholders = placeholders
        );

        let mut query = sqlx::query(&sql);

        // Bind seed IDs
        for seed_str in &seed_strings {
            query = query.bind(seed_str);
        }

        // Bind max hops
        query = query.bind(max_hops as i32);

        let rows = query.fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let memory = self.row_to_memory(row).await?;
            results.push(memory);
        }

        debug!("Graph traversal found {} memories", results.len());
        Ok(results)
    }

    async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>> {
        debug!("Finding consolidation candidates (namespace: {:?})", namespace);

        // Get all active memories in namespace with embeddings
        let memories_sql = if let Some(ref ns) = namespace {
            let ns_json = serde_json::to_string(ns)?;
            format!(
                r#"
                SELECT DISTINCT m.*
                FROM memories m
                JOIN vec_memories v ON m.id = v.memory_id
                WHERE m.namespace = ?
                  AND m.is_archived = 0
                  AND m.superseded_by IS NULL
                ORDER BY m.created_at DESC
                LIMIT 100
                "#
            )
        } else {
            r#"
                SELECT DISTINCT m.*
                FROM memories m
                JOIN vec_memories v ON m.id = v.memory_id
                WHERE m.is_archived = 0
                  AND m.superseded_by IS NULL
                ORDER BY m.created_at DESC
                LIMIT 100
                "#
            .to_string()
        };

        let mut query = sqlx::query(&memories_sql);
        if let Some(ref ns) = namespace {
            let ns_json = serde_json::to_string(ns)?;
            query = query.bind(ns_json);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(self.row_to_memory(row).await?);
        }

        debug!("Found {} memories to compare for consolidation", memories.len());

        // Find similar pairs using vector similarity
        let mut candidates = Vec::new();
        let similarity_threshold = 0.85; // High similarity suggests potential duplicates

        for i in 0..memories.len() {
            if let Some(ref embedding_i) = memories[i].embedding {
                // Use vector search to find similar memories
                let similar = self
                    .vector_search(embedding_i, 5, namespace.clone())
                    .await?;

                for result in similar {
                    // Skip self-comparison
                    if result.memory.id == memories[i].id {
                        continue;
                    }

                    // Only consider high-similarity pairs
                    if result.score >= similarity_threshold {
                        // Avoid duplicate pairs (A,B) and (B,A)
                        let should_add = memories
                            .iter()
                            .position(|m| m.id == result.memory.id)
                            .map(|j| i < j)
                            .unwrap_or(false);

                        if should_add {
                            debug!(
                                "Consolidation candidate: {} <-> {} (similarity: {:.2})",
                                memories[i].id, result.memory.id, result.score
                            );
                            candidates.push((memories[i].clone(), result.memory));
                        }
                    }
                }
            }
        }

        debug!("Found {} consolidation candidate pairs", candidates.len());
        Ok(candidates)
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

    async fn hybrid_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
        max_results: usize,
        expand_graph: bool,
    ) -> Result<Vec<SearchResult>> {
        debug!("Hybrid search: {} (expand_graph: {})", query, expand_graph);

        // Phase 1: Keyword search with FTS5
        let keyword_results = self.keyword_search(query, namespace.clone()).await?;

        if keyword_results.is_empty() {
            debug!("No keyword matches found");
            return Ok(vec![]);
        }

        // Phase 2: Optionally expand via graph traversal
        let mut all_memories = std::collections::HashMap::new();

        // Add keyword results with initial scores
        for result in keyword_results {
            all_memories.insert(
                result.memory.id,
                (result.memory.clone(), 1.0, 0), // (memory, keyword_score, depth)
            );
        }

        if expand_graph {
            debug!("Expanding graph from {} seed memories", all_memories.len());

            // Use top 5 keyword results as seeds
            let seed_ids: Vec<_> = all_memories.keys().take(5).copied().collect();
            let graph_memories = self.graph_traverse(&seed_ids, 2).await?;

            // Add graph-expanded memories with decay based on presence in seeds
            for memory in graph_memories {
                if !all_memories.contains_key(&memory.id) {
                    // Not a keyword match, lower score
                    all_memories.insert(memory.id, (memory, 0.3, 1));
                }
            }
        }

        // Phase 3: Hybrid ranking
        let now = Utc::now();
        let mut scored_results: Vec<_> = all_memories
            .into_values()
            .map(|(memory, keyword_score, depth)| {
                // Normalize importance (1-10) to 0.0-1.0
                let importance_score = memory.importance as f32 / 10.0;

                // Recency score (exponential decay, half-life = 30 days)
                let age_days = (now - memory.created_at).num_days() as f32;
                let recency_score = (-age_days / 30.0).exp();

                // Graph proximity score (1.0 for seeds, decay for expanded)
                let graph_score = 1.0 / (1.0 + depth as f32);

                // Hybrid score combining all factors
                // Weights: keyword (50%), graph (20%), importance (20%), recency (10%)
                let score = 0.5 * keyword_score
                    + 0.2 * graph_score
                    + 0.2 * importance_score
                    + 0.1 * recency_score;

                let match_reason = if keyword_score > 0.5 {
                    format!("keyword_match (score: {:.2})", score)
                } else {
                    format!("graph_expansion (score: {:.2})", score)
                };

                SearchResult {
                    memory,
                    score,
                    match_reason,
                }
            })
            .collect();

        // Sort by score descending
        scored_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        scored_results.truncate(max_results);

        debug!("Hybrid search returned {} results", scored_results.len());
        Ok(scored_results)
    }

    async fn list_memories(
        &self,
        namespace: Option<Namespace>,
        limit: usize,
        sort_by: crate::storage::MemorySortOrder,
    ) -> Result<Vec<MemoryNote>> {
        use crate::storage::MemorySortOrder;

        debug!("Listing memories (namespace: {:?}, limit: {}, sort: {:?})", namespace, limit, sort_by);

        let order_clause = match sort_by {
            MemorySortOrder::Recent => "created_at DESC",
            MemorySortOrder::Importance => "importance DESC, created_at DESC",
            MemorySortOrder::AccessCount => "access_count DESC, created_at DESC",
        };

        let sql = if namespace.is_some() {
            format!(
                "SELECT * FROM memories WHERE namespace = ? AND is_archived = 0 ORDER BY {} LIMIT ?",
                order_clause
            )
        } else {
            format!(
                "SELECT * FROM memories WHERE is_archived = 0 ORDER BY {} LIMIT ?",
                order_clause
            )
        };

        let mut query = sqlx::query(&sql);

        if let Some(ns) = namespace {
            let ns_str = serde_json::to_string(&ns)?;
            query = query.bind(ns_str);
        }

        query = query.bind(limit as i64);

        let rows = query.fetch_all(&self.pool).await?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(self.row_to_memory(row).await?);
        }

        debug!("Listed {} memories", memories.len());
        Ok(memories)
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

        // Run migrations to set up schema
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

    #[tokio::test]
    async fn test_graph_traversal() {
        use crate::types::{LinkType, MemoryLink};

        let storage = SqliteStorage::new("sqlite::memory:").await.unwrap();
        storage.run_migrations().await.unwrap();

        // Create a chain of linked memories: A -> B -> C -> D
        let memory_a = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "Memory A".to_string(),
            summary: "First memory".to_string(),
            keywords: vec![],
            tags: vec![],
            context: "Test".to_string(),
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
            embedding_model: "test".to_string(),
        };

        let memory_b = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            content: "Memory B".to_string(),
            summary: "Second memory".to_string(),
            links: vec![MemoryLink {
                target_id: memory_a.id,
                link_type: LinkType::References,
                strength: 0.8,
                reason: "Extends A".to_string(),
                created_at: Utc::now(),
            }],
            ..memory_a.clone()
        };

        let memory_c = MemoryNote {
            id: MemoryId::new(),
            content: "Memory C".to_string(),
            summary: "Third memory".to_string(),
            links: vec![MemoryLink {
                target_id: memory_b.id,
                link_type: LinkType::Extends,
                strength: 0.9,
                reason: "Extends B".to_string(),
                created_at: Utc::now(),
            }],
            ..memory_a.clone()
        };

        let memory_d = MemoryNote {
            id: MemoryId::new(),
            content: "Memory D".to_string(),
            summary: "Fourth memory".to_string(),
            links: vec![MemoryLink {
                target_id: memory_c.id,
                link_type: LinkType::Implements,
                strength: 0.7,
                reason: "Implements C".to_string(),
                created_at: Utc::now(),
            }],
            ..memory_a.clone()
        };

        // Store all memories
        storage.store_memory(&memory_a).await.unwrap();
        storage.store_memory(&memory_b).await.unwrap();
        storage.store_memory(&memory_c).await.unwrap();
        storage.store_memory(&memory_d).await.unwrap();

        // Test: traverse 1 hop from A should find A and B
        let results = storage.graph_traverse(&[memory_a.id], 1).await.unwrap();
        assert!(results.len() >= 2, "Should find at least A and B");
        let ids: Vec<MemoryId> = results.iter().map(|m| m.id).collect();
        assert!(ids.contains(&memory_a.id));
        assert!(ids.contains(&memory_b.id));

        // Test: traverse 2 hops from A should find A, B, and C
        let results = storage.graph_traverse(&[memory_a.id], 2).await.unwrap();
        assert!(results.len() >= 3, "Should find at least A, B, and C");
        let ids: Vec<MemoryId> = results.iter().map(|m| m.id).collect();
        assert!(ids.contains(&memory_a.id));
        assert!(ids.contains(&memory_b.id));
        assert!(ids.contains(&memory_c.id));

        // Test: traverse 3 hops from A should find all memories
        let results = storage.graph_traverse(&[memory_a.id], 3).await.unwrap();
        assert_eq!(results.len(), 4, "Should find all 4 memories");

        // Test: traverse 0 hops should return empty
        let results = storage.graph_traverse(&[memory_a.id], 0).await.unwrap();
        assert_eq!(results.len(), 0);

        // Test: traverse from empty seeds should return empty
        let results = storage.graph_traverse(&[], 3).await.unwrap();
        assert_eq!(results.len(), 0);
    }
}
