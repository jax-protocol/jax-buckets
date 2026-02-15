# Desktop App FUSE/Non-FUSE Build Separation

- **Status:** In Progress
- **Priority:** High

## Objective

Produce separate FUSE and non-FUSE desktop builds via CI/CD, publish both on the releases page, and document building from source with/without FUSE.

## Background

Feature flags exist in code but CI/releases/docs only cover the FUSE path. Systems without FUSE support (Windows, some Linux configs) can't use pre-built desktop releases. The default (normal) build is **without** FUSE. The FUSE variant gets a `_fuse` suffix in artifact names. FUSE support is only offered on macOS Apple Silicon for now.

## Implementation Steps

### 1. CI: Add non-FUSE desktop build check

Update `ci-tauri.yml` to add a second job (or matrix) that builds without FUSE (`cargo build --no-default-features --features custom-protocol`) and without `libfuse3-dev` installed, ensuring the non-FUSE path stays compilable.

### 2. Release: Build four desktop artifacts

Update `release-desktop.yml` with a matrix producing:

- `Jax_*_aarch64.dmg` — macOS Apple Silicon, no FUSE
- `Jax_*_aarch64_fuse.dmg` — macOS Apple Silicon, with FUSE (requires macFUSE install step)
- `Jax_*_x64.dmg` — macOS Intel, no FUSE
- `Jax_*_amd64.deb` / `Jax_*_amd64.AppImage` — Linux, no FUSE

### 3. Release notes

Update the release body template to list all four variants with platform and FUSE labels.

### 4. Docs: Update INSTALL.md

Add a "Building Without FUSE" section explaining `--no-default-features`, what's lost (mount commands), and when to use it. Update the desktop download table to show all four variants (macOS aarch64, macOS aarch64 FUSE, macOS x64, Linux amd64). Note FUSE mount support is only available on macOS Apple Silicon currently.

### 5. E2E tests: FUSE-conditional test gating

Update e2e tests so FUSE-dependent tests are gated behind `#[cfg(feature = "fuse")]`. When FUSE tests are skipped (non-FUSE build or unsupported platform), emit a warning explaining why (e.g., "Skipping FUSE tests: built without `fuse` feature" or "Skipping FUSE tests: platform not supported"). This ensures `cargo test` passes cleanly on all build variants without silently hiding test coverage gaps.

### 6. Docs: Update README.md

Add a Downloads section linking to releases, noting standard and `_fuse` variants.

## Files to Modify

- `.github/workflows/ci-tauri.yml`
- `.github/workflows/release-desktop.yml`
- `agents/INSTALL.md`
- `README.md`

## Acceptance Criteria

- CI builds desktop app both with and without FUSE feature
- Non-FUSE CI build does NOT require libfuse3-dev / macFUSE
- E2E tests gate FUSE-dependent tests behind feature flag and emit a warning on unsupported builds/platforms
- Release workflow produces four artifacts: macOS aarch64, macOS aarch64 FUSE, macOS x64, Linux amd64
- All variants available on GitHub releases page
- INSTALL.md documents building with/without FUSE, lists which platforms support FUSE (macOS Apple Silicon only)
- README references download variants and links to releases page
- `cargo build` / `cargo test` / `cargo clippy` / `cargo fmt --check` pass

## Progress Log

### 2026-02-15 - Started
- Beginning implementation
- All 6 implementation steps already completed in prior commits
- CI matrix with FUSE/no-FUSE variants in ci-tauri.yml
- Release workflow produces 4 artifacts (macOS aarch64, macOS aarch64 FUSE, macOS x64, Linux amd64)
- Release notes include download table with all variants
- INSTALL.md updated with download table, "Building Without FUSE" section, platform support notes
- E2E tests gated behind #[cfg(feature = "fuse")] with warning in mount_feature_gate.rs
- README.md updated with Downloads section
