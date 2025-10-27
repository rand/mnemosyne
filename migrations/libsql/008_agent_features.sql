-- Mnemosyne v2.0: Agent Features Migration
-- Version: 2.0.0
-- Created: 2025-10-27
--
-- This migration adds agent-specific features for the multi-agent architecture:
-- - Agent ownership and visibility tracking
-- - Role-based access control
-- - Audit trail for memory modifications
--
-- Agent roles: orchestrator, optimizer, reviewer, executor

-- ============================================================================
-- Agent Ownership and Visibility
-- ============================================================================

-- Add agent ownership columns to memories table
ALTER TABLE memories ADD COLUMN created_by TEXT;  -- Agent role that created this memory
ALTER TABLE memories ADD COLUMN modified_by TEXT; -- Agent role that last modified this memory
ALTER TABLE memories ADD COLUMN visible_to TEXT NOT NULL DEFAULT '[]';  -- JSON array of agent roles

-- Create indexes for efficient agent-specific queries
CREATE INDEX IF NOT EXISTS idx_memories_created_by ON memories(created_by);
CREATE INDEX IF NOT EXISTS idx_memories_modified_by ON memories(modified_by);

-- Validate agent roles (check constraint would be ideal but SQLite has limited support)
-- Valid roles: 'orchestrator', 'optimizer', 'reviewer', 'executor'

-- ============================================================================
-- Memory Modifications Audit Trail
-- ============================================================================

-- Audit trail for all memory operations by agents
-- Tracks who modified what, when, and how
CREATE TABLE IF NOT EXISTS memory_modifications (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID
    memory_id TEXT NOT NULL,
    agent_role TEXT NOT NULL CHECK(agent_role IN (
        'orchestrator',
        'optimizer',
        'reviewer',
        'executor',
        'human'  -- Allow human users
    )),
    modification_type TEXT NOT NULL CHECK(modification_type IN (
        'create',
        'update',
        'delete',
        'archive',
        'unarchive',
        'supersede'
    )),
    timestamp INTEGER NOT NULL,  -- Unix timestamp
    changes TEXT,  -- JSON object describing what changed (optional)

    FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
);

