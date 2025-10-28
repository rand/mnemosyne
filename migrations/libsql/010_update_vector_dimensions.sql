-- Migration 010: Update vector dimensions for local embeddings
-- This migration updates the memory_vectors table to use 768 dimensions
-- for local embedding models (nomic-embed-text-v1.5) instead of 1536 (Voyage AI)

-- Drop the existing trigger first
DROP TRIGGER IF EXISTS cleanup_orphaned_vectors;

-- Drop the existing virtual table
-- Note: Any existing embeddings will be lost and need to be regenerated
DROP TABLE IF EXISTS memory_vectors;

-- Create new vec0 virtual table with 768 dimensions
-- This matches the output of nomic-embed-text-v1.5 model
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[768]
);

-- Recreate the trigger to cleanup orphaned vectors
CREATE TRIGGER IF NOT EXISTS cleanup_orphaned_vectors
AFTER DELETE ON memories
BEGIN
    DELETE FROM memory_vectors WHERE memory_id = OLD.id;
END;

-- Note: Existing memories will need to have their embeddings regenerated
-- using the CLI command: mnemosyne embed --all
