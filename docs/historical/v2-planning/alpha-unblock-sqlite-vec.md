# UNBLOCK: sqlite-vec with libsql

**Date**: 2025-10-27
**From**: Main Agent (Coordinator)
**To**: Sub-Agent Alpha
**Priority**: CRITICAL PATH

---

## Problem

libsql doesn't support loadable SQLite extensions the same way as rusqlite. We cannot use the standard sqlite-vec approach of:

```rust
conn.load_extension("vec0")?; // Doesn't work with libsql
```

---

## Solution: Dual Storage Approach

Use **both** libsql and rusqlite in Mnemosyne:
- **libsql**: Primary storage for memories (existing)
- **rusqlite**: Secondary storage for vectors only (new)

This approach is safe because:
1. Both read/write to same SQLite database file
2. Vectors are in separate `vec0` virtual table (no conflicts)
3. Memory IDs link the two systems
4. Performance impact minimal (vectors only accessed on search)

---

## Implementation Plan

### Step 1: Add rusqlite Dependency

**File**: `Cargo.toml`

```toml
[dependencies]
# Existing
libsql = { version = "0.2", features = ["core", "remote", "replication"] }

# Add for vector operations
rusqlite = { version = "0.31", features = ["bundled", "vtab"] }
sqlite-vec = "0.1.1"  # Provides vec0 virtual table
```

### Step 2: Create VectorStorage with rusqlite

**File**: `src/storage/vectors.rs`

```rust
use rusqlite::{Connection, Result as SqliteResult};
use sqlite_vec::sqlite3_vec_init;

pub struct SqliteVectorStorage {
    conn: Connection,
    dimensions: usize,
}

impl SqliteVectorStorage {
    pub fn new(db_path: &str, dimensions: usize) -> Result<Self, StorageError> {
        // Open database with rusqlite
        let conn = Connection::open(db_path)?;

        // Load sqlite-vec extension
        unsafe {
            sqlite_vec::sqlite3_vec_init(
                conn.handle() as *mut _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )?;
        }

        Ok(Self { conn, dimensions })
    }

    pub fn create_vec_table(&self) -> SqliteResult<()> {
        self.conn.execute(
            &format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
                    memory_id TEXT PRIMARY KEY,
                    embedding FLOAT[{}]
                )",
                self.dimensions
            ),
            [],
        )?;
        Ok(())
    }
}

#[async_trait]
impl VectorStorage for SqliteVectorStorage {
    async fn store_vector(&self, memory_id: &MemoryId, embedding: &[f32]) -> Result<()> {
        let id = memory_id.to_string();
        let embedding_json = serde_json::to_string(embedding)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding)
             VALUES (?, vec_f32(?))",
            rusqlite::params![id, embedding_json],
        )?;

        Ok(())
    }

    async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<(MemoryId, f32)>> {
        let query_json = serde_json::to_string(query_embedding)?;

        let mut stmt = self.conn.prepare(
            "SELECT memory_id, distance
             FROM memory_vectors
             WHERE embedding MATCH vec_f32(?)
             AND distance <= ?
             ORDER BY distance
             LIMIT ?",
        )?;

        let max_distance = 1.0 - min_similarity; // Convert similarity to distance

        let results: Vec<(MemoryId, f32)> = stmt
            .query_map(
                rusqlite::params![query_json, max_distance, limit as i64],
                |row| {
                    let id_str: String = row.get(0)?;
                    let distance: f32 = row.get(1)?;
                    Ok((MemoryId::from_str(&id_str).unwrap(), 1.0 - distance))
                },
            )?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(results)
    }

    // ... other trait methods
}
```

### Step 3: Integration with LibSqlStorage

**File**: `src/search/hybrid.rs`

```rust
pub struct DefaultHybridSearcher {
    embeddings: Arc<dyn EmbeddingService>,
    vectors: Arc<SqliteVectorStorage>,  // Uses rusqlite
    storage: Arc<LibSqlStorage>,        // Uses libsql
    weights: SearchWeights,
}

impl DefaultHybridSearcher {
    pub fn new(
        embeddings: Arc<dyn EmbeddingService>,
        vectors: Arc<SqliteVectorStorage>,
        storage: Arc<LibSqlStorage>,
        weights: SearchWeights,
    ) -> Self {
        Self { embeddings, vectors, storage, weights }
    }

    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<ScoredMemory>> {
        // 1. Generate query embedding
        let query_embedding = self.embeddings.embed(query).await?;

        // 2. Parallel search
        let (vector_results, keyword_results, graph_results) = tokio::join!(
            self.vectors.search_similar(&query_embedding, 20, 0.7), // rusqlite
            self.storage.search_fts5(query, 20),                    // libsql
            self.storage.search_graph(query, 2, 5)                  // libsql
        );

        // 3. Merge and rank...
    }
}
```

### Step 4: Migration Schema

**File**: `migrations/006_vector_search.sql`

```sql
-- This migration is run via rusqlite with sqlite-vec loaded

-- Create vec0 virtual table
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536]
);

-- Index for efficient lookups (standard SQLite index)
CREATE INDEX IF NOT EXISTS idx_memory_vectors_id
ON memory_vectors(memory_id);

-- Trigger to cleanup orphaned vectors when memories deleted
CREATE TRIGGER IF NOT EXISTS cleanup_orphaned_vectors
AFTER DELETE ON memories
BEGIN
    DELETE FROM memory_vectors WHERE memory_id = OLD.id;
END;
```

### Step 5: Database Initialization

**File**: `src/storage/mod.rs`

```rust
pub async fn initialize_storage(db_path: &str) -> Result<(LibSqlStorage, SqliteVectorStorage)> {
    // Initialize libsql storage (existing)
    let libsql_storage = LibSqlStorage::new(db_path).await?;

    // Initialize rusqlite vector storage (new)
    let vector_storage = SqliteVectorStorage::new(db_path, VOYAGE_EMBEDDING_DIM)?;
    vector_storage.create_vec_table()?;

    Ok((libsql_storage, vector_storage))
}
```

---

## Why This Works

1. **Same Database File**: Both connections read/write to same `.db` file
2. **No Conflicts**: Virtual table `memory_vectors` is separate from `memories` table
3. **Consistency**: Memory IDs link the two systems
4. **Performance**: Vector operations are rare (search only), so dual connection overhead is minimal
5. **Safety**: rusqlite is read-only for main tables, write-only for vec0 table
6. **Future**: When libsql supports extensions, we can migrate to single connection

---

## Migration Path

**Phase 1** (Now): Dual storage approach
**Phase 2** (Future): When libsql supports sqlite-vec, refactor to single connection
**Phase 3** (v2.1+): Consider pure Rust vector implementation if needed

---

## Action Items for Alpha

1. ✅ Add dependencies to Cargo.toml
2. ✅ Implement `SqliteVectorStorage` with rusqlite
3. ✅ Create migration 006 for vec0 table
4. ✅ Update `HybridSearcher` to use both storages
5. ✅ Write integration tests
6. ✅ Benchmark performance (should be <10ms for vector search)

---

## Expected Timeline

- **Day 1**: Add dependencies, implement SqliteVectorStorage
- **Day 2**: Create migration, test vec0 table creation
- **Day 3**: Integration with HybridSearcher
- **Day 4**: Tests and benchmarks
- **Day 5**: Week 2 complete ✅

This unblocks Week 2 and gets Vector Search back on track.

---

**Status**: Solution provided
**Next**: Alpha implements dual storage approach
**ETA**: Week 2 completion by 2025-11-11

Main Agent
