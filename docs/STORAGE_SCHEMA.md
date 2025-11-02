# Mnemosyne Storage Schema

**Last Updated**: 2025-11-02
**Database**: LibSQL (SQLite-compatible with native vector support)
**Schema Version**: 13 migrations applied
**Source**: `migrations/libsql/`

---

## Overview

Mnemosyne uses **LibSQL** (fork of SQLite) as its primary storage backend, providing:
- **Native vector storage** with `F32_BLOB` type (no extensions needed)
- **FTS5 full-text search** for keyword matching
- **Graph relationships** via `memory_links` table
- **Audit trail** for all operations
- **JSON columns** for flexible schema (Namespace, WorkItem specs)

**Performance**: Sub-millisecond retrieval, ~1MB per 1000 memories

---

## Tables

### `memories`

Primary memory storage with all metadata including embeddings.

```sql
CREATE TABLE memories (
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
        'architecture_decision', 'code_pattern', 'bug_fix',
        'configuration', 'constraint', 'entity', 'insight',
        'reference', 'preference', 'task', 'agent_event'
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

    -- Computational (native vector support)
    embedding_model TEXT NOT NULL,
    embedding F32_BLOB(384),  -- 384-dimensional vector

    FOREIGN KEY (superseded_by) REFERENCES memories(id)
);
```

**Indexes**:
```sql
-- Namespace + importance for efficient project/global queries
CREATE INDEX idx_memories_namespace_importance
ON memories(namespace, importance DESC);

-- Creation time for temporal queries
CREATE INDEX idx_memories_created_at
ON memories(created_at DESC);

-- Memory type filtering
CREATE INDEX idx_memories_type
ON memories(memory_type);

-- Access tracking
CREATE INDEX idx_memories_last_accessed
ON memories(last_accessed_at DESC);

-- Archived memories
CREATE INDEX idx_memories_archived
ON memories(is_archived, updated_at);
```

**JSON Columns**:

`namespace` (TEXT):
```json
// Global
{"type": "global"}

// Project
{"type": "project", "name": "mnemosyne"}

// Session
{"type": "session", "project": "mnemosyne", "session_id": "20251102-143022"}
```

`keywords`, `tags`, `related_files`, `related_entities` (TEXT as JSON arrays):
```json
["keyword1", "keyword2", "keyword3"]
```

**Vector Storage**:
- Type: `F32_BLOB(384)` - LibSQL native vector type
- Dimensions: 384 (text-embedding-3-small model)
- Search: Uses cosine similarity via LibSQL vector functions

---

### `memory_links`

Knowledge graph edges with typed relationships.

```sql
CREATE TABLE memory_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    link_type TEXT NOT NULL CHECK(link_type IN (
        'extends', 'contradicts', 'implements',
        'references', 'supersedes'
    )),
    strength REAL NOT NULL DEFAULT 0.5 CHECK(strength BETWEEN 0.0 AND 1.0),
    reason TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_traversed_at TIMESTAMP,
    user_created INTEGER NOT NULL DEFAULT 0,

    FOREIGN KEY (source_id) REFERENCES memories(id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES memories(id) ON DELETE CASCADE,

    UNIQUE (source_id, target_id, link_type)
);
```

**Indexes**:
```sql
-- Forward graph traversal (A → B)
CREATE INDEX idx_memory_links_source
ON memory_links(source_id, link_type);

-- Reverse graph traversal (B ← A)
CREATE INDEX idx_memory_links_target
ON memory_links(target_id, link_type);

-- Link strength for decay processing
CREATE INDEX idx_memory_links_strength
ON memory_links(strength, last_traversed_at);
```

**Link Types**:
- `extends`: B builds upon A
- `contradicts`: B invalidates A
- `implements`: B implements concept in A
- `references`: B cites A
- `supersedes`: B replaces A

**Link Strength**:
- Initial: 1.0 (user-created), 0.5-0.8 (LLM-generated)
- Decays over time if not traversed
- Protected from decay if `user_created = 1`

---

### `audit_log`

Immutable audit trail for all memory operations.

```sql
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    operation TEXT NOT NULL CHECK(operation IN (
        'create', 'update', 'archive', 'supersede',
        'link_create', 'link_update', 'link_delete',
        'consolidate'
    )),
    memory_id TEXT,
    metadata TEXT NOT NULL,  -- JSON object with operation details

    FOREIGN KEY (memory_id) REFERENCES memories(id)
);
```

