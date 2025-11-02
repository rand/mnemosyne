-- Fix FTS triggers to only fire when indexed columns change
-- This prevents "unsafe use of virtual table" errors during direct SQL updates

-- Drop old UPDATE trigger
DROP TRIGGER IF EXISTS memories_au;

-- Recreate with conditional logic - only update FTS when indexed columns change
CREATE TRIGGER memories_au AFTER UPDATE ON memories
WHEN OLD.content != NEW.content
  OR OLD.summary != NEW.summary
  OR OLD.keywords != NEW.keywords
  OR OLD.tags != NEW.tags
  OR OLD.context != NEW.context
BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, summary, keywords, tags, context)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.summary, OLD.keywords, OLD.tags, OLD.context);
    INSERT INTO memories_fts(rowid, content, summary, keywords, tags, context)
    VALUES (NEW.rowid, NEW.content, NEW.summary, NEW.keywords, NEW.tags, NEW.context);
END;
