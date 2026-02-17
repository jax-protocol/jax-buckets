# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [0.1.0] - 2025-10-12

### Added
- Initial release
- CLI tool for JaxBucket
- Encrypted storage bucket management

## v0.1.9 (2026-02-17)

### New Features

 - <csr-id-e7a06101d010e4065849d8feef0ea82edf7a61c0/> add negative cache and separate TTLs for FUSE performance
   Add a negative cache to avoid repeated lookups for non-existent paths
   (common with macOS resource forks like ._* files). Separate metadata and
   content TTLs so directory listings expire faster while file content stays
   cached longer. Add GET /api/v0/mounts/:id/cache-stats endpoint for
   debugging cache behavior.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 2 calendar days.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#80](https://github.com/jax-protocol/jax-fs/issues/80), [#84](https://github.com/jax-protocol/jax-fs/issues/84)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#80](https://github.com/jax-protocol/jax-fs/issues/80)**
    - Add negative cache and separate TTLs for FUSE performance ([`e7a0610`](https://github.com/jax-protocol/jax-fs/commit/e7a06101d010e4065849d8feef0ea82edf7a61c0))
 * **[#84](https://github.com/jax-protocol/jax-fs/issues/84)**
    - Add FUSE/non-FUSE desktop build separation ([`95bdccd`](https://github.com/jax-protocol/jax-fs/commit/95bdccd33f73d585a65fd8da4d84c718761c7915))
</details>

## v0.1.8 (2026-02-14)

### New Features

<csr-id-63712a7a66ce843e31c3a300ed3159b3a9042e2f/>

 - <csr-id-c63681313cfb66b28eec389c1e7147bdfafad39d/> fix port default, add health/shares commands, gate mount behind fuse
   * feat(cli): fix port default, add health/shares commands, gate mount behind fuse
- Fix --remote default: derive from config api_port (fallback 5001)
     instead of hardcoded port 3000
- Add `jax health` command: checks config dir, livez, readyz endpoints
- Add `jax bucket shares` subcommand group:
     - `shares create` to share a bucket with a peer
     - `shares ls` to list shares on a bucket
- `shares ls` to list shares on a bucket
* feat(fuse): implement setattr and xattr stubs for FUSE compatibility
- setattr: handles truncate (size) and mtime changes
- handle_truncate helper: resizes files via write buffers or Mount
- xattr stubs (setxattr, getxattr, listxattr, removexattr): return
     ENOTSUP for macOS compatibility

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#73](https://github.com/jax-protocol/jax-fs/issues/73), [#77](https://github.com/jax-protocol/jax-fs/issues/77), [#78](https://github.com/jax-protocol/jax-fs/issues/78)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#73](https://github.com/jax-protocol/jax-fs/issues/73)**
    - Implement missing FUSE operations for Unix command compatibility ([`63712a7`](https://github.com/jax-protocol/jax-fs/commit/63712a7a66ce843e31c3a300ed3159b3a9042e2f))
 * **[#77](https://github.com/jax-protocol/jax-fs/issues/77)**
    - Fix port default, add health/shares commands, gate mount behind fuse ([`c636813`](https://github.com/jax-protocol/jax-fs/commit/c63681313cfb66b28eec389c1e7147bdfafad39d))
 * **[#78](https://github.com/jax-protocol/jax-fs/issues/78)**
    - Bump jax-object-store v0.1.1, jax-daemon v0.1.8 ([`4311b03`](https://github.com/jax-protocol/jax-fs/commit/4311b03c6cb012b0e35a018750bbf03e6b574282))
</details>

<csr-unknown>
Add POST /api/v0/bucket/shares endpoint for listing sharesGate mount CLI commands behind #[cfg(feature = “fuse”)]Remove untested bucket sync commandUpdate PROJECT_LAYOUT.md with new commands and structure implement missing FUSE operations for Unix command compatibilityAdd missing FUSE operations that were causing “Function not implemented”errors for standard Unix commands (touch, mv, echo > file, truncate):<csr-unknown/>

## v0.1.7 (2026-02-13)

### New Features (BREAKING)

<csr-id-a413ee6c2157ffec2f39a9b2df6ea389e3988df2/>

 - <csr-id-ec12a4b6731782a787a29c90a440417916c26157/> add FUSE filesystem for mounting buckets as local directories
   * feat!: add FUSE filesystem for mounting buckets as local directories
* feat!: restructure daemon, add Tauri desktop app with full UI
- Remove Askama HTML UI (replaced by Tauri desktop app)
- Split HTTP server into run_api (private) and run_gateway (public)
- Export start_service + ShutdownHandle for embedding
- Add bucket_log history queries with published field
- Replace --app-port/--gateway-port with --api-port/--gateway-port
- Tauri backend with direct ServiceState IPC (no HTTP proxying)
- SolidJS frontend: Explorer, Viewer, Editor, History, Settings pages
- File explorer with breadcrumbs, upload, mkdir, delete, rename, move
- File viewer for text, markdown, images, video, audio
- Version history with read-only browsing of past versions
- Settings: auto-launch toggle, theme switcher, local config display
- SharePanel for per-bucket peer sharing from Explorer
- System tray with Open, Status, Quit
- Tauri capabilities for dialog and autostart permissions
- Separate CI (ci-tauri.yml) and release (release-desktop.yml) workflows

### Bug Fixes

 - <csr-id-76d456262a6fa4f16b4dfb6e7e120ac057bc47da/> use gateway URL for download button instead of localhost API
   The download button was using the localhost API URL which doesn't work
   for remote read-only nodes that don't expose the API over the internet.
   Now it uses the same gateway URL pattern as the share button, ensuring
   downloads work consistently across all node types.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#62](https://github.com/jax-protocol/jax-fs/issues/62), [#64](https://github.com/jax-protocol/jax-fs/issues/64), [#65](https://github.com/jax-protocol/jax-fs/issues/65)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#62](https://github.com/jax-protocol/jax-fs/issues/62)**
    - Restructure daemon, add Tauri desktop app with full UI ([`a413ee6`](https://github.com/jax-protocol/jax-fs/commit/a413ee6c2157ffec2f39a9b2df6ea389e3988df2))
 * **[#64](https://github.com/jax-protocol/jax-fs/issues/64)**
    - Add FUSE filesystem for mounting buckets as local directories ([`ec12a4b`](https://github.com/jax-protocol/jax-fs/commit/ec12a4b6731782a787a29c90a440417916c26157))
 * **[#65](https://github.com/jax-protocol/jax-fs/issues/65)**
    - Bump jax-object-store v0.1.0, jax-common v0.1.6, jax-daemon v0.1.7 ([`f0219f2`](https://github.com/jax-protocol/jax-fs/commit/f0219f2d882d65272b5cbe81a39680a06006a0d3))
</details>

<csr-unknown>
JaxFs: FUSE filesystem using fuser with all 10 core operationsMountManager: Lifecycle management (start, stop, auto-mount)InodeTable: Bidirectional inode ↔ path mappingFileCache: LRU cache with TTL for content and metadataSyncEvents: Cache invalidation on peer syncSQLite fuse_mounts table for mount persistencemount_queries.rs for CRUD + status operationsREST API at /api/v0/mounts/ (create, list, get, update, delete, start, stop)CLI commands: jax mount list|add|remove|start|stop|setAuto-mount on daemon startup, graceful unmount on shutdownPlatform-specific unmount (macOS umount, Linux fusermount -u)IPC commands for full mount managementSimplified mountBucket/unmountBucket API with auto mount point selectionOne-click Mount/Unmount buttons on Buckets pageAdvanced Mounts page for manual mount point configurationmacOS: /Volumes/<bucket-name> with Finder sidebar integrationLinux: /media/$USER/<bucket-name>Privilege escalation: AppleScript (macOS), pkexec (Linux)Naming conflict resolution with numeric suffixesDirect Mount access (not HTTP) to avoid self-call deadlockmacOS mount options: volname, local, noappledouble for FindermacOS resource fork filtering (._* files)Write buffering with sync-on-first-write for pending filesfuse feature enabled by default (runtime detection for availability)<csr-unknown/>

## v0.1.6 (2025-11-18)

<csr-id-ef5cd61f032d20ff42ea68caf22a4ac46355c137/>
<csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/>
<csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/>
<csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/>

### Chore

 - <csr-id-ef5cd61f032d20ff42ea68caf22a4ac46355c137/> bump jax-service and jax-bucket to 0.1.2
 - <csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/> updated readme reference
 - <csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/> update internal manifest versions

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

## v0.1.5 (2025-11-17)

<csr-id-ef5cd61f032d20ff42ea68caf22a4ac46355c137/>
<csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/>
<csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/>
<csr-id-1b2d7c55806152c9e67d452c90543966f1e6b7d6/>

### Chore

 - <csr-id-ef5cd61f032d20ff42ea68caf22a4ac46355c137/> bump jax-service and jax-bucket to 0.1.2
 - <csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/> updated readme reference
 - <csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/> update internal manifest versions

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

## v0.1.4 (2025-11-15)

## v0.1.3 (2025-11-15)

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

## v0.1.2 (2025-10-13)

<csr-id-ef5cd61f032d20ff42ea68caf22a4ac46355c137/>
<csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/>
<csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/>

### Chore

 - <csr-id-ef5cd61f032d20ff42ea68caf22a4ac46355c137/> bump jax-service and jax-bucket to 0.1.2
 - <csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/> updated readme reference

### Chore

 - <csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/> update internal manifest versions

## v0.1.1 (2025-10-12)

<csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/>
<csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/>

### Chore

 - <csr-id-20eab70de45b734acd0e44f4340dcb6659b32e84/> update internal manifest versions
 - <csr-id-d0a31f491f14927e4b5453daceeaafc963dd4171/> updated readme reference

