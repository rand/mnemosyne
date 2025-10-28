-- Mnemosyne Initial Schema for LibSQL/Turso
-- Version: 0.1.0
-- Created: 2025-10-27
--
-- This schema uses LibSQL's native vector search capabilities with F32_BLOB
-- for embedding storage, eliminating the need for separate extension loading.

-- Enable foreign keys (must be set per connection)
PRAGMA foreign_keys = ON;

-- ============================================================================
-- Memories Table
-- ============================================================================
-- Core memory storage with all metadata INCLUDING embeddings (native vector support)

CREATE TABLE IF NOT EXISTS memories (
    -- Identity
    id TEXT PRIMARY KEY NOT NULL,
    namespace TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Content (human-readable)
    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    keywords TEXT NOT NULL,  -- JSON array: ["keyword1", "keyword2"]
    tags TEXT NOT NULL,      -- JSON array: ["tag1", "tag2"]
    context TEXT NOT NULL,   -- When/why this is relevant

    -- Classification
    memory_type TEXT NOT NULL CHECK(memory_type IN (
        'architecture_decision',
        'code_pattern',
        'bug_fix',
        'configuration',
        'constraint',
        'entity',
        'insight',
        'reference',
        'preference'
    )),
    importance INTEGER NOT NULL CHECK(importance BETWEEN 1 AND 10),
    confidence REAL NOT NULL CHECK(confidence BETWEEN 0.0 AND 1.0),

    -- Relationships
    related_files TEXT NOT NULL DEFAULT '[]',    -- JSON array
    related_entities TEXT NOT NULL DEFAULT '[]', -- JSON array

    -- Lifecycle
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK(is_archived IN (0, 1)),
    superseded_by TEXT,

    -- Computational (with native vector support)
    embedding_model TEXT NOT NULL,
    embedding F32_BLOB(384),  -- Native vector storage (384 dimensions)

    -- Foreign keys
    FOREIGN KEY (superseded_by) REFERENCES memories(id)
);

-- ============================================================================
-- Memory Links Table
-- ============================================================================
-- Knowledge graph edges with typed relationships

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

    -- Prevent duplicate links
    UNIQUE (source_id, target_id, link_type)
);

-- ============================================================================
-- Audit Log Table
-- ============================================================================
-- Immutable audit trail for all memory operations

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
    metadata TEXT NOT NULL,  -- JSON object with operation-specific data (renamed from 'details' to avoid reserved keyword)

    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

-- ============================================================================
-- Full-Text Search (FTS5)
-- ============================================================================
-- Virtual table for fast keyword search

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    content,
    summary,
    keywords,
    tags,
    context,
    content='memories',
    content_rowid='rowid',
    tokenize='porter'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content, summary, keywords, tags, context)
    VALUES (NEW.rowid, NEW.content, NEW.summary, NEW.keywords, NEW.tags, NEW.context);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, summary, keywords, tags, context)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.summary, OLD.keywords, OLD.tags, OLD.context);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, summary, keywords, tags, context)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.summary, OLD.keywords, OLD.tags, OLD.context);
    INSERT INTO memories_fts(rowid, content, summary, keywords, tags, context)
    VALUES (NEW.rowid, NEW.content, NEW.summary, NEW.keywords, NEW.tags, NEW.context);
END;

-- ============================================================================
-- Metadata Table
-- ============================================================================
-- Store schema version and configuration

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert initial metadata
INSERT OR IGNORE INTO metadata (key, value) VALUES ('schema_version', '1');
INSERT OR IGNORE INTO metadata (key, value) VALUES ('created_at', datetime('now'));
INSERT OR IGNORE INTO metadata (key, value) VALUES ('vector_search_enabled', 'true');
INSERT OR IGNORE INTO metadata (key, value) VALUES ('embedding_dimension', '384');

-- ============================================================================
-- Views for Common Queries
-- ============================================================================

-- Active memories (not archived)
CREATE VIEW IF NOT EXISTS active_memories AS
SELECT * FROM memories WHERE is_archived = 0;

-- High importance memories
CREATE VIEW IF NOT EXISTS important_memories AS
SELECT * FROM memories WHERE importance >= 8 AND is_archived = 0;

-- Recent memories (last 30 days)
CREATE VIEW IF NOT EXISTS recent_memories AS
SELECT * FROM memories
WHERE updated_at >= datetime('now', '-30 days')
ORDER BY updated_at DESC;

-- Memory stats by namespace
CREATE VIEW IF NOT EXISTS memory_stats AS
SELECT
    namespace,
    COUNT(*) as total_count,
    SUM(CASE WHEN is_archived = 0 THEN 1 ELSE 0 END) as active_count,
    SUM(CASE WHEN is_archived = 1 THEN 1 ELSE 0 END) as archived_count,
    AVG(importance) as avg_importance,
    MAX(updated_at) as last_updated
FROM memories
GROUP BY namespace;
