-- Add requirement tracking columns to work_items
-- Migration: 012
-- Date: 2025-11-01 (recovered from production schema 2025-11-04)
--
-- This migration was originally applied to production databases but never
-- committed to git (ghost migration). Recovered from production schema.
--
-- Adds columns for tracking requirements in work items:
-- - requirements: JSON array of extracted requirements
-- - requirement_status: JSON tracking satisfaction state per requirement
-- - implementation_evidence: JSON evidence that requirements were met

-- Add requirement tracking columns to work_items
ALTER TABLE work_items ADD COLUMN requirements TEXT;
ALTER TABLE work_items ADD COLUMN requirement_status TEXT;
ALTER TABLE work_items ADD COLUMN implementation_evidence TEXT;

-- Add index for requirement queries
CREATE INDEX IF NOT EXISTS idx_work_items_requirements ON work_items(requirements);
