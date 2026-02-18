# CLI Overhaul: Pretty Output, Consistency, and Op System Documentation

- **Status:** Planned
- **Priority:** Urgent

## Objective

Make the `jax` CLI expressive and consistent. Today the CLI outputs plain unformatted text, has no color, no progress indicators, and inconsistent bucket name/ID handling across commands. This issue covers three goals:

1. Rich, colorful human output with tables, spinners, and styled errors
2. Consistent bucket name-or-ID resolution across all commands
3. Document the op system architecture and the boundary between ops and formatting

The CLI is a human interface. For machine-readable access, consumers should use the daemon's HTTP API directly (`localhost:{api_port}/api/v0/...`), which already returns JSON for every operation. No `--json` flag is needed.

## Current State (Audit)

### Output is plain text everywhere

Every `Op` returns `type Output = String` built with `format!()`. No colors, no icons, no tables (except a hand-rolled fixed-width table in `mount list`). Long operations like `bucket clone` and `bucket add` block silently with no progress feedback.

### Bucket name vs ID is inconsistent

| Accepts name or ID | Accepts ID only |
|---|---|
| `bucket add` | `bucket publish` |
| `bucket ls` | `bucket shares create` |
| `bucket cat` | `bucket shares ls` |
| `bucket clone` | `bucket shares remove` |
| `mount add` | |

Commands that support names use `--bucket-id <UUID>` / `--name <STRING>` as mutually exclusive args and resolve via `client.resolve_bucket_name()`. Commands that only accept IDs force users to look up IDs manually first — a poor experience.

### Error formatting is minimal

All errors go through `eprintln!("Error: {}", e)` with no color, no chain display, and a flat exit code of 1.

### The op system is undocumented

The `Op` trait, `command_enum!` macro, `OpContext`, and the dispatch boundary in `main.rs` form the backbone of the CLI but are not documented anywhere beyond a one-line mention in `agents/PROJECT_LAYOUT.md`. There are no docs explaining:
- What the op system is and why it exists
- How to add a new command
- Where formatting belongs (ops vs boundary)
- The error handling contract (`thiserror` per-op, `Display` at boundary)

## Design

### New dependencies

```toml
owo-colors = { version = "4", features = ["supports-colors"] }  # colors/styling
comfy-table = "7"                                                 # table formatting
indicatif = { version = "0.17", features = ["tokio"] }           # spinners/progress
```

**Why these:**
- `owo-colors` — zero-alloc, no global mutex, proper `NO_COLOR`/`FORCE_COLOR` support. Recommended by Rain's Rust CLI Recommendations over `colored` and `termcolor`.
- `comfy-table` — terminal-width-aware column layout, handles alignment and truncation. Simpler than `tabled` for our needs.
- `indicatif` — de facto standard for progress bars and spinners in async Rust. `Send + Sync`, works with tokio.

No error framework change needed — `thiserror` per-op is already clean. We just add colored error formatting at the boundary.

### Architecture: structured op outputs

Ops stop returning `String` and instead return typed output structs. The `Display` impl on each output struct handles all pretty formatting (colors, tables, layout). `main.rs` remains a thin dispatch boundary.

**Change op output types from `String` to typed structs:**

```rust
// Before:
impl Op for BucketCreate {
    type Output = String;
    async fn execute(&self, ctx: &OpContext) -> Result<String, ...> {
        Ok(format!("Created bucket: {} (id: {}) ...", ...))
    }
}

// After:
pub struct BucketCreateOutput {
    pub name: String,
    pub bucket_id: Uuid,
    pub created_at: OffsetDateTime,
}

impl fmt::Display for BucketCreateOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}\n  {} {}\n  {} {}",
            "Created".green().bold(),
            "bucket".dimmed(),
            self.name.bold(),
            "id:".dimmed(), self.bucket_id,
            "at:".dimmed(), self.created_at)
    }
}
```

**The boundary stays simple:**

```rust
// main.rs — unchanged in structure
match args.command.execute(&ctx).await {
    Ok(output) => {
        println!("{output}");
        std::process::exit(0);
    }
    Err(e) => { /* colored error chain */ }
}
```

**Update `command_enum!` macro** to require `Display` on output types (already required, just needs the bound to remain).

### The op system boundary

This is the key architectural principle to document and enforce:

**Ops are pure.** An op receives an `OpContext` (API client + config path), does its work via HTTP calls to the daemon, and returns a typed `Result<Output, Error>`. Ops never:
- Print to stdout/stderr directly
- Read terminal state (width, color support, TTY detection)
- Show spinners or progress bars
- Make formatting decisions about colors or layout

**Formatting lives in `Display` impls.** Each output struct's `Display` impl owns all presentation logic — colors, tables, layout. This is where `owo-colors` and `comfy-table` are used.

