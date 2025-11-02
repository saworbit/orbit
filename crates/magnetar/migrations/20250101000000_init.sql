-- Magnetar Initial Schema
-- Creates tables for job state management with DAG dependency support

-- Main chunks table
CREATE TABLE IF NOT EXISTS chunks (
    job_id INTEGER NOT NULL,
    chunk INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'processing', 'done', 'failed')),
    error TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (job_id, chunk)
);

-- Index for fast status queries (most common operation)
CREATE INDEX IF NOT EXISTS idx_chunks_status ON chunks(job_id, status);

-- Index for ordered scanning
CREATE INDEX IF NOT EXISTS idx_chunks_job_chunk ON chunks(job_id, chunk);

-- Dependencies table for DAG support
CREATE TABLE IF NOT EXISTS dependencies (
    job_id INTEGER NOT NULL,
    chunk INTEGER NOT NULL,
    depends_on INTEGER NOT NULL,
    PRIMARY KEY (job_id, chunk, depends_on),
    FOREIGN KEY (job_id, chunk) REFERENCES chunks(job_id, chunk) ON DELETE CASCADE
);

-- Index for dependency lookups
CREATE INDEX IF NOT EXISTS idx_deps_lookup ON dependencies(job_id, chunk);
CREATE INDEX IF NOT EXISTS idx_deps_reverse ON dependencies(job_id, depends_on);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_chunks_timestamp
AFTER UPDATE ON chunks
FOR EACH ROW
BEGIN
    UPDATE chunks SET updated_at = CURRENT_TIMESTAMP
    WHERE job_id = NEW.job_id AND chunk = NEW.chunk;
END;
