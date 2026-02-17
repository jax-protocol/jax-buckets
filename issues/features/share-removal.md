# Share Removal

**Status:** Done
**Priority:** High

## Objective

Allow bucket owners to revoke shares by removing principals from a bucket. Peers must reject removal requests from non-owners during sync.

## Background

Once a bucket is shared with a peer, there is currently no way to revoke that access. Owners need the ability to remove principals for:

- **Access revocation** — remove a peer who should no longer have access
- **Security** — ensure compromised keys can be removed promptly
- **Lifecycle management** — clean up shares when peers leave a group

Only the bucket owner should be authorized to remove shares. Non-owner removal attempts must be rejected both locally and during peer sync to prevent unauthorized access revocation.

## Implementation Steps

### 1. Core share removal logic

**File:** `crates/common/src/mount/mount_inner.rs`

Added `remove_share` method to `Mount` and `MountError::Unauthorized` variant.

### 2. Peer-side validation

**File:** `crates/common/src/peer/sync/sync_bucket.rs`

Added step 5 to `verify_author` that checks if shares were removed and verifies the author had Owner role in the previous manifest. Added `ProvenanceError::UnauthorizedShareRemoval`.

### 3. CLI command

**File:** `crates/daemon/src/http_server/api/v0/bucket/unshare.rs`

Added `POST /api/v0/bucket/unshare` endpoint with `UnshareRequest`/`UnshareResponse`. CLI subcommand at `jax bucket shares remove` via `crates/daemon/src/cli/ops/bucket/shares/remove.rs`.

### 4. Desktop app UI

**Target:** Tauri desktop app (`crates/desktop/src/`)

Add a remove button to each entry in the share list. On click:
- Confirm the action with the user
- Call the share removal API
- Refresh the share list

**Files:**
- `crates/desktop/src-tauri/src/commands/bucket.rs` — add `remove_share` IPC command
- `crates/desktop/src/lib/api.ts` — add `removeShare` API function
- Share list component — add remove button per share entry

### 5. End-to-end tests

**File:** `crates/common/tests/share_removal.rs`

Using `common::setup_test_env()` and `common::fork_mount()` patterns, add tests for:

- **Owner can remove a share** — owner removes a principal, verify it is no longer in the manifest
- **Non-owner removal is rejected** — a non-owner attempts removal, verify it returns an error
- **Peers reject sync from unauthorized removal** — simulate a sync where a non-owner removed a share, verify the peer rejects it

## Files Modified

| File | Changes |
|------|---------|
| `crates/common/src/mount/mount_inner.rs` | Added `remove_share` method, `MountError::Unauthorized` |
| `crates/common/src/peer/sync/sync_bucket.rs` | Added share removal validation in `verify_author` |
| `crates/common/src/peer/sync/mod.rs` | Added `ProvenanceError::UnauthorizedShareRemoval` |
| `crates/daemon/src/http_server/api/v0/bucket/unshare.rs` | New `POST /unshare` endpoint |
| `crates/daemon/src/http_server/api/v0/bucket/mod.rs` | Wired unshare route |
| `crates/daemon/src/cli/ops/bucket/shares/remove.rs` | New `shares remove` CLI subcommand |
| `crates/daemon/src/cli/ops/bucket/shares/mod.rs` | Wired Remove variant |
| `crates/desktop/src-tauri/src/commands/bucket.rs` | Added `remove_share` IPC command |
| `crates/desktop/src-tauri/src/lib.rs` | Registered `remove_share` command |
| `crates/desktop/src/lib/api.ts` | Added `removeShare` API function |
| `crates/desktop/src/components/SharePanel.tsx` | Added Remove button per share entry |
| `crates/common/tests/share_removal.rs` | Integration tests |

## Acceptance Criteria

- [x] Owner can remove a share via CLI (`jax bucket shares remove`)
- [x] Owner can remove a share via desktop app UI
- [x] Non-owner removal attempts return an error
- [x] Peers reject share removal events from non-owners during sync
- [x] E2e tests cover owner removal, non-owner rejection, and removed peer access
- [x] `cargo build` compiles
- [x] `cargo test` passes
- [x] `cargo fmt --check` passes