-- Indexes for efficient audit queries
CREATE INDEX IF NOT EXISTS idx_modifications_memory ON memory_modifications(memory_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_modifications_agent ON memory_modifications(agent_role, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_modifications_type ON memory_modifications(modification_type, timestamp DESC);

-- ============================================================================
-- Link Traversal Tracking for Decay
-- ============================================================================

-- Add traversal tracking to memory links for link strength decay
-- (Used by evolution system in migration 007, but added here for agent features)
ALTER TABLE memory_links ADD COLUMN last_traversed_at INTEGER;  -- Unix timestamp
ALTER TABLE memory_links ADD COLUMN traversal_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memory_links ADD COLUMN user_created INTEGER NOT NULL DEFAULT 0 CHECK(user_created IN (0, 1));

-- Index for efficient decay queries
CREATE INDEX IF NOT EXISTS idx_links_traversal ON memory_links(last_traversed_at);

-- ============================================================================
-- Agent Session Tracking
-- ============================================================================

-- Track agent sessions for prefetching and analytics
CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY NOT NULL,  -- Session UUID
    agent_role TEXT NOT NULL CHECK(agent_role IN (
        'orchestrator',
        'optimizer',
        'reviewer',
        'executor'
    )),
    started_at INTEGER NOT NULL,   -- Unix timestamp
    ended_at INTEGER,                -- Unix timestamp (null if active)
    work_phase TEXT,                 -- 'planning', 'implementation', 'review', etc.
    memories_accessed INTEGER NOT NULL DEFAULT 0,
    cache_hits INTEGER NOT NULL DEFAULT 0,
    cache_misses INTEGER NOT NULL DEFAULT 0
);

-- Indexes for session queries
CREATE INDEX IF NOT EXISTS idx_sessions_agent ON agent_sessions(agent_role, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_phase ON agent_sessions(work_phase, started_at DESC);

-- ============================================================================
-- Memory Co-Access Patterns
-- ============================================================================

-- Track which memories are accessed together for prefetching
-- Used to implement co-access pattern detection
CREATE TABLE IF NOT EXISTS memory_coaccesses (
    memory_id_1 TEXT NOT NULL,
    memory_id_2 TEXT NOT NULL,
    coaccess_count INTEGER NOT NULL DEFAULT 1,
    last_coaccessed_at INTEGER NOT NULL,  -- Unix timestamp
    agent_role TEXT,  -- Optional: which agent sees this pattern

    PRIMARY KEY (memory_id_1, memory_id_2),
    FOREIGN KEY (memory_id_1) REFERENCES memories(id) ON DELETE CASCADE,
    FOREIGN KEY (memory_id_2) REFERENCES memories(id) ON DELETE CASCADE,

    -- Ensure consistent ordering (memory_id_1 < memory_id_2)
    CHECK (memory_id_1 < memory_id_2)
);

-- Index for efficient co-access queries
CREATE INDEX IF NOT EXISTS idx_coaccesses_memory1 ON memory_coaccesses(memory_id_1, coaccess_count DESC);
CREATE INDEX IF NOT EXISTS idx_coaccesses_memory2 ON memory_coaccesses(memory_id_2, coaccess_count DESC);

-- ============================================================================
-- Agent Memory Preferences
-- ============================================================================

-- Store agent-specific preferences and scoring weights
CREATE TABLE IF NOT EXISTS agent_preferences (
    agent_role TEXT PRIMARY KEY NOT NULL CHECK(agent_role IN (
        'orchestrator',
        'optimizer',
        'reviewer',
        'executor'
    )),
    importance_weights TEXT NOT NULL DEFAULT '{"base":0.3,"access":0.3,"recency":0.3,"relevance":0.1}',  -- JSON
    prefetch_enabled INTEGER NOT NULL DEFAULT 1 CHECK(prefetch_enabled IN (0, 1)),
    prefetch_size INTEGER NOT NULL DEFAULT 1000,  -- Max memories in cache
    cache_ttl_seconds INTEGER NOT NULL DEFAULT 3600,  -- 1 hour default
    custom_filters TEXT DEFAULT '{}',  -- JSON for additional filters
    updated_at INTEGER NOT NULL
);

-- Insert default preferences for each agent role
INSERT OR IGNORE INTO agent_preferences (agent_role, importance_weights, updated_at) VALUES
    ('orchestrator', '{"base":0.3,"access":0.2,"recency":0.4,"relevance":0.1}', unixepoch()),
    ('optimizer', '{"base":0.4,"access":0.3,"recency":0.1,"relevance":0.2}', unixepoch()),
    ('reviewer', '{"base":0.5,"access":0.1,"recency":0.2,"relevance":0.2}', unixepoch()),
    ('executor', '{"base":0.2,"access":0.4,"recency":0.3,"relevance":0.1}', unixepoch());

-- ============================================================================
-- Update Metadata
-- ============================================================================

-- Update schema version
UPDATE metadata SET value = '8' WHERE key = 'schema_version';
INSERT OR IGNORE INTO metadata (key, value) VALUES ('agent_features_enabled', 'true');
INSERT OR IGNORE INTO metadata (key, value) VALUES ('agent_features_version', '2.0.0');

-- ============================================================================
-- Views for Agent Queries
-- ============================================================================

-- Agent-specific memory views (filtered by created_by)
CREATE VIEW IF NOT EXISTS orchestrator_memories AS
SELECT * FROM memories WHERE created_by = 'orchestrator' AND is_archived = 0;

CREATE VIEW IF NOT EXISTS optimizer_memories AS
SELECT * FROM memories WHERE created_by = 'optimizer' AND is_archived = 0;

CREATE VIEW IF NOT EXISTS reviewer_memories AS
SELECT * FROM memories WHERE created_by = 'reviewer' AND is_archived = 0;

CREATE VIEW IF NOT EXISTS executor_memories AS
SELECT * FROM memories WHERE created_by = 'executor' AND is_archived = 0;

-- Memory modification summary by agent
CREATE VIEW IF NOT EXISTS agent_modification_stats AS
SELECT
    agent_role,
    modification_type,
    COUNT(*) as count,
    MAX(timestamp) as last_modification
FROM memory_modifications
GROUP BY agent_role, modification_type
ORDER BY agent_role, modification_type;

-- Cache performance by agent
CREATE VIEW IF NOT EXISTS agent_cache_stats AS
SELECT
    agent_role,
    COUNT(*) as total_sessions,
    SUM(memories_accessed) as total_accessed,
    SUM(cache_hits) as total_hits,
    SUM(cache_misses) as total_misses,
    CAST(SUM(cache_hits) AS REAL) / NULLIF(SUM(cache_hits) + SUM(cache_misses), 0) as hit_rate,
    AVG(ended_at - started_at) as avg_session_duration_seconds
FROM agent_sessions
WHERE ended_at IS NOT NULL
GROUP BY agent_role;

-- Top co-accessed memory pairs
CREATE VIEW IF NOT EXISTS top_coaccesses AS
SELECT
    m1.summary as memory_1_summary,
    m2.summary as memory_2_summary,
    c.coaccess_count,
    c.agent_role,
    datetime(c.last_coaccessed_at, 'unixepoch') as last_coaccessed
FROM memory_coaccesses c
JOIN memories m1 ON c.memory_id_1 = m1.id
JOIN memories m2 ON c.memory_id_2 = m2.id
WHERE c.coaccess_count >= 3
ORDER BY c.coaccess_count DESC
LIMIT 100;
