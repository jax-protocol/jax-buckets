# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [0.1.0] - 2025-10-12

### Added
- Initial release
- Core data structures and cryptography
- End-to-end encrypted P2P storage primitives

## v0.1.7 (2026-02-17)

### New Features

 - <csr-id-c3abb856836a0e904cd487170abea4a37cf15a54/> add bucket publish CLI command
   - Add `jax bucket publish --bucket-id <UUID>` subcommand
   - Add owner-only validation to publish endpoint (HTTP 403 for non-owners)
   - Add integration tests for owner publish and publish/unpublish round-trip
   - Update PROJECT_LAYOUT.md and issue ticket

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#85](https://github.com/jax-protocol/jax-fs/issues/85)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#85](https://github.com/jax-protocol/jax-fs/issues/85)**
    - Add bucket publish CLI command ([`c3abb85`](https://github.com/jax-protocol/jax-fs/commit/c3abb856836a0e904cd487170abea4a37cf15a54))
</details>

## v0.1.6 (2026-02-13)

### New Features

<csr-id-0a6d4fe6379ad7b96bf2f2169fb70d4e7d05f5bc/>
<csr-id-b62a25cf7f6b86d18a262281127fa16d94d6ed58/>
<csr-id-cabccaca7a0cbd91b294d5d96a1cc9992c8ffef3/>
<csr-id-7f4dcb71a245455d6818b117bcea4ac76ac677c8/>
<csr-id-7af5ca16a8e0748a922a39e3e8fecb1a7411e3db/>
<csr-id-75f36dfd89913f4296dc1e9e8f0dd4b24d903fe7/>
<csr-id-b30cb13139cc12ec1d4f31e2e8d14cfcfbf00865/>

 - <csr-id-30f511b983bf98d49081ef6aa6ad6e99b5c82c8f/> complete SQLite + S3 blob store with iroh-blobs integration
   * feat: implement iroh-blobs Store backend for S3 blob store
- Add S3Actor to handle all ~20 proto::Request command variants
- Add S3Store wrapper implementing iroh-blobs Store API
- Add bucket existence check on S3 initialization (fail-fast)
- Add ensure_bucket to bin/minio for auto-creation in dev
- Update e2e skill with sync timing guidance (60s wait)
* feat: add sync validation for signed manifests
- Check signature is valid
- Check author was in previous manifest's shares (prevents self-authorization)
- Validate entire manifest chain, not just the latest
- Accept unsigned manifests with warning (migration mode)
* feat: add pluggable conflict resolution for PathOpLog merges
- LastWriteWins: Higher timestamp wins (default CRDT behavior)
- BaseWins: Local operations always win
- ForkOnConflict: Keep both, return unresolved conflicts
* feat: add jax-blobs-store crate with SQLite + object storage backend
- SQLite for metadata (hash, size, state, timestamps)
- Pluggable object storage backends (S3/MinIO/local/memory)
- Content-addressed storage using BLAKE3 hashes (iroh-blobs compatible)
- Recovery support to rebuild metadata from object storage
- Add ManifestError type for signing/verification errors
- Add sign() and verify_signature() methods to Manifest
- Sign manifests automatically in Mount::init() and Mount::save()
- Store SecretKey in MountInner for signing
- Enable serde feature for ed25519-dalek
- Add comprehensive unit tests for signing and tamper detection
* feat: add mirror principal role and bucket publishing workflow
- Mirror principals can sync buckets but cannot decrypt until published
- Extended /share endpoint with role parameter (defaults to owner)
- Added /publish endpoint to grant mirrors decryption access
- Mirrors start with Option<SecretShare> None until bucket is published
- MirrorCannotMount error when unpublished mirror tries to load bucket
* feat: add path operation CRDT for conflict-free sync
* feat: add mv operation to Mount for moving/renaming files and directories

### Bug Fixes

 - <csr-id-2edfaf0ccb6fd91c08e5676385a5e2ec732040b8/> sync from available peers instead of failing if one is offline
   * fix: sync from available peers instead of failing if one is offline

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#24](https://github.com/jax-protocol/jax-fs/issues/24), [#27](https://github.com/jax-protocol/jax-fs/issues/27), [#32](https://github.com/jax-protocol/jax-fs/issues/32), [#36](https://github.com/jax-protocol/jax-fs/issues/36), [#49](https://github.com/jax-protocol/jax-fs/issues/49), [#50](https://github.com/jax-protocol/jax-fs/issues/50), [#52](https://github.com/jax-protocol/jax-fs/issues/52), [#57](https://github.com/jax-protocol/jax-fs/issues/57), [#58](https://github.com/jax-protocol/jax-fs/issues/58), [#65](https://github.com/jax-protocol/jax-fs/issues/65)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#24](https://github.com/jax-protocol/jax-fs/issues/24)**
    - Sync from available peers instead of failing if one is offline ([`2edfaf0`](https://github.com/jax-protocol/jax-fs/commit/2edfaf0ccb6fd91c08e5676385a5e2ec732040b8))
 * **[#27](https://github.com/jax-protocol/jax-fs/issues/27)**
    - Add mv operation to Mount ([`b30cb13`](https://github.com/jax-protocol/jax-fs/commit/b30cb13139cc12ec1d4f31e2e8d14cfcfbf00865))
 * **[#32](https://github.com/jax-protocol/jax-fs/issues/32)**
    - Add path operation CRDT for conflict-free sync ([`75f36df`](https://github.com/jax-protocol/jax-fs/commit/75f36dfd89913f4296dc1e9e8f0dd4b24d903fe7))
 * **[#36](https://github.com/jax-protocol/jax-fs/issues/36)**
    - Add mirror principal role and bucket publishing workflow ([`7af5ca1`](https://github.com/jax-protocol/jax-fs/commit/7af5ca16a8e0748a922a39e3e8fecb1a7411e3db))
 * **[#49](https://github.com/jax-protocol/jax-fs/issues/49)**
    - Add pluggable conflict resolution for PathOpLog merges ([`b62a25c`](https://github.com/jax-protocol/jax-fs/commit/b62a25cf7f6b86d18a262281127fa16d94d6ed58))
 * **[#50](https://github.com/jax-protocol/jax-fs/issues/50)**
    - Add author and signature fields to Manifest ([`7f4dcb7`](https://github.com/jax-protocol/jax-fs/commit/7f4dcb71a245455d6818b117bcea4ac76ac677c8))
 * **[#52](https://github.com/jax-protocol/jax-fs/issues/52)**
    - Add SQLite + object storage blob store backend ([`cabccac`](https://github.com/jax-protocol/jax-fs/commit/cabccaca7a0cbd91b294d5d96a1cc9992c8ffef3))
 * **[#57](https://github.com/jax-protocol/jax-fs/issues/57)**
    - Add sync validation for signed manifests ([`0a6d4fe`](https://github.com/jax-protocol/jax-fs/commit/0a6d4fe6379ad7b96bf2f2169fb70d4e7d05f5bc))
 * **[#58](https://github.com/jax-protocol/jax-fs/issues/58)**
    - Complete SQLite + S3 blob store with iroh-blobs integration ([`30f511b`](https://github.com/jax-protocol/jax-fs/commit/30f511b983bf98d49081ef6aa6ad6e99b5c82c8f))
 * **[#65](https://github.com/jax-protocol/jax-fs/issues/65)**
    - Bump jax-object-store v0.1.0, jax-common v0.1.6, jax-daemon v0.1.7 ([`f0219f2`](https://github.com/jax-protocol/jax-fs/commit/f0219f2d882d65272b5cbe81a39680a06006a0d3))
</details>

<csr-unknown>
The S3 blob store now fully integrates with iroh-blobs protocol,enabling P2P sync with blobs stored in S3/MinIO. add sync validation for signed manifestsValidate incoming manifests during bucket sync:Add SyncError enum with variants for validation failures.Add ProvenanceResult enum for internal validation results. add pluggable conflict resolution for PathOpLog mergesAdd ConflictResolver trait with three built-in strategies:Add merge_with_resolver() to PathOpLog for conflict-aware merging.Export conflict types from mount module.Includes 23 new tests for conflict detection and resolution. add SQLite + object storage blob store backendNew crate providing blob storage with: add author and signature fields to ManifestImplements signed-manifest-authorization ticket 0. add mirror principal role and bucket publishing workflowImplement polymorphic principal roles (Owner and Mirror) with publishing: add path operation CRDT for conflict-free syncIntroduce a lightweight Conflict-free Replicated Data Type (CRDT) to trackfilesystem path operations (add, remove, mkdir, mv) across peers. The operationlog is stored as a separate encrypted blob (not in the manifest) to avoidleaking directory structure information. Enables deterministic conflictresolution during peer sync using Lamport timestamps and peer IDs.🤖 Generated with https://claude.com/claude-codeClaude Code add mv operation to MountAdds a new mv method to the Mount struct that allows moving or renamingfiles and directories. The operation preserves the existing NodeLink (nore-encryption of content needed), creates intermediate directories ifneeded, and properly tracks all new node hashes in pins.🤖 Generated with https://claude.com/claude-codeClaude CodeAllow sync operations to work with multiple peers from bucket shares,falling back to other peers if the preferred one is unreachable. Thisfixes the bug where sync fails entirely if not all peers are online.🤖 Generated with https://claude.com/claude-codeClaude Code<csr-unknown/>

## v0.1.5 (2025-11-18)

<csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/>

### Bug Fixes

 - <csr-id-2f3e70f535b5aff4a13ea4df9bbf59047d0dd8c9/> own

### Other

 - <csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/> Consolidate peer state management into unified architecture
   * fix: refacoted state
   
   * fix: better api
   
   * progress
   
   * saving work
   
   * fix: bucket log trait
   
   * saving work
   
   * fix: more refavctor
   
   * feat: job model
   
   * feat: intergrate new protocl peer into example service
   
   * fix: node back to running
   
   * feat: working demo again
   
   * fix: rm test data
   
   * chore: move peer builder to its own file
   
   * fix: split out sync managet into its own thing
   
   * feat: bunch of ui updates
   
   * feat: actual fucking file viewer
   
   * fix: oops
   
   * ci: fix
   
   * ci: fix
   
   * fix: video playing

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#15](https://github.com/jax-protocol/jax-fs/issues/15), [#16](https://github.com/jax-protocol/jax-fs/issues/16), [#18](https://github.com/jax-protocol/jax-fs/issues/18)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#15](https://github.com/jax-protocol/jax-fs/issues/15)**
    - Bump jax-common v0.1.5, jax-bucket v0.1.6 ([`c239f47`](https://github.com/jax-protocol/jax-fs/commit/c239f477f3353c779bb731b2027edde31598dad7))
 * **[#16](https://github.com/jax-protocol/jax-fs/issues/16)**
    - Bump jax-common v0.1.5, jax-bucket v0.1.6 ([`a5d2374`](https://github.com/jax-protocol/jax-fs/commit/a5d2374b45790c295d43f7c66159d46ac2c15bf4))
 * **[#18](https://github.com/jax-protocol/jax-fs/issues/18)**
    - Bump jax-common v0.1.5, jax-bucket v0.1.6 ([`414464a`](https://github.com/jax-protocol/jax-fs/commit/414464a83b79b34590fed77df3dd500fe22a59c2))
 * **Uncategorized**
    - Bump jax-common v0.1.5, jax-bucket v0.1.6 ([`96d3bb8`](https://github.com/jax-protocol/jax-fs/commit/96d3bb8821d510e36c3385ce943afc3ca53fa547))
</details>

## v0.1.4 (2025-11-17)

<csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/>

### Bug Fixes

 - <csr-id-2f3e70f535b5aff4a13ea4df9bbf59047d0dd8c9/> own

### Other

 - <csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/> Consolidate peer state management into unified architecture
   * fix: refacoted state
   
   * fix: better api
   
   * progress
   
   * saving work
   
   * fix: bucket log trait
   
   * saving work
   
   * fix: more refavctor
   
   * feat: job model
   
   * feat: intergrate new protocl peer into example service
   
   * fix: node back to running
   
   * feat: working demo again
   
   * fix: rm test data
   
   * chore: move peer builder to its own file
   
   * fix: split out sync managet into its own thing
   
   * feat: bunch of ui updates
   
   * feat: actual fucking file viewer
   
   * fix: oops
   
   * ci: fix
   
   * ci: fix
   
   * fix: video playing

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#11](https://github.com/jax-protocol/jax-fs/issues/11), [#12](https://github.com/jax-protocol/jax-fs/issues/12)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#11](https://github.com/jax-protocol/jax-fs/issues/11)**
    - Alex/misc fixes ([`2fb5ea6`](https://github.com/jax-protocol/jax-fs/commit/2fb5ea6e39a4f4d1cdfb9668511fabe731a22e92))
 * **[#12](https://github.com/jax-protocol/jax-fs/issues/12)**
    - Bump jax-common v0.1.4, jax-bucket v0.1.5 ([`9517f35`](https://github.com/jax-protocol/jax-fs/commit/9517f35911441ae4b7ce93c75774b1cdb47a7731))
</details>

## v0.1.3 (2025-11-15)

### Bug Fixes

 - <csr-id-2f3e70f535b5aff4a13ea4df9bbf59047d0dd8c9/> own

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Adjusting changelogs prior to release of jax-common v0.1.3, jax-bucket v0.1.4 ([`96c3c3f`](https://github.com/jax-protocol/jax-fs/commit/96c3c3fdd170dcfa12c4c08f23b09d077ea543c2))
    - Bump jax-common v0.1.2 ([`e1d5272`](https://github.com/jax-protocol/jax-fs/commit/e1d5272f93e6b1eeb60c0ccbf4976a5247fdc952))
    - Own ([`2f3e70f`](https://github.com/jax-protocol/jax-fs/commit/2f3e70f535b5aff4a13ea4df9bbf59047d0dd8c9))
</details>

## v0.1.2 (2025-11-15)

<csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/>

### Other

 - <csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/> Consolidate peer state management into unified architecture
   * fix: refacoted state
   
   * fix: better api
   
   * progress
   
   * saving work
   
   * fix: bucket log trait
   
   * saving work
   
   * fix: more refavctor
   
   * feat: job model
   
   * feat: intergrate new protocl peer into example service
   
   * fix: node back to running
   
   * feat: working demo again
   
   * fix: rm test data
   
   * chore: move peer builder to its own file
   
   * fix: split out sync managet into its own thing
   
   * feat: bunch of ui updates
   
   * feat: actual fucking file viewer
   
   * fix: oops
   
   * ci: fix
   
   * ci: fix
   
   * fix: video playing

### Bug Fixes

 - <csr-id-2f3e70f535b5aff4a13ea4df9bbf59047d0dd8c9/> own

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#5](https://github.com/jax-protocol/jax-fs/issues/5)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#5](https://github.com/jax-protocol/jax-fs/issues/5)**
    - Consolidate peer state management into unified architecture ([`1b2d7c5`](https://github.com/jax-protocol/jax-fs/commit/1b2d7c55806152c9e67d452c90543966f1e6b7d6))
 * **Uncategorized**
    - Bump jax-common v0.1.2, jax-bucket v0.1.3 ([`625a2eb`](https://github.com/jax-protocol/jax-fs/commit/625a2eb01786f8367e0446da8420c233447c0793))
</details>

## v0.1.1 (2025-10-13)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Adjusting changelogs prior to release of jax-common v0.1.1, jax-service v0.1.2, jax-bucket v0.1.2 ([`7cb3b73`](https://github.com/jax-protocol/jax-fs/commit/7cb3b737b9febdcc7612cf9b827b7b63ee9fbb4f))
    - Adjusting changelogs prior to release of jax-common v0.1.1, jax-service v0.1.1, jax-bucket v0.1.1 ([`e053057`](https://github.com/jax-protocol/jax-fs/commit/e0530577122769502f93af02296d02430f5e1f13))
    - Chore: restructure workspace and setup   independent versioning ([`325e79b`](https://github.com/jax-protocol/jax-fs/commit/325e79b23b66d0a086a639130ade90ba11fd4a4d))
</details>

