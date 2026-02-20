# CLI Architecture: The Op Pattern and the Formatting Boundary

## The Problem

CLI tools tend to accumulate formatting logic inside command handlers. A command parses args, does work, builds a string with `format!()`, and prints it. This works until you want to change how output looks — then you're editing business logic to fix presentation. Colors, tables, progress bars, and error formatting all end up tangled with the actual operation.

The solution is a clean split: **ops do work and return data, formatting is a separate concern at the boundary.**

## The Op Pattern

Every CLI command is a struct that implements the `Op` trait:

```rust
#[async_trait::async_trait]
pub trait Op: Send + Sync {
    type Error: Error + Send + Sync + 'static;
    type Output;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error>;
}
```

An op receives an `OpContext` (an API client and optional config path), does its work, and returns typed data or a typed error. That's it. The op never prints, never colors, never touches the terminal.

### OpContext

```rust
#[derive(Clone)]
pub struct OpContext {
    pub client: ApiClient,
    pub config_path: Option<PathBuf>,
    pub progress: indicatif::MultiProgress,
}
```

The context is constructed once in `main()` and passed to every op. It contains everything an op needs to talk to the daemon. The remote URL is resolved from: explicit `--remote` flag > config file `api_port` > hardcoded default. The `MultiProgress` handle is hidden when stdout is not a TTY.

### Anatomy of an Op

Here's the full pattern for a single command. Every op file follows this structure:

```rust
// 1. The command struct — clap parses args into this
#[derive(Args, Debug, Clone)]
pub struct MyCommand {
    /// The bucket to operate on (name or UUID)
    pub bucket: String,

    /// Some flag
    #[arg(long)]
    pub verbose: bool,
}

// 2. The error enum — one variant per failure mode
#[derive(Debug, thiserror::Error)]
pub enum MyCommandError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("not found: {0}")]
    NotFound(String),
}

// 3. The Op impl — does work, returns data
#[async_trait::async_trait]
impl Op for MyCommand {
    type Error = MyCommandError;
    type Output = MyCommandOutput;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let bucket_id = resolve_bucket(&mut client, &self.bucket).await?;
        let response = client.call(SomeRequest { bucket_id }).await?;
        Ok(MyCommandOutput { name: response.name, bucket_id })
    }
}
```

### The `command_enum!` Macro

Clap needs a single enum of all subcommands. Writing the dispatch by hand for every command is boilerplate. The `command_enum!` macro generates it:

```rust
command_enum! {
    (Create, CreateRequest),
    (List, ListRequest),
    (Add, add::Add),
    (Ls, ls::Ls),
}
```

This expands to:

1. **`Command`** — a clap `#[derive(Subcommand)]` enum with a variant per op
2. **`OpOutput`** — an enum wrapping each op's `Output` type
3. **`OpError`** — an enum wrapping each op's `Error` type (with `#[error(transparent)]`)
4. **Blanket `Op` impl for `Command`** — dispatches `execute()` to the right variant
5. **`Display` impl for `OpOutput`** — delegates to each variant's `Display`

This means adding a new command is: write the struct + error + `Op` impl, then add one line to the `command_enum!`.

### Nesting

Subcommand groups (like `bucket shares create`) use nested `command_enum!` invocations. The `bucket` module has its own `command_enum!` generating a `BucketCommand`, then wraps it:

```rust
// bucket/mod.rs
command_enum! {
    (Create, CreateRequest),
    (List, ListRequest),
    (Add, add::Add),
    // ...
}

pub type BucketCommand = Command;

#[derive(Args, Debug, Clone)]
pub struct Bucket {
    #[command(subcommand)]
    pub command: BucketCommand,
}

impl Op for Bucket {
    type Error = OpError;
    type Output = OpOutput;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        self.command.execute(ctx).await
    }
}
```

The top-level `command_enum!` in `main.rs` then includes `(Bucket, Bucket)` alongside other top-level commands like `(Health, Health)`.

## The Formatting Boundary

The boundary is `main.rs`. It is deliberately thin:

```rust
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let remote = resolve_remote(args.remote, args.config_path.clone());
    let ctx = OpContext::new(remote, args.config_path).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    match args.command.execute(&ctx).await {
        Ok(output) => {
            println!("{output}");   // <-- Display impl does all formatting
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
```

All presentation logic lives in `Display` impls on the output types. The boundary just calls `println!("{output}")`.

### Where Each Concern Lives

```
┌─────────────────┐     ┌───────────────┐     ┌──────────────────┐
│   main.rs       │     │ Op::execute   │     │ Display impl     │
│   (boundary)    │────>│ (pure logic)  │────>│ (presentation)   │
│                 │     │               │     │                  │
│ • parse args    │     │ • HTTP calls  │     │ • colors         │
│ • build context │     │ • validation  │     │ • tables         │
│ • println!()    │     │ • returns     │     │ • layout         │
│ • error chain   │     │   typed data  │     │ • human-readable │
│ • exit codes    │     │               │     │   formatting     │
└─────────────────┘     └───────────────┘     └──────────────────┘
```

**Ops never:**
- Print to stdout/stderr
- Use color or styling
- Check terminal width or TTY status
- Show spinners or progress bars (see exception below)

**Display impls own:**
- All color and styling (via `owo-colors`)
- Table layout (via `comfy-table`)
- Human-readable formatting of UUIDs, timestamps, byte sizes, hashes

