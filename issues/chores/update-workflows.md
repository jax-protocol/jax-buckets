# Update Workflows for CLI and Desktop

- **Status:** Done
- **Priority:** Normal

## Objective

Enable users to easily update their CLI and desktop app installations in response to minor version changes and patches without rebuilding from source or manually downloading distributions.

## Background

Currently, updating jax requires manual effort:
- **CLI**: Users must run `cargo install --force jax-daemon` (requires Rust toolchain) or rebuild from source
- **Desktop**: Users must manually download new `.dmg`/`.deb`/`.AppImage` from GitHub releases

This friction discourages users from staying up-to-date with bug fixes and improvements. We need streamlined update paths for both distribution channels.

## Implementation Steps

### 1. Add CLI Binaries to GitHub Releases

**Files:** `.github/workflows/release-cli.yml` (new)

Create a workflow to build and upload CLI binaries alongside desktop releases:
- Build `jax-daemon` for: macOS (arm64, x64), Linux (x64)
- Upload binaries to GitHub releases with consistent naming:
  - `jax-daemon-{version}-darwin-arm64`
  - `jax-daemon-{version}-darwin-x64`
  - `jax-daemon-{version}-linux-x64`
- Trigger on `jax-daemon-v*` tags (already created by `publish-crate.yml`)

### 2. Create CLI Install Script

**Files:** `install.sh` (new, repo root)

Create a shell script for one-line installation and updates:
```bash
curl -fsSL https://raw.githubusercontent.com/jax-protocol/jax-fs/main/install.sh | sh
```

Script functionality:
- Detect OS and architecture
- Download latest binary from GitHub releases API
- Install to `~/.local/bin` (or `/usr/local/bin` with sudo)
- Add to PATH if needed
- Support `--version` flag to install specific version
- Re-running updates to latest version

### 3. Update Installation Documentation

**Files:** `agents/INSTALL.md`, `README.md`

Document all three CLI installation methods in order of preference:

1. **Install script** (recommended for most users)
   ```bash
   curl -fsSL https://raw.githubusercontent.com/jax-protocol/jax-fs/main/install.sh | sh
   ```

2. **Cargo** (for Rust developers)
   ```bash
   cargo install jax-daemon
   ```

3. **Build from source** (for contributors)
   ```bash
   git clone https://github.com/jax-protocol/jax-fs
   cd jax-fs
   cargo build --release
   ```

### 4. Integrate Tauri Updater for Desktop

**Files:**
- `crates/desktop/src-tauri/Cargo.toml`
- `crates/desktop/src-tauri/tauri.conf.json`
- `crates/desktop/src-tauri/src/lib.rs`
- `crates/desktop/src/` (frontend components)

Add Tauri updater plugin:
- Add `tauri-plugin-updater` dependency
- Configure updater in `tauri.conf.json` with GitHub releases endpoint
- Show notification when update available
- Prompt user to install (not auto-update)
- Handle update download and restart

### 5. Generate Update Manifest in Release Workflow

**Files:** `.github/workflows/release-desktop.yml`

Modify desktop release workflow to generate Tauri update manifest:
- Create `latest.json` with version, release notes, and download URLs
- Upload manifest to GitHub releases or separate endpoint
- Include platform-specific signatures for secure updates

## Files Modified

| File | Changes |
|------|---------|
| `.github/workflows/release-cli.yml` | New workflow for CLI binary releases (incl. FUSE variant) |
| `install.sh` | New install/update script with `--fuse`/`--no-fuse` flags |
| `crates/daemon/src/cli/ops/update.rs` | New `jax update` CLI command |
| `crates/daemon/src/cli/ops/mod.rs` | Register update command |
| `crates/daemon/src/cli/mod.rs` | Export Update |
| `crates/daemon/src/main.rs` | Add Update to command_enum |
| `agents/INSTALL.md` | Document all installation methods incl. FUSE |
| `README.md` | Update quick start with install script |
| `crates/desktop/src-tauri/Cargo.toml` | Add tauri-plugin-updater |
| `crates/desktop/src-tauri/tauri.conf.json` | Configure updater endpoint |
| `crates/desktop/src-tauri/src/lib.rs` | Initialize updater plugin |
| `crates/desktop/src/pages/Settings.tsx` | Add update check UI to Settings |
| `crates/desktop/package.json` | Add plugin-updater, plugin-process deps |
| `.github/workflows/release-desktop.yml` | Add update manifest generation + signing docs |
| `agents/PROJECT_LAYOUT.md` | Document new files |

## Acceptance Criteria

- [x] CLI binaries are published to GitHub releases on tag push
- [x] FUSE variant published for macOS Apple Silicon
- [x] Install script works on macOS (arm64, x64) and Linux (x64)
- [x] Install script supports `--fuse` flag and interactive prompt
- [x] Running install script again updates to latest version
- [x] `jax update` CLI command detects install method, platform, and FUSE
- [x] Desktop app shows notification when update is available
- [x] User can install update from within the app
- [x] INSTALL.md documents all installation methods
- [x] `cargo build`, `cargo test`, `cargo fmt --check` pass
