# Contributing Guide

This guide covers how to contribute to jax-bucket, whether you're an AI agent or a human developer.

## For AI Agents

### Getting Started

1. **Run `cargo build`** - Verify the project compiles
2. **Read the relevant docs** - Start with [PROJECT_LAYOUT.md](./PROJECT_LAYOUT.md) and [RUST_PATTERNS.md](./RUST_PATTERNS.md)
3. **Understand the task** - Use planning mode to analyze requirements before coding
4. **Follow existing patterns** - Match the style and structure of existing code

### Key Constraints

- **Work only in your workspace** - Don't access files outside your Conductor workspace
- **All tests must pass** - Run `cargo test` before submitting
- **Clippy must be clean** - Run `cargo clippy` and fix all warnings
- **Follow Rust idioms** - Use `?` for error propagation, prefer iterators over loops

### Code Quality Expectations

- Follow [RUST_PATTERNS.md](./RUST_PATTERNS.md) for Rust code
- Use `thiserror` for error types, not `anyhow` in library code
- Write tests for new functionality
- Keep functions focused - single responsibility
- Document public APIs with rustdoc comments

### File Naming Conventions

- Use `snake_case` for all file names (standard Rust convention)
- Example: `mount_inner.rs`, `secret_share.rs`, `blobs_store.rs`
- Module files use `mod.rs` or the module name directly

### Naming Philosophy

**Prefer descriptive names over short ones.** Clarity is more important than brevity.

- Function/file names should describe what they do
- Don't abbreviate unless universally understood
- Type names should be nouns, function names should be verbs

**Examples:**
```rust
// Good - descriptive
pub async fn add_principal(&mut self, peer: PublicKey, role: PrincipalRole)
pub fn is_published(&self) -> bool
pub struct MirrorCannotMount;

// Bad - too short or ambiguous
pub async fn add(&mut self, p: PublicKey, r: PrincipalRole)
pub fn published(&self) -> bool
pub struct MirrorError;
```

### Before Submitting

1. Run `cargo build` - Must compile without errors
2. Run `cargo test` - All tests must pass
3. Run `cargo clippy` - Fix all warnings
4. Run `cargo fmt` - Code must be formatted
5. Write descriptive commit messages
6. Create PR with clear summary

---

## For Human Developers

### Development Setup

1. **Clone the repository**
   ```bash
   git clone git@github.com:jax-protocol/jax-buckets.git
   cd jax-buckets
   ```

2. **Install Rust** (if not already installed)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Build the project**
   ```bash
   cargo build
   ```

4. **Run tests**
   ```bash
   cargo test
   ```

### Working with Conductor

If you're using [Conductor](https://www.conductor.build/) for parallel agent development:

1. Workspaces are created in `.conductor/<workspace-name>/`
2. Each workspace is an isolated git worktree
3. See [PR_WORKFLOW.md](./PR_WORKFLOW.md) for details

### Code Review Guidelines

When reviewing PRs:

**Do check:**
- Does the code solve the stated problem?
- Are there appropriate tests?
- Does it follow existing patterns?
- Is the error handling appropriate?
- Are there security concerns (especially around crypto)?

**Don't worry about:**
- Formatting - `cargo fmt` enforces this
- Simple linting - `cargo clippy` catches this

### Architecture Decisions

Before making significant changes:

1. **Discuss first** - Open an issue or discuss in PR
2. **Document the decision** - Update relevant docs or create an issue
3. **Follow established patterns** - Or document why you're deviating

Key architectural principles:
- Content-addressed storage - all data is blobs
- Encrypted by default - all bucket content is encrypted
- P2P first - sync between peers without central server
- Principal-based access control - Owners and Mirrors

---

## Commit Conventions

Use conventional commit prefixes:

| Prefix | Use For |
|--------|---------|
| `feat:` | New features |
| `fix:` | Bug fixes |
| `refactor:` | Code refactoring (no behavior change) |
| `chore:` | Maintenance tasks, dependency updates |
| `docs:` | Documentation changes |
| `test:` | Test additions or modifications |
| `perf:` | Performance improvements |

Example:
```
feat: add mirror principal role and bucket publishing workflow

- Implement PrincipalRole::Mirror for read-only peers
- Add publish/unpublish methods to Manifest
- Extend /share endpoint with role parameter
- Add integration tests for mirror mounting

Co-Authored-By: Claude <noreply@anthropic.com>
```

---

## Pull Request Process

1. **Create a branch** - Use descriptive names (e.g., `feature/mirror-publishing`)
2. **Make changes** - Follow patterns, write tests
3. **Run checks** - `cargo build && cargo test && cargo clippy`
4. **Push and create PR** - Use descriptive title and summary
5. **Wait for CI** - All checks must pass
6. **Address feedback** - Respond to review comments
7. **Merge** - Squash merge to main

See [PR_WORKFLOW.md](./PR_WORKFLOW.md) for detailed instructions.

---

## Getting Help

- **Documentation issues** - Update the relevant doc and submit a PR
- **Bug reports** - Open a GitHub issue with reproduction steps
- **Feature requests** - Open a GitHub issue with use case description