**Indexes**:
```sql
CREATE INDEX idx_audit_log_memory
ON audit_log(memory_id, timestamp DESC);

CREATE INDEX idx_audit_log_timestamp
ON audit_log(timestamp DESC);

CREATE INDEX idx_audit_log_operation
ON audit_log(operation, timestamp DESC);
```

**Metadata Examples**:
```json
// Create operation
{
  "user": "agent",
  "importance": 8,
  "memory_type": "architecture_decision"
}

// Consolidate operation
{
  "merged_into": "mem-abc-123",
  "reason": "Duplicate information",
  "superseded_ids": ["mem-def-456", "mem-ghi-789"]
}
```

---

### `memories_fts`

FTS5 virtual table for fast keyword search.

```sql
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content,
    summary,
    keywords,
    tags,
    context,
    content='memories',
    content_rowid='rowid',
    tokenize='porter'
);
```

**Tokenizer**: Porter stemming algorithm (English)

**Synchronized via Triggers**:
- `memories_ai`: After INSERT, add to FTS
- `memories_ad`: After DELETE, remove from FTS
- `memories_au`: After UPDATE, update FTS (conditional in migration 003)

**Query Syntax**:
```sql
-- Simple keyword match
SELECT * FROM memories_fts WHERE memories_fts MATCH 'authentication';

-- Phrase search
SELECT * FROM memories_fts WHERE memories_fts MATCH '"API design"';

-- Boolean operators
SELECT * FROM memories_fts WHERE memories_fts MATCH 'auth AND (jwt OR oauth)';

-- With ranking
SELECT *, rank FROM memories_fts
WHERE memories_fts MATCH 'database'
ORDER BY rank;
```

---

### `memory_evolution_log`

Tracks consolidation and evolution operations.

```sql
CREATE TABLE memory_evolution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    operation_type TEXT NOT NULL CHECK(operation_type IN (
        'consolidate', 'supersede', 'archive', 'importance_recalibration'
    )),
    source_ids TEXT NOT NULL,  -- JSON array of source memory IDs
    result_id TEXT,            -- Resulting memory ID (for merge/consolidate)
    reason TEXT NOT NULL,
    metadata TEXT,             -- JSON with operation-specific details

    FOREIGN KEY (result_id) REFERENCES memories(id)
);
```

**Indexes**:
```sql
CREATE INDEX idx_evolution_log_timestamp
ON memory_evolution_log(timestamp DESC);

CREATE INDEX idx_evolution_log_result
ON memory_evolution_log(result_id);
```

---

### `work_items`

Orchestration work queue (multi-agent system).

```sql
CREATE TABLE work_items (
    id TEXT PRIMARY KEY NOT NULL,
    phase TEXT NOT NULL CHECK(phase IN (
        'prompt', 'spec', 'full_spec', 'plan', 'artifacts'
    )),
    spec TEXT NOT NULL,  -- JSON: WorkItemSpec
    state TEXT NOT NULL CHECK(state IN (
        'pending', 'in_progress', 'blocked', 'complete', 'failed'
    )),
    assigned_agent TEXT,
    dependencies TEXT NOT NULL DEFAULT '[]',  -- JSON array of WorkItemId
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);
```

**Indexes**:
```sql
CREATE INDEX idx_work_items_state
ON work_items(state, phase);

CREATE INDEX idx_work_items_agent
ON work_items(assigned_agent, state);

CREATE INDEX idx_work_items_phase
ON work_items(phase, state);
```

**Spec JSON Structure**:
```json
{
  "task_description": "Implement user authentication",
  "requirements": ["JWT tokens", "Refresh mechanism"],
  "typed_holes": [
    {"name": "AuthService", "interface": "authenticate(credentials)"}
  ],
  "constraints": ["Must complete in <200ms"],
  "test_plan": "Unit + integration tests"
}
```

---

### `agent_events`

Multi-agent coordination events.

```sql
CREATE TABLE agent_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    event_type TEXT NOT NULL CHECK(event_type IN (
        'agent_started', 'agent_completed', 'agent_failed',
        'memory_recalled', 'memory_stored', 'work_assigned',
        'dependency_resolved', 'quality_gate_passed',
        'quality_gate_failed'
    )),
    agent_id TEXT NOT NULL,
    work_item_id TEXT,
    payload TEXT NOT NULL,  -- JSON with event-specific data

    FOREIGN KEY (work_item_id) REFERENCES work_items(id)
);
```

