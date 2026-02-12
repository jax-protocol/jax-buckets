# Daemon Local Cache for FUSE Performance

**Status:** Planned
**Related:** FUSE Integration epic

## Objective

Add a local caching layer in the daemon to speed up FUSE filesystem operations by reducing repeated decryption and blob fetches.

## Background

FUSE operations like `readdir`, `getattr`, and `read` currently hit the full path through Mount → PathOpLog → blob store on every call. File managers like Finder make many repeated calls for the same paths (icon previews, metadata, spotlight indexing). Without caching, each call:

1. Decrypts the manifest chain
2. Resolves the path through PathOpLog
3. Fetches and decrypts blob content

This causes noticeable latency, especially for directories with many files.

## Proposed Cache Layers

### 1. Metadata Cache (directory listings + attributes)

**Scope:** Per-mount, in-memory
**TTL:** 30-60 seconds (configurable)
**Invalidation:** On local write, on sync event from peer

Cache `ls()` results and file attributes:
- `cache_key: (bucket_id, path)` → `Vec<FileEntry>` or `FileAttr`
- Use `moka` (already a dependency for FUSE) for TTL + LRU eviction

### 2. Content Cache (file data)

**Scope:** Per-mount, in-memory with size limit
**TTL:** 5 minutes (configurable)
**Max size:** 100MB default (configurable via mount settings)

Cache decrypted file content for recently accessed files:
- `cache_key: (bucket_id, path, link_hash)` → `Vec<u8>`
- Include `link_hash` in key so stale content is never served
- LRU eviction when size limit reached

### 3. Negative Cache (non-existent paths)

**Scope:** Per-mount, in-memory
**TTL:** 10 seconds

Cache "file not found" results to avoid repeated lookups for paths that don't exist (common with macOS `._*` resource fork queries).

## Cache Invalidation

### Local writes
- `write` / `create` / `mkdir` / `unlink` / `rename` → invalidate affected paths + parent directory

### Remote sync
- Subscribe to `SyncEvent::BucketUpdated` from MountManager
- On sync event → invalidate entire metadata cache for that bucket
- Content cache can remain valid if keyed by `link_hash`

## Implementation Steps

### 1. Add cache to JaxFs

**File:** `crates/daemon/src/fuse/jax_fs.rs`

```rust
pub struct JaxFs {
    // existing fields...
    metadata_cache: moka::sync::Cache<(Uuid, String), CachedMetadata>,
    content_cache: moka::sync::Cache<(Uuid, String, String), Vec<u8>>,
    negative_cache: moka::sync::Cache<(Uuid, String), ()>,
}
```

### 2. Add cache configuration to mount settings

**File:** `crates/daemon/src/fuse/mount_manager.rs`

Extend `FuseMountConfig`:
```rust
pub cache_size_mb: u32,      // already exists
pub cache_ttl_secs: u32,     // already exists
pub metadata_ttl_secs: u32,  // new
```

### 3. Wire invalidation to sync events

**File:** `crates/daemon/src/fuse/jax_fs.rs`

In the sync event listener:
```rust
if let SyncEvent::MountInvalidated { mount_id } = event {
    metadata_cache.invalidate_all();
    // content_cache stays valid (keyed by link_hash)
}
```

### 4. Add cache stats endpoint (optional)

**File:** `crates/daemon/src/http_server/api/v0/mounts/`

Add `GET /api/v0/mounts/:id/cache-stats` for debugging:
- Hit/miss ratio
- Current size
- Entry count

## Acceptance Criteria

- [ ] Repeated `ls` on same directory uses cache (sub-millisecond)
- [ ] Repeated `cat` on same file uses cache
- [ ] Local writes invalidate affected cache entries
- [ ] Remote sync invalidates metadata cache
- [ ] Cache respects configured size limits
- [ ] Cache respects configured TTL
- [ ] No stale data served after writes or sync

## Performance Target

- `readdir` on cached directory: < 1ms
- `getattr` on cached file: < 0.5ms
- `read` on cached file: < 1ms + memcpy time
