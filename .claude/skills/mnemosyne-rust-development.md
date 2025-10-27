---
name: mnemosyne-rust-development
description: Rust development patterns and practices for Mnemosyne codebase
---

# Mnemosyne Rust Development

**Scope**: Rust-specific patterns, architecture, and best practices for Mnemosyne
**Lines**: ~400
**Last Updated**: 2025-10-27

## When to Use This Skill

Activate this skill when:
- Working on Mnemosyne's Rust codebase
- Implementing new MCP tools or services
- Modifying storage layer (SQLite + FTS5)
- Working with Tokio async runtime
- Creating or updating PyO3 Python bindings
- Writing tests for Rust components
- Understanding Mnemosyne's architecture

## Codebase Structure

```
src/
├── main.rs                 # CLI entry point
├── lib.rs                  # Library exports
├── mcp/                    # MCP server layer
│   ├── protocol.rs         # JSON-RPC 2.0 types
│   ├── server.rs           # Async MCP server
│   └── tools.rs            # 8 OODA-aligned tools
├── services/               # Business logic layer
│   ├── llm.rs              # Claude Haiku integration
│   └── namespace.rs        # Git + CLAUDE.md detection
├── storage/                # Storage layer
│   ├── mod.rs              # Storage trait + SQLite impl
│   ├── schema.rs           # Database schema
│   └── fts.rs              # FTS5 search impl
├── types/                  # Core domain types
│   ├── memory.rs           # MemoryNote, MemoryType
│   ├── link.rs             # SemanticLink, LinkType
│   └── namespace.rs        # Namespace enum
├── config/                 # Configuration management
│   └── keychain.rs         # OS keychain integration
└── python/                 # PyO3 bindings
    ├── mod.rs              # Python module
    ├── coordinator.rs      # PyCoordinator
    └── storage.rs          # PyStorage
```

## Core Architecture Patterns

### Result-Based Error Handling

**All public functions return `Result<T, E>`**:
```rust
pub async fn store_memory(
    &self,
    note: MemoryNote
) -> Result<MemoryId, StorageError> {
    // Validate input
    if note.content.is_empty() {
        return Err(StorageError::InvalidInput("content cannot be empty"));
    }

    // Perform operation
    let id = self.insert_memory(&note).await?;

    // Return success
    Ok(id)
}
```

**Error types are domain-specific**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Memory not found: {id}")]
    NotFound { id: MemoryId },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### Async/Await with Tokio

**All I/O operations are async**:
```rust
use tokio::runtime::Runtime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = SqliteStorage::new("mnemosyne.db").await?;
    let memory = storage.get_memory(id).await?;
    Ok(())
}
```

**Test utilities**:
```rust
#[tokio::test]
async fn test_memory_storage() {
    let storage = create_test_storage().await;
    let id = storage.store_memory(test_memory()).await.unwrap();
    assert_memory_exists(&storage, id).await;
}
```

### Type Safety with Newtypes

**Strong typing prevents errors**:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(Uuid);

impl MemoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, ParseError> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

// Can't accidentally use String where MemoryId expected
fn load_memory(id: MemoryId) -> Result<MemoryNote, StorageError> {
    // Type system ensures id is valid UUID
    ...
}
```

### Builder Pattern for Complex Types

**Use builders for construction**:
```rust
let memory = MemoryNote::builder()
    .content("Architecture decision")
    .namespace(Namespace::project("mnemosyne"))
    .importance(9)
    .memory_type(MemoryType::ArchitectureDecision)
    .tags(vec!["architecture", "rust", "storage"])
    .build()?;
```

## SQLite + FTS5 Patterns

### Schema Design

**Core tables**:
```sql
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    summary TEXT,
    namespace TEXT NOT NULL,
    importance INTEGER NOT NULL,
    memory_type TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    accessed_at INTEGER NOT NULL,
    access_count INTEGER DEFAULT 0,
    is_archived BOOLEAN DEFAULT FALSE
);

CREATE VIRTUAL TABLE memories_fts USING fts5(
    content, summary, keywords, tags,
    content='memories', content_rowid='rowid'
);

