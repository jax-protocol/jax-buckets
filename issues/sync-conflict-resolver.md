# FUSE Write Persistence and Sync Issues

**Status:** Implemented
**Related:** `agents/concepts/fuse-architecture.md`

## Summary

Three related issues with FUSE write handling:

1. **Writes are never persisted** - `mount.save()` is never called
2. **Sync replaces mount** - loses in-memory changes
3. **No conflict resolution** - well-tested API exists but isn't wired in

## Issue 1: FUSE Writes Not Persisted

### Current Behavior

```
User writes via FUSE
    ↓
flush() calls mount.add(path, data)
    ↓
mount.add():
  - Encrypts and stores blob ✓
  - Updates entry tree ✓
  - Records in ops_log ✓
  - Does NOT call save() ✗
    ↓
Manifest still points to old entry tree
    ↓
Daemon restart → file "disappears"
(blob exists but is orphaned)
```

### Evidence

```bash
grep -r "\.save\(" crates/daemon/src/fuse/
# No results
```

The FUSE code never calls `mount.save()` or `peer.save_mount()`.

### Fix

Call `save()` at appropriate times:

```rust
// Option A: Save on flush (consistent but expensive)
fn flush(&mut self, ...) {
    if dirty {
        mount.add(path, data).await?;
        peer.save_mount(&mount, false).await?;  // ADD THIS
    }
}

// Option B: Save periodically (batches writes)
// Spawn background task that saves every N seconds if dirty

// Option C: Save on release/close
fn release(&mut self, ...) {
    if any_dirty_handles_for_this_inode {
        peer.save_mount(&mount, false).await?;
    }
}
```

**Recommendation**: Start with Option A (save on flush) for correctness, optimize later.

---

## Issue 2: Sync Replaces Mount

### Current Behavior

```rust
// mount_manager.rs:on_bucket_synced()
let new_mount = self.peer.mount(bucket_id).await?;
*live_mount.mount.write().await = new_mount;  // REPLACE
```

If the live mount has unsaved changes (ops_log non-empty), they are lost.

### Fix

Check before replacing:

```rust
pub async fn on_bucket_synced(&self, bucket_id: Uuid) -> Result<(), MountError> {
    for (mount_id, live_mount) in mounts.iter() {
        if *live_mount.config.bucket_id == bucket_id {
            let mut mount = live_mount.mount.write().await;

            // Check if mount has unsaved changes
            let has_local_changes = !mount.inner().await.ops_log().is_empty();

            if has_local_changes {
                // Save local changes first, then reload
                self.peer.save_mount(&mount, false).await?;
            }

            // Now safe to reload
            *mount = self.peer.mount(bucket_id).await?;
            live_mount.cache.invalidate_all();
        }
    }
}
```

This saves local changes before reloading, preventing data loss.

---

## Issue 3: No Conflict Resolution

### What Exists

Well-tested conflict resolution in `crates/common/src/mount/`:

- `ConflictResolver` trait with 4 strategies
- `PathOpLog.merge_with_resolver()`
- `Mount.merge_from()` for 3-way merge
- Comprehensive tests in `tests/conflict_resolution.rs`

### What's Missing

None of this is called from real code paths. The sync engine just appends manifests:

```rust
// sync_bucket.rs:apply_manifest_chain()
peer.logs().append(bucket_id, name, link, ...).await?;
// No merge, no conflict resolution
```

### When Conflicts Actually Happen

```
Peer A: Creates /config.json (saves, syncs)
Peer B: Creates /config.json (saves, syncs)
    ↓
Both peers receive each other's manifest
    ↓
Last one to sync "wins" - other version is lost from current state
(Both are in ops_log history, but no conflict file created)
```

### Fix

When we fix Issue 2 by saving before reload, we can use merge instead:

```rust
if has_local_changes {
    let incoming = self.peer.mount(bucket_id).await?;
    let resolver = ConflictFile::new();
    let (result, _) = mount.merge_from(&incoming, &resolver, self.peer.blobs()).await?;

    // Log conflicts
    for conflict in &result.conflicts_resolved {
        tracing::info!("Resolved conflict: {:?}", conflict);
    }
} else {
    *mount = self.peer.mount(bucket_id).await?;
}
```

---

## Implementation Order

1. **Fix Issue 1 first** - Without persistence, nothing else matters
2. **Fix Issue 2** - Save before reload prevents data loss
3. **Fix Issue 3** - Use merge_from() with ConflictFile resolver

## Files to Modify

| File | Change |
|------|--------|
| `crates/daemon/src/fuse/jax_fs.rs` | Add save() call in flush() |
| `crates/daemon/src/fuse/mount_manager.rs` | Save before reload, use merge_from() |

## Acceptance Criteria

- [x] FUSE writes persist across daemon restart (save() called via channel after flush)
- [x] Sync doesn't lose local unsaved changes (save before reload in on_bucket_synced)
- [x] Concurrent edits create conflict files (`file@hash`) (ConflictFile resolver wired in)
- [x] All existing tests pass
- [x] `cargo clippy` clean

## Test Scenario

```
1. Mount bucket via FUSE
2. Create /test.txt with "local content"
3. On another peer, create /test.txt with "remote content"
4. Sync the remote peer's changes
5. Verify:
   - /test.txt exists (one version)
   - /test.txt@<hash> exists (conflict copy)
   - Both contents preserved
6. Restart daemon
7. Verify files still exist (persistence works)
```
