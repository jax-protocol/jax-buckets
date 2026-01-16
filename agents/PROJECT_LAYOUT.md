# Project Layout

This document describes the structure of the jax-bucket workspace.

## Workspace Structure

The project is a **Cargo workspace** with multiple crates.

```
jax-bucket/
├── crates/
│   ├── app/                    # Main application crate
│   │   ├── src/
│   │   │   ├── cli/            # CLI commands (init, mount, add, sync, etc.)
│   │   │   ├── daemon/         # Long-running daemon
│   │   │   │   ├── http_server/
│   │   │   │   │   ├── api/    # REST API endpoints
│   │   │   │   │   └── gateway/# HTTP gateway for serving bucket content
│   │   │   │   ├── peer/       # P2P networking (iroh)
│   │   │   │   └── sync/       # Bucket synchronization
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   │
│   └── common/                 # Shared library crate
│       ├── src/
│       │   ├── crypto/         # Encryption primitives
│       │   │   ├── keys.rs     # PublicKey, SecretKey
│       │   │   ├── secret.rs   # Bucket encryption Secret
│       │   │   └── secret_share.rs # X25519 secret sharing
│       │   ├── mount/          # Bucket data structures
│       │   │   ├── manifest.rs # Encrypted bucket manifest
│       │   │   ├── mount_inner.rs # Mount operations
│       │   │   ├── principal.rs   # PrincipalRole (Owner, Mirror)
│       │   │   └── node.rs     # File tree nodes
│       │   ├── peer/           # P2P protocol
│       │   │   ├── blobs_store.rs # Content-addressed blob storage
│       │   │   └── protocol/   # Wire protocol messages
│       │   └── lib.rs
│       ├── tests/              # Integration tests
│       └── Cargo.toml
│
├── issues/                     # Issue tracking (epics and tickets)
├── agents/                     # Agent documentation (this folder)
├── Cargo.toml                  # Workspace root
└── Cargo.lock
```

## Crates

### app (jax-bucket binary)

**Location:** `crates/app/`

The main application providing CLI and daemon functionality:

- **CLI** (`src/cli/`): User-facing commands
  - `init` - Create a new bucket
  - `add` - Add files to bucket
  - `ls` - List bucket contents
  - `sync` - Sync with peers
  - `share` - Share bucket with peer
  - `publish` - Publish bucket to mirrors

- **Daemon** (`src/daemon/`): Background services
  - HTTP server with REST API and gateway
  - Peer-to-peer networking via iroh
  - Bucket synchronization

- **HTTP Server** (`src/daemon/http_server/`):
  - `api/v0/bucket/` - REST endpoints for bucket operations
  - `gateway/` - HTTP serving of bucket content with URL rewriting

### common (jax-common library)

**Location:** `crates/common/`

Shared library with core data structures and crypto:

- **Crypto** (`src/crypto/`):
  - `Secret` - AES-GCM encryption key for bucket content
  - `SecretShare` - X25519 encrypted share of bucket secret
  - `SecretKey`/`PublicKey` - Ed25519 identity keys

- **Mount** (`src/mount/`):
  - `Mount` - In-memory bucket with file operations
  - `Manifest` - Encrypted bucket metadata (shares, pins, entry point)
  - `Share` - Principal with optional secret share
  - `PrincipalRole` - Owner (full access) or Mirror (read after publish)
  - `Node` - File tree nodes with content links

- **Peer** (`src/peer/`):
  - `BlobsStore` - Content-addressed storage via iroh-blobs
  - Protocol messages for P2P communication

## Key Concepts

### Content-Addressed Storage

All data is stored as content-addressed blobs using BLAKE3 hashes:
- File content is encrypted and stored as blobs
- Manifests link to blobs via CIDs (Content Identifiers)
- No traditional database - state is reconstructed from blobs

### Encrypted Manifests

Each bucket has an encrypted manifest containing:
- `shares` - Map of public keys to encrypted secret shares
- `pins` - Pinned blob hashes for the bucket
- `entry` - Root of the file tree
- `previous` - Link to previous manifest (for history)

### Principal Roles

Two types of principals can access buckets:
- **Owner**: Full read/write access, always has secret share
- **Mirror**: Can sync data, only decrypts after bucket is published

### P2P Sync

Buckets sync between peers using iroh:
- Peers exchange manifest and blob hashes
- Missing blobs are fetched from available peers
- Conflict-free merging via operation CRDTs

## Build Commands

```bash
# Build all crates
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Run tests for specific crate
cargo test -p jax-common
cargo test -p jax-bucket

# Run clippy
cargo clippy

# Format code
cargo fmt
```

## Adding New Functionality

### New CLI Command

1. Add command module in `crates/app/src/cli/`
2. Register in `crates/app/src/cli/mod.rs`
3. Add to CLI enum in command definition

### New API Endpoint

1. Add handler in `crates/app/src/daemon/http_server/api/v0/bucket/`
2. Register route in `mod.rs`

### New Core Functionality

1. Add to `crates/common/src/` in appropriate module
2. Export in module's `mod.rs`
3. Write unit tests in same file or integration tests in `tests/`
