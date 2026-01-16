# Task Template

Copy and customize this template for each well-scoped ticket. This is what you paste into your Claude Code conversation to start a task.

---

## How to Use

1. Copy the template below into your Claude Code conversation
2. Fill in the "Your Mission" section with your specific task
3. Start in planning mode - let Claude Code analyze and propose a strategy
4. Execute - once the plan is approved, work autonomously
5. Verify - ensure [success criteria](./SUCCESS_CRITERIA.md) are met before creating PR

---

# Template

```markdown
# Goal

Complete the following well-scoped ticket within your branch.

**During planning phase**: Collect all information needed to clarify ambiguous requirements.

**Once executing**: Complete tasks without further intervention. You have leeway to accomplish tasks as you see fit.

**Once satisfied**: Push changes and open a descriptive PR for review.

## References

- [Project Layout](./agents/PROJECT_LAYOUT.md) - Crate structure
- [Rust Patterns](./agents/RUST_PATTERNS.md) - Architecture patterns
- [Storage](./agents/STORAGE.md) - Blob storage and encryption
- [Success Criteria](./agents/SUCCESS_CRITERIA.md) - CI requirements
- [PR Workflow](./agents/PR_WORKFLOW.md) - Git and PR conventions
- [Issues](./agents/ISSUES.md) - Issue tracking conventions

## Constraints

- Run `cargo build` at the start to verify compilation
- Ensure `cargo test` passes before creating PR
- Ensure `cargo clippy` has no warnings
- Follow existing patterns in the codebase

---

# Your Mission

[Describe your task here. Be specific about:
- What needs to be built or fixed
- Any relevant context or background
- Expected behavior or acceptance criteria
- Any constraints or requirements
- Files or areas of the codebase to focus on]
```

---

## Example Mission

```markdown
# Your Mission

## Ticket: Add Unpublish Endpoint

### Description
Implement the ability to unpublish a bucket, revoking decryption access from all mirror peers.

### Requirements
- Add an unpublish endpoint to the REST API (`POST /bucket/{bucket_id}/unpublish`)
- Revoke secret shares from all mirrors (set to None)
- Return confirmation with count of affected mirrors

### Acceptance Criteria
- Mirrors lose decryption access after unpublish
- Owners retain full access
- Mirrors can sync but cannot decrypt
- All tests pass
- `cargo clippy` has no warnings

### Files to Consider
- API: `crates/app/src/daemon/http_server/api/v0/bucket/`
- Mount: `crates/common/src/mount/mount_inner.rs`
- Manifest: `crates/common/src/mount/manifest.rs`
```

---

## Tips for Success

1. **Start with planning** - Use planning mode to break down the task
2. **Read relevant code first** - Understand existing patterns before making changes
3. **Run `cargo build`** at the start - Catch compilation issues early
4. **Test incrementally** - Run tests as you go, not just at the end
5. **Follow existing patterns** - Match the style and structure of existing code
6. **Keep it focused** - Stick to the defined scope, avoid scope creep
7. **Verify all checks** - Run `cargo build && cargo test && cargo clippy` before PR
