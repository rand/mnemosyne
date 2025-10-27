-- Add sqlite-vec support for vector similarity search
-- Version: 0.1.0
-- Created: 2025-10-27

-- Note: This migration requires the sqlite-vec extension to be loaded
-- The extension is loaded automatically by the Rust code using load_extension()

-- Drop the old BLOB-based embeddings table
DROP TABLE IF EXISTS memory_embeddings;

-- Create new virtual table using vec0 for efficient vector search
-- Using 384 dimensions to match EMBEDDING_DIM in embedding service
CREATE VIRTUAL TABLE IF NOT EXISTS vec_memories USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding float[384]
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_vec_memories_id ON vec_memories(memory_id);

-- Update metadata
INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', '3');
INSERT OR REPLACE INTO metadata (key, value) VALUES ('vector_search_enabled', 'true');
INSERT OR REPLACE INTO metadata (key, value) VALUES ('embedding_dimension', '384');