**The boundary owns:**
- Arg parsing and context construction
- Calling `execute()` and printing via `Display`
- Error formatting (colored error chain to stderr)
- Exit codes (0 success, 1 error)

### The Progress Exception

Spinners and progress bars are the one thing that must happen *during* execution, not after. The approach: pass an `indicatif::MultiProgress` handle through `OpContext`. Ops that need progress create bars on it. When stdout is not a TTY, the handle is a no-op (indicatif supports this natively with `ProgressDrawTarget::hidden()`).

This is a controlled leak — the op knows it *might* show progress, but doesn't decide *how* or *whether* to render it. The boundary decides that when it constructs the context.

## Bucket Resolution

Every command that operates on a bucket accepts a single positional `<BUCKET>` argument that can be either a name or a UUID. The `resolve_bucket()` helper in `client.rs` handles this:

```rust
pub async fn resolve_bucket(client: &mut ApiClient, identifier: &str) -> Result<Uuid, ApiError> {
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return Ok(uuid);
    }
    client.resolve_bucket_name(identifier).await
}
```

Commands that previously used `--bucket-id`/`--name` flag groups or only accepted UUIDs now use this pattern:

```rust
pub struct MyCommand {
    /// Bucket name or UUID
    pub bucket: String,
    // ...
}

async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
    let mut client = ctx.client.clone();
    let bucket_id = resolve_bucket(&mut client, &self.bucket).await?;
    // ...
}
```

## Typed Outputs with Styled Display

Every op returns a typed output struct with a `Display` impl. The `Display` impl owns all presentation logic — colors, tables, layout. Example:

```rust
#[derive(Debug)]
pub struct CreateOutput {
    pub name: String,
    pub bucket_id: Uuid,
    pub created_at: OffsetDateTime,
}

impl fmt::Display for CreateOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} bucket {}",
            "Created".green().bold(),
            self.name.bold())?;
        writeln!(f, "  {} {}", "id:".dimmed(), self.bucket_id)?;
        write!(f, "  {} {}", "at:".dimmed(), self.created_at)
    }
}

impl Op for Create {
    type Output = CreateOutput;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let request = CreateRequest { name: self.name.clone() };
        let response = client.call(request).await?;
        Ok(CreateOutput {
            name: response.name,
            bucket_id: response.bucket_id,
            created_at: response.created_at,
        })
    }
}
```

The op just returns the data it already had. Formatting lives in a separate `Display` impl where it can use colors, alignment, and structure without cluttering the business logic.

**Color conventions:**
- Action words (Created, Uploaded, Cloned, etc.): `.green().bold()`
- Labels (id:, at:, link:): `.dimmed()`
- Names/values: `.bold()` for primary identifiers
- Failure states: `.red()` or `.red().bold()`
- OK status: `.green()`

### Tables

For list-style output, `Display` builds a `comfy-table::Table`:

```rust
impl fmt::Display for BucketListOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut table = Table::new();
        table.set_header(vec!["NAME", "ID", "LINK"]);
        for b in &self.buckets {
            table.add_row(vec![&b.name, &b.id.to_string(), &b.link_hash]);
        }
        write!(f, "{table}")
    }
}
```

`comfy-table` auto-detects terminal width and adjusts column sizes. No manual padding math.

### Error Chains

Errors at the boundary get colored chain display:

```rust
Err(e) => {
    eprintln!("{} {e}", "error:".red().bold());
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        eprintln!("  {} {cause}", "caused by:".yellow());
        source = cause.source();
    }
    std::process::exit(1);
}
```

Individual ops keep using `thiserror` — nothing changes about how errors are defined or propagated.

### NO_COLOR and TTY Detection

`owo-colors` with the `supports-colors` feature respects the `NO_COLOR` env var and terminal capability detection. Colors are automatically suppressed when:
- `NO_COLOR` is set (any value)
- stdout is not a TTY (piped to another command)
- `TERM=dumb`

This requires using `if_supports_color()` or the `Stream` parameter on color methods. No special handling in ops — this is purely a `Display` impl concern.

## No `--json` Flag

The CLI is a human interface. For machine-readable output, use the daemon's HTTP API directly (`localhost:{api_port}/api/v0/...`) — it returns JSON for every operation. Duplicating that in the CLI adds complexity (Serialize bounds on every output type, format dispatch at the boundary, suppressing colors/spinners) for marginal benefit.

If this is ever needed, the typed output structs make it trivial to add: slap `#[derive(Serialize)]` on them and add a format branch at the boundary. But start without it.

## Adding a New Command

1. Create `crates/daemon/src/cli/ops/<group>/<name>.rs`
2. Define the arg struct with `#[derive(Args)]`
3. Define the output struct with a `Display` impl
4. Define the error enum with `#[derive(thiserror::Error)]`
5. Implement `Op` — return the output struct, never print
6. Add the variant to the parent `command_enum!`

That's it. The macro handles all dispatch wiring.

## Reference Projects

Other Rust CLIs using variants of the execution/formatting split:

- **cargo** — commands return structured results, formatting happens in the `shell` module. Progress via `indicatif`. Colors via `anstyle`.
- **ripgrep** — the `Printer` trait separates search execution from output formatting. Different printers handle text, JSON, and summary modes.
- **eza** — file metadata collection is separate from grid/table/long-format rendering. Colors via `nu-ansi-term`.
- **rustup** — operations return results, the `term2` module handles all terminal interaction.

The common thread: commands are pure functions from input to structured output. A separate layer decides how to render that output for humans.
