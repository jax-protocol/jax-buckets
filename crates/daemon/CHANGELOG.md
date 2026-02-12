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

## v0.1.7 (2026-02-12)

### New Features (BREAKING)

 - <csr-id-a413ee6c2157ffec2f39a9b2df6ea389e3988df2/> restructure daemon, add Tauri desktop app with full UI
   * feat!: restructure daemon, add Tauri desktop app with full UI
   
   Rename crates/app → crates/daemon (lib+bin) and create crates/desktop
   (Tauri 2.0 + SolidJS). The daemon becomes a headless service with
   separate API and gateway ports for security isolation. The desktop app
   embeds the daemon in-process with direct ServiceState access for IPC.
   
   Daemon changes:
   - Remove Askama HTML UI (replaced by Tauri desktop app)
   - Split HTTP server into run_api (private) and run_gateway (public)
   - Export start_service + ShutdownHandle for embedding
   - Add bucket_log history queries with published field
   - Replace --app-port/--gateway-port with --api-port/--gateway-port
   
   Desktop app (crates/desktop):
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

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#62](https://github.com/jax-protocol/jax-fs/issues/62)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#62](https://github.com/jax-protocol/jax-fs/issues/62)**
    - Restructure daemon, add Tauri desktop app with full UI ([`a413ee6`](https://github.com/jax-protocol/jax-fs/commit/a413ee6c2157ffec2f39a9b2df6ea389e3988df2))
</details>

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

