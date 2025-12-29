-- Create merge_log table for tracking reconciliation events
-- This allows us to know which orphaned branches have been merged
-- and prevents showing "orphaned branches detected" warnings for already-merged branches

CREATE TABLE merge_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- The bucket this merge belongs to
    bucket_id TEXT NOT NULL,

    -- Source: the orphaned branch being merged FROM
    link_from VARCHAR(255) NOT NULL,       -- The orphaned branch link
    height_from INTEGER NOT NULL,          -- Height of the orphaned branch

    -- Target: the canonical head we merged ONTO
    link_onto VARCHAR(255) NOT NULL,       -- The canonical head before merge
    height_onto INTEGER NOT NULL,          -- Height of canonical head before merge

    -- Result: the new head after merge
    result_link VARCHAR(255) NOT NULL,     -- New canonical head after merge
    result_height INTEGER NOT NULL,        -- Should be height_onto + 1

    -- Merge metadata
    ops_merged INTEGER NOT NULL DEFAULT 0, -- Number of operations merged from this branch
    merged_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent duplicate merges of same branch
    UNIQUE(bucket_id, link_from)
);

-- Index for efficient bucket queries
CREATE INDEX idx_merge_log_bucket ON merge_log(bucket_id);

-- Index for filtering orphaned branches (check if link_from already merged)
CREATE INDEX idx_merge_log_link_from ON merge_log(link_from);

-- Index for finding merges that created a specific result
CREATE INDEX idx_merge_log_result ON merge_log(result_link);
