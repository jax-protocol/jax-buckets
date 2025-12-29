-- Allow multiple entries at the same height (forks)
-- SQLite doesn't support ALTER TABLE DROP CONSTRAINT, so we recreate the table

-- Create new table without the UNIQUE(bucket_id, height) constraint
CREATE TABLE bucket_log_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- The UUID of the bucket this log entry belongs to
    bucket_id TEXT NOT NULL,
    -- The friendly name of the bucket at this point in time
    name TEXT NOT NULL,
    -- The current link at this log entry (stored as base32 CID string)
    current_link VARCHAR(255) NOT NULL,
    -- The previous link (null for genesis)
    previous_link VARCHAR(255),
    -- The height of this entry in the log chain
    height INTEGER NOT NULL,
    -- When this log entry was created
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Ensure one entry per link per bucket (this is still correct - same link can't appear twice)
    UNIQUE(bucket_id, current_link)
);

-- Copy data from old table
INSERT INTO bucket_log_new (id, bucket_id, name, current_link, previous_link, height, created_at)
SELECT id, bucket_id, name, current_link, previous_link, height, created_at FROM bucket_log;

-- Drop old table
DROP TABLE bucket_log;

-- Rename new table
ALTER TABLE bucket_log_new RENAME TO bucket_log;

-- Recreate indexes
CREATE INDEX idx_bucket_log_bucket_height ON bucket_log(bucket_id, height DESC);
CREATE INDEX idx_bucket_log_bucket_link ON bucket_log(bucket_id, current_link);
