# Bundle CLI Binary with Desktop Release

**Status:** Planned

## Objective

Include the `jax` CLI binary in the desktop app GitHub release so users get both the GUI and CLI from a single download.

## Background

The desktop app (`jax-desktop`) embeds the daemon library in-process but does not include the `jax` CLI binary. Currently users would need `cargo install` or a separate build to get CLI access. Bundling the CLI with the desktop release eliminates this friction.

## Approach

Use the existing `release-desktop.yml` workflow (triggered by `jax-desktop-v*` tags) to also build the `jax` CLI binary and attach it to the same GitHub release.

## Implementation Steps

### 1. Build CLI binary in release workflow

**Modify:** `.github/workflows/release-desktop.yml`

After the Tauri build step, add a step to build the CLI:

```yaml
- name: Build CLI binary
  run: cargo build --release -p jax-daemon --bin jax

- name: Package CLI binary
  run: |
    # macOS
    tar -czf jax-cli-${{ matrix.target }}.tar.gz -C target/release jax
    # Windows would be .zip with jax.exe
```

### 2. Attach CLI binary to GitHub release

Add the CLI archive as an additional release asset alongside the Tauri bundles (`.dmg`, `.msi`, `.AppImage`).

### 3. Update release notes template

Include installation instructions for both:
- Desktop app: download and install `.dmg` / `.msi` / `.AppImage`
- CLI only: download `jax-cli-<target>.tar.gz`, extract, add to PATH

## Acceptance Criteria

- [ ] `jax` CLI binary built for all three platforms (macOS, Windows, Linux)
- [ ] CLI binary attached to GitHub release as `.tar.gz` (macOS/Linux) or `.zip` (Windows)
- [ ] Release includes both Tauri bundles and CLI binaries
- [ ] CLI binary works standalone (no desktop app required)
