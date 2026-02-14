# Bucket Publish CLI Command

- **Status:** Planned
- **Priority:** High

## Objective

Add a `jax bucket publish` CLI command that publishes a bucket, making its contents decryptable by all mirrors. Must respect the same business logic as the desktop UI: only the bucket owner can publish, and peers must reject publish operations from non-owners during sync.

## Background

Publishing a bucket stores the bucket secret in plaintext in the manifest, allowing mirrors to decrypt content. The daemon already exposes a `POST /api/v0/bucket/publish` endpoint and the desktop app uses it via Tauri IPC commands (`publish_bucket`, `is_published`). However, there is no CLI command to publish a bucket — users must use the desktop app.

The CLI should mirror the desktop UI behavior:
- Only the bucket owner can publish
- Non-owner publish attempts return an error
- Peers reject publish operations from non-owners during sync
- Publication status is tracked per-version in the bucket log

## Implementation Steps

### 1. Add `publish` CLI subcommand

**File:** `crates/daemon/src/cli/ops/bucket/publish.rs` (new)

Follow the pattern from `crates/daemon/src/cli/ops/bucket/shares/create.rs`. The command calls the existing `POST /api/v0/bucket/publish` endpoint.

```
jax bucket publish --bucket-id <UUID>
```

On success, print the new bucket link. On failure (not owner, bucket not found), print the error.

### 2. Add `is-published` CLI subcommand

**File:** `crates/daemon/src/cli/ops/bucket/is_published.rs` (new)

Query publication status via `POST /api/v0/bucket/latest-published`. Print whether the current HEAD is published and the published version height/link if available.

```
jax bucket is-published --bucket-id <UUID>
```

### 3. Wire up CLI commands

**File:** `crates/daemon/src/cli/ops/bucket/mod.rs`

Add `Publish` and `IsPublished` variants to the bucket subcommand enum and dispatch to the new handlers.

### 4. Owner-only validation in publish endpoint

**File:** `crates/daemon/src/http_server/api/v0/bucket/publish.rs`

Verify the publish endpoint checks that the caller is the bucket owner before publishing. If the check is missing, add it and return an appropriate error (e.g., HTTP 403).

### 5. Peer-side rejection of non-owner publishes

**File:** `crates/common/src/peer.rs`

During sync, peers must validate that publish operations originate from a bucket owner. Reject publish events from non-owners and log the unauthorized attempt.

### 6. Unit tests

Add unit tests covering:
- **Owner can publish** — owner publishes, verify manifest has plaintext secret and `is_published()` returns true
- **Non-owner publish is rejected** — non-owner attempts publish, verify error returned
- **Idempotent publish** — publishing an already-published bucket succeeds without error

### 7. E2E tests

**File:** `crates/common/tests/bucket_publish.rs` (new)

Using `common::setup_test_env()` and `common::fork_mount()` patterns, add end-to-end tests that exercise the full publish flow across peers:

- **Owner publishes and mirror can decrypt** — owner creates bucket, adds mirror, publishes; mirror syncs and successfully decrypts content using the plaintext secret
- **Mirror cannot publish** — mirror attempts to publish a bucket it doesn't own; verify the operation is rejected with an appropriate error
- **Peer rejects non-owner publish during sync** — owner shares with two peers; non-owner peer fabricates a publish event; syncing peer rejects the unauthorized publish and retains the unpublished state
- **Publish then unpublish round-trip** — owner publishes, verifies mirrors can decrypt, then saves without publish flag; verify the plaintext secret is removed from the manifest on next sync
- **CLI round-trip** — spin up a daemon, create a bucket, run `jax bucket publish`, verify `jax bucket is-published` reports published status, verify the bucket link is returned

## Files to Modify

| File | Changes |
|------|---------|
| `crates/daemon/src/cli/ops/bucket/publish.rs` | New: `publish` subcommand |
| `crates/daemon/src/cli/ops/bucket/is_published.rs` | New: `is-published` subcommand |
| `crates/daemon/src/cli/ops/bucket/mod.rs` | Wire up new subcommands |
| `crates/daemon/src/http_server/api/v0/bucket/publish.rs` | Add owner-only validation if missing |
| `crates/common/src/peer.rs` | Validate publish origin during sync |
| `crates/common/tests/bucket_publish.rs` | New: E2E tests for publish flow |

## Acceptance Criteria

- [ ] `jax bucket publish --bucket-id <UUID>` publishes the bucket and prints the new link
- [ ] `jax bucket is-published --bucket-id <UUID>` prints the publication status
- [ ] Only the bucket owner can publish; non-owners get an error
- [ ] Peers reject publish operations from non-owners during sync
- [ ] Unit tests cover owner publish, non-owner rejection, and idempotent publish
- [ ] E2E tests cover mirror decryption after publish, non-owner rejection, peer sync rejection, publish/unpublish round-trip, and CLI round-trip
- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt --check` passes
