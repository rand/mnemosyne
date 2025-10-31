-- Requirement Tracking for Work Items
-- Enables traceability between requirements and implementation

-- Add requirements column
ALTER TABLE work_items ADD COLUMN requirements TEXT;

-- Add requirement_status column
ALTER TABLE work_items ADD COLUMN requirement_status TEXT;

-- Add implementation_evidence column
ALTER TABLE work_items ADD COLUMN implementation_evidence TEXT;

-- Index for querying work items with unsatisfied requirements
CREATE INDEX IF NOT EXISTS idx_work_items_requirements ON work_items(requirements);
