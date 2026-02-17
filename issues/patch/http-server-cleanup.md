# HTTP Server Module Cleanup

**Priority:** Urgent
**Type:** Patch/Refactor

## Problem

The `http_server` module has structural issues:

1. **Misleading module names** - `html/` only contains gateway handlers
2. **Monolithic handler** - `html/gateway/mod.rs` is 881 lines
3. **Inconsistent query params** - JSON requires Accept header, `deep` unused
4. **No per-file pattern** - doesn't follow API's request/handler/error structure

## Solution

### 1. Rename `html/` → `gateway/`

Move gateway-specific code under a properly named module.

### 2. Split `gateway/mod.rs` into:
- `mod.rs` - router only
- `index.rs` - gateway homepage
- `directory.rs` - directory listing handler (self-contained with its types/helpers)
- `file.rs` - file serving handler (self-contained with its types/helpers)

### 3. Simplify query parameters

**Before:**
- `at` - version hash
- `download` - force download
- `view` - show viewer UI
- `deep` - recursive listing
- Accept header for JSON

**After:**
- `at` - version hash (keep)
- `download` - force download (keep)
- `deep` - recursive listing (keep)
- `viewer` - opt-in HTML viewer UI (new)

Default behavior (no flags): raw JSON for directories, raw file bytes for files.
With `?viewer=true`: HTML explorer UI for directories, HTML viewer UI for files.

Remove `view` and `json` (superseded by `viewer` flag with inverted defaults).

### 4. Self-contained handler files

Each handler file contains everything it needs - no shared types module:
- Query structs, response structs, templates
- Handler function
- Helper functions (inline)
- Error responses (inline)

### 5. Template links include `?viewer=true`

All links in gateway HTML templates (index, explorer, file viewer) include
`?viewer=true` so navigation stays within the viewer experience.

## Acceptance Criteria

- [x] `cargo build` passes
- [x] `cargo test` passes
- [x] `cargo clippy` passes
- [x] Default (no flags) returns JSON for directories, raw file for files
- [x] `?viewer=true` shows HTML explorer/viewer UI
- [x] `?download` still forces file download
- [x] All viewer template links include `?viewer=true`

## Status

**In Progress** - PR #102, design revised 2026-02-17.

### 2026-02-17 - Initial implementation
- Renamed html/ → gateway/ module
- Split 881-line monolith into mod.rs, index.rs, directory.rs, file.rs
- PR: #102

### 2026-02-17 - Design change: ?viewer flag
- Removed `?json` flag (was added in initial implementation)
- Added `?viewer=true` flag with inverted defaults
- Default: raw JSON for directories, raw file bytes for files
- `?viewer=true`: HTML explorer/viewer UI
- Updated all gateway templates to include `?viewer=true` in links
- Decision: API-first design — programmatic access is the default, human browsing is opt-in

### 2026-02-17 - E2E verification
- Updated `bin/dev_/api.sh`: removed `Accept: application/json` header from `api_fetch()` (no longer needed since directories return JSON by default)
- E2E gateway verification confirmed all behaviors:
  - Default dir → JSON listing
  - `?viewer=true` dir → HTML explorer
  - Default file → raw bytes
  - `?viewer=true` file → HTML viewer
  - `?download=true` → attachment disposition
  - Template links propagate `?viewer=true`
- Cross-node sync verified: mirror node sees bucket, serves files via S3 gateway
- MinIO blob storage confirmed
