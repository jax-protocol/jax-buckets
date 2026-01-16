# PR Workflow & Parallel Development Guide

This document describes the pull request workflow for jax-bucket, including parallel development approaches with Claude Code agents.

## Overview

We enable parallel development streams, allowing multiple well-scoped tasks to be worked on simultaneously in complete isolation. This approach is effective when working with Claude Code agents on discrete, non-overlapping work items.

**Two approaches are supported:**

1. **Conductor** (recommended) - A Mac app that orchestrates multiple Claude Code agents in isolated workspaces
2. **Git Worktrees** (manual) - Traditional worktree-based isolation for manual setup

---

## Conductor Workflow (Recommended)

[Conductor](https://www.conductor.build/) is a Mac app that automates parallel agent development. It handles workspace isolation, agent orchestration, and provides a unified interface for managing multiple tasks.

### How It Works

1. **Conductor creates isolated workspaces** - Each task runs in its own `.conductor/<workspace-name>` directory
2. **Agents work independently** - Multiple Claude Code agents can run in parallel without conflicts
3. **Unified management** - Monitor all agents, view progress, and manage tasks from one interface

### Working in a Conductor Workspace

When you're in a Conductor workspace (e.g., `/path/to/jax-bucket/.conductor/my-task/`):

```bash
# You're already in an isolated workspace
# Build the project
cargo build

# Run tests
cargo test

# Check for lint warnings
cargo clippy
```

### Conductor-Specific Constraints

- **Work only in your workspace** - Don't read or write files outside `.conductor/<your-workspace>/`
- **All tests must pass** - Run `cargo test` before creating PR

### Creating PRs from Conductor

Conductor workspaces are git worktrees under the hood. Create PRs normally:

```bash
git add .
git commit -m "feat: your changes"
git push -u origin <branch-name>
gh pr create --title "feat: description" --body "..."
```

---

## Git Worktree Workflow (Manual)

For environments without Conductor, or when manual control is needed, use git worktrees directly.

### Create a New Worktree

```bash
# Create worktree with new branch
git worktree add ../jax-bucket-feature feature/my-feature

# Or from existing branch
git worktree add ../jax-bucket-fix fix/my-fix origin/fix/my-fix
```

### Managing Worktrees

```bash
# List active worktrees
git worktree list

# Remove completed worktree
git worktree remove ../jax-bucket-feature
```

---

## Branch Naming Conventions

Follow these conventions for branch names:

- **Features**: `feature/short-description` (e.g., `feature/mirror-publishing`)
- **Bug fixes**: `fix/issue-description` (e.g., `fix/sync-timeout`)
- **Chores**: `chore/task-description` (e.g., `chore/update-deps`)
- **Refactoring**: `refactor/component-name` (e.g., `refactor/mount-structure`)

---

## Development Process

### Initial Setup

```bash
# Navigate to your workspace (Conductor) or worktree (manual)
cd /path/to/workspace

# Build project
cargo build

# Run tests
cargo test

# Check linting
cargo clippy
```

### Working with Claude Code

1. **Plan Mode** - Start by understanding the task and proposing a strategy
2. **Iterate on Strategy** - Refine the approach before execution
3. **Execute** - Work autonomously within defined parameters
4. **Verify** - Ensure success criteria are met

### Success Criteria

**You are not allowed to finish in a state where CI is failing.**

Before considering work complete, ensure these commands pass:

```bash
cargo build      # Must compile
cargo test       # All tests pass
cargo clippy     # No warnings
cargo fmt --check  # Code formatted
```

---

## Creating a Pull Request

### Pre-PR Checklist

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt --check` passes
- [ ] Changes are committed with descriptive messages
- [ ] Branch is pushed to remote

### Create PR with gh CLI

```bash
# Ensure changes are committed
git add .
git commit -m "feat: add mirror principal role

- Implement PrincipalRole::Mirror for read-only peers
- Add publish/unpublish methods to Manifest
- Extend /share endpoint with role parameter

Co-Authored-By: Claude <noreply@anthropic.com>"

# Push to remote
git push -u origin feature/mirror-publishing

# Create PR
gh pr create --title "feat: add mirror principal role" --body "$(cat <<'EOF'
## Summary
- Added PrincipalRole::Mirror for read-only peers
- Mirrors can sync but only decrypt after publish
- Extended /share endpoint with role parameter
- Added /publish endpoint

## Test Plan
- [x] Unit tests for Share and Manifest
- [x] Integration tests for mirror mounting
- [x] `cargo test` passes
- [x] `cargo clippy` clean

Generated with [Claude Code](https://claude.ai/code)
EOF
)"
```

### PR Title Conventions

Use conventional commit prefixes:

- `feat:` - New features
- `fix:` - Bug fixes
- `refactor:` - Code refactoring without functionality changes
- `chore:` - Maintenance tasks, dependency updates
- `docs:` - Documentation changes
- `test:` - Test additions or modifications
- `perf:` - Performance improvements

---

## CI/CD Pipeline

### GitHub Actions Workflow

Our CI pipeline runs automatically on every push and PR:

**Checks that run:**
1. **Build** - `cargo build`
2. **Tests** - `cargo test`
3. **Clippy** - `cargo clippy`
4. **Format** - `cargo fmt --check`

**All checks must pass before merging.**

---

## Code Review Guidelines

### What Reviewers Should Check

1. **Functionality** - Does the code solve the stated problem?
2. **Tests** - Are there appropriate tests? Do they pass?
3. **Error Handling** - Are errors handled appropriately?
4. **Security** - Especially for crypto code, are there any vulnerabilities?
5. **Patterns** - Does it follow existing patterns in the codebase?

### What Reviewers Can Skip

- **Formatting** - `cargo fmt` enforces this
- **Basic linting** - `cargo clippy` catches this

---

## Merge Strategy

1. **Squash merge** - Combine commits into one clean commit
2. **Use conventional commit** - PR title becomes commit message
3. **Delete branch after merge** - Keep the repo clean
