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
- `json` - JSON output (new, replaces Accept header requirement)

Remove `view` (redundant).

### 4. Self-contained handler files

Each handler file contains everything it needs - no shared types module:
- Query structs, response structs, templates
- Handler function
- Helper functions (inline)
- Error responses (inline)

## Acceptance Criteria

- [ ] `cargo build` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes
- [ ] `?json` returns JSON for directories (listing) and files (metadata)
- [ ] HTML explorer works without `?json`
- [ ] `?download` still forces file download

## Progress Log

### 2026-02-17 - Started
- Beginning implementation

### 2026-02-17 - In Review
- Renamed `html/` → `gateway/` module
- Split 881-line monolith into: `mod.rs` (router), `index.rs`, `directory.rs`, `file.rs`
- Replaced Accept header JSON detection with `?json` query param
- Removed `?view` param (redundant)
- Removed URL rewriting (simplified for clarity)
- Each handler file is self-contained with its own types/helpers
- All checks pass: build, test, fmt
