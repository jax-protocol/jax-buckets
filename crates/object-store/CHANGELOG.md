# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.2 (2026-02-15)

### New Features

 - <csr-id-a55c8dab42d2ba0b7ed3c145d44276d4a78cb951/> add native file export with save dialog
   * feat(desktop): add native file export with save dialog
   
   Add export/save-as functionality using tauri-plugin-dialog's save dialog.
   Files can be exported from both the Explorer file list (Export button) and
   the Viewer page (Save As button). The backend reads file content from the
   bucket and writes it directly to the user-chosen path on disk.

### Bug Fixes

 - <csr-id-ea35ed166fc376c5fef0950a61c705ba1344e58e/> document ImportBao size rejections as upstream iroh-blobs issue
   Root cause: iroh-blobs v0.95.0 BAO protocol reads blob size as raw
   8-byte LE from the wire without framing or checksums. During P2P
   discovery, stream corruption produces garbage u64 values. Rejections
   are transient — retries succeed once a clean stream arrives.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#81](https://github.com/jax-protocol/jax-fs/issues/81), [#82](https://github.com/jax-protocol/jax-fs/issues/82)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#81](https://github.com/jax-protocol/jax-fs/issues/81)**
    - Add native file export with save dialog ([`a55c8da`](https://github.com/jax-protocol/jax-fs/commit/a55c8dab42d2ba0b7ed3c145d44276d4a78cb951))
 * **[#82](https://github.com/jax-protocol/jax-fs/issues/82)**
    - Document ImportBao size rejections as upstream iroh-blobs issue ([`ea35ed1`](https://github.com/jax-protocol/jax-fs/commit/ea35ed166fc376c5fef0950a61c705ba1344e58e))
</details>

## v0.1.1 (2026-02-14)

### Bug Fixes

 - <csr-id-227fb3f9fc3d4c82381ddf85643ceaf20afd6000/> correct crate publish order and add missing README
   - Reorder publish steps: object-store → common → daemon
- Add README.md for jax-object-store (required by crates.io)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 1 calendar day.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#78](https://github.com/jax-protocol/jax-fs/issues/78)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#78](https://github.com/jax-protocol/jax-fs/issues/78)**
    - Bump jax-object-store v0.1.1, jax-daemon v0.1.8 ([`4311b03`](https://github.com/jax-protocol/jax-fs/commit/4311b03c6cb012b0e35a018750bbf03e6b574282))
 * **Uncategorized**
    - Correct crate publish order and add missing README ([`227fb3f`](https://github.com/jax-protocol/jax-fs/commit/227fb3f9fc3d4c82381ddf85643ceaf20afd6000))
</details>

## v0.1.0 (2026-02-13)

### New Features

 - <csr-id-30f511b983bf98d49081ef6aa6ad6e99b5c82c8f/> complete SQLite + S3 blob store with iroh-blobs integration
   * feat: implement iroh-blobs Store backend for S3 blob store

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#58](https://github.com/jax-protocol/jax-fs/issues/58), [#65](https://github.com/jax-protocol/jax-fs/issues/65)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#58](https://github.com/jax-protocol/jax-fs/issues/58)**
    - Complete SQLite + S3 blob store with iroh-blobs integration ([`30f511b`](https://github.com/jax-protocol/jax-fs/commit/30f511b983bf98d49081ef6aa6ad6e99b5c82c8f))
 * **[#65](https://github.com/jax-protocol/jax-fs/issues/65)**
    - Bump jax-object-store v0.1.0, jax-common v0.1.6, jax-daemon v0.1.7 ([`f0219f2`](https://github.com/jax-protocol/jax-fs/commit/f0219f2d882d65272b5cbe81a39680a06006a0d3))
</details>

<csr-unknown>
Add S3Actor to handle all ~20 proto::Request command variantsAdd S3Store wrapper implementing iroh-blobs Store APIAdd bucket existence check on S3 initialization (fail-fast)Add ensure_bucket to bin/minio for auto-creation in devUpdate e2e skill with sync timing guidance (60s wait)<csr-unknown/>

