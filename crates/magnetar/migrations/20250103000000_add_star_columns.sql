-- Phase 3: Add Star ID columns to jobs table
-- This enables the Nucleus to orchestrate jobs across remote Stars

-- Add source and destination Star IDs
-- NULL means local execution (on the Nucleus itself)
-- Non-NULL means remote execution (on the specified Star)

ALTER TABLE jobs ADD COLUMN source_star_id TEXT;
ALTER TABLE jobs ADD COLUMN dest_star_id TEXT;

-- Index for Star-based queries (e.g., "show all jobs for Star X")
CREATE INDEX IF NOT EXISTS idx_jobs_source_star ON jobs(source_star_id);
CREATE INDEX IF NOT EXISTS idx_jobs_dest_star ON jobs(dest_star_id);

-- Composite index for cross-Star transfers
CREATE INDEX IF NOT EXISTS idx_jobs_stars ON jobs(source_star_id, dest_star_id);