**Indexes**:
```sql
CREATE INDEX idx_agent_events_timestamp
ON agent_events(timestamp DESC);

CREATE INDEX idx_agent_events_type
ON agent_events(event_type, timestamp DESC);

CREATE INDEX idx_agent_events_agent
ON agent_events(agent_id, timestamp DESC);
```

---

## Indexes Summary

**High-Impact Indexes** (query optimization):
1. `idx_memories_namespace_importance` - Primary query pattern
2. `idx_memory_links_source` - Graph traversal forward
3. `idx_memory_links_target` - Graph traversal reverse
4. `idx_memories_created_at` - Recent memories
5. `idx_memories_type` - Filter by type

**Specialized Indexes**:
- Evolution tracking: `idx_evolution_log_*`
- Orchestration: `idx_work_items_*`, `idx_agent_events_*`
- Audit: `idx_audit_log_*`

**Index Strategy**:
- Compound indexes for common filter combinations
- Descending order for timestamp columns (latest first)
- Covering indexes avoided (balance query speed vs. write cost)

---

## Migration History

| # | Migration | Purpose |
|---|-----------|---------|
| 001 | initial_schema.sql | Core tables, FTS5, indexes |
| 002 | add_indexes.sql | Performance indexes |
| 003 | audit_trail.sql | Audit logging |
| 003 | fix_fts_triggers.sql | Conditional FTS updates (SQLite 3.43.2+) |
| 006 | vector_search.sql | Vector embeddings (native F32_BLOB) |
| 007 | evolution.sql | Memory consolidation tables |
| 008 | agent_features.sql | Orchestration tables |
| 009 | evaluation_system.sql | Privacy-preserving evaluation |
| 010 | update_vector_dimensions.sql | Adjust vector size |
| 011 | work_items.sql | Work queue for agents |
| 012 | requirement_tracking.sql | Reviewer agent requirements |
| 013 | add_task_and_agent_event_types.sql | New memory/event types |

**Applied via**: `libsql_migration` crate at startup

---

## Query Patterns

### Hybrid Search

Combines FTS5 keyword search, vector similarity, and graph connectivity.

```sql
-- 1. Keyword search (FTS5)
SELECT m.*, rank as fts_score
FROM memories_fts
JOIN memories m ON m.rowid = memories_fts.rowid
WHERE memories_fts MATCH ?
  AND json_extract(m.namespace, '$.type') = 'project'
ORDER BY rank
LIMIT 10;

-- 2. Vector similarity (LibSQL native)
SELECT m.*, vector_distance_cos(m.embedding, ?) as similarity
FROM memories m
WHERE json_extract(m.namespace, '$.type') = 'project'
  AND m.embedding IS NOT NULL
ORDER BY similarity DESC
LIMIT 10;

-- 3. Graph traversal (recursive CTE)
WITH RECURSIVE graph_walk(id, depth) AS (
  SELECT id, 0 FROM memories WHERE id = ?
  UNION ALL
  SELECT ml.target_id, gw.depth + 1
  FROM memory_links ml
  JOIN graph_walk gw ON ml.source_id = gw.id
  WHERE gw.depth < 2
)
SELECT DISTINCT m.*
FROM graph_walk gw
JOIN memories m ON m.id = gw.id;

-- 4. Combined scoring
SELECT
  m.*,
  (0.4 * fts_score + 0.4 * vector_score + 0.2 * graph_score) as relevance
FROM ...
ORDER BY relevance DESC;
```

### Namespace Queries

**JSON Extraction** (since namespaces are JSON-serialized):

```sql
-- Global namespace
WHERE json_extract(namespace, '$.type') = 'global'

-- Project namespace
WHERE json_extract(namespace, '$.type') = 'project'
  AND json_extract(namespace, '$.name') = 'mnemosyne'

-- Session namespace
WHERE json_extract(namespace, '$.type') = 'session'
  AND json_extract(namespace, '$.project') = 'mnemosyne'
```

