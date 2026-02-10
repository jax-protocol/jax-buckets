# Tauri Desktop App

**Status:** Planned
**Track:** Local
**Depends on:** Ticket 3 (daemon simplification)
**Blocks:** None (FUSE can proceed in parallel)
**Reference:** `amiller68/tauri-app-explore` branch
**Supersedes:** Old ticket 4 (desktop integration), old ticket 5 (Tauri migration)

## Objective

Build a Tauri 2.0 desktop app with a SolidJS frontend that replaces the Askama web UI. The app embeds/manages the daemon and provides full bucket management through IPC commands.

## Architecture

```
jax daemon (headless service)
├── P2P peer
├── REST API at /api/v0/*
├── Gateway at /gw/*
└── Health at /_status/*

jax-desktop (Tauri app)
├── SolidJS frontend (Vite)
├── Tauri IPC → Rust backend
│   ├── Embeds/spawns daemon
│   └── IPC commands mirror REST API
├── System tray (Open, Status, Quit)
└── Auto-launch on boot
```

The Tauri app is a **separate binary** that uses shared crates (`jax-common`, `jax-object-store`). It either embeds the daemon in-process or manages it as a child process.

## Implementation Plan (2 PRs)

### PR 4a: Tauri Scaffold + IPC

#### 1. Project setup

**Create:** `crates/desktop/`
- Tauri 2.0 project with `cargo tauri init`
- SolidJS + Vite frontend in `crates/desktop/ui/`
- `crates/desktop/src-tauri/` for Rust backend
- Add to workspace `Cargo.toml`

**Dependencies:**
- `tauri` 2.x
- `tauri-plugin-shell` (if spawning daemon as child)
- `tauri-plugin-dialog` (native file dialogs)
- `@solidjs/router` for SPA routing
- `vite` + `vite-plugin-solid`

#### 2. Rust backend with daemon embedding

**Create:** `crates/desktop/src-tauri/src/main.rs`
- On app start: spawn daemon in-process (reuse `spawn_service`)
- Or spawn as child process and connect via REST
- Manage daemon lifecycle (start on launch, stop on quit)

#### 3. IPC commands

**Create:** `crates/desktop/src-tauri/src/commands/`

Mirror the REST API as Tauri IPC commands:

| IPC Command | Maps to | Description |
|-------------|---------|-------------|
| `create_bucket` | `POST /api/v0/bucket/` | Create bucket |
| `list_buckets` | `POST /api/v0/bucket/list` | List buckets |
| `add_files` | `POST /api/v0/bucket/add` | Upload files |
| `update_file` | `POST /api/v0/bucket/update` | Update file |
| `delete_file` | `POST /api/v0/bucket/delete` | Delete file/dir |
| `mkdir` | `POST /api/v0/bucket/mkdir` | Create directory |
| `rename` | `POST /api/v0/bucket/rename` | Rename |
| `move_file` | `POST /api/v0/bucket/mv` | Move |
| `ls` | `POST /api/v0/bucket/ls` | List directory |
| `cat` | `POST/GET /api/v0/bucket/cat` | Read file |
| `share` | `POST /api/v0/bucket/share` | Share bucket |
| `publish` | `POST /api/v0/bucket/publish` | Publish version |
| `export` | `POST /api/v0/bucket/export` | Export bucket |
| `ping` | `POST /api/v0/bucket/ping` | Ping peers |

#### 4. System tray

- Tray icon with menu: Open Window, Status, Quit
- Status shows node ID and peer count
- Quit gracefully shuts down daemon

#### 5. Minimal SolidJS shell

- App layout with sidebar navigation
- Placeholder pages for bucket list, explorer
- Wired to IPC commands to verify round-trip works

### PR 4b: SolidJS UI Pages

#### 1. Bucket list page
- Grid/list of buckets with name, ID, peer count
- Create bucket button with dialog
- Share/publish actions

#### 2. File explorer page
- Directory tree or breadcrumb navigation
- File/folder listing with icons
- Upload files (native file dialog via Tauri)
- Create directory
- Delete, rename, move operations
- Download files

#### 3. File viewer page
- Text/code files with syntax highlighting
- Markdown rendering
- Image preview
- Hex dump for binary files
- Raw download button

#### 4. File editor page
- Text editor for code/markdown files
- Save triggers `update_file` IPC

#### 5. History/logs page
- Bucket version history
- View content at specific version

#### 6. Peers page
- Connected peers for bucket
- Ping action

## Files Summary

### PR 4a

| Action | Path | Description |
|--------|------|-------------|
| Create | `crates/desktop/` | Tauri project root |
| Create | `crates/desktop/Cargo.toml` | Rust crate config |
| Create | `crates/desktop/src-tauri/` | Tauri Rust backend |
| Create | `crates/desktop/src-tauri/src/main.rs` | Entry point + daemon lifecycle |
| Create | `crates/desktop/src-tauri/src/commands/` | IPC command handlers |
| Create | `crates/desktop/src-tauri/tauri.conf.json` | Tauri config |
| Create | `crates/desktop/ui/` | SolidJS frontend |
| Create | `crates/desktop/ui/package.json` | JS dependencies |
| Create | `crates/desktop/ui/vite.config.ts` | Vite config |
| Create | `crates/desktop/ui/src/` | SolidJS source |
| Modify | `Cargo.toml` | Add workspace member |

### PR 4b

| Action | Path | Description |
|--------|------|-------------|
| Create | `crates/desktop/ui/src/pages/` | All SolidJS pages |
| Create | `crates/desktop/ui/src/components/` | Shared UI components |
| Create | `crates/desktop/ui/src/lib/` | API client, utilities |

## Acceptance Criteria

### PR 4a
- [ ] `cargo tauri dev` launches app with webview
- [ ] Daemon starts automatically when app launches
- [ ] IPC commands work end-to-end (create bucket, list, add file, cat)
- [ ] System tray shows icon with Open/Status/Quit menu
- [ ] Quitting app gracefully stops daemon
- [ ] `cargo test` passes (workspace)
- [ ] `cargo clippy` has no warnings

### PR 4b
- [ ] Bucket list page shows all buckets
- [ ] File explorer supports navigation, upload, download, delete, mkdir
- [ ] File viewer renders text, markdown, images
- [ ] File editor can modify and save files
- [ ] History page shows version log
- [ ] Peers page shows connected peers
- [ ] Native file dialog for upload/export

## Verification

```bash
# Development
cd crates/desktop && cargo tauri dev

# Build
cd crates/desktop && cargo tauri build

# Verify IPC
# 1. App launches, daemon starts
# 2. Create bucket via UI
# 3. Upload file via native dialog
# 4. Browse files in explorer
# 5. View file content
# 6. Edit and save file
# 7. Verify tray menu works
# 8. Quit app, verify daemon stops
```
