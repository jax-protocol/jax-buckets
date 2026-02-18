# Desktop Sidecar Daemon Support

- **Status:** In Review
- **Priority:** High

## Objective

Allow the desktop app to connect to an already-running standalone `jax-daemon` process instead of always starting an embedded daemon. If a daemon is detected on the configured API port at startup, the desktop app uses it as-is; otherwise it falls back to spawning an embedded daemon.

## Background

The desktop app currently starts the daemon in-process via `spawn_daemon()` in `crates/desktop/src-tauri/src/lib.rs`. If a standalone `jax-daemon` is already running on the same ports, the embedded daemon fails to bind (`EADDRINUSE`), the error is only logged, and the app runs without a functioning daemon.

Additionally, 17 of 33 Tauri IPC commands access `ServiceState` directly via `get_service()`, tightly coupling the desktop to in-process daemon internals. Only 3 commands (`create_bucket`, `share_bucket`, `ping_peer`) use the daemon's HTTP API. This coupling prevents the desktop from working as a pure client against an external daemon.

The daemon already exposes a comprehensive HTTP API at `/api/v0/bucket/*` and `/api/v0/mounts/*` that covers nearly all operations the desktop needs. Converting all commands to HTTP and adding a few missing endpoints enables full sidecar support.

## Implementation Steps

### Phase 1: Add Missing Daemon HTTP Endpoints

#### 1. Add `POST /api/v0/bucket/history` endpoint

**File:** `crates/daemon/src/http_server/api/v0/bucket/history.rs` (new)

Add a history endpoint that returns paginated bucket version logs. Calls `state.database().get_bucket_logs(&bucket_id, page, page_size)` and maps entries to a JSON response with `link_hash`, `height`, `published`, and `created_at` fields.

Wire into the bucket router in `crates/daemon/src/http_server/api/v0/bucket/mod.rs`.

#### 2. Add `at` version parameter to `LsRequest`

**File:** `crates/daemon/src/http_server/api/v0/bucket/ls.rs`

Add `at: Option<String>` to `LsRequest`, following the same pattern used in `CatRequest` (`cat.rs` lines 104-126). When present, load the mount from the specified link hash instead of the current HEAD.

#### 3. Add `POST /api/v0/bucket/is-published` endpoint

**File:** `crates/daemon/src/http_server/api/v0/bucket/is_published.rs` (new)

Simple boolean endpoint that returns whether the current HEAD of a bucket is published. Avoids the desktop needing to compose multiple API calls.

### Phase 2: Convert Desktop Commands to HTTP

#### 4. Create shared HTTP client helper

**File:** `crates/desktop/src-tauri/src/commands/bucket.rs`

Replace per-command `reqwest::Client::new()` with a shared helper that constructs a client with the daemon base URL. All commands will use this instead of `get_service()`.

#### 5. Convert bucket commands to HTTP

**File:** `crates/desktop/src-tauri/src/commands/bucket.rs`

Convert all 17 direct-access commands to use HTTP API calls. The existing `create_bucket`, `share_bucket`, and `ping_peer` commands serve as the template. Mapping:

| Desktop Command | HTTP Endpoint |
|---|---|
| `list_buckets` | `POST /api/v0/bucket/list` |
| `delete_bucket` | `POST /api/v0/bucket/delete` (path="/") |
| `ls` | `POST /api/v0/bucket/ls` |
| `cat` | `POST /api/v0/bucket/cat` |
| `add_file` | `POST /api/v0/bucket/add` (multipart) |
| `update_file` | `POST /api/v0/bucket/update` (multipart) |
| `rename_path` | `POST /api/v0/bucket/rename` |
| `move_path` | `POST /api/v0/bucket/mv` |
| `mkdir` | `POST /api/v0/bucket/mkdir` |
| `delete_path` | `POST /api/v0/bucket/delete` |
| `publish_bucket` | `POST /api/v0/bucket/publish` |
| `is_published` | `POST /api/v0/bucket/is-published` (new) |
| `get_history` | `POST /api/v0/bucket/history` (new) |
| `ls_at_version` | `POST /api/v0/bucket/ls` (with `at` param) |
| `cat_at_version` | `POST /api/v0/bucket/cat` (with `at` param) |
| `get_bucket_shares` | `POST /api/v0/bucket/shares` |
| `upload_native_files` | `POST /api/v0/bucket/add` (multipart) |

#### 6. Convert daemon status commands to HTTP

**File:** `crates/desktop/src-tauri/src/commands/daemon.rs`

- `get_status`: Use `/_status/livez` + `/_status/identity` + stored port info
- `get_identity`: Use `/_status/identity`
- `get_config_info`: Read from local `AppState::load()` (paths are deterministic from `~/.jax`)

#### 7. Convert mount commands to HTTP

**File:** `crates/desktop/src-tauri/src/commands/mount.rs`

