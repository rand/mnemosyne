//! Vector storage implementation using sqlite-vec
//!
//! This module provides vector similarity search using the dual storage approach:
//! - rusqlite with sqlite-vec extension for vector operations
//! - Same database file as libsql (no conflicts)
//! - Separate vec0 virtual table for embeddings
//! - Connection pooling for concurrent access (deadpool-sqlite)

use crate::error::{MnemosyneError, Result};
use crate::types::MemoryId;
use deadpool_sqlite::{Config, Pool, Runtime};
use rusqlite::Result as SqliteResult;
use std::path::Path;
use tracing::{debug, info};

/// Default connection pool size
const DEFAULT_POOL_SIZE: usize = 20;

/// Vector storage backend using sqlite-vec with connection pooling
pub struct SqliteVectorStorage {
    pool: Pool,
    dimensions: usize,
}

impl SqliteVectorStorage {
    /// Create a new vector storage instance with connection pooling
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file (same as libsql)
    /// * `dimensions` - Vector dimension size (typically 1536 for Voyage embeddings)
    ///
    /// # Example
    /// ```ignore
    /// let vector_storage = SqliteVectorStorage::new("mnemosyne.db", 1536)?;
    /// ```
    pub fn new<P: AsRef<Path>>(db_path: P, dimensions: usize) -> Result<Self> {
        Self::with_pool_size(db_path, dimensions, DEFAULT_POOL_SIZE)
    }

    /// Create a new vector storage instance with custom pool size
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `dimensions` - Vector dimension size
    /// * `pool_size` - Maximum number of connections in the pool
    pub fn with_pool_size<P: AsRef<Path>>(
        db_path: P,
        dimensions: usize,
        pool_size: usize,
    ) -> Result<Self> {
        let path_str = db_path.as_ref().to_string_lossy().to_string();
        info!(
            "Creating vector storage pool at: {} (dimensions: {}, pool_size: {})",
            path_str, dimensions, pool_size
        );

        // Load sqlite-vec extension as auto-extension
        // This ensures it's available for all connections in the pool
        unsafe {
            use rusqlite::ffi::sqlite3_auto_extension;

            #[allow(clippy::missing_transmute_annotations)]
            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        // Create connection pool configuration
        let config = Config::new(path_str);
        let pool = config.create_pool(Runtime::Tokio1).map_err(|e| {
            MnemosyneError::Database(format!("Failed to create connection pool: {}", e))
        })?;

        info!("Vector storage connection pool created successfully");

        Ok(Self { pool, dimensions })
    }

    /// Create the vec0 virtual table for vector storage
    ///
    /// This should be called once during initialization or migration.
    /// It's safe to call multiple times (uses IF NOT EXISTS).
    pub async fn create_vec_table(&self) -> Result<()> {
        info!(
            "Creating vec0 virtual table for vectors (dimensions: {})",
            self.dimensions
        );

        let sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
                memory_id TEXT PRIMARY KEY,
                embedding FLOAT[{}]
            )",
            self.dimensions
        );

        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        conn.interact(move |conn| {
            conn.execute(&sql, []).map_err(|e| {
                MnemosyneError::Database(format!("Failed to create vec0 table: {}", e))
            })
        })
        .await
        .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        // Note: Virtual tables don't support traditional indexes
        // The vec0 module handles indexing internally