CREATE TABLE semantic_links (
    from_id TEXT NOT NULL,
    to_id TEXT NOT NULL,
    link_type TEXT NOT NULL,
    strength REAL NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (from_id, to_id, link_type),
    FOREIGN KEY (from_id) REFERENCES memories(id),
    FOREIGN KEY (to_id) REFERENCES memories(id)
);
```

### Query Patterns

**Hybrid search (FTS5 + semantic)**:
```rust
pub async fn search_memories(
    &self,
    query: &str,
    namespace: &Namespace,
    min_importance: u8
) -> Result<Vec<MemoryNote>, StorageError> {
    // 1. FTS5 keyword search
    let fts_results = sqlx::query_as!(
        MemoryRow,
        r#"
        SELECT m.*
        FROM memories m
        JOIN memories_fts fts ON m.rowid = fts.rowid
        WHERE fts MATCH ?
          AND m.namespace LIKE ?
          AND m.importance >= ?
          AND m.is_archived = FALSE
        ORDER BY rank
        LIMIT 50
        "#,
        query,
        namespace.to_glob(),
        min_importance
    )
    .fetch_all(&self.pool)
    .await?;

    // 2. Semantic similarity (future: vector embeddings)
    // 3. Graph traversal for related memories
    // 4. Importance weighting

    Ok(fts_results.into_iter().map(|r| r.into()).collect())
}
```

**Graph traversal with recursive CTE**:
```rust
pub async fn get_memory_graph(
    &self,
    seed_ids: &[MemoryId],
    max_hops: usize
) -> Result<MemoryGraph, StorageError> {
    let query = format!(
        r#"
        WITH RECURSIVE graph(id, depth) AS (
            -- Base case: seed memories
            SELECT id, 0 as depth
            FROM unnest(?) AS id

            UNION

            -- Recursive case: follow links
            SELECT sl.to_id as id, g.depth + 1 as depth
            FROM graph g
            JOIN semantic_links sl ON sl.from_id = g.id
            WHERE g.depth < ?
              AND sl.strength >= 0.3
        )
        SELECT DISTINCT m.*
        FROM graph g
        JOIN memories m ON m.id = g.id
        WHERE m.is_archived = FALSE
        "#,
        max_hops
    );

    let memories = sqlx::query_as(&query)
        .bind(seed_ids)
        .fetch_all(&self.pool)
        .await?;

    Ok(MemoryGraph::from_memories(memories))
}
```

### Transaction Management

**Use transactions for multi-step operations**:
```rust
pub async fn consolidate_memories(
    &self,
    memory_ids: &[MemoryId],
    consolidated: MemoryNote
) -> Result<MemoryId, StorageError> {
    let mut tx = self.pool.begin().await?;

    // 1. Insert consolidated memory
    let new_id = sqlx::query_scalar!(
        "INSERT INTO memories (...) VALUES (...) RETURNING id",
        // ...
    )
    .fetch_one(&mut *tx)
    .await?;

    // 2. Create supersession links
    for old_id in memory_ids {
        sqlx::query!(
            "INSERT INTO semantic_links (from_id, to_id, link_type, strength)
             VALUES (?, ?, 'Supersedes', 1.0)",
            new_id, old_id
        )
        .execute(&mut *tx)
        .await?;
    }

    // 3. Archive old memories
    sqlx::query!(
        "UPDATE memories SET is_archived = TRUE WHERE id IN (?)",
        memory_ids
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(new_id)
}
```

## PyO3 Bindings Patterns

### Exposing Rust to Python

**Module definition**:
```rust
use pyo3::prelude::*;

#[pymodule]
fn mnemosyne_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyStorage>()?;
    m.add_class::<PyCoordinator>()?;
    m.add_class::<PyMemory>()?;
    Ok(())
}
```

**Class wrapping**:
```rust
#[pyclass]
pub struct PyStorage {
    storage: Arc<SqliteStorage>,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl PyStorage {
    #[new]
    pub fn new(db_path: String) -> PyResult<Self> {
        let runtime = Runtime::new()?;
        let storage = runtime.block_on(async {
            SqliteStorage::new(&db_path).await
        })?;

        Ok(Self {
            storage: Arc::new(storage),
            runtime: Arc::new(runtime),
        })
    }

    pub fn store(&self, content: String, namespace: String) -> PyResult<String> {
        let memory = MemoryNote::builder()
            .content(content)
            .namespace(Namespace::from_str(&namespace)?)
            .build()?;

        let id = self.runtime.block_on(async {
            self.storage.store_memory(memory).await
        })?;

        Ok(id.to_string())
    }
}
```

**Error conversion**:
```rust
impl From<StorageError> for PyErr {
    fn from(err: StorageError) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
    }
}
```

## Testing Patterns

### Unit Tests

**Test individual functions**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = create_test_storage().await;

        let memory = MemoryNote::builder()
            .content("test content")
            .namespace(Namespace::Global)
            .importance(5)
            .build()
            .unwrap();

        let id = storage.store_memory(memory.clone()).await.unwrap();
        let retrieved = storage.get_memory(id).await.unwrap();

        assert_eq!(retrieved.content, "test content");
        assert_eq!(retrieved.importance, 5);
    }
}
```

