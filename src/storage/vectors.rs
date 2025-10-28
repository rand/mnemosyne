//! Vector storage implementation using sqlite-vec
//!
//! This module provides vector similarity search using the dual storage approach:
//! - rusqlite with sqlite-vec extension for vector operations
//! - Same database file as libsql (no conflicts)
//! - Separate vec0 virtual table for embeddings

use crate::error::{MnemosyneError, Result};
use crate::types::MemoryId;
use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;
use tracing::{debug, info};

/// Vector storage backend using sqlite-vec
pub struct SqliteVectorStorage {
    conn: Connection,
    dimensions: usize,
}

impl SqliteVectorStorage {
    /// Create a new vector storage instance
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
        let path_str = db_path.as_ref().to_string_lossy();
        info!("Opening vector storage at: {} (dimensions: {})", path_str, dimensions);

        // Load sqlite-vec extension BEFORE opening connection
        // Register it as an auto-extension so it's available for all connections
        // This pattern is taken directly from sqlite-vec's own tests
        unsafe {
            use rusqlite::ffi::sqlite3_auto_extension;

            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const ()
            )));
        }

        // Open database with rusqlite (extension will be auto-loaded)
        let conn = Connection::open(db_path)
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        info!("sqlite-vec extension registered and loaded successfully");

        Ok(Self { conn, dimensions })
    }

    /// Create the vec0 virtual table for vector storage
    ///
    /// This should be called once during initialization or migration.
    /// It's safe to call multiple times (uses IF NOT EXISTS).
    pub fn create_vec_table(&self) -> Result<()> {
        info!("Creating vec0 virtual table for vectors (dimensions: {})", self.dimensions);

        let sql = format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
                memory_id TEXT PRIMARY KEY,
                embedding FLOAT[{}]
            )",
            self.dimensions
        );

        self.conn
            .execute(&sql, [])
            .map_err(|e| MnemosyneError::Database(format!("Failed to create vec0 table: {}", e)))?;

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
    pub fn store_vector(&self, memory_id: &MemoryId, embedding: &[f32]) -> Result<()> {
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

        self.conn
            .execute(
                "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding)
                 VALUES (?, vec_f32(?))",
                rusqlite::params![id, embedding_json],
            )
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to store vector: {}", e))
            })?;

        debug!("Vector stored successfully for memory: {}", memory_id);
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
    pub fn get_vector(&self, memory_id: &MemoryId) -> Result<Option<Vec<f32>>> {
        debug!("Retrieving vector for memory: {}", memory_id);

        let id = memory_id.to_string();
        let mut stmt = self
            .conn
            .prepare("SELECT embedding FROM memory_vectors WHERE memory_id = ?")
            .map_err(|e| MnemosyneError::Database(format!("Failed to prepare query: {}", e)))?;

        let result: SqliteResult<Vec<u8>> = stmt.query_row(rusqlite::params![id], |row| row.get(0));

        match result {
            Ok(blob) => {
                // Convert blob to Vec<f32>
                let floats = blob
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                debug!("Vector retrieved successfully for memory: {}", memory_id);
                Ok(Some(floats))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!("No vector found for memory: {}", memory_id);
                Ok(None)
            }
            Err(e) => Err(MnemosyneError::Database(format!(
                "Failed to retrieve vector: {}",
                e
            ))),
        }
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
    pub fn search_similar(
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

        let mut stmt = self
            .conn
            .prepare(
                "SELECT memory_id, distance
                 FROM memory_vectors
                 WHERE embedding MATCH vec_f32(?)
                 ORDER BY distance
                 LIMIT ?",
            )
            .map_err(|e| MnemosyneError::Database(format!("Failed to prepare search: {}", e)))?;

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

        debug!("Vector search returned {} results", results.len());
        Ok(results)
    }

    /// Delete a vector for a memory
    ///
    /// # Arguments
    /// * `memory_id` - The memory identifier
    pub fn delete_vector(&self, memory_id: &MemoryId) -> Result<()> {
        debug!("Deleting vector for memory: {}", memory_id);

        let id = memory_id.to_string();
        self.conn
            .execute(
                "DELETE FROM memory_vectors WHERE memory_id = ?",
                rusqlite::params![id],
            )
            .map_err(|e| MnemosyneError::Database(format!("Failed to delete vector: {}", e)))?;

        debug!("Vector deleted successfully for memory: {}", memory_id);
        Ok(())
    }

    /// Count total vectors in storage
    ///
    /// # Returns
    /// * Total number of vectors stored
    pub fn count_vectors(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memory_vectors", [], |row| row.get(0))
            .map_err(|e| MnemosyneError::Database(format!("Failed to count vectors: {}", e)))?;

        Ok(count as usize)
    }

    /// Batch store multiple vectors (more efficient than individual stores)
    ///
    /// # Arguments
    /// * `vectors` - Slice of (memory_id, embedding) tuples
    ///
    /// # Returns
    /// * Number of vectors successfully stored
    pub fn batch_store_vectors(&mut self, vectors: &[(MemoryId, Vec<f32>)]) -> Result<usize> {
        info!("Batch storing {} vectors", vectors.len());

        let tx = self
            .conn
            .transaction()
            .map_err(|e| MnemosyneError::Database(format!("Failed to begin transaction: {}", e)))?;

        let mut count = 0;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding)
                     VALUES (?, vec_f32(?))",
                )
                .map_err(|e| {
                    MnemosyneError::Database(format!("Failed to prepare batch insert: {}", e))
                })?;

            for (memory_id, embedding) in vectors {
                if embedding.len() != self.dimensions {
                    debug!(
                        "Skipping vector {} due to dimension mismatch: expected {}, got {}",
                        memory_id,
                        self.dimensions,
                        embedding.len()
                    );
                    continue;
                }

                let id = memory_id.to_string();
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

        tx.commit()
            .map_err(|e| MnemosyneError::Database(format!("Failed to commit transaction: {}", e)))?;

        info!("Batch stored {} vectors successfully", count);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_storage() -> (SqliteVectorStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = SqliteVectorStorage::new(db_path, 3).unwrap();
        storage.create_vec_table().unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_store_and_retrieve_vector() {
        let (storage, _temp) = create_test_storage();
        let memory_id = MemoryId::new();
        let embedding = vec![1.0, 2.0, 3.0];

        // Store vector
        storage.store_vector(&memory_id, &embedding).unwrap();

        // Retrieve vector
        let retrieved = storage.get_vector(&memory_id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.len(), 3);

        // Check values are close (floating point comparison)
        for (a, b) in embedding.iter().zip(retrieved.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn test_dimension_mismatch() {
        let (storage, _temp) = create_test_storage();
        let memory_id = MemoryId::new();
        let wrong_embedding = vec![1.0, 2.0]; // Wrong dimension (2 instead of 3)

        let result = storage.store_vector(&memory_id, &wrong_embedding);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("dimension mismatch"));
    }

    #[test]
    fn test_search_similar() {
        let (storage, _temp) = create_test_storage();

        // Store three vectors
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        let id3 = MemoryId::new();

        // Similar vectors
        storage.store_vector(&id1, &[1.0, 0.0, 0.0]).unwrap();
        storage.store_vector(&id2, &[0.9, 0.1, 0.0]).unwrap();

        // Different vector
        storage.store_vector(&id3, &[0.0, 0.0, 1.0]).unwrap();

        // Search for vectors similar to [1.0, 0.0, 0.0]
        let query = [1.0, 0.0, 0.0];
        let results = storage.search_similar(&query, 3, 0.5).unwrap();

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

    #[test]
    fn test_delete_vector() {
        let (storage, _temp) = create_test_storage();
        let memory_id = MemoryId::new();
        let embedding = vec![1.0, 2.0, 3.0];

        // Store and verify
        storage.store_vector(&memory_id, &embedding).unwrap();
        assert!(storage.get_vector(&memory_id).unwrap().is_some());

        // Delete and verify
        storage.delete_vector(&memory_id).unwrap();
        assert!(storage.get_vector(&memory_id).unwrap().is_none());
    }

    #[test]
    fn test_count_vectors() {
        let (storage, _temp) = create_test_storage();

        assert_eq!(storage.count_vectors().unwrap(), 0);

        // Store some vectors
        storage.store_vector(&MemoryId::new(), &[1.0, 0.0, 0.0]).unwrap();
        storage.store_vector(&MemoryId::new(), &[0.0, 1.0, 0.0]).unwrap();
        storage.store_vector(&MemoryId::new(), &[0.0, 0.0, 1.0]).unwrap();

        assert_eq!(storage.count_vectors().unwrap(), 3);
    }

    #[test]
    fn test_batch_store() {
        let (mut storage, _temp) = create_test_storage();

        let vectors = vec![
            (MemoryId::new(), vec![1.0, 0.0, 0.0]),
            (MemoryId::new(), vec![0.0, 1.0, 0.0]),
            (MemoryId::new(), vec![0.0, 0.0, 1.0]),
        ];

        let count = storage.batch_store_vectors(&vectors).unwrap();
        assert_eq!(count, 3);
        assert_eq!(storage.count_vectors().unwrap(), 3);
    }
}
