-- Additional indexes for performance optimization (LibSQL)
-- Version: 0.1.0
-- Created: 2025-10-27

-- ============================================================================
-- Basic Indexes for Common Queries
-- ============================================================================

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
-- Covering Indexes for Common Query Patterns
-- ============================================================================

-- Memory retrieval by namespace and importance (common filter)
CREATE INDEX IF NOT EXISTS idx_memories_namespace_importance_active
    ON memories(namespace, importance DESC, is_archived)
    WHERE is_archived = 0;

-- Recent memories by namespace
CREATE INDEX IF NOT EXISTS idx_memories_namespace_updated
    ON memories(namespace, updated_at DESC)
    WHERE is_archived = 0;

-- Tag search optimization
CREATE INDEX IF NOT EXISTS idx_memories_tags ON memories(tags);

-- ============================================================================
-- Link Graph Optimization
-- ============================================================================

-- Indexes for graph traversal
CREATE INDEX IF NOT EXISTS idx_links_source ON memory_links(source_id);
CREATE INDEX IF NOT EXISTS idx_links_target ON memory_links(target_id);
CREATE INDEX IF NOT EXISTS idx_links_type ON memory_links(link_type);
CREATE INDEX IF NOT EXISTS idx_links_strength ON memory_links(strength DESC);

-- Bidirectional lookup
CREATE INDEX IF NOT EXISTS idx_links_source_target ON memory_links(source_id, target_id);
CREATE INDEX IF NOT EXISTS idx_links_target_source ON memory_links(target_id, source_id);

-- Find all memories linked FROM a source (outbound)
CREATE INDEX IF NOT EXISTS idx_links_outbound
    ON memory_links(source_id, link_type, strength DESC);

-- Find all memories linking TO a target (inbound)
CREATE INDEX IF NOT EXISTS idx_links_inbound
    ON memory_links(target_id, link_type, strength DESC);

-- ============================================================================
-- Temporal Queries
-- ============================================================================

-- Memories due for archival (old and low importance)
CREATE INDEX IF NOT EXISTS idx_memories_archival_candidates
    ON memories(updated_at, importance)
    WHERE is_archived = 0;

-- Access pattern tracking
CREATE INDEX IF NOT EXISTS idx_memories_access_pattern
    ON memories(last_accessed_at DESC, access_count DESC)
    WHERE is_archived = 0;

-- ============================================================================
-- Audit Log Optimization
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_memory_id ON audit_log(memory_id);
CREATE INDEX IF NOT EXISTS idx_audit_operation ON audit_log(operation);

-- ============================================================================
-- Vector Search Index (LibSQL Native)
-- ============================================================================
-- Create native vector index for efficient similarity search using LibSQL's
-- built-in vector capabilities. This uses the F32_BLOB embedding column.

CREATE INDEX IF NOT EXISTS idx_memories_vector ON memories (
    libsql_vector_idx(embedding, 'metric=cosine')
);

-- Update metadata to reflect vector index
INSERT OR REPLACE INTO metadata (key, value) VALUES ('vector_index_created', datetime('now'));
