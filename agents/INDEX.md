# Agent Documentation

This directory contains documentation designed for AI agents (and human developers) working on jax-bucket.

## Quick Start

1. Run `cargo build` to ensure dependencies compile
2. Follow the patterns in [RUST_PATTERNS.md](./RUST_PATTERNS.md)
3. Ensure [SUCCESS_CRITERIA.md](./SUCCESS_CRITERIA.md) are met before creating a PR

---

## Document Index

| Document | Purpose | When to Read |
|----------|---------|--------------|
| [concepts/](./concepts/) | Architecture and design concepts | Understanding the system |
| [INSTALL.md](./INSTALL.md) | Installation and setup guide | Setting up JaxBucket |
| [DEVELOPMENT.md](./DEVELOPMENT.md) | Development environment setup | Contributing code |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | How to contribute (agents & humans) | First time contributing |
| [PROJECT_LAYOUT.md](./PROJECT_LAYOUT.md) | Crate structure and packages | Understanding the codebase |
| [RUST_PATTERNS.md](./RUST_PATTERNS.md) | Rust architecture patterns | Writing Rust code |
| [SUCCESS_CRITERIA.md](./SUCCESS_CRITERIA.md) | CI requirements and checks | Before creating a PR |
| [RELEASE.md](./RELEASE.md) | Release process and automation | Publishing crates |
| [ISSUES.md](./ISSUES.md) | Issue and ticket conventions | Planning work |

---

## Document Summaries

### [concepts/](./concepts/)
Detailed architecture documentation:
- [overview.md](./concepts/overview.md) - High-level architecture and core concepts
- [data-model.md](./concepts/data-model.md) - Buckets, Manifests, Nodes, Pins, Bucket Log
- [cryptography.md](./concepts/cryptography.md) - Identity, key sharing, content encryption
- [synchronization.md](./concepts/synchronization.md) - Peer structure and sync protocol
- [security.md](./concepts/security.md) - Threat model, best practices, implementation

### [INSTALL.md](./INSTALL.md)
Installation and setup:
- System requirements (OS, libraries, hardware)
- Installation methods (crates.io, git, source)
- Initial configuration and daemon setup
- Running as a background service

### [DEVELOPMENT.md](./DEVELOPMENT.md)
Development environment:
- Prerequisites and tool installation
- Two-node P2P development setup with tmux
- Testing and debugging techniques
- Common development tasks

### [CONTRIBUTING.md](./CONTRIBUTING.md)
How to contribute to the project:
- **For AI agents**: Constraints, code quality expectations, submission checklist
- **For humans**: Dev setup, worktrees for parallel development
- Commit conventions (tied to release automation)

### [PROJECT_LAYOUT.md](./PROJECT_LAYOUT.md)
Exhaustive tree of project files with crate descriptions.

### [RUST_PATTERNS.md](./RUST_PATTERNS.md)
Architecture patterns for Rust code:
- Error handling with thiserror
- Async patterns with tokio
- Serialization with serde and IPLD DAG-CBOR

### [SUCCESS_CRITERIA.md](./SUCCESS_CRITERIA.md)
What "done" means:
- `cargo build` must succeed
- `cargo test` must pass
- `cargo clippy` must be clean
- No compiler warnings

### [RELEASE.md](./RELEASE.md)
Release process and crate publishing:
- Conventional commits drive semver bumps
- Automated release PRs via `release-pr.yml`
- Publishing to crates.io via `publish-crate.yml`
- Manual release with `cargo-smart-release`

### [ISSUES.md](./ISSUES.md)
How issues are organized in `issues/`:
- Standalone issues as single files
- Epics as directories with `index.md`
- Tickets as 0-indexed files (`0-first.md`, `1-second.md`)

---

## Key Constraints

1. **Run `cargo build` first** - Verify compilation
2. **All tests must pass** - `cargo test`
3. **Clippy must be clean** - `cargo clippy`
4. **Follow existing patterns** - Match codebase style

---

## Crate-Specific Documentation

Highly targeted agent docs live within each crate's `agents/` subdirectory:

```
crates/
├── app/agents/           # App-specific docs (CLI, daemon, templates)
└── common/agents/        # Common crate docs (if needed)
```

This keeps specialized documentation close to the code it describes.

---

## External Resources

- [Iroh Documentation](https://iroh.computer/docs) - P2P networking and blobs
- [IPLD](https://ipld.io/) - Linked data format
