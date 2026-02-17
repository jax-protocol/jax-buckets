# Make FUSE an Optional Build Feature

**Status:** Planned

## Objective

Make FUSE support a compile-time feature flag rather than a hard dependency, allowing builds without macFUSE/libfuse installed.

## Background

Currently FUSE is a required dependency which causes several problems:
- macFUSE must be installed to build `jax-desktop`
- Cross-compiling for Intel on ARM runners fails (pkg-config doesn't support cross-compilation for FUSE)
- Users who don't need mount functionality still need to install macFUSE
- CI builds require FUSE installation on all platforms

## Approach

Add a `fuse` feature flag that gates all FUSE-related code behind conditional compilation.

## Implementation Steps

### 1. Add feature flag to jax-common

**Modify:** `crates/common/Cargo.toml`

```toml
[features]
default = []
fuse = ["fuser"]

[dependencies]
fuser = { version = "0.15", optional = true }
```

### 2. Gate FUSE code with cfg

**Modify:** Mount-related modules

```rust
#[cfg(feature = "fuse")]
mod fuse_mount;

#[cfg(feature = "fuse")]
pub use fuse_mount::FuseMount;
```

### 3. Update daemon feature flags

**Modify:** `crates/daemon/Cargo.toml`

```toml
[features]
default = ["fuse"]
fuse = ["common/fuse"]
```

### 4. Update desktop app

**Modify:** `crates/desktop/src-tauri/Cargo.toml`

Enable `fuse` feature only when building with FUSE support:

```toml
[features]
default = ["fuse"]
fuse = ["jax-common/fuse"]
```

### 5. Update CI workflows

**Modify:** `.github/workflows/release-desktop.yml`

- Default builds include FUSE (for full-featured releases)
- Add option for FUSE-less builds for cross-compilation

## Acceptance Criteria

- [ ] `cargo build` works without macFUSE installed (without `fuse` feature)
- [ ] `cargo build --features fuse` requires macFUSE and enables mount support
- [ ] Desktop app builds with FUSE by default
- [ ] Cross-compilation works when FUSE feature is disabled
- [ ] Desktop app degrades gracefully when FUSE unavailable (grayed out mount button, helpful error)
