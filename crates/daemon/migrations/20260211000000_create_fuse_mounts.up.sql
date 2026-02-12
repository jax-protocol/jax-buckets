-- Create fuse_mounts table for FUSE mount configurations
CREATE TABLE fuse_mounts (
    -- Primary key: UUID for the mount
    mount_id TEXT PRIMARY KEY,
    -- The bucket this mount is associated with
    bucket_id TEXT NOT NULL,
    -- Local filesystem path where bucket is mounted
    mount_point TEXT NOT NULL UNIQUE,
    -- Whether this mount is enabled
    enabled INTEGER NOT NULL DEFAULT 1,
    -- Whether to auto-mount on daemon startup
    auto_mount INTEGER NOT NULL DEFAULT 0,
    -- Whether the mount is read-only
    read_only INTEGER NOT NULL DEFAULT 0,
    -- Cache size in megabytes
    cache_size_mb INTEGER NOT NULL DEFAULT 100,
    -- Cache TTL in seconds
    cache_ttl_secs INTEGER NOT NULL DEFAULT 60,
    -- Current status: stopped, starting, running, stopping, error
    status TEXT NOT NULL DEFAULT 'stopped',
    -- Error message if status is 'error'
    error_message TEXT,
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient queries by bucket
CREATE INDEX idx_fuse_mounts_bucket_id ON fuse_mounts(bucket_id);

-- Index for auto-mount queries on startup
CREATE INDEX idx_fuse_mounts_auto_mount ON fuse_mounts(auto_mount) WHERE auto_mount = 1 AND enabled = 1;
