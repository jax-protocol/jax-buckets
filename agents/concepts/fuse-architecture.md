# FUSE Architecture

This document describes how the FUSE filesystem integration works in jax-bucket.

## Overview

FUSE (Filesystem in Userspace) allows mounting buckets as local directories. Users can read and write files using standard filesystem tools (ls, cat, cp, etc.) without knowing about the underlying bucket/sync infrastructure.

## Components

```
┌─────────────────────────────────────────────────────────────────┐
│                         Kernel (FUSE)                           │
│                                                                 │
│  ls /mnt/bucket    cat /mnt/bucket/file.txt    echo > file.txt │
└─────────────────────────────────┬───────────────────────────────┘
                                  │ FUSE protocol
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                       JaxFs (fuser crate)                       │
│                                                                 │
│  Implements fuser::Filesystem trait                             │
│  Translates FUSE ops to Mount operations                        │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ InodeTable  │  │  FileCache  │  │  WriteBuffers           │  │
│  │ path ↔ ino  │  │  LRU + TTL  │  │  fh → pending data      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────┬───────────────────────────────┘
                                  │ Direct Rust calls
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Mount                                  │
│                                                                 │
│  In-memory representation of bucket state                       │
│  - entry: Node (directory tree)                                 │
│  - ops_log: PathOpLog (CRDT operations)                         │
│  - manifest: Manifest metadata                                  │
│                                                                 │
│  Methods: ls(), cat(), add(), rm(), mkdir(), mv()               │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                        BlobsStore                               │
│                                                                 │
│  Content-addressed storage (iroh-blobs)                         │
│  All data encrypted with ChaCha20-Poly1305                      │
└─────────────────────────────────────────────────────────────────┘
```

## Key Files

| File | Purpose |
|------|---------|
| `crates/daemon/src/fuse/mod.rs` | Module exports |
| `crates/daemon/src/fuse/jax_fs.rs` | FUSE filesystem implementation |
| `crates/daemon/src/fuse/mount_manager.rs` | Mount lifecycle management |
| `crates/daemon/src/fuse/inode_table.rs` | Path ↔ inode mapping |
| `crates/daemon/src/fuse/cache.rs` | LRU content/attr cache |
| `crates/daemon/src/fuse/sync_events.rs` | Sync event types |

## Data Flow

### Read Path

```
User: cat /mnt/bucket/docs/readme.md
                │
                ▼
┌─────────────────────────────────┐
│  FUSE read(ino, offset, size)   │
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  InodeTable: ino → "/docs/readme.md"
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  FileCache.get_content(path)    │
│                                 │
│  HIT? → Return cached data      │
│  MISS? → Continue...            │
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  Mount.cat(path)                │
│                                 │
│  1. Traverse entry tree         │
│  2. Find NodeLink for path      │
│  3. Fetch blob from BlobsStore  │
│  4. Decrypt with node's Secret  │
│  5. Return plaintext bytes      │
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  FileCache.put_content(path)    │
│  Return data to FUSE            │
└─────────────────────────────────┘
```

### Write Path

```
User: echo "hello" > /mnt/bucket/new.txt
                │
                ▼
┌─────────────────────────────────┐
│  FUSE create(parent, name)      │
│  → Returns file handle (fh)     │
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  FUSE write(ino, fh, data)      │
│  → Buffers in WriteBuffers[fh]  │
│  → Marks dirty = true           │
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  FUSE flush(ino, fh)            │
│                                 │
│  If dirty:                      │
│    Mount.add(path, buffer_data) │
│    - Encrypts data              │
│    - Stores blob                │
│    - Updates entry tree         │
│    - Records in ops_log         │
│    Mark clean                   │
│    Invalidate cache             │
└─────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│  FUSE release(ino, fh)          │
│  → Removes WriteBuffer[fh]      │
└─────────────────────────────────┘
```

### What `Mount.add()` Does

When FUSE calls `mount.add(path, data)`:

1. **Encrypt**: Generate new `Secret`, encrypt data with ChaCha20-Poly1305
2. **Store blob**: `blobs.put_stream(encrypted)` → returns content hash
3. **Create NodeLink**: Links hash + secret for decryption
4. **Update entry tree**: Traverse to parent, insert new NodeLink
5. **Store updated nodes**: Parent nodes stored to blobs
6. **Track pins**: Add all new hashes to `pins` set
7. **Record operation**: `ops_log.record(Add, path, link)`

**Important**: `add()` does NOT call `save()`. The manifest is not updated.

## MountManager

The `MountManager` handles mount lifecycle:

```rust
pub struct MountManager {
    /// Live mounts: mount_id → LiveMount
    mounts: RwLock<HashMap<Uuid, LiveMount>>,
    /// Database for config persistence
    db: Database,
    /// Peer for loading mounts
    peer: Peer<Database>,
    /// Event channel for sync notifications
    sync_tx: broadcast::Sender<SyncEvent>,
}

pub struct LiveMount {
    /// The bucket mount (kept alive)
    pub mount: Arc<RwLock<Mount>>,
    /// FUSE session handle
    pub session: Option<BackgroundSession>,
    /// File cache
    pub cache: FileCache,
    /// Config from database
    pub config: FuseMount,
}
```

### Lifecycle

```
mount start
    │
    ├── Load mount from peer: peer.mount(bucket_id)
    ├── Create FileCache
    ├── Create JaxFs with mount reference
    ├── Spawn fuser::BackgroundSession
    ├── Store in mounts HashMap
    └── Update DB status → Running

mount stop
    │
    ├── Take session (drops it, unmounts)
    ├── Platform unmount (umount/fusermount)
    └── Update DB status → Stopped

on_bucket_synced (called when sync completes)
    │
    ├── Find affected mounts
    ├── [CURRENT] Replace mount with fresh peer.mount()
    ├── Invalidate cache
    └── Emit SyncEvent::MountInvalidated
```

## Cache Architecture

```rust
pub struct FileCache {
    /// Content cache: path → file bytes
    content: Cache<String, CachedContent>,
    /// Attribute cache: path → size, is_dir, mtime
    attrs: Cache<String, CachedAttr>,
    /// Directory listing cache: path → entries
    dirs: Cache<String, Vec<CachedDirEntry>>,
}
```

- **Backend**: `moka` (concurrent LRU cache)
- **TTL**: Configurable per-mount (default 60s)
- **Size limit**: Configurable per-mount (default 100MB)
- **Invalidation**: On local writes, on sync events

## Sync Integration

When a remote peer syncs new changes:

```
Remote peer announces new manifest
        │
        ▼
Sync engine downloads and applies manifest
        │
        ▼
MountManager.on_bucket_synced(bucket_id)
        │
        ▼
For each affected mount:
  - Invalidate cache
  - Emit SyncEvent::MountInvalidated
        │
        ▼
JaxFs sync listener receives event
  - Calls cache.invalidate_all()
  - Next FUSE read sees updated content
```

## Current Gaps

### Gap 1: No Persistence of Writes

FUSE writes are not persisted to a new manifest:

```
User writes file → mount.add() → blob stored, entry updated
                                         ↓
                              But mount.save() never called!
                                         ↓
                              Manifest still points to old state
                                         ↓
                              Daemon restart → writes "lost"
                              (blobs exist but are orphaned)
```

**Fix needed**: Call `save()` at appropriate times (flush, periodic, unmount).

### Gap 2: Sync Replaces Mount

When sync happens, local unsaved changes are lost:

```
User writes file → mount has changes in ops_log
        │
        ▼
Sync arrives → on_bucket_synced()
        │
        ▼
*live_mount.mount = peer.mount(bucket_id)  // REPLACE!
        │
        ▼
Old mount dropped → ops_log and entry changes LOST
```

**Fix needed**: Merge instead of replace when ops_log is non-empty.

### Gap 3: No Conflict Resolution

Even if we fix Gap 2, there's no conflict resolution wired in:

- `mount.merge_from()` exists but is never called
- `ConflictFile` resolver exists but is never used
- User could lose data during concurrent edits

**Fix needed**: Wire conflict resolution into the merge path.

## Configuration

Mount configuration is stored in SQLite (`fuse_mounts` table):

| Column | Type | Description |
|--------|------|-------------|
| mount_id | UUID | Primary key |
| bucket_id | UUID | Which bucket to mount |
| mount_point | TEXT | Local filesystem path |
| enabled | BOOL | Is this mount enabled? |
| auto_mount | BOOL | Start on daemon startup? |
| read_only | BOOL | Prevent writes? |
| cache_size_mb | INT | Cache size limit |
| cache_ttl_secs | INT | Cache entry TTL |
| status | TEXT | stopped/starting/running/error |

## Platform Support

| Platform | FUSE Implementation | Unmount Command |
|----------|--------------------|-----------------|
| macOS | macFUSE | `umount` / `diskutil unmount force` |
| Linux | FUSE kernel module | `fusermount -u` / `fusermount -uz` |

Mount options vary by platform (see `mount_manager.rs:305-325`).