**The boundary (`main.rs`) owns:**
- Parsing args and constructing `OpContext`
- Calling `execute()` and printing via `Display`
- Error formatting (colored chain display to stderr)
- Exit codes

**Progress indicators are the one exception.** Spinners and progress bars must be shown *during* op execution, not after. These are passed into ops via `OpContext` (e.g., an `indicatif::MultiProgress` handle) and suppressed when stdout is not a TTY.

```
┌─────────────┐     ┌──────────────┐     ┌────────────────┐
│  main.rs    │     │  Op::execute │     │  Display impl  │
│  (boundary) │────>│  (pure logic)│────>│  (formatting)  │
│             │     │              │     │                │
│ • parse args│     │ • HTTP calls │     │ • colors       │
│ • build ctx │     │ • returns    │     │ • tables       │
│ • print     │     │   typed data │     │ • layout       │
│ • errors    │     │              │     │                │
└─────────────┘     └──────────────┘     └────────────────┘
```

### Consistent bucket resolution

Every command that operates on a bucket should accept a single positional `<BUCKET>` argument that can be either a name or a UUID. The resolution logic already exists in `client.resolve_bucket_name()` and the pattern in `mount add` (`Uuid::parse_str` first, then resolve by name).

Extract this into a shared helper:

```rust
/// Resolve a bucket identifier (name or UUID) to a UUID.
pub async fn resolve_bucket(client: &ApiClient, identifier: &str) -> Result<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return Ok(uuid);
    }
    client.resolve_bucket_name(identifier).await
}
```

Then convert all commands to use it:

| Command | Before | After |
|---|---|---|
| `bucket publish` | `--bucket-id <UUID>` only | `<BUCKET>` (name or ID) |
| `bucket shares create` | `bucket_id` field only | `<BUCKET>` (name or ID) |
| `bucket shares ls` | `bucket_id` field only | `<BUCKET>` (name or ID) |
| `bucket shares remove` | `bucket_id` field only | `<BUCKET>` (name or ID) |
| `bucket add` | `--bucket-id` / `--name` flags | `<BUCKET>` (name or ID) |
| `bucket ls` | `--bucket-id` / `--name` flags | `<BUCKET>` (name or ID) |
| `bucket cat` | `--bucket-id` / `--name` flags | `<BUCKET>` (name or ID) |
| `bucket clone` | `--bucket-id` / `--name` flags | `<BUCKET>` (name or ID) |

This also simplifies the clap definitions — no more mutually exclusive flag groups.

## Implementation Steps

### Phase 1: Foundation

#### 1. Add dependencies

Add `owo-colors`, `comfy-table`, `indicatif` to `crates/daemon/Cargo.toml`.

#### 2. Extract `resolve_bucket` helper

Create shared bucket resolution function. Update all commands to use a single `<BUCKET>` positional argument.

#### 3. Update `command_enum!` macro

Ensure the macro requires `Display` on output types. No `Serialize` bound needed — the CLI is a human interface, not a data API.

### Phase 2: Convert Commands

#### 4. Convert bucket commands to structured outputs

For each bucket command: define an output struct with `Display`, replace `format!()` string building with struct construction, and add styled formatting in the `Display` impl.

Target output style (examples):

```
$ jax bucket create my-photos
Created bucket my-photos
  id:  a1b2c3d4-...
  at:  2026-02-18 10:30:00 UTC

$ jax bucket list
NAME         ID                                    LINK
my-photos    a1b2c3d4-e5f6-7890-abcd-ef1234567890  bafk...abc
documents    f9e8d7c6-b5a4-3210-fedc-ba0987654321  bafk...def

$ jax bucket ls my-photos
TYPE  NAME           SIZE      HASH
dir   vacation/      -         bafk...123
file  readme.txt     1.2 KB    bafk...456
file  photo.jpg      3.4 MB    bafk...789

$ jax bucket add my-photos ./photos/
Uploading 42 files...
████████████████████████████████████████ 42/42
Uploaded 42 file(s) to my-photos
  link:  bafk...newlink
```

#### 5. Convert mount commands to structured outputs

Same pattern. Replace the hand-rolled table in `mount list` with `comfy-table`. Remove the `mount list --json` flag (use the HTTP API for machine-readable mount data). Add structured outputs to all mount commands.

#### 6. Convert status commands to structured outputs

`health`, `init`, `version` — add structured output types with styled display.

### Phase 3: Polish

#### 7. Add spinners for slow operations

Add `indicatif` spinners to commands that hit the network:
- `bucket add` / `bucket clone` — progress bar with file count
- `bucket publish` — spinner
- `bucket shares create` — spinner
- `mount start` — spinner
- `health` — spinner while probing endpoints

Pass progress handles via `OpContext`. Suppress when stdout is not a TTY.

#### 8. Improve error formatting

Add colored error chain display at the boundary:

