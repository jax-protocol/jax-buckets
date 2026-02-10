# Daemon Simplification

**Status:** Done
**Track:** Common
**Depends on:** Tickets 0-2 (complete)
**Blocks:** Ticket 4 (Tauri app)

## Objective

Consolidate the daemon to a single port serving both REST API and gateway content under URL prefixes. Remove the Askama HTML UI entirely -- the SolidJS Tauri app (ticket 4) replaces it.

## Motivation

The current daemon has three modes (`--gateway`, `--with-gateway`, default) running on up to two ports. This was over-engineered. The daemon should be a headless service: P2P peer + HTTP server. The UI moves to Tauri.

## Architecture (Before → After)

**Before:**
```
jax daemon (default)         → App server (Askama UI + REST API) on port 8080
jax daemon --with-gateway    → App server on 8080 + Gateway on 9092
jax daemon --gateway         → Gateway-only on 9092
```

**After:**
```
jax daemon                   → Single server on one port
  /api/v0/*                  → REST API (bucket CRUD, file ops)
  /gw/*                      → Gateway (published content serving)
  /_status/*                 → Health checks
```

No Askama UI. No multi-port configuration. One daemon, one port.

## Implementation Steps

### 1. Consolidate HTTP server to single port

**Modify:** `crates/app/src/daemon/http_server/mod.rs`
- Remove `run_app()` and `run_gateway()` split
- Create single `run()` function with one axum Router
- Nest API routes under `/api/v0/`
- Nest gateway routes under `/gw/`
- Mount health routes at `/_status/`
- Keep static assets at `/static/`

### 2. Simplify daemon CLI args

**Modify:** `crates/app/src/ops/daemon.rs`
- Remove `--gateway`, `--with-gateway`, `--gateway-port` flags
- Add `--port` flag (single port, default 8080)
- Keep `--api-url`, `--gateway-url` for external URL configuration
- Keep `--log-dir`

### 3. Simplify ServiceConfig

**Modify:** `crates/app/src/daemon/config.rs`
- Replace `app_port: Option<u16>` and `gateway_port: Option<u16>` with `port: u16`
- Remove mode-related logic

### 4. Simplify spawn_service

**Modify:** `crates/app/src/daemon/process/mod.rs`
- Remove conditional app/gateway spawning
- Always spawn peer + single HTTP server
- Update shutdown logic

### 5. Remove Askama HTML UI

**Delete files:**
- `crates/app/src/daemon/http_server/html/` (entire directory)
- `crates/app/templates/layouts/`
- `crates/app/templates/pages/index.html`
- `crates/app/templates/pages/not_found.html`
- `crates/app/templates/pages/buckets/` (entire directory)
- `crates/app/templates/components/` (entire directory)

**Keep:**
- `crates/app/templates/pages/gateway/` (gateway explorer/viewer templates)
- Gateway HTML handlers (read-only file explorer for published content)

**Modify:** `crates/app/Cargo.toml`
- Remove `askama` dependency if no longer needed (check if gateway templates still use it)
- If gateway templates still need Askama, keep it but scope it down

### 6. Update HTTP server config

**Modify:** `crates/app/src/daemon/http_server/config.rs`
- Simplify to single listen address
- Keep `api_url` and `gateway_url` for link generation

### 7. Update gateway index

**Modify:** `crates/app/src/daemon/http_server/gateway_index.rs`
- This becomes the root `/` handler (or `/gw/` handler)
- Lists published buckets

## Files Summary

| Action | File | Changes |
|--------|------|---------|
| Modify | `crates/app/src/ops/daemon.rs` | Remove multi-mode flags, add `--port` |
| Modify | `crates/app/src/daemon/config.rs` | Single port config |
| Modify | `crates/app/src/daemon/process/mod.rs` | Single server spawn |
| Modify | `crates/app/src/daemon/http_server/mod.rs` | Unified router |
| Modify | `crates/app/src/daemon/http_server/config.rs` | Simplified config |
| Modify | `crates/app/src/daemon/http_server/gateway_index.rs` | Route adjustment |
| Delete | `crates/app/src/daemon/http_server/html/` | Askama UI handlers |
| Delete | `crates/app/templates/pages/buckets/` | Bucket management templates |
| Delete | `crates/app/templates/pages/index.html` | Dashboard template |
| Delete | `crates/app/templates/components/` | UI components |
| Delete | `crates/app/templates/layouts/` | Base layouts (if unused by gateway) |

## Acceptance Criteria

- [x] `jax daemon` starts single HTTP server on one port
- [x] `/api/v0/bucket/*` endpoints work (all existing REST API)
- [x] `/gw/:bucket_id/*` serves published content (HTML explorer + JSON)
- [x] `/_status/livez`, `/_status/readyz`, `/_status/version`, `/_status/identity` work
- [x] No Askama UI routes (`/buckets`, `/buckets/:id`, etc.) exist
- [x] No `--gateway`, `--with-gateway` flags
- [x] P2P peer always runs
- [x] `cargo test` passes
- [x] `cargo clippy` has no warnings

## Actual Implementation Notes

**CLI changes:**
- `jax init --api-port <port> --gateway-port <port>` (defaults: 5001, 8080)
- `jax daemon --api-port <port> --gateway-port <port>` (optional overrides, defaults from config)
- Removed `--api-url` flag (was in spec as "keep" but not needed without browser-based UI)
- Kept `--gateway-url` for generating share links

**Config changes:**
- `AppConfig.api_port` + `AppConfig.gateway_port` (separate ports for security)
- `ServiceConfig.api_port` + `ServiceConfig.gateway_port`
- API server is private (mutation/RPC), gateway server is public (read-only GET)

**Dev environment:**
- `nodes.toml` has `api_port` + `gateway_port` per node
- Node roles renamed: owner (primary), _owner (replica), mirror
- Ports: owner API:5002/GW:8081, _owner API:5003/GW:8082, mirror API:5004/GW:8083
- Default ports 5001/8080 reserved for local development

## Verification

```bash
# Start daemon
jax daemon

# REST API works (on API port)
curl -X POST http://localhost:5001/api/v0/bucket/list

# Gateway works (on gateway port)
curl http://localhost:8080/gw/<bucket-id>/
curl -H "Accept: application/json" http://localhost:8080/gw/<bucket-id>/

# Health checks work (on both ports)
curl http://localhost:5001/_status/livez
curl http://localhost:8080/_status/livez

# Ports are isolated
curl http://localhost:8080/api/v0/bucket/list  # 404 (gateway port, no API)
curl http://localhost:5001/gw/<bucket-id>/     # 404 (API port, no gateway)

# Old Askama routes are gone
curl http://localhost:5001/buckets  # 404
```
