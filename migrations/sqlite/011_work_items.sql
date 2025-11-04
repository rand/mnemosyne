-- Add work_items and memory_modification_log tables for orchestration
-- Migration: 011
-- Date: 2025-10-30 (recovered from production schema 2025-11-04)
--
-- This migration was originally applied to production databases but never
-- committed to git (ghost migration). Recovered from production schema.
--
-- Tables:
-- - work_items: Track agent work assignments, dependencies, and execution state
-- - memory_modification_log: Audit trail of agent memory modifications

-- ============================================================================
-- Work Items Table
-- ============================================================================
-- Tracks tasks assigned to agents with dependencies and review system

CREATE TABLE IF NOT EXISTS work_items (
    -- Identity
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    original_intent TEXT NOT NULL,

    -- Assignment
    agent_role TEXT NOT NULL,
    state TEXT NOT NULL,
    phase TEXT NOT NULL,
    priority INTEGER NOT NULL,

    -- Dependencies
    dependencies TEXT, -- JSON array of WorkItemIds

    -- Timestamps
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,

    -- Error handling
    error TEXT,
    timeout_secs INTEGER,

    -- Review system
    review_feedback TEXT, -- JSON array of strings
    suggested_tests TEXT, -- JSON array of strings
    review_attempt INTEGER DEFAULT 0,

    -- Context tracking
    execution_memory_ids TEXT, -- JSON array of MemoryIds
    consolidated_context_id TEXT, -- MemoryId
    estimated_context_tokens INTEGER DEFAULT 0,

    -- Git integration
    assigned_branch TEXT,
    file_scope TEXT -- JSON array of file paths
);

-- Indexes for work item queries
CREATE INDEX IF NOT EXISTS idx_work_items_state ON work_items(state);
CREATE INDEX IF NOT EXISTS idx_work_items_phase ON work_items(phase);
CREATE INDEX IF NOT EXISTS idx_work_items_agent ON work_items(agent_role);
CREATE INDEX IF NOT EXISTS idx_work_items_review_attempt ON work_items(review_attempt);

-- ============================================================================
-- Memory Modification Log Table
-- ============================================================================
-- Audit trail of agent memory modifications for tracking memory evolution

CREATE TABLE IF NOT EXISTS memory_modification_log (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    agent_role TEXT NOT NULL,
    modification_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    changes TEXT,
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

-- Indexes for modification log queries
CREATE INDEX IF NOT EXISTS idx_modification_memory ON memory_modification_log(memory_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_modification_agent ON memory_modification_log(agent_role, modification_type);
CREATE INDEX IF NOT EXISTS idx_modification_timestamp ON memory_modification_log(timestamp DESC);
