# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.3 (2026-02-20)

### New Features

 - <csr-id-c339f04cd771efb6195c1779d9bd29b7a55027c7/> make blobs store configurable (separate paths + max import size)
   * feat: make blobs store configurable with separate DB/object paths and max import size
   
   - Update ObjectStore::new_local to accept separate db_path and objects_path
     instead of deriving both from a single data_dir
   - Make MAX_IMPORT_SIZE configurable via ObjectStoreActor instead of hardcoded
     constant, exposed as DEFAULT_MAX_IMPORT_SIZE (1GB)
   - Add optional db_path field to BlobStoreConfig::Filesystem variant for
     separate SQLite metadata DB location
   - Add max_import_size to AppConfig with serde default for backward compat
   - Thread max_import_size through setup_blobs_store, Blobs::setup, and
     ServiceConfig to the actor
   - Add *_with_max_import_size constructors to BlobsStore and ObjectStore
 - <csr-id-506bfcfcbb4f74fd93a1006e3f10a2e80c7b617b/> add partial blob tracking and BAO tree traversal
   * feat(object-store): add partial blob tracking and BAO tree traversal
   
   - Track partial blob state in SQLite metadata (partial/complete lifecycle)
   - Store BAO outboard data alongside blob data for verified streaming
   - Replace simplified leaf-only export_bao with proper BAO tree traversal
     using bao-tree's traverse_selected_rec (sends correct parent + leaf items)
   - Support range-filtered BAO export via ChunkRanges parameter
   - Skip re-import of already-complete blobs in import_bao
   - Report partial blob status in BlobStatus and observe commands
   - Add put_with_outboard, insert_partial, mark_complete, get_state,
     get_outboard, put_outboard methods to BlobStore
   - Add insert_partial_blob, mark_blob_complete, get_blob_state to Database
   - Make put_outboard/get_outboard non-test-only on Storage
   - Add 9 new tests covering partial blob lifecycle, outboard storage,
     BAO verified streaming export, and state transitions
 - <csr-id-57e828e933e11b23f817b09906dfb9e74ece9912/> stream recovery listing to avoid OOM on large stores
   * feat: stream recovery listing instead of collecting all keys into memory
   
   Replace list_data_hashes() with list_data_hashes_stream() that returns
   a Stream instead of Vec, preventing OOM on stores with millions of blobs.
   Add progress logging every 1000 blobs during recovery.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#108](https://github.com/jax-protocol/jax-fs/issues/108), [#110](https://github.com/jax-protocol/jax-fs/issues/110), [#111](https://github.com/jax-protocol/jax-fs/issues/111)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#108](https://github.com/jax-protocol/jax-fs/issues/108)**
    - Stream recovery listing to avoid OOM on large stores ([`57e828e`](https://github.com/jax-protocol/jax-fs/commit/57e828e933e11b23f817b09906dfb9e74ece9912))
 * **[#110](https://github.com/jax-protocol/jax-fs/issues/110)**
    - Make blobs store configurable (separate paths + max import size) ([`c339f04`](https://github.com/jax-protocol/jax-fs/commit/c339f04cd771efb6195c1779d9bd29b7a55027c7))
 * **[#111](https://github.com/jax-protocol/jax-fs/issues/111)**
    - Add partial blob tracking and BAO tree traversal ([`506bfcf`](https://github.com/jax-protocol/jax-fs/commit/506bfcfcbb4f74fd93a1006e3f10a2e80c7b617b))
</details>

## v0.1.2 (2026-02-17)

### New Features

 - <csr-id-a55c8dab42d2ba0b7ed3c145d44276d4a78cb951/> add native file export with save dialog
   * feat(desktop): add native file export with save dialog

### Bug Fixes

 - <csr-id-ea35ed166fc376c5fef0950a61c705ba1344e58e/> document ImportBao size rejections as upstream iroh-blobs issue
   Root cause: iroh-blobs v0.95.0 BAO protocol reads blob size as raw
   8-byte LE from the wire without framing or checksums. During P2P
   discovery, stream corruption produces garbage u64 values. Rejections
   are transient — retries succeed once a clean stream arrives.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 2 calendar days.
 - 3 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#105](https://github.com/jax-protocol/jax-fs/issues/105), [#81](https://github.com/jax-protocol/jax-fs/issues/81), [#82](https://github.com/jax-protocol/jax-fs/issues/82)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#105](https://github.com/jax-protocol/jax-fs/issues/105)**
    - Bump jax-object-store v0.1.2, jax-common v0.1.7, jax-daemon v0.1.9 ([`32b5b30`](https://github.com/jax-protocol/jax-fs/commit/32b5b3096ad278823f98ed59917a7a2401e78b15))
 * **[#81](https://github.com/jax-protocol/jax-fs/issues/81)**
    - Add native file export with save dialog ([`a55c8da`](https://github.com/jax-protocol/jax-fs/commit/a55c8dab42d2ba0b7ed3c145d44276d4a78cb951))
 * **[#82](https://github.com/jax-protocol/jax-fs/issues/82)**
    - Document ImportBao size rejections as upstream iroh-blobs issue ([`ea35ed1`](https://github.com/jax-protocol/jax-fs/commit/ea35ed166fc376c5fef0950a61c705ba1344e58e))
</details>

<csr-unknown>
Add export/save-as functionality using tauri-plugin-dialog’s save dialog.Files can be exported from both the Explorer file list (Export button) andthe Viewer page (Save As button). The backend reads file content from thebucket and writes it directly to the user-chosen path on disk.<csr-unknown/>

## v0.1.1 (2026-02-14)

### Bug Fixes

 - <csr-id-227fb3f9fc3d4c82381ddf85643ceaf20afd6000/> correct crate publish order and add missing README
   - Reorder publish steps: object-store → common → daemon

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

<csr-unknown>
Add README.md for jax-object-store (required by crates.io)<csr-unknown/>

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

