# Agent Documentation

This directory contains documentation designed for AI agents (and human developers) working on jax-bucket.

## Quick Start

1. Read [TASK_TEMPLATE.md](./TASK_TEMPLATE.md) to understand how to start a task
2. Run `cargo build` to ensure dependencies compile
3. Follow the patterns in [RUST_PATTERNS.md](./RUST_PATTERNS.md)
4. Ensure [SUCCESS_CRITERIA.md](./SUCCESS_CRITERIA.md) are met before creating a PR

---

## Document Index

| Document | Purpose | When to Read |
|----------|---------|--------------|
| [CONTRIBUTING.md](./CONTRIBUTING.md) | How to contribute (agents & humans) | First time contributing |
| [TASK_TEMPLATE.md](./TASK_TEMPLATE.md) | Task template for starting work | Beginning of every task |
| [PROJECT_LAYOUT.md](./PROJECT_LAYOUT.md) | Crate structure and packages | Understanding the codebase |
| [STORAGE.md](./STORAGE.md) | Content-addressed blob storage | Working with data persistence |
| [RUST_PATTERNS.md](./RUST_PATTERNS.md) | Rust architecture patterns | Writing Rust code |
| [SUCCESS_CRITERIA.md](./SUCCESS_CRITERIA.md) | CI requirements and checks | Before creating a PR |
| [PR_WORKFLOW.md](./PR_WORKFLOW.md) | Git, branching, and PR conventions | Creating PRs |
| [ISSUES.md](./ISSUES.md) | Issue and ticket conventions | Planning work |

---

## Document Summaries

### [CONTRIBUTING.md](./CONTRIBUTING.md)
How to contribute to the project:
- **For AI agents**: Constraints, code quality expectations, submission checklist
- **For humans**: Dev setup, code review guidelines, architecture decisions
- Commit conventions and PR process

### [TASK_TEMPLATE.md](./TASK_TEMPLATE.md)
Template for starting a new task. Copy this into your Claude Code conversation and fill in the mission section.

### [PROJECT_LAYOUT.md](./PROJECT_LAYOUT.md)
Describes the workspace crate structure:
- **app crate**: CLI and daemon (`jax-bucket` binary)
- **common crate**: Crypto, mount, peer protocol, blob storage
- **Key concepts**: Content-addressed storage, encrypted manifests, P2P sync

### [STORAGE.md](./STORAGE.md)
Data persistence and content-addressed blob storage:
- Iroh blobs for content storage
- Encrypted manifests with secret sharing
- No traditional database - all data is in blobs

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

### [PR_WORKFLOW.md](./PR_WORKFLOW.md)
Git and PR conventions:
- Branch naming
- Commit conventions
- PR templates and CI pipeline

### [ISSUES.md](./ISSUES.md)
How issues are tracked:
- Epics for large features
- Tickets for discrete tasks
- Status tracking

---

## Key Constraints

1. **Run `cargo build` first** - Verify compilation
2. **All tests must pass** - `cargo test`
3. **Clippy must be clean** - `cargo clippy`
4. **Follow existing patterns** - Match codebase style

---

## External Resources

- [Iroh Documentation](https://iroh.computer/docs) - P2P networking and blobs
- [IPLD](https://ipld.io/) - Linked data format
