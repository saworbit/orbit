-- Add artifacts table for GhostFS file hierarchy
-- Maps job outputs to a virtual filesystem tree

CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY,           -- UUID artifact ID (e.g., "artifact_a1b2c3d4")
    job_id INTEGER NOT NULL,       -- FK to jobs table
    parent_id TEXT,                -- NULL for root, otherwise FK to artifacts.id
    name TEXT NOT NULL,            -- Filename (e.g., "video.mp4" or "logs/")
    size INTEGER NOT NULL DEFAULT 0, -- File size in bytes (0 for directories)
    is_dir BOOLEAN NOT NULL DEFAULT 0, -- 1 for directory, 0 for file
    mtime INTEGER NOT NULL DEFAULT (strftime('%s', 'now')), -- Unix timestamp
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

-- Index for parent_id lookups (used by readdir)
CREATE INDEX IF NOT EXISTS idx_artifacts_parent ON artifacts(parent_id);

-- Index for job_id filtering (critical for multi-job isolation)
CREATE INDEX IF NOT EXISTS idx_artifacts_job ON artifacts(job_id);

-- Composite index for lookup operations (parent + name)
CREATE INDEX IF NOT EXISTS idx_artifacts_lookup ON artifacts(parent_id, name);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_artifacts_timestamp
AFTER UPDATE ON artifacts
FOR EACH ROW
BEGIN
    UPDATE artifacts SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;