**Helper Function** (in Rust):
```rust
pub fn namespace_where_clause(ns: &Namespace) -> String {
    match ns {
        Namespace::Global =>
            "json_extract(namespace, '$.type') = 'global'".to_string(),
        Namespace::Project { name } =>
            format!("json_extract(namespace, '$.type') = 'project' \
                     AND json_extract(namespace, '$.name') = '{}'", name),
        Namespace::Session { project, session_id } =>
            format!("json_extract(namespace, '$.type') = 'session' \
                     AND json_extract(namespace, '$.project') = '{}' \
                     AND json_extract(namespace, '$.session_id') = '{}'",
                    project, session_id),
    }
}
```

---

## Performance Considerations

### Query Optimization

**Fast Queries** (<5ms):
- Namespace + importance filter (uses index)
- Memory type filter (uses index)
- Recent memories (uses created_at index)
- Graph traversal (limited depth)

**Slower Queries** (50-200ms):
- Full vector similarity scan (all embeddings)
- Deep graph traversal (depth > 3)
- Complex FTS5 queries with many terms

**Optimization Strategies**:
1. **Limit result sets**: Always use `LIMIT` clause
2. **Index hints**: Use `INDEXED BY` if needed
3. **Namespace filtering**: Apply early in query
4. **Vector search**: Pre-filter by namespace/type before vector comparison
5. **Graph depth**: Limit to 2-3 hops

### Write Performance

**Fast Writes** (<1ms):
- Insert memory (without embedding)
- Create link
- Update access count

**Slower Writes** (10-50ms):
- Insert with embedding generation (LLM + embedding service)
- FTS5 trigger updates on large content
- Consolidation operations (multiple reads/writes)

**Batch Operations**:
- Use transactions for multiple inserts
- Batch link creation
- Defer FTS updates for bulk operations

---

## Backup & Migration

### Backup

```bash
# Online backup (LibSQL)
libsql dump mnemosyne.db > backup.sql

# File copy (when not in use)
cp mnemosyne.db mnemosyne.db.backup
```

### Export/Import

```rust
// Export memories to JSON
mnemosyne export --namespace "project:myapp" --output memories.json

// Import memories from JSON
mnemosyne import --input memories.json --namespace "project:myapp"
```

### Schema Migration

**Adding a Column**:
```sql
-- migrations/libsql/014_add_column.sql
ALTER TABLE memories ADD COLUMN new_field TEXT DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_memories_new_field ON memories(new_field);
```

**Migration applied automatically** on startup via `libsql_migration` crate.

---

## Troubleshooting

### FTS Trigger Errors

**Issue**: "unsafe use of virtual table 'memories_fts'"

**Cause**: SQLite 3.43.2+ enforces stricter FTS5 virtual table rules.

**Solution**: Apply migration `003_fix_fts_triggers.sql` which makes UPDATE trigger conditional:
```sql
CREATE TRIGGER memories_au AFTER UPDATE ON memories
WHEN OLD.content != NEW.content
  OR OLD.summary != NEW.summary
  -- ... other indexed fields
BEGIN
  -- Update FTS only when indexed fields change
END;
```

### Namespace Query Failures

**Issue**: Namespace queries return wrong results or fail.

**Cause**: Namespace column contains JSON, not plain text.

**Solution**: Use `json_extract()` for querying:
```sql
-- Wrong
WHERE namespace = 'project:mnemosyne'

-- Correct
WHERE json_extract(namespace, '$.type') = 'project'
  AND json_extract(namespace, '$.name') = 'mnemosyne'
```

### Vector Search Performance

**Issue**: Vector similarity queries are slow (>500ms).

**Cause**: Scanning all embeddings without pre-filtering.

**Solution**: Apply namespace/type filters before vector comparison:
```sql
-- Slow: scans all embeddings
SELECT *, vector_distance_cos(embedding, ?) as sim
FROM memories
ORDER BY sim LIMIT 10;

-- Fast: pre-filter by namespace
SELECT *, vector_distance_cos(embedding, ?) as sim
FROM memories
WHERE json_extract(namespace, '$.type') = 'project'
  AND json_extract(namespace, '$.name') = 'myapp'
ORDER BY sim LIMIT 10;
```

---

## See Also

- [AGENT_GUIDE.md](../AGENT_GUIDE.md) - Development guide
- [TYPES_REFERENCE.md](TYPES_REFERENCE.md) - Type system reference
- [ARCHITECTURE.md](../ARCHITECTURE.md) - System architecture
- `migrations/libsql/` - Migration source files
- `src/storage/libsql.rs` - Storage implementation
