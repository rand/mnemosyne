//! Test utilities for storage initialization
//!
//! Provides embedded schema initialization for test environments,
//! avoiding filesystem dependencies for in-memory databases.

use crate::error::Result;
use crate::storage::libsql::LibsqlStorage;
use crate::ConnectionMode;
use std::sync::Arc;

/// Create an in-memory storage backend with schema pre-initialized
///
/// This uses embedded SQL to initialize the schema, avoiding filesystem
/// dependencies that can be unreliable in test contexts.
pub async fn create_test_storage() -> Result<Arc<LibsqlStorage>> {
    // Create in-memory storage WITHOUT schema validation
    let storage = Arc::new(LibsqlStorage::new_with_validation(
        ConnectionMode::InMemory,
        true  // create_if_missing - will run migrations
    ).await?);

    Ok(storage)
}

/// Minimal embedded schema for testing
///
/// This is a subset of the full schema, containing only what's needed
/// for basic memory storage tests. For full schema, use filesystem migrations.
pub const EMBEDDED_TEST_SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

-- Memories Table (minimal version for testing)
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY NOT NULL,
    namespace TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    keywords TEXT NOT NULL,
    tags TEXT NOT NULL,
    context TEXT NOT NULL,
    memory_type TEXT NOT NULL CHECK(memory_type IN (
        'architecture_decision',
        'code_pattern',
        'bug_fix',
        'configuration',
        'constraint',
        'entity',
        'insight',
        'reference',
        'preference',
        'agent_event'
    )),
    importance INTEGER NOT NULL CHECK(importance BETWEEN 1 AND 10),
    confidence REAL NOT NULL CHECK(confidence BETWEEN 0.0 AND 1.0),
    related_files TEXT NOT NULL DEFAULT '[]',
    related_entities TEXT NOT NULL DEFAULT '[]',
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    is_archived INTEGER NOT NULL DEFAULT 0,
    superseded_by TEXT,
    embedding_model TEXT NOT NULL DEFAULT '',
    embedding F32_BLOB(384),
    FOREIGN KEY (superseded_by) REFERENCES memories(id)
);

-- Memory Links Table
CREATE TABLE IF NOT EXISTS memory_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    link_type TEXT NOT NULL CHECK(link_type IN (
        'extends',
        'contradicts',
        'implements',
        'references',
        'supersedes'
    )),
    strength REAL NOT NULL DEFAULT 0.5 CHECK(strength BETWEEN 0.0 AND 1.0),
    reason TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (source_id) REFERENCES memories(id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES memories(id) ON DELETE CASCADE,
    UNIQUE (source_id, target_id, link_type)
);

-- Audit Log Table
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    operation TEXT NOT NULL CHECK(operation IN (
        'create',
        'update',
        'archive',
        'supersede',
        'link_create',
        'link_update',
        'link_delete',
        'consolidate'
    )),
    memory_id TEXT,
    metadata TEXT NOT NULL,
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

-- Metadata Table
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO metadata (key, value) VALUES ('schema_version', '1');
INSERT OR IGNORE INTO metadata (key, value) VALUES ('created_at', datetime('now'));

-- Basic indexes for testing
CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at DESC);
"#;

/// Create an in-memory storage with embedded schema (for tests that need immediate schema)
///
/// Use this when the normal migration system isn't working in your test context.
///
/// Note: This actually uses a temporary file-based database because libsql's in-memory
/// databases don't share state across connections (each get_conn() call would see
/// an empty database). Using a temp file is more reliable and still fast for tests.
pub async fn create_test_storage_with_embedded_schema() -> Result<Arc<LibsqlStorage>> {
    use libsql::params;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Use atomic counter to ensure unique database file per test
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);

    // Create a temporary file for the database
    // This ensures all connections see the same schema and tests don't conflict
    let temp_path = env::temp_dir().join(format!("mnemosyne_test_{}_{}.db", std::process::id(), counter));

    // Clean up any existing test database
    let _ = std::fs::remove_file(&temp_path);

    let db = libsql::Builder::new_local(&temp_path)
        .build()
        .await
        .map_err(|e| crate::error::MnemosyneError::Database(format!("Failed to create test database: {}", e)))?;

    // Create storage instance using the test constructor
    let storage = LibsqlStorage::from_database(db);

    // Execute embedded schema directly
    let conn = storage.get_conn()?;

    // Execute schema statements one by one
    // We need to split by semicolons and execute each CREATE statement separately
    eprintln!("Executing embedded schema...");
    for statement in EMBEDDED_TEST_SCHEMA.split(';') {
        // Remove comment lines and trim
        let statement: String = statement
            .lines()
            .filter(|line| !line.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if !statement.is_empty() {
            eprintln!("Executing: {}", &statement[..statement.len().min(80)]);
            conn.execute(&statement, params![]).await
                .map_err(|e| crate::error::MnemosyneError::Database(format!("Failed to execute statement: {} - Error: {}", &statement[..statement.len().min(100)], e)))?;
        }
    }

    // Verify tables were created
    let mut rows = conn
        .query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name", params![])
        .await
        .map_err(|e| crate::error::MnemosyneError::Database(format!("Failed to query tables: {}", e)))?;

    let mut tables = Vec::new();
    while let Some(row) = rows.next().await
        .map_err(|e| crate::error::MnemosyneError::Database(format!("Failed to get next row: {}", e)))? {
        if let Ok(name) = row.get::<String>(0) {
            tables.push(name);
        }
    }

    eprintln!("Created tables: {:?}", tables);

    if !tables.contains(&"memories".to_string()) {
        return Err(crate::error::MnemosyneError::Database(
            "Schema creation failed: memories table not found".to_string()
        ));
    }

    Ok(Arc::new(storage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageBackend;
    use crate::types::{MemoryId, MemoryNote, MemoryType, Namespace};
    use chrono::Utc;

    #[tokio::test]
    async fn test_embedded_schema_storage() {
        let storage = create_test_storage_with_embedded_schema()
            .await
            .expect("Failed to create test storage");

        // Try to store a memory
        let now = Utc::now();
        let memory = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Session {
                project: "test".to_string(),
                session_id: "test-1".to_string(),
            },
            created_at: now,
            updated_at: now,
            content: "Test content".to_string(),
            summary: "Test summary".to_string(),
            keywords: vec!["test".to_string()],
            tags: vec!["test".to_string()],
            context: "test context".to_string(),
            memory_type: MemoryType::Insight,
            importance: 5,
            confidence: 0.9,
            links: Vec::new(),
            related_files: Vec::new(),
            related_entities: Vec::new(),
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        storage.store_memory(&memory).await.expect("Failed to store memory");

        // Verify we can retrieve it
        let retrieved = storage.get_memory(memory.id).await.expect("Failed to retrieve memory");
        assert_eq!(retrieved.content, "Test content");
    }
}
