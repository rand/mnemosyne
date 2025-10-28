-- Migration 006: Vector search with sqlite-vec
-- This migration creates the vec0 virtual table for vector similarity search
-- Note: This migration is executed via rusqlite with sqlite-vec extension loaded

-- Create vec0 virtual table for embeddings
-- This table stores memory ID -> embedding mappings
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536]
);

-- Note: Virtual tables do not support traditional indexes
-- The vec0 module handles indexing internally for similarity search

-- Trigger to cleanup orphaned vectors when memories are deleted
-- This ensures referential integrity between memories and memory_vectors tables
CREATE TRIGGER IF NOT EXISTS cleanup_orphaned_vectors
AFTER DELETE ON memories
BEGIN
    DELETE FROM memory_vectors WHERE memory_id = OLD.id;
END;
