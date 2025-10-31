-- Requirement Tracking for Work Items
-- Enables traceability between requirements and implementation

-- Add requirement tracking columns to work_items table
ALTER TABLE work_items ADD COLUMN requirements TEXT; -- JSON array of requirement strings
ALTER TABLE work_items ADD COLUMN requirement_status TEXT; -- JSON object mapping requirement to RequirementStatus
ALTER TABLE work_items ADD COLUMN implementation_evidence TEXT; -- JSON object mapping requirement to array of MemoryIds

-- Index for querying work items with unsatisfied requirements
-- This helps identify stuck work items that need attention
CREATE INDEX IF NOT EXISTS idx_work_items_requirements ON work_items(requirements);
