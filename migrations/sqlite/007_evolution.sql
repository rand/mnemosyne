-- Migration 007: Evolution System Support
--
-- This migration adds support for background evolution jobs including:
-- - Access tracking for importance recalibration
-- - Archival support for unused memories
-- - Link traversal tracking for decay
-- - Job execution history for monitoring

-- ==============================================================================
-- ACCESS TRACKING (for importance recalibration)
-- ==============================================================================

-- Track how many times each memory has been accessed
ALTER TABLE memories ADD COLUMN access_count INTEGER DEFAULT 0 NOT NULL;

-- Track when memory was last accessed
ALTER TABLE memories ADD COLUMN last_accessed_at INTEGER;

-- Create index for efficient querying by access patterns
CREATE INDEX IF NOT EXISTS idx_memories_access ON memories(access_count, last_accessed_at);

-- ==============================================================================
-- ARCHIVAL SUPPORT
-- ==============================================================================

-- Track when memory was archived (NULL = active, timestamp = archived)
ALTER TABLE memories ADD COLUMN archived_at INTEGER;

-- Index for efficiently finding archived/active memories
CREATE INDEX IF NOT EXISTS idx_memories_archived ON memories(archived_at);

-- Index for finding archival candidates (low importance + old)
CREATE INDEX IF NOT EXISTS idx_memories_archival_candidates
ON memories(importance, last_accessed_at)
WHERE archived_at IS NULL;

-- ==============================================================================
-- LINK TRAVERSAL TRACKING (for decay)
-- ==============================================================================

-- Track when link was last traversed (followed during recall/search)
ALTER TABLE memory_links ADD COLUMN last_traversed_at INTEGER;

-- Track whether link was manually created by user (don't decay these)
ALTER TABLE memory_links ADD COLUMN user_created BOOLEAN DEFAULT 0 NOT NULL;

-- Index for finding links that need decay
CREATE INDEX IF NOT EXISTS idx_links_traversal
ON memory_links(last_traversed_at, strength);

-- ==============================================================================
-- JOB EXECUTION HISTORY
-- ==============================================================================

-- Track all evolution job executions for monitoring and debugging
CREATE TABLE IF NOT EXISTS evolution_job_runs (
    id TEXT PRIMARY KEY,
    job_name TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'failed', 'timeout')),
    memories_processed INTEGER DEFAULT 0,
    changes_made INTEGER DEFAULT 0,
    error_message TEXT
);

-- Index for querying job history by name and time
CREATE INDEX IF NOT EXISTS idx_job_runs_name
ON evolution_job_runs(job_name, completed_at DESC);

-- Index for finding running jobs
CREATE INDEX IF NOT EXISTS idx_job_runs_status
ON evolution_job_runs(status, started_at);

-- ==============================================================================
-- IMPORTANCE HISTORY (optional, for analysis)
-- ==============================================================================

-- Track importance changes over time for analysis
CREATE TABLE IF NOT EXISTS importance_history (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    old_importance REAL NOT NULL,
    new_importance REAL NOT NULL,
    reason TEXT,
    FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
);

-- Index for querying importance history by memory
CREATE INDEX IF NOT EXISTS idx_importance_history_memory
ON importance_history(memory_id, timestamp DESC);

-- ==============================================================================
-- HELPER VIEWS
-- ==============================================================================

-- View for finding archival candidates
CREATE VIEW IF NOT EXISTS v_archival_candidates AS
SELECT
    id,
    content,
    importance,
    access_count,
    last_accessed_at,
    created_at,
    (julianday('now') - julianday(created_at, 'unixepoch')) as days_old,
    COALESCE(
        (julianday('now') - julianday(last_accessed_at, 'unixepoch')),
        (julianday('now') - julianday(created_at, 'unixepoch'))
    ) as days_since_access
FROM memories
WHERE archived_at IS NULL
  AND (
    (access_count = 0 AND days_since_access > 180) OR
    (importance < 3.0 AND days_since_access > 90) OR
    (importance < 2.0 AND days_since_access > 30)
  );

-- View for finding links that need decay
CREATE VIEW IF NOT EXISTS v_link_decay_candidates AS
SELECT
    id,
    source_id,
    target_id,
    strength,
    last_traversed_at,
    created_at,
    COALESCE(
        (julianday('now') - julianday(last_traversed_at, 'unixepoch')),
        (julianday('now') - julianday(created_at, 'unixepoch'))
    ) as days_since_traversal
FROM memory_links
WHERE user_created = 0
  AND strength > 0.1
  AND days_since_traversal > 30;

-- View for job execution summary
CREATE VIEW IF NOT EXISTS v_job_execution_summary AS
SELECT
    job_name,
    COUNT(*) as total_runs,
    SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END) as successful_runs,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed_runs,
    SUM(CASE WHEN status = 'timeout' THEN 1 ELSE 0 END) as timeout_runs,
    AVG(memories_processed) as avg_memories_processed,
    AVG(changes_made) as avg_changes_made,
    MAX(completed_at) as last_run_at
FROM evolution_job_runs
WHERE status != 'running'
GROUP BY job_name;

-- ==============================================================================
-- VALIDATION
-- ==============================================================================

-- Ensure access_count is non-negative
CREATE TRIGGER IF NOT EXISTS validate_access_count
BEFORE UPDATE ON memories
FOR EACH ROW
WHEN NEW.access_count < 0
BEGIN
    SELECT RAISE(ABORT, 'access_count cannot be negative');
END;

-- Ensure archived_at is in the past
CREATE TRIGGER IF NOT EXISTS validate_archived_at
BEFORE UPDATE ON memories
FOR EACH ROW
WHEN NEW.archived_at IS NOT NULL AND NEW.archived_at > unixepoch('now')
BEGIN
    SELECT RAISE(ABORT, 'archived_at cannot be in the future');
END;

-- Ensure last_accessed_at doesn't go backwards
CREATE TRIGGER IF NOT EXISTS validate_last_accessed_at
BEFORE UPDATE ON memories
FOR EACH ROW
WHEN NEW.last_accessed_at IS NOT NULL
    AND OLD.last_accessed_at IS NOT NULL
    AND NEW.last_accessed_at < OLD.last_accessed_at
BEGIN
    SELECT RAISE(ABORT, 'last_accessed_at cannot move backwards in time');
END;
