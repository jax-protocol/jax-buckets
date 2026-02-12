-- Drop fuse_mounts table and indexes
DROP INDEX IF EXISTS idx_fuse_mounts_auto_mount;
DROP INDEX IF EXISTS idx_fuse_mounts_bucket_id;
DROP TABLE IF EXISTS fuse_mounts;
