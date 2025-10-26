-- Additional indexes for performance optimization
-- Version: 0.1.0
-- Created: 2025-10-26

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
-- Note: JSON extraction in SQLite is less efficient, but we can help with partial matching
CREATE INDEX IF NOT EXISTS idx_memories_tags ON memories(tags);

-- ============================================================================
-- Link Graph Optimization
-- ============================================================================

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