Convert all mount commands to use the daemon's `/api/v0/mounts/*` HTTP endpoints. The desktop becomes a thin client — the daemon owns all mount state, FUSE lifecycle, sync integration, conflict resolution, and cache management. The desktop only handles platform-specific concerns that require a GUI context.

**Goes through HTTP (daemon owns it):**

| Desktop Command | HTTP Endpoint |
|---|---|
| `list_mounts` | `GET /api/v0/mounts/` |
| `create_mount` | `POST /api/v0/mounts/` |
| `get_mount` | `GET /api/v0/mounts/:id` |
| `update_mount` | `PATCH /api/v0/mounts/:id` |
| `delete_mount` | `DELETE /api/v0/mounts/:id` |
| `start_mount` | `POST /api/v0/mounts/:id/start` |
| `stop_mount` | `POST /api/v0/mounts/:id/stop` |

**Stays local in the desktop (no HTTP):**

- `is_fuse_available` / `check_fuse_installed` — local system check (`/dev/fuse`, macFUSE presence)
- Mount point path generation — platform-specific defaults (`/Volumes/`, `/media/$USER/`, `/mnt/`)
- Privilege escalation for creating mount directories — requires GUI context (`osascript` on macOS, `pkexec` on Linux)

**Composing the convenience commands:** The current `mount_bucket` one-shot command (auto-generate mount point + create config + start) does not need a dedicated daemon endpoint. Instead, the desktop composes the existing HTTP calls: generate the mount point path locally, `POST /api/v0/mounts/` to create, then `POST /api/v0/mounts/:id/start` to start. Similarly, `unmount_bucket` composes list + stop + delete. This keeps platform-specific logic (path generation, privilege escalation) in the desktop where it belongs, and avoids adding desktop-specific conveniences to the daemon API.

### Phase 3: Add Sidecar Detection

#### 8. Refactor `DaemonInner` to remove `ServiceState`

**File:** `crates/desktop/src-tauri/src/lib.rs`

Replace `DaemonInner` with:

```rust
pub struct DaemonInner {
    pub api_port: u16,
    pub gateway_port: u16,
    pub jax_dir: PathBuf,
    pub mode: DaemonMode,
}

pub enum DaemonMode {
    Embedded,
    Sidecar,
}
```

Remove the `ServiceState` field and the `jax_daemon::ServiceState` import.

#### 9. Add daemon detection at startup

**File:** `crates/desktop/src-tauri/src/lib.rs`

Before starting the embedded daemon, probe `http://localhost:{api_port}/_status/livez` with a short timeout (~1 second):

- **200 OK**: Sidecar mode. Store the port info, skip `start_service()`.
- **Connection refused / timeout**: Embedded mode. Start the daemon as before.

Optionally verify via `/_status/identity` that the detected daemon is the same node (matching secret key).

#### 10. Surface connection mode in UI

Emit a Tauri event indicating whether the daemon is embedded or sidecar, so the frontend can display connection status.

## Files to Modify

| File | Changes |
|------|---------|
| `crates/daemon/src/http_server/api/v0/bucket/history.rs` | New: bucket version history endpoint |
| `crates/daemon/src/http_server/api/v0/bucket/is_published.rs` | New: is-published boolean endpoint |
| `crates/daemon/src/http_server/api/v0/bucket/ls.rs` | Add `at` version parameter to `LsRequest` |
| `crates/daemon/src/http_server/api/v0/bucket/mod.rs` | Wire new endpoints into router |
| `crates/desktop/src-tauri/src/lib.rs` | Refactor `DaemonInner`, add sidecar detection |
| `crates/desktop/src-tauri/src/commands/bucket.rs` | Convert 17 commands from direct `ServiceState` to HTTP |
| `crates/desktop/src-tauri/src/commands/daemon.rs` | Convert status commands to HTTP / local reads |
| `crates/desktop/src-tauri/src/commands/mount.rs` | Convert mount commands to HTTP |

## Acceptance Criteria

- [x] Desktop connects to an already-running `jax-daemon` without starting a second instance
- [x] Desktop falls back to embedded daemon when no sidecar is detected
- [x] All Tauri commands work identically in both embedded and sidecar modes
- [x] No direct `ServiceState` or `MountManager` access remains in the desktop crate
- [x] `POST /api/v0/bucket/history` endpoint returns paginated version logs
- [x] `POST /api/v0/bucket/ls` supports `at` parameter for version-specific listing
- [x] `is-published` replaced with `bucket stat` endpoint (height, version, peers, publish state)
- [x] Connection mode (embedded/sidecar) is surfaced to the UI
- [x] Daemon going away mid-session produces clear errors (not silent failures)
- [x] Desktop uses `ApiClient` from daemon crate (no hand-rolled HTTP calls)
- [x] `cargo build` compiles
- [x] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [x] `cargo fmt --check` passes
