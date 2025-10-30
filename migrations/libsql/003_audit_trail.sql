-- Add audit trail table for tracking memory modifications
CREATE TABLE IF NOT EXISTS memory_modification_log (
    id TEXT PRIMARY KEY,
    memory_id TEXT NOT NULL,
    agent_role TEXT NOT NULL,
    modification_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    changes TEXT,
    FOREIGN KEY (memory_id) REFERENCES memories(id)
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_modification_memory ON memory_modification_log(memory_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_modification_agent ON memory_modification_log(agent_role, modification_type);
CREATE INDEX IF NOT EXISTS idx_modification_timestamp ON memory_modification_log(timestamp DESC);
