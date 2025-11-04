-- Fix audit_log table schema: Remove details column, ensure metadata is NOT NULL
-- Date: 2025-11-04
--
-- Background: The audit_log table had schema drift where some databases had
-- both 'details' and 'metadata' columns. This migration ensures all databases
-- have only 'metadata TEXT NOT NULL' with proper CHECK constraint.
--
-- This fixes "table audit_log has no column named metadata" errors that blocked
-- memory storage operations.

-- Create correct audit_log table
CREATE TABLE audit_log_new (
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
    metadata TEXT NOT NULL,  -- JSON object with operation-specific data
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

-- Migrate existing data
-- Use metadata if present, fall back to details (for databases with old schema)
INSERT INTO audit_log_new (id, timestamp, operation, memory_id, metadata)
SELECT
    id,
    timestamp,
    operation,
    memory_id,
    COALESCE(
        NULLIF(metadata, ''),
        NULLIF(details, ''),
        '{}'
    )
FROM audit_log;

-- Replace old table
DROP TABLE audit_log;
ALTER TABLE audit_log_new RENAME TO audit_log;

-- Recreate indexes
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_memory_id ON audit_log(memory_id);
CREATE INDEX idx_audit_operation ON audit_log(operation);
