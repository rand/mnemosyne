-- Work Items Persistence
-- Enables work items to persist across sessions with full context

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

-- Index for querying by state (critical for work queue recovery)
CREATE INDEX IF NOT EXISTS idx_work_items_state ON work_items(state);

-- Index for querying by phase
CREATE INDEX IF NOT EXISTS idx_work_items_phase ON work_items(phase);

-- Index for querying by agent role
CREATE INDEX IF NOT EXISTS idx_work_items_agent ON work_items(agent_role);

-- Index for review attempts (to find struggling work items)
CREATE INDEX IF NOT EXISTS idx_work_items_review_attempt ON work_items(review_attempt);
