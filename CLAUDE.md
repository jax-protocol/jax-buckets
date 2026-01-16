# Claude Code Instructions for jax-bucket

## Project Overview

jax-bucket is a P2P encrypted storage system built in Rust. It uses content-addressed blob storage via iroh-blobs, AES-GCM encryption, and X25519 secret sharing for access control.

## Quick Start

```bash
cargo build      # Build all crates
cargo test       # Run all tests
cargo clippy     # Check for lint warnings
cargo fmt        # Format code
```

## Project Structure

- `crates/app/` - Main binary (`jax-bucket`) with CLI and daemon
- `crates/common/` - Shared library (`jax-common`) with crypto, mount, and peer modules
- `agents/` - Agent documentation (read these first)
- `issues/` - Issue tracking (epics and tickets)

## Key Documentation

Before starting work, read the relevant docs in `agents/`:

- `PROJECT_LAYOUT.md` - Crate structure and modules
- `RUST_PATTERNS.md` - Error handling, async patterns, serialization
- `STORAGE.md` - Content-addressed storage and encryption
- `SUCCESS_CRITERIA.md` - CI requirements (must pass before PR)
- `PR_WORKFLOW.md` - Git and PR conventions

## Constraints

1. **All CI checks must pass** before creating a PR:
   - `cargo build` - Must compile
   - `cargo test` - All tests pass
   - `cargo clippy` - No warnings
   - `cargo fmt --check` - Code formatted

2. **Follow existing patterns** - Match the style of existing code

3. **Use thiserror for errors** - Not anyhow in library code

4. **Write tests** - Unit tests in `#[cfg(test)]` modules, integration tests in `tests/`

## Key Concepts

- **PrincipalRole**: Owner (full access) or Mirror (read after publish)
- **Share**: Principal with optional SecretShare (None for unpublished mirrors)
- **Manifest**: Encrypted bucket metadata linking to content blobs
- **BlobsStore**: Content-addressed storage via iroh-blobs

## Common Tasks

### Adding an API endpoint

1. Create handler in `crates/app/src/daemon/http_server/api/v0/bucket/`
2. Register route in `mod.rs`

### Adding core functionality

1. Add to `crates/common/src/` in appropriate module
2. Export in module's `mod.rs`
3. Write unit tests in same file

### Running specific tests

```bash
cargo test -p jax-common              # Common crate only
cargo test -p jax-bucket              # App crate only
cargo test test_mirror_can_mount      # Specific test
cargo test --test mount_tests         # Integration tests
```

## Do Not

- Push to main directly - create a PR
- Skip clippy warnings - fix them
- Add debug code (println!, dbg!) to commits
- Create documentation files unless explicitly asked
