# FUSE Mount Management System Implementation Guide

This document describes the FUSE filesystem implementation for jax-bucket, including the architecture, bug fixes, and a complete mount management system. This serves as a reference for reimplementing or extending this work.

**Reference Commit**: `e4b2bbf` on branch `amiller68/fs-over-blobstore-v1`

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [FUSE Filesystem Basics](#fuse-filesystem-basics)
3. [Current Implementation: HTTP-Based Approach](#current-implementation-http-based-approach)
4. [Bug Fixes Implemented](#bug-fixes-implemented)
5. [Mount Persistence System](#mount-persistence-system)
6. [REST API Endpoints](#rest-api-endpoints)
7. [CLI Commands](#cli-commands)
8. [Daemon Integration](#daemon-integration)
9. [Code Patterns](#code-patterns)
10. [Future Considerations](#future-considerations)

---

## Architecture Overview

The jax-bucket FUSE implementation allows users to mount a remote bucket as a local filesystem. The architecture follows this data flow:

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Local FS      │     │   FUSE Process   │     │    Daemon       │
│   Operations    │────▶│   (jax_fs.rs)    │────▶│   HTTP API      │
│   (read/write)  │     │                  │     │                 │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                         │
                                                         ▼
                                                 ┌─────────────────┐
                                                 │     Mount       │
                                                 │   (bucket.rs)   │
                                                 │                 │
                                                 └─────────────────┘
                                                         │
                                                         ▼
                                                 ┌─────────────────┐
                                                 │   Blob Store    │
                                                 │   + PathOpLog   │
                                                 │     (CRDT)      │
                                                 └─────────────────┘
```

### Key Components

1. **`JaxFs`** (`crates/app/src/fuse/jax_fs.rs`) - FUSE filesystem implementation using the `fuser` crate (v0.14)
2. **`Mount`** (`crates/common/src/mount/bucket.rs`) - Abstraction over blob store with file/directory operations
3. **`PathOpLog`** (`crates/common/src/mount/path_ops.rs`) - CRDT for conflict-free filesystem sync
4. **`MountManager`** (`crates/app/src/daemon/mount_manager.rs`) - Manages FUSE process lifecycle
5. **Daemon HTTP API** - Endpoints for FUSE to call when files are modified

---

## FUSE Filesystem Basics

FUSE (Filesystem in Userspace) allows implementing filesystems without kernel modifications. The `fuser` crate provides a Rust interface.

### Key FUSE Operations

| Operation | Purpose | Implementation Notes |
|-----------|---------|---------------------|
| `lookup` | Resolve path to inode | Maps paths to inodes via `path_to_inode` HashMap |
| `getattr` | Get file/directory attributes | Returns size, permissions, timestamps |
| `readdir` | List directory contents | Uses `Mount::ls()` |
| `read` | Read file contents | Uses `Mount::read()` with LRU cache |
| `write` | Write to file | Buffers writes, flushes on `flush`/`release` |
| `create` | Create new file | Adds to `pending_creates`, actual creation on first write |
| `mkdir` | Create directory | Uses `Mount::mkdir()` |
| `unlink` | Delete file | Uses `Mount::rm()` |
| `rmdir` | Delete directory | Uses `Mount::rm()` |
| `rename` | Move/rename file | Uses `Mount::mv()` |

### Inode Management

The implementation uses a bidirectional mapping:
- `path_to_inode: HashMap<PathBuf, u64>` - Path to inode lookup
- `inode_to_path: HashMap<u64, PathBuf>` - Inode to path lookup
- `next_inode: AtomicU64` - Counter for generating new inodes

Root directory is always inode 1 (FUSE_ROOT_ID).

---

## Current Implementation: HTTP-Based Approach

The FUSE process communicates with the daemon via HTTP rather than direct `Mount` access.

### Why HTTP?

1. **Daemon as coordination point**: The daemon manages sync, and all modifications should go through it
2. **CRDT integration**: The `PathOpLog` CRDT needs to be wired into the sync protocol, which the daemon handles
3. **Process isolation**: FUSE runs as a separate process, simplifying lifecycle management

### API Endpoints Used by FUSE

```rust
// In jax_fs.rs
async fn api_add(&self, path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or(Path::new(""));
    let filename = path.file_name().unwrap().to_string_lossy();

    let url = format!(
        "{}/api/v0/buckets/{}/fs?mount_path={}&filename={}",
        self.api_base, self.bucket_id,
        urlencoding::encode(&parent.to_string_lossy()),
        urlencoding::encode(&filename)
    );

    self.client.post(&url)
        .header("Content-Type", "application/octet-stream")
        .body(data.to_vec())
        .send().await?;
    Ok(())
}
```

**Critical**: The `mount_path` parameter must be the PARENT directory, not the full path. The server extracts the filename separately.

---

## Bug Fixes Implemented

### 1. First-Write Failure

**Problem**: Files created via FUSE failed on first write because the daemon didn't know about them yet.

**Root Cause**: `create()` only added the file to local tracking (`pending_creates`), but the daemon had no record until `flush()`.

**Solution**: Added synchronous flush on first write to pending files:

```rust
// In write() method
fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], ...) {
    let path = self.inode_to_path.read().unwrap().get(&ino).cloned();
    if let Some(ref p) = path {
        let is_pending = self.pending_creates.read().unwrap().contains(p);

        if is_pending && offset == 0 {
            // First write to a pending file - flush synchronously
            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Err(e) = rt.block_on(self.api_add(p, data)) {
                reply.error(libc::EIO);
                return;
            }
            self.pending_creates.write().unwrap().remove(p);
            reply.written(data.len() as u32);
            return;
        }

        // Normal buffered write path...
    }
}
```

### 2. Path Duplication Bug

**Problem**: Creating `/test.md` resulted in `/test.md/test.md` on the server.

**Root Cause**: FUSE sent full path as `mount_path` AND server extracted filename, causing duplication.

**Solution**: Send parent directory as `mount_path`:

```rust
// WRONG (caused duplication)
let url = format!("...?mount_path={}&filename={}",
    full_path,  // /test.md
    filename    // test.md
);  // Server created: /test.md/test.md

// CORRECT
let url = format!("...?mount_path={}&filename={}",
    parent_dir, // "" (empty for root)
    filename    // test.md
);  // Server created: /test.md
```

### 3. macOS Resource Fork Filtering

**Problem**: macOS Finder creates `._*` resource fork files that cluttered the filesystem.

**Solution**: Filter them in `lookup()` and `create()`:

```rust
fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    let name_str = name.to_string_lossy();

    // Filter macOS resource forks
    if name_str.starts_with("._") {
        reply.error(ENOENT);
        return;
    }
    // ...
}
```

---

## Mount Persistence System

### Database Schema

```sql
-- migrations/20260105000000_create_fuse_mounts.up.sql
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

### Database Query Layer

```rust
// crates/app/src/daemon/database/mount_queries.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountStatus {
    Stopped,
    Running,
    Error,
}

#[derive(Debug, Clone)]
pub struct MountInfo {
    pub mount_id: Uuid,
    pub bucket_id: Uuid,
    pub mount_point: PathBuf,
    pub enabled: bool,
    pub auto_mount: bool,
    pub read_only: bool,
    pub cache_size_mb: i64,
    pub cache_ttl_secs: i64,
    pub pid: Option<i64>,
    pub status: MountStatus,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Database {
    pub async fn create_mount(&self, bucket_id: Uuid, mount_point: &Path, ...) -> Result<MountInfo>;
    pub async fn get_mount(&self, mount_id: Uuid) -> Result<Option<MountInfo>>;
    pub async fn get_mount_by_path(&self, mount_point: &Path) -> Result<Option<MountInfo>>;
    pub async fn list_mounts(&self) -> Result<Vec<MountInfo>>;
    pub async fn list_auto_mounts(&self) -> Result<Vec<MountInfo>>;
    pub async fn update_mount_status(&self, mount_id: Uuid, status: MountStatus, pid: Option<i64>, error: Option<&str>) -> Result<()>;
    pub async fn update_mount(&self, mount_id: Uuid, ...) -> Result<MountInfo>;
    pub async fn delete_mount(&self, mount_id: Uuid) -> Result<()>;
}
```

### Mount Manager

The `MountManager` tracks active FUSE processes and handles lifecycle:

```rust
// crates/app/src/daemon/mount_manager.rs

pub struct ActiveMount {
    pub mount_id: Uuid,
    pub child: Child,
    pub mount_point: PathBuf,
}

pub struct MountManager {
    database: Database,
    active_mounts: RwLock<HashMap<Uuid, ActiveMount>>,
}

impl MountManager {
    /// Start a mount - spawns `jax bucket fuse` subprocess
    pub async fn start_mount(&self, mount_id: Uuid) -> Result<()> {
        let mount_info = self.database.get_mount(mount_id).await?
            .ok_or_else(|| anyhow!("Mount not found"))?;

        // Ensure mount point exists
        tokio::fs::create_dir_all(&mount_info.mount_point).await?;

        // Spawn FUSE process
        let child = Command::new(std::env::current_exe()?)
            .args([
                "bucket", "fuse",
                "--bucket-id", &mount_info.bucket_id.to_string(),
                "--mount-point", &mount_info.mount_point.to_string_lossy(),
                "--cache-size-mb", &mount_info.cache_size_mb.to_string(),
                "--cache-ttl-secs", &mount_info.cache_ttl_secs.to_string(),
            ])
            .spawn()?;

        let pid = child.id();
        self.database.update_mount_status(mount_id, MountStatus::Running, pid.map(|p| p as i64), None).await?;

        self.active_mounts.write().await.insert(mount_id, ActiveMount {
            mount_id,
            child,
            mount_point: mount_info.mount_point,
        });

        Ok(())
    }

    /// Stop a mount - unmount then kill process
    pub async fn stop_mount(&self, mount_id: Uuid) -> Result<()> {
        if let Some(mut active) = self.active_mounts.write().await.remove(&mount_id) {
            // Platform-specific unmount
            #[cfg(target_os = "macos")]
            let _ = Command::new("umount").arg(&active.mount_point).status();

            #[cfg(target_os = "linux")]
            let _ = Command::new("fusermount").args(["-u", &active.mount_point.to_string_lossy()]).status();

            // Kill process
            let _ = active.child.kill();
        }

        self.database.update_mount_status(mount_id, MountStatus::Stopped, None, None).await?;
        Ok(())
    }

    /// Start all mounts configured for auto-mount
    pub async fn start_auto_mounts(&self) {
        let auto_mounts = self.database.list_auto_mounts().await.unwrap_or_default();
        for mount in auto_mounts {
            if let Err(e) = self.start_mount(mount.mount_id).await {
                tracing::error!("Failed to auto-mount {}: {}", mount.mount_point.display(), e);
            }
        }
    }

    /// Stop all running mounts (for graceful shutdown)
    pub async fn stop_all_mounts(&self) {
        let mount_ids: Vec<_> = self.active_mounts.read().await.keys().copied().collect();
        for mount_id in mount_ids {
            if let Err(e) = self.stop_mount(mount_id).await {
                tracing::error!("Failed to stop mount {}: {}", mount_id, e);
            }
        }
    }
}
```

---

## REST API Endpoints

All endpoints are under `/api/v0/mounts/`:

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| POST | `/` | `create.rs` | Create mount configuration |
| GET | `/` | `list.rs` | List all mounts |
| GET | `/:id` | `get_mount.rs` | Get single mount |
| PATCH | `/:id` | `update.rs` | Update mount config |
| DELETE | `/:id` | `delete_mount.rs` | Delete mount config |
| POST | `/:id/start` | `start.rs` | Start mount (spawn FUSE) |
| POST | `/:id/stop` | `stop.rs` | Stop mount (kill FUSE) |

### Example: Start Mount Endpoint

```rust
// crates/app/src/daemon/http_server/api/v0/mounts/start.rs

pub async fn start_mount(
    State(state): State<Arc<crate::daemon::State>>,
    Path(mount_id): Path<Uuid>,
) -> Result<Json<StartResponse>, AppError> {
    state.mount_manager().start_mount(mount_id).await?;

    let mount = state.database().get_mount(mount_id).await?
        .ok_or_else(|| anyhow!("Mount not found"))?;

    Ok(Json(StartResponse {
        mount_id,
        status: mount.status.to_string(),
        pid: mount.pid,
    }))
}
```

### Router Setup

```rust
// crates/app/src/daemon/http_server/api/v0/mounts/mod.rs

pub fn router() -> Router<Arc<State>> {
    Router::new()
        .route("/", post(create::create_mount))
        .route("/", get(list::list_mounts))
        .route("/:id", get(get_mount::get_mount))
        .route("/:id", patch(update::update_mount))
        .route("/:id", delete(delete_mount::delete_mount))
        .route("/:id/start", post(start::start_mount))
        .route("/:id/stop", post(stop::stop_mount))
}
```

---

## CLI Commands

All commands are under `jax mount`:

| Command | Description |
|---------|-------------|
| `jax mount list [--json]` | List all configured mounts |
| `jax mount add <bucket> <path> [--auto-mount] [--read-only]` | Add mount configuration |
| `jax mount remove <id> [-f]` | Remove mount configuration |
| `jax mount start <id>` | Start a mount |
| `jax mount stop <id>` | Stop a mount |
| `jax mount set <id> [options]` | Update mount settings |

### Example: Add Command

```rust
// crates/app/src/ops/mount/add.rs

#[derive(Debug, Args)]
pub struct Add {
    /// Bucket name or ID to mount
    #[arg(value_name = "BUCKET")]
    bucket: String,

    /// Local path to mount the bucket at
    #[arg(value_name = "PATH")]
    mount_point: PathBuf,

    /// Automatically mount on daemon startup
    #[arg(long)]
    auto_mount: bool,

    /// Mount as read-only
    #[arg(long)]
    read_only: bool,

    /// Cache size in MB
    #[arg(long, default_value = "100")]
    cache_size_mb: i64,

    /// Cache TTL in seconds
    #[arg(long, default_value = "60")]
    cache_ttl_secs: i64,
}

impl Add {
    pub async fn run(self, client: &ApiClient) -> Result<AddOutput> {
        let response = client.post("/api/v0/mounts")
            .json(&CreateMountRequest {
                bucket: self.bucket,
                mount_point: self.mount_point,
                auto_mount: self.auto_mount,
                read_only: self.read_only,
                cache_size_mb: self.cache_size_mb,
                cache_ttl_secs: self.cache_ttl_secs,
            })
            .send().await?;

        let mount: MountResponse = response.json().await?;
        Ok(AddOutput { mount })
    }
}
```

### Command Enum Pattern

```rust
// crates/app/src/ops/mount/mod.rs

command_enum! {
    #[derive(Debug, Clone, Subcommand)]
    pub enum Command {
        /// List all configured FUSE mounts
        List(list::List),
        /// Add a new FUSE mount configuration
        Add(add::Add),
        /// Remove a FUSE mount configuration
        Remove(remove::Remove),
        /// Start a FUSE mount
        Start(start::Start),
        /// Stop a FUSE mount
        Stop(stop::Stop),
        /// Update FUSE mount settings
        Set(set::Set),
    }
}
```

---

## Daemon Integration

### State Structure

```rust
// crates/app/src/daemon/state.rs

pub struct State {
    database: Database,
    peer: Peer<Database>,
    mount_manager: Arc<MountManager>,
}

impl State {
    pub async fn new(database: Database, peer: Peer<Database>) -> Result<Self> {
        let mount_manager = Arc::new(MountManager::new(database.clone()));
        Ok(Self { database, peer, mount_manager })
    }

    pub fn mount_manager(&self) -> &Arc<MountManager> {
        &self.mount_manager
    }
}
```

### Auto-Mount on Startup

```rust
// crates/app/src/daemon/process/mod.rs

pub async fn run(config: &ProcessConfig) -> Result<()> {
    // ... initialization ...

    // Start auto-mount configured FUSE mounts (non-blocking)
    let mount_manager = state.mount_manager().clone();
    tokio::spawn(async move {
        tracing::info!("Starting auto-mount of configured FUSE mounts...");
        mount_manager.start_auto_mounts().await;
    });

    // ... start HTTP server ...

    // Wait for shutdown signal
    let _ = graceful_waiter.await;

    // Graceful shutdown: stop all running FUSE mounts
    tracing::info!("Stopping all FUSE mounts...");
    state.mount_manager().stop_all_mounts().await;

    Ok(())
}
```

---

## Code Patterns

### 1. SQLx Query Caching

After adding new queries, run:

```bash
DATABASE_URL="sqlite:///path/to/dev.db" cargo sqlx prepare --workspace
```

This generates `.sqlx/` query cache files needed for compile-time verification.

### 2. API Response Types

Use serde for JSON serialization:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct MountResponse {
    pub mount_id: Uuid,
    pub bucket_id: Uuid,
    pub mount_point: String,
    pub status: String,
    // ...
}
```

### 3. Error Handling

Use `anyhow` for internal errors, convert to `AppError` at API boundary:

```rust
pub async fn handler(...) -> Result<Json<Response>, AppError> {
    let result = do_something().await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(result))
}
```

### 4. Platform-Specific Code

Use conditional compilation for platform differences:

```rust
#[cfg(target_os = "macos")]
fn unmount(path: &Path) -> io::Result<()> {
    Command::new("umount").arg(path).status()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn unmount(path: &Path) -> io::Result<()> {
    Command::new("fusermount").args(["-u", &path.to_string_lossy()]).status()?;
    Ok(())
}
```

---

## Future Considerations

### 1. Direct Mount Access vs HTTP

The current HTTP-based approach adds latency. Once `PathOpLog` CRDT is fully integrated into the sync protocol, consider:

- Passing `Mount` directly to `JaxFs` for read operations
- Only using HTTP for write operations that need CRDT coordination
- Or implementing a local socket for faster IPC

### 2. Health Monitoring

The `MountManager` could implement periodic health checks:

```rust
pub async fn health_check(&self) {
    for (mount_id, active) in self.active_mounts.read().await.iter() {
        // Check if process is still running
        // Restart if crashed and auto_mount is enabled
    }
}
```

### 3. Write Buffering Improvements

Current implementation buffers writes per-file. Consider:

- Coalescing writes across files
- Background flush thread
- Write-ahead logging for durability

### 4. Caching Strategy

Current LRU cache is per-file. Consider:

- Block-level caching for large files
- Predictive prefetching
- Cache invalidation on sync events

---

## Summary

This implementation provides:

1. **Robust FUSE filesystem** with proper inode management and operation handling
2. **HTTP-based coordination** through the daemon for sync integration
3. **Bug fixes** for first-write failures, path duplication, and macOS resource forks
4. **Persistent mount configuration** via SQLite database
5. **Process lifecycle management** through MountManager
6. **Full REST API** for programmatic control
7. **CLI commands** for user interaction
8. **Daemon integration** with auto-mount and graceful shutdown

Reference the commit `e4b2bbf` on branch `amiller68/fs-over-blobstore-v1` for the complete implementation.
