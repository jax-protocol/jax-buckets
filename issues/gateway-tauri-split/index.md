# Gateway and Tauri Split

## Background

The original `jax daemon` combined full local client functionality (Askama UI, REST API, P2P sync) with gateway serving. This epic restructures the architecture:

- **Daemon** becomes a headless service: P2P peer + API server (private) + gateway server (public) on separate ports
- **Tauri app** replaces the Askama web UI with a native SolidJS desktop app
- **FUSE** mounts buckets as local filesystems via a standalone crate

## Architecture (Target)

```
jax daemon (headless service)
├── P2P peer (owner/mirror roles)
├── API server on port 5001 (private, mutation/RPC)
│   ├── REST API at /api/v0/*
│   └── Health checks at /_status/*
├── Gateway server on port 8080 (public, read-only)
│   ├── Content serving at /gw/*
│   └── Health checks at /_status/*

jax-desktop (Tauri app - separate binary)
├── SolidJS frontend (Vite)
├── Tauri IPC → shared Rust crates
├── System tray + auto-launch
└── Full bucket management UI

FUSE mounts (optional daemon feature)
├── crates/fuse/ standalone crate
├── Mount buckets as local filesystems
└── Managed via REST API + CLI
```

## Tickets

| # | Ticket | Status | Track |
|---|--------|--------|-------|
| 0 | [Gateway subcommand](./0-gateway-subcommand.md) | Done | Gateway |
| 1 | [SQLite blob store](./1-sqlite-blobstore.md) | Done | Gateway |
| 2 | [Conflict resolution](./2-conflict-resolution.md) | Done | Common |
| 3 | [Daemon simplification](./3-daemon-simplification.md) | Done | Common |
| 4 | [Tauri desktop app](./4-tauri-app.md) | Planned | Local |
| 5 | [FUSE integration](./5-fuse-integration.md) | Planned | Local |

## PR Plan

```
PR 3:  Daemon simplification
       - Split API (port 5001) and gateway (port 8080) onto separate ports
       - Remove Askama HTML UI
       - Remove --gateway / --with-gateway flags
       │
       ▼
PR 4a: Tauri scaffold + IPC
       - Tauri 2.0 project in crates/desktop/
       - SolidJS + Vite setup
       - IPC commands mirroring REST API
       - System tray
       - Daemon lifecycle management
       │
       ▼
PR 4b: SolidJS UI pages
       - Bucket list, file explorer, viewer, editor
       - History, peers pages
       - Native file dialogs
       │
       ▼ (can run in parallel with 4b)
PR 5:  FUSE integration
       - New crates/fuse/ crate
       - JaxFs filesystem, MountManager
       - SQLite persistence, REST API, CLI commands
       - Feature-gated in daemon
```

## Execution Order

**Stage 1 (Foundation) - Complete:**
- Ticket 0: Gateway subcommand
- Ticket 1: SQLite blob store
- Ticket 2: Conflict resolution

**Stage 2 (Restructure) - Complete:**
- Ticket 3: Daemon simplification (prerequisite for Tauri)

**Stage 3 (Desktop + Filesystem):**
- Ticket 4: Tauri desktop app (PR 4a then 4b)
- Ticket 5: FUSE integration (can start after ticket 3, parallel with 4b)

## Superseded Tickets

Old tickets 3-5 have been archived as `_old-*` files:
- `_old-3-fuse-integration.md` → replaced by ticket 5 (new crate approach)
- `_old-4-desktop-integration.md` → superseded by ticket 4 (Tauri replaces native tray crates)
- `_old-5-tauri-migration.md` → promoted and expanded as ticket 4

## Reference Branches

| Branch | Reference For |
|--------|---------------|
| `amiller68/sqlite-minio-blobs` | SQLite + Object Storage blob backend |
| `amiller68/fs-over-blobstore-v1` | FUSE implementation |
| `amiller68/conflict-resolution` | Conflict resolution strategies |
| `amiller68/tauri-app-explore` | Tauri + SolidJS prototype |
