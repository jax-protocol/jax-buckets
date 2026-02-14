# Share Removal

**Status:** Planned
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

**File:** `crates/common/src/mount.rs`

Add a `remove_share` method to `Mount` that:
- Verifies the caller is the bucket owner
- Removes the principal (by public key) from the bucket manifest
- Returns an error if the caller is not the owner

```rust
pub async fn remove_share(&self, peer_public_key: PublicKey) -> Result<()> {
    // Verify caller is owner
    // Remove principal from manifest
    // Persist updated manifest
}
```

### 2. Peer-side validation

**File:** `crates/common/src/peer.rs`

During sync, peers must validate that share removal operations originate from an owner. Reject removal events from non-owners and log the unauthorized attempt.

### 3. CLI command

**File:** `crates/daemon/src/daemon/http_server/api/v0/bucket/share.rs`

Add a `jax share remove` subcommand:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ShareRemoveRequest {
    #[arg(long)]
    pub bucket_id: Uuid,

    #[arg(long)]
    pub peer_public_key: String,
}
```

Wire up the API handler to call `mount.remove_share()`.

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

## Files to Modify

| File | Changes |
|------|---------|
| `crates/common/src/mount.rs` | Add `remove_share` method with owner verification |
| `crates/common/src/peer.rs` | Validate share removal origin during sync |
| `crates/daemon/src/daemon/http_server/api/v0/bucket/share.rs` | Add `share remove` subcommand and API handler |
| `crates/desktop/src-tauri/src/commands/bucket.rs` | Add `remove_share` IPC command |
| `crates/desktop/src/lib/api.ts` | Add `removeShare` API function |
| `crates/desktop/src/` (share list component) | Add remove button per share entry |
| `crates/common/tests/share_removal.rs` | E2e tests for share removal scenarios |

## Acceptance Criteria

- [ ] Owner can remove a share via CLI (`jax share remove`)
- [ ] Owner can remove a share via desktop app UI
- [ ] Non-owner removal attempts return an error
- [ ] Peers reject share removal events from non-owners during sync
- [ ] E2e tests cover owner removal, non-owner rejection, and peer sync rejection
- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt --check` passes