### Integration Tests

**Test full workflows**:
```rust
// tests/integration/memory_workflow.rs
#[tokio::test]
async fn test_memory_creation_and_retrieval() {
    let storage = setup_test_storage().await;
    let llm = setup_test_llm().await;

    // Store memory with enrichment
    let memory = create_memory_with_enrichment(
        &storage,
        &llm,
        "Architecture decision: use SQLite"
    ).await.unwrap();

    // Search for memory
    let results = storage.search_memories(
        "architecture",
        &Namespace::project("mnemosyne"),
        7
    ).await.unwrap();

    assert!(results.iter().any(|m| m.id == memory.id));
}
```

### Test Utilities

**Common test helpers**:
```rust
pub async fn create_test_storage() -> SqliteStorage {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();

    let storage = SqliteStorage::new(db_path).await.unwrap();
    storage.initialize_schema().await.unwrap();
    storage
}

pub fn test_memory() -> MemoryNote {
    MemoryNote::builder()
        .content("Test memory content")
        .namespace(Namespace::Global)
        .importance(5)
        .memory_type(MemoryType::CodePattern)
        .build()
        .unwrap()
}
```

## Common Patterns

### Dependency Injection

**Use traits for testability**:
```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store_memory(&self, note: MemoryNote) -> Result<MemoryId, StorageError>;
    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote, StorageError>;
    // ...
}

// Production implementation
pub struct SqliteStorage { ... }

#[async_trait]
impl StorageBackend for SqliteStorage { ... }

// Test implementation
pub struct MockStorage { ... }

#[async_trait]
impl StorageBackend for MockStorage { ... }
```

### Configuration Management

**Use config structs**:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MnemosyneConfig {
    pub database_url: String,
    pub anthropic_api_key: Option<String>,
    pub log_level: String,
    pub max_memory_age_days: u32,
}

impl MnemosyneConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::from_env().map_err(ConfigError::from)
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        toml::from_str(&contents).map_err(ConfigError::from)
    }
}
```

## Best Practices

### Error Handling

**DO**:
- Return `Result<T, E>` from all fallible operations
- Use `thiserror` for domain-specific error types
- Provide context with error messages
- Convert errors at API boundaries

**DON'T**:
- Use `.unwrap()` in production code
- Use `.expect()` without clear justification
- Swallow errors silently
- Return generic `Box<dyn Error>`

### Async Patterns

**DO**:
- Use `async/await` for all I/O operations
- Spawn tasks for parallel work
- Use `tokio::select!` for concurrent operations
- Test async code with `#[tokio::test]`

**DON'T**:
- Block the async runtime with sync operations
- Create too many tasks (use bounded channels)
- Forget to `.await` async functions
- Mix sync and async inappropriately

### Type Safety

**DO**:
- Use newtypes for domain concepts (MemoryId, Namespace)
- Leverage `#[derive]` macros
- Make illegal states unrepresentable
- Use enums for variants

**DON'T**:
- Use stringly-typed APIs
- Abuse `String` for structured data
- Ignore compiler warnings
- Over-use `Any` or dynamic typing

## Performance Considerations

### Database Optimization

**Index strategy**:
```sql
CREATE INDEX idx_memories_namespace ON memories(namespace);
CREATE INDEX idx_memories_importance ON memories(importance);
CREATE INDEX idx_memories_created_at ON memories(created_at);
CREATE INDEX idx_links_from ON semantic_links(from_id);
CREATE INDEX idx_links_to ON semantic_links(to_id);
```

**Query optimization**:
- Use `EXPLAIN QUERY PLAN` to analyze queries
- Limit result sets with `LIMIT` clauses
- Use prepared statements (sqlx does this automatically)
- Batch operations when possible

### Memory Management

**Use Arc for shared ownership**:
```rust
pub struct ServiceLayer {
    storage: Arc<SqliteStorage>,
    llm: Arc<LlmService>,
}
```

**Avoid unnecessary clones**:
```rust
// BAD
fn process_memory(memory: MemoryNote) { ... }

// GOOD
fn process_memory(memory: &MemoryNote) { ... }
```

## Further Reading

- `ARCHITECTURE.md`: System design and component overview
- `mnemosyne-mcp-protocol.md`: MCP server implementation patterns
- Rust Book: https://doc.rust-lang.org/book/
- Tokio Tutorial: https://tokio.rs/tokio/tutorial
- SQLx Documentation: https://github.com/launchbadge/sqlx
