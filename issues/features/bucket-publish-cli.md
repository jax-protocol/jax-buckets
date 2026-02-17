# Bucket Publish CLI Command

- **Status:** Done
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

### 2. Wire up CLI command

**File:** `crates/daemon/src/cli/ops/bucket/mod.rs`

Add `Publish` variant to the bucket subcommand enum and dispatch to the new handler.

### 3. Owner-only validation in publish endpoint

**File:** `crates/daemon/src/http_server/api/v0/bucket/publish.rs`

Verify the publish endpoint checks that the caller is the bucket owner before publishing. If the check is missing, add it and return an appropriate error (e.g., HTTP 403).

### 4. Integration tests

**File:** `crates/common/tests/bucket_publish.rs` (new)

- **Owner publishes and mirror can decrypt** — owner creates bucket, adds mirror, publishes; mirror syncs and successfully decrypts content using the plaintext secret
- **Publish then unpublish round-trip** — owner publishes, verifies mirrors can decrypt, then saves without publish flag; verify the plaintext secret is removed from the manifest on next sync

## Files to Modify

| File | Changes |
|------|---------|
| `crates/daemon/src/cli/ops/bucket/publish.rs` | New: `publish` subcommand |
| `crates/daemon/src/cli/ops/bucket/mod.rs` | Wire up new subcommand |
| `crates/daemon/src/http_server/api/v0/bucket/publish.rs` | Add owner-only validation |
| `crates/common/tests/bucket_publish.rs` | New: integration tests for publish flow |

## Acceptance Criteria

- [x] `jax bucket publish --bucket-id <UUID>` publishes the bucket and prints the new link
- [x] Only the bucket owner can publish; non-owners get HTTP 403
- [x] Integration tests cover owner publish and publish/unpublish round-trip
- [x] `cargo build` compiles
- [x] `cargo test` passes
- [x] `cargo fmt --check` passes