```rust
Err(e) => {
    eprintln!("{} {e}", "error:".red().bold());
    let mut source = e.source();
    while let Some(cause) = source {
        eprintln!("  {} {cause}", "caused by:".yellow());
        source = cause.source();
    }
    std::process::exit(1);
}
```

#### 9. Respect NO_COLOR and non-TTY

`owo-colors` handles `NO_COLOR`/`FORCE_COLOR` env vars via `supports-colors`. Ensure we also disable colors when stdout is not a TTY (piping to another command).

### Phase 4: Documentation

#### 10. Document the op system in `agents/CLI.md`

Create `agents/CLI.md` covering:

- **Overview** — what the op system is: a pattern where each CLI command is a struct implementing the `Op` trait, with a `command_enum!` macro that wires subcommands into clap dispatch.
- **Key types** — `Op` trait (`execute` returns `Result<Output, Error>`), `OpContext` (API client + config path), `OpOutput` / `OpError` (generated enums that wrap per-command types).
- **The `command_enum!` macro** — what it generates (the `Command`, `OpOutput`, `OpError` enums and the blanket `Op` impl), how nesting works (bucket subcommands use their own `command_enum!` which feeds into the top-level one).
- **Adding a new command** — step-by-step: create the op struct with `#[derive(Args)]`, define output and error types, implement `Op`, register in the parent `command_enum!`.
- **The formatting boundary** — ops return typed data, `Display` impls own presentation, `main.rs` owns dispatch/errors/exit codes. Why this matters: ops stay testable, formatting is swappable, colors never leak into business logic.
- **Progress indicators** — the exception to "ops don't touch the terminal": passed in via `OpContext`, suppressed for non-TTY.
- **Error contract** — each op defines its own error enum with `thiserror`, errors are `Display`ed at the boundary with colored chain formatting.
- **Bucket resolution** — all bucket commands use `resolve_bucket()` with a single `<BUCKET>` positional arg.

Update `agents/PROJECT_LAYOUT.md` to reference `CLI.md` from the `src/cli/` section.

## Files to Modify

| File | Changes |
|------|---------|
| `crates/daemon/Cargo.toml` | Add `owo-colors`, `comfy-table`, `indicatif` |
| `crates/daemon/src/cli/op.rs` | Update `command_enum!` macro bounds |
| `crates/daemon/src/cli/ops/bucket/create.rs` | Structured output + styled display |
| `crates/daemon/src/cli/ops/bucket/list.rs` | Structured output + `comfy-table` |
| `crates/daemon/src/cli/ops/bucket/add.rs` | Structured output + progress bar + `<BUCKET>` arg |
| `crates/daemon/src/cli/ops/bucket/ls.rs` | Structured output + `comfy-table` + `<BUCKET>` arg |
| `crates/daemon/src/cli/ops/bucket/cat.rs` | Structured output + `<BUCKET>` arg |
| `crates/daemon/src/cli/ops/bucket/clone.rs` | Structured output + progress bar + `<BUCKET>` arg |
| `crates/daemon/src/cli/ops/bucket/publish.rs` | Structured output + spinner + `<BUCKET>` arg |
| `crates/daemon/src/cli/ops/bucket/shares/*.rs` | Structured output + `<BUCKET>` arg |
| `crates/daemon/src/cli/ops/mount/*.rs` | Structured outputs + `comfy-table` for list |
| `crates/daemon/src/cli/ops/mount/list.rs` | Remove `--json` flag |
| `crates/daemon/src/cli/ops/health.rs` | Structured output + spinner |
| `crates/daemon/src/cli/ops/init.rs` | Structured output |
| `crates/daemon/src/cli/ops/version.rs` | Structured output |
| `crates/daemon/src/http_server/api/client/client.rs` | Extract `resolve_bucket` helper |
| `crates/daemon/src/main.rs` | Colored error formatting |
| `agents/CLI.md` | New: op system and formatting boundary documentation |
| `agents/PROJECT_LAYOUT.md` | Reference `CLI.md` from `src/cli/` section |

## Acceptance Criteria

- [ ] All commands produce styled, colored output
- [ ] Every bucket command accepts a `<BUCKET>` arg that works with both names and UUIDs
- [ ] No command requires a raw UUID when a name would suffice
- [ ] Tables (`bucket list`, `bucket ls`, `mount list`) use `comfy-table` with terminal-width-aware layout
- [ ] Long operations (`add`, `clone`, `publish`) show progress indicators
- [ ] Colors are suppressed when `NO_COLOR` is set or stdout is not a TTY
- [ ] Error output shows the full error chain with color
- [ ] `mount list --json` flag is removed (use HTTP API for machine-readable data)
- [ ] `agents/CLI.md` documents the op system, formatting boundary, and how to add commands
- [ ] `agents/PROJECT_LAYOUT.md` references `CLI.md`
- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt --check` passes
