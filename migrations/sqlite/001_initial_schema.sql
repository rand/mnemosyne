-- Mnemosyne Initial Schema for SQLite
-- Version: 0.1.0
-- Created: 2025-10-26

-- Enable foreign keys (must be set per connection)
PRAGMA foreign_keys = ON;

-- ============================================================================
-- Memories Table
-- ============================================================================
-- Core memory storage with all metadata except embeddings (stored separately)

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
        'preference',
        'task',
        'agent_event',
        'constitution',
        'feature_spec',
        'implementation_plan',
        'task_breakdown',
        'quality_checklist',
        'clarification'
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

    -- Computational
    embedding_model TEXT NOT NULL,

    -- Foreign keys
    FOREIGN KEY (superseded_by) REFERENCES memories(id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_updated_at ON memories(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_memory_type ON memories(memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_is_archived ON memories(is_archived);
CREATE INDEX IF NOT EXISTS idx_memories_superseded_by ON memories(superseded_by);

-- Composite indexes for common filter combinations
CREATE INDEX IF NOT EXISTS idx_memories_namespace_type
    ON memories(namespace, memory_type);
CREATE INDEX IF NOT EXISTS idx_memories_namespace_archived
    ON memories(namespace, is_archived);
CREATE INDEX IF NOT EXISTS idx_memories_type_importance
    ON memories(memory_type, importance DESC);

-- ============================================================================
-- Memory Embeddings Table
-- ============================================================================
-- Vector embeddings stored separately for efficient similarity search
-- Note: Using standard BLOB for now; sqlite-vec integration will enhance this

CREATE TABLE IF NOT EXISTS memory_embeddings (
    memory_id TEXT PRIMARY KEY NOT NULL,
    embedding BLOB NOT NULL,  -- Serialized f32 vector
    dimension INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_embeddings_memory_id ON memory_embeddings(memory_id);

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

-- Indexes for graph traversal
CREATE INDEX IF NOT EXISTS idx_links_source ON memory_links(source_id);
CREATE INDEX IF NOT EXISTS idx_links_target ON memory_links(target_id);
CREATE INDEX IF NOT EXISTS idx_links_type ON memory_links(link_type);
CREATE INDEX IF NOT EXISTS idx_links_strength ON memory_links(strength DESC);

-- Bidirectional lookup
CREATE INDEX IF NOT EXISTS idx_links_source_target ON memory_links(source_id, target_id);
CREATE INDEX IF NOT EXISTS idx_links_target_source ON memory_links(target_id, source_id);

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

-- Indexes for audit queries
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_memory_id ON audit_log(memory_id);
CREATE INDEX IF NOT EXISTS idx_audit_operation ON audit_log(operation);

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
