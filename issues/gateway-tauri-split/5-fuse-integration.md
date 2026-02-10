# FUSE Mount Integration

**Status:** Planned
**Track:** Local
**Depends on:** Ticket 3 (daemon simplification)
**Reference:** `amiller68/fs-over-blobstore-v1` branch, `issues/fuse-mount-system.md`

## Objective

Add FUSE mount support so users can mount buckets as local filesystems. Implemented as a standalone `crates/fuse` crate, integrated into the daemon as a feature flag.

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Local FS    │────▶│  JaxFs       │────▶│  Daemon      │
│  Operations  │     │  (FUSE)      │     │  REST API    │
└──────────────┘     └──────────────┘     └──────────────┘
                                                │
                                                ▼
                                          ┌──────────────┐
                                          │  Mount       │
                                          │  + BlobStore │
                                          │  + PathOpLog │
                                          └──────────────┘
```

The FUSE process communicates with the daemon via HTTP. This keeps the daemon as the single coordination point for sync and CRDT operations.

## Implementation Steps

### 1. Create `crates/fuse/` crate

**Create:** `crates/fuse/Cargo.toml`
- Depends on `fuser` crate
- Depends on `jax-common` for Mount types
- Feature-flagged in workspace

**Create:** `crates/fuse/src/lib.rs`
- Public exports

### 2. Implement JaxFs FUSE filesystem

**Create:** `crates/fuse/src/jax_fs.rs`
- Implements `fuser::Filesystem` trait
- Inode ↔ path bidirectional mapping
- Operations: lookup, getattr, readdir, read, write, create, mkdir, unlink, rmdir, rename
- macOS resource fork filtering (`._*` files)
- Write buffering with flush on release
- Synchronous flush for first write to pending files (bug fix from reference)

### 3. Implement MountManager

**Create:** `crates/fuse/src/mount_manager.rs`
- Spawn/stop FUSE processes
- Track running mounts (mount_id → child process)
- Auto-mount on startup
- Graceful unmount on shutdown
- Platform-specific unmount (`umount` on macOS, `fusermount -u` on Linux)

### 4. Add SQLite mount persistence

**Create:** migration for `fuse_mounts` table

```sql
CREATE TABLE fuse_mounts (
    mount_id TEXT PRIMARY KEY,
    bucket_id TEXT NOT NULL,
    mount_point TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL DEFAULT 1,
    auto_mount INTEGER NOT NULL DEFAULT 0,
    read_only INTEGER NOT NULL DEFAULT 0,
    cache_size_mb INTEGER NOT NULL DEFAULT 100,
    cache_ttl_secs INTEGER NOT NULL DEFAULT 60,
    pid INTEGER,
    status TEXT NOT NULL DEFAULT 'stopped',
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**Modify:** `crates/daemon/src/daemon/database/` - Add mount queries

### 5. Add REST API endpoints

**Create:** `crates/daemon/src/daemon/http_server/api/v0/mounts/`

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v0/mounts/` | Create mount config |
| GET | `/api/v0/mounts/` | List all mounts |
| GET | `/api/v0/mounts/:id` | Get mount |
| PATCH | `/api/v0/mounts/:id` | Update mount config |
| DELETE | `/api/v0/mounts/:id` | Delete mount config |
| POST | `/api/v0/mounts/:id/start` | Start mount |
| POST | `/api/v0/mounts/:id/stop` | Stop mount |

### 6. Add CLI commands

**Create:** `crates/daemon/src/ops/mount.rs`

| Command | Description |
|---------|-------------|
| `jax mount list [--json]` | List mounts |
| `jax mount add <bucket> <path> [--auto-mount] [--read-only]` | Add mount |
| `jax mount remove <id> [-f]` | Remove mount |
| `jax mount start <id>` | Start mount |
| `jax mount stop <id>` | Stop mount |
| `jax mount set <id> [options]` | Update settings |

### 7. Wire into daemon

**Modify:** `crates/daemon/src/daemon/state.rs` - Add MountManager
**Modify:** `crates/daemon/src/daemon/process/mod.rs` - Auto-mount on startup, unmount on shutdown
**Modify:** `crates/daemon/Cargo.toml` - Add `jax-fuse` dependency behind feature flag

## Files Summary

| Action | Path | Description |
|--------|------|-------------|
| Create | `crates/fuse/Cargo.toml` | New crate |
| Create | `crates/fuse/src/lib.rs` | Public exports |
| Create | `crates/fuse/src/jax_fs.rs` | FUSE filesystem |
| Create | `crates/fuse/src/mount_manager.rs` | Process lifecycle |
| Create | `crates/daemon/src/daemon/database/mount_queries.rs` | SQLite queries |
| Create | `crates/daemon/src/daemon/http_server/api/v0/mounts/` | REST endpoints |
| Create | `crates/daemon/src/ops/mount.rs` | CLI commands |
| Create | Migration SQL | fuse_mounts table |
| Modify | `Cargo.toml` | Add workspace member |
| Modify | `crates/daemon/Cargo.toml` | Add jax-fuse dep (feature-gated) |
| Modify | `crates/daemon/src/daemon/state.rs` | Add MountManager |
| Modify | `crates/daemon/src/daemon/process/mod.rs` | Auto-mount, shutdown |
| Modify | `crates/daemon/src/main.rs` | Add mount subcommand |

## Acceptance Criteria

- [ ] `jax mount add my-bucket ~/mounts/my-bucket` creates mount config
- [ ] `jax mount start <id>` spawns FUSE process
- [ ] FUSE operations work: read, write, mkdir, rm, mv, rename
- [ ] Mounts persist across daemon restarts
- [ ] Auto-mount on startup works
- [ ] Graceful unmount on shutdown
- [ ] macOS and Linux supported
- [ ] Feature-gated: compiles without fuse deps when feature disabled
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings

## Verification

```bash
# Build with fuse feature
cargo build --features fuse

# Start daemon
jax daemon

# Add and start mount
jax mount add my-bucket ~/mounts/my-bucket --auto-mount
jax mount start <mount-id>

# Verify filesystem
ls ~/mounts/my-bucket
echo "test" > ~/mounts/my-bucket/test.txt
cat ~/mounts/my-bucket/test.txt
mkdir ~/mounts/my-bucket/subdir

# Stop and clean up
jax mount stop <mount-id>
jax mount remove <mount-id>
```