        info!("Vector table created successfully");
        Ok(())
    }

    /// Store a vector embedding for a memory
    ///
    /// # Arguments
    /// * `memory_id` - The memory identifier
    /// * `embedding` - The embedding vector (must match dimensions)
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if dimensions mismatch or database error
    pub async fn store_vector(&self, memory_id: &MemoryId, embedding: &[f32]) -> Result<()> {
        if embedding.len() != self.dimensions {
            return Err(MnemosyneError::Other(format!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.dimensions,
                embedding.len()
            )));
        }

        debug!("Storing vector for memory: {}", memory_id);

        let id = memory_id.to_string();
        let embedding_json = serde_json::to_string(embedding)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize embedding: {}", e)))?;

        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        let memory_id_clone = memory_id.clone();
        conn.interact(move |conn| -> Result<()> {
            // Virtual tables don't support INSERT OR REPLACE, so delete first if exists
            conn.execute(
                "DELETE FROM memory_vectors WHERE memory_id = ?",
                rusqlite::params![&id],
            )
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to delete existing vector: {}", e))
            })?;

            // Then insert the new vector
            conn.execute(
                "INSERT INTO memory_vectors (memory_id, embedding)
                 VALUES (?, vec_f32(?))",
                rusqlite::params![&id, &embedding_json],
            )
            .map_err(|e| MnemosyneError::Database(format!("Failed to store vector: {}", e)))?;

            Ok(())
        })
        .await
        .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        debug!("Vector stored successfully for memory: {}", memory_id_clone);
        Ok(())
    }

    /// Retrieve a vector embedding for a memory
    ///
    /// # Arguments
    /// * `memory_id` - The memory identifier
    ///
    /// # Returns
    /// * `Some(Vec<f32>)` if vector exists
    /// * `None` if not found
    pub async fn get_vector(&self, memory_id: &MemoryId) -> Result<Option<Vec<f32>>> {
        debug!("Retrieving vector for memory: {}", memory_id);

        let id = memory_id.to_string();

        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        let memory_id_clone = memory_id.clone();
        let result = conn
            .interact(move |conn| -> Result<Option<Vec<f32>>> {
                let mut stmt = conn
                    .prepare("SELECT embedding FROM memory_vectors WHERE memory_id = ?")
                    .map_err(|e| {
                        MnemosyneError::Database(format!("Failed to prepare query: {}", e))
                    })?;

                let result: SqliteResult<Vec<u8>> =
                    stmt.query_row(rusqlite::params![id], |row| row.get(0));

                match result {
                    Ok(blob) => {
                        // Convert blob to Vec<f32>
                        let floats = blob
                            .chunks_exact(4)
                            .map(|chunk| {
                                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                            })
                            .collect();
                        Ok(Some(floats))
                    }
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(MnemosyneError::Database(format!(
                        "Failed to retrieve vector: {}",
                        e
                    ))),
                }
            })
            .await
            .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        if result.is_some() {
            debug!(
                "Vector retrieved successfully for memory: {}",
                memory_id_clone
            );
        } else {
            debug!("No vector found for memory: {}", memory_id_clone);
        }

        Ok(result)
    }

    /// Search for similar vectors using KNN
    ///
    /// # Arguments
    /// * `query_embedding` - The query vector
    /// * `limit` - Maximum number of results to return
    /// * `min_similarity` - Minimum similarity threshold (0.0 to 1.0)
    ///
    /// # Returns
    /// * Vector of (MemoryId, similarity_score) tuples, sorted by descending similarity
    ///
    /// # Note
    /// - Similarity is cosine similarity: 1.0 = identical, 0.0 = orthogonal, -1.0 = opposite
    /// - Distance from sqlite-vec is 1 - cosine_similarity, so we convert it back
    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<(MemoryId, f32)>> {
        if query_embedding.len() != self.dimensions {
            return Err(MnemosyneError::Other(format!(
                "Query embedding dimension mismatch: expected {}, got {}",
                self.dimensions,
                query_embedding.len()
            )));
        }

        debug!(
            "Searching for similar vectors (limit: {}, min_similarity: {})",
            limit, min_similarity
        );

        let query_json = serde_json::to_string(query_embedding)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize query: {}", e)))?;

        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        let results = conn
            .interact(move |conn| -> Result<Vec<(MemoryId, f32)>> {
                let mut stmt = conn
                    .prepare(
                        "SELECT memory_id, distance
                     FROM memory_vectors
                     WHERE embedding MATCH vec_f32(?)
                     ORDER BY distance
                     LIMIT ?",
                    )
                    .map_err(|e| {
                        MnemosyneError::Database(format!("Failed to prepare search: {}", e))
                    })?;

                let results: SqliteResult<Vec<(MemoryId, f32)>> = stmt
                    .query_map(rusqlite::params![query_json, limit as i64], |row| {
                        let id_str: String = row.get(0)?;
                        let distance: f32 = row.get(1)?;

                        // Convert distance to similarity (distance = 1 - cosine_similarity)
                        // For cosine distance, distance = 1 - similarity
                        let similarity = 1.0 - distance;

                        Ok((
                            MemoryId::from_string(&id_str).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?,
                            similarity,
                        ))
                    })
                    .and_then(|mapped| mapped.collect::<SqliteResult<Vec<_>>>());

                let mut results = results.map_err(|e| {
                    MnemosyneError::Database(format!("Failed to execute vector search: {}", e))
                })?;

                // Filter by minimum similarity
                results.retain(|(_, sim)| *sim >= min_similarity);

                Ok(results)
            })
            .await
            .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        debug!("Vector search returned {} results", results.len());
        Ok(results)
    }

    /// Delete a vector for a memory
    ///
    /// # Arguments
    /// * `memory_id` - The memory identifier
    pub async fn delete_vector(&self, memory_id: &MemoryId) -> Result<()> {
        debug!("Deleting vector for memory: {}", memory_id);

        let id = memory_id.to_string();

        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        let memory_id_clone = memory_id.clone();
        conn.interact(move |conn| -> Result<()> {
            conn.execute(
                "DELETE FROM memory_vectors WHERE memory_id = ?",
                rusqlite::params![id],
            )
            .map_err(|e| MnemosyneError::Database(format!("Failed to delete vector: {}", e)))?;
            Ok(())
        })
        .await
        .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        debug!(
            "Vector deleted successfully for memory: {}",
            memory_id_clone
        );
        Ok(())
    }

    /// Count total vectors in storage
    ///
    /// # Returns
    /// * Total number of vectors stored
    pub async fn count_vectors(&self) -> Result<usize> {
        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        let count = conn
            .interact(|conn| -> Result<usize> {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM memory_vectors", [], |row| row.get(0))
                    .map_err(|e| {
                        MnemosyneError::Database(format!("Failed to count vectors: {}", e))
                    })?;
                Ok(count as usize)
            })
            .await
            .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        Ok(count)
    }

    /// Batch store multiple vectors (more efficient than individual stores)
    ///
    /// # Arguments
    /// * `vectors` - Slice of (memory_id, embedding) tuples
    ///
    /// # Returns
    /// * Number of vectors successfully stored
    pub async fn batch_store_vectors(&self, vectors: &[(MemoryId, Vec<f32>)]) -> Result<usize> {
        info!("Batch storing {} vectors", vectors.len());

        let dimensions = self.dimensions;
        let vectors_owned: Vec<(String, Vec<f32>)> = vectors
            .iter()
            .map(|(id, emb)| (id.to_string(), emb.clone()))
            .collect();

        let conn = self.pool.get().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection from pool: {}", e))
        })?;

        let count = conn
            .interact(move |conn| -> Result<usize> {
                let tx = conn.transaction().map_err(|e| {
                    MnemosyneError::Database(format!("Failed to begin transaction: {}", e))
                })?;

                let mut count = 0;
                {
                    let mut stmt = tx
                        .prepare(
                            "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding)
                         VALUES (?, vec_f32(?))",
                        )
                        .map_err(|e| {
                            MnemosyneError::Database(format!(
                                "Failed to prepare batch insert: {}",
                                e
                            ))
                        })?;

                    for (id, embedding) in &vectors_owned {
                        if embedding.len() != dimensions {
                            debug!(
                                "Skipping vector {} due to dimension mismatch: expected {}, got {}",
                                id,
                                dimensions,
                                embedding.len()
                            );
                            continue;
                        }

                        let embedding_json = serde_json::to_string(embedding).map_err(|e| {
                            MnemosyneError::Other(format!("Failed to serialize embedding: {}", e))
                        })?;

                        stmt.execute(rusqlite::params![id, embedding_json])
                            .map_err(|e| {
                                MnemosyneError::Database(format!("Failed to insert vector: {}", e))
                            })?;

                        count += 1;
                    }
                }

                tx.commit().map_err(|e| {
                    MnemosyneError::Database(format!("Failed to commit transaction: {}", e))
                })?;

                Ok(count)
            })
            .await
            .map_err(|e| MnemosyneError::Database(format!("Pool interaction failed: {}", e)))??;

        info!("Batch stored {} vectors successfully", count);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (SqliteVectorStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = SqliteVectorStorage::new(db_path, 3).unwrap();
        storage.create_vec_table().await.unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_store_and_retrieve_vector() {
        let (storage, _temp) = create_test_storage().await;
        let memory_id = MemoryId::new();
        let embedding = vec![1.0, 2.0, 3.0];

        // Store vector
        storage.store_vector(&memory_id, &embedding).await.unwrap();

        // Retrieve vector
        let retrieved = storage.get_vector(&memory_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.len(), 3);

        // Check values are close (floating point comparison)
        for (a, b) in embedding.iter().zip(retrieved.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[tokio::test]
    async fn test_dimension_mismatch() {
        let (storage, _temp) = create_test_storage().await;
        let memory_id = MemoryId::new();
        let wrong_embedding = vec![1.0, 2.0]; // Wrong dimension (2 instead of 3)

        let result = storage.store_vector(&memory_id, &wrong_embedding).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("dimension mismatch"));
    }

    #[tokio::test]
    async fn test_search_similar() {
        let (storage, _temp) = create_test_storage().await;

        // Store three vectors
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        let id3 = MemoryId::new();

        // Similar vectors
        storage.store_vector(&id1, &[1.0, 0.0, 0.0]).await.unwrap();
        storage.store_vector(&id2, &[0.9, 0.1, 0.0]).await.unwrap();

        // Different vector
        storage.store_vector(&id3, &[0.0, 0.0, 1.0]).await.unwrap();

        // Search for vectors similar to [1.0, 0.0, 0.0]
        let query = [1.0, 0.0, 0.0];
        let results = storage.search_similar(&query, 3, 0.5).await.unwrap();

        // Should find id1 (exact match) and id2 (similar), but not id3 (different)
        assert!(results.len() >= 2);

        // First result should be id1 with high similarity
        let (found_id, similarity) = &results[0];
        assert_eq!(*found_id, id1);
        assert!(*similarity > 0.99);

        // Second result should be id2 with lower but still high similarity
        let (found_id, similarity) = &results[1];
        assert_eq!(*found_id, id2);
        assert!(*similarity > 0.5);
    }

    #[tokio::test]
    async fn test_delete_vector() {
        let (storage, _temp) = create_test_storage().await;
        let memory_id = MemoryId::new();
        let embedding = vec![1.0, 2.0, 3.0];

        // Store and verify
        storage.store_vector(&memory_id, &embedding).await.unwrap();
        assert!(storage.get_vector(&memory_id).await.unwrap().is_some());

        // Delete and verify
        storage.delete_vector(&memory_id).await.unwrap();
        assert!(storage.get_vector(&memory_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_count_vectors() {
        let (storage, _temp) = create_test_storage().await;

        assert_eq!(storage.count_vectors().await.unwrap(), 0);

        // Store some vectors
        storage
            .store_vector(&MemoryId::new(), &[1.0, 0.0, 0.0])
            .await
            .unwrap();
        storage
            .store_vector(&MemoryId::new(), &[0.0, 1.0, 0.0])
            .await
            .unwrap();
        storage
            .store_vector(&MemoryId::new(), &[0.0, 0.0, 1.0])
            .await
            .unwrap();

        assert_eq!(storage.count_vectors().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_batch_store() {
        let (storage, _temp) = create_test_storage().await;

        let vectors = vec![
            (MemoryId::new(), vec![1.0, 0.0, 0.0]),
            (MemoryId::new(), vec![0.0, 1.0, 0.0]),
            (MemoryId::new(), vec![0.0, 0.0, 1.0]),
        ];

        let count = storage.batch_store_vectors(&vectors).await.unwrap();
        assert_eq!(count, 3);
        assert_eq!(storage.count_vectors().await.unwrap(), 3);
    }
}
