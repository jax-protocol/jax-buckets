# Usage

This document covers the jax-bucket binary, CLI commands, daemon mode, and available features.

## Binary

jax-bucket provides a single binary that operates in two modes:

1. **CLI mode**: Execute commands and exit
2. **Daemon mode**: Run as a background service with HTTP API

```bash
# Build the binary
cargo build --release

# The binary is at target/release/jax-bucket
```

## Global Options

```bash
jax-bucket [OPTIONS] <COMMAND>

Options:
  --remote <URL>       API endpoint (default: http://localhost:3000)
  --config-path <PATH> Config directory (default: ~/.jax)
```

## Commands

### init

Initialize a new jax configuration directory.

```bash
jax-bucket init
```

Creates:
- `~/.jax/` directory
- Identity keypair
- Local database

### daemon

Start the background service.

```bash
jax-bucket daemon
```

The daemon:
- Starts the HTTP API server (default port 3000)
- Initializes P2P networking
- Begins syncing with known peers
- Serves the web UI

### version

Display version information.

```bash
jax-bucket version
```

## Bucket Commands

All bucket operations are under the `bucket` subcommand:

```bash
jax-bucket bucket <SUBCOMMAND>
```

### create

Create a new bucket.

```bash
jax-bucket bucket create <NAME>

# Example
jax-bucket bucket create my-documents
```

### list

List all buckets.

```bash
jax-bucket bucket list
```

### add

Add a file to a bucket.

```bash
jax-bucket bucket add <BUCKET_ID> <SOURCE_PATH> [DEST_PATH]

# Examples
jax-bucket bucket add abc123 ./photo.jpg              # Adds as /photo.jpg
jax-bucket bucket add abc123 ./photo.jpg /images/     # Adds as /images/photo.jpg
```

### ls

List contents of a bucket directory.

```bash
jax-bucket bucket ls <BUCKET_ID> [PATH]

# Examples
jax-bucket bucket ls abc123           # List root
jax-bucket bucket ls abc123 /images   # List /images directory
```

### cat

Read file contents.

```bash
jax-bucket bucket cat <BUCKET_ID> <PATH>

# Example
jax-bucket bucket cat abc123 /readme.txt
```

### share

Add a peer as a principal on a bucket.

```bash
jax-bucket bucket share <BUCKET_ID> --public-key <PEER_PUBLIC_KEY> [--role <ROLE>]

# Examples
jax-bucket bucket share abc123 --public-key z6Mk... --role owner
jax-bucket bucket share abc123 --public-key z6Mk... --role mirror
```

Roles:
- `owner` - Full access with encrypted share
- `mirror` - Sync-only until published

### clone

Clone a bucket from a peer.

```bash
jax-bucket bucket clone <TICKET>
```

The ticket contains the bucket ID, peer address, and your encrypted share.

### sync

Manually trigger sync for a bucket.

```bash
jax-bucket bucket sync <BUCKET_ID>
```

## HTTP API

When the daemon is running, it exposes a REST API:

### Health Endpoints

```
GET /health/live      # Liveness check
GET /health/ready     # Readiness check
GET /health/version   # Version info
```

### Bucket API (v0)

```
POST   /api/v0/bucket/create     # Create bucket
GET    /api/v0/bucket/list       # List buckets
POST   /api/v0/bucket/:id/add    # Add file
GET    /api/v0/bucket/:id/ls     # List directory
GET    /api/v0/bucket/:id/cat    # Read file
DELETE /api/v0/bucket/:id/delete # Delete file
POST   /api/v0/bucket/:id/mkdir  # Create directory
POST   /api/v0/bucket/:id/mv     # Move/rename
POST   /api/v0/bucket/:id/share  # Add principal
POST   /api/v0/bucket/:id/publish # Publish to mirrors
PUT    /api/v0/bucket/:id/rename # Rename bucket
PUT    /api/v0/bucket/:id/update # Update metadata
GET    /api/v0/bucket/:id/export # Export bucket
GET    /api/v0/bucket/ping       # Ping peer
```

## Web UI

The daemon serves a web interface at `http://localhost:3000/`:

- **Index**: List of all buckets
- **File Explorer**: Browse bucket contents
- **File Viewer**: View file contents
- **File Editor**: Edit text files
- **History**: View version history
- **Peers**: Manage peer connections

## Gateway

The gateway serves bucket content over HTTP for web access:

```
GET /gateway/<bucket-id>/<path>
```

Features:
- URL rewriting for clean paths
- Index file support (serves `index.html` for directories)
- Content-type detection

## Features

### Encryption

All content is encrypted with AES-256-GCM:
- Per-bucket encryption keys
- Per-item nonces
- Authenticated encryption

### Versioning

Every change creates a new version:
- Immutable history
- View any past state
- Conflict resolution via version links

### P2P Sync

Automatic synchronization:
- Connect to peers by address
- Exchange bucket manifests
- Transfer only missing blobs
- Resume interrupted transfers

### Access Control

Granular permissions:
- Owner: full access
- Mirror: read-only after publish
- Add/remove principals dynamically
- Publish/unpublish to control mirror access

### Content Addressing

Data stored by hash:
- Automatic deduplication
- Integrity verification
- Efficient sync (only transfer missing data)

## Configuration

Default config location: `~/.jax/`

```
~/.jax/
├── identity.key     # Ed25519 private key
├── database.sqlite  # Local metadata database
└── blobs/           # Content-addressed blob storage
```

## Environment Variables

```bash
RUST_LOG=debug       # Enable debug logging
RUST_BACKTRACE=1     # Enable backtraces on panic
```

## Examples

### Create and populate a bucket

```bash
# Initialize (first time only)
jax-bucket init

# Start daemon in background
jax-bucket daemon &

# Create a bucket
jax-bucket bucket create photos

# Add files
jax-bucket bucket add <bucket-id> ~/Pictures/vacation.jpg /vacation/
jax-bucket bucket add <bucket-id> ~/Pictures/wedding.jpg /wedding/

# List contents
jax-bucket bucket ls <bucket-id>
```

### Share with another peer

```bash
# Add peer as mirror
jax-bucket bucket share <bucket-id> --public-key <peer-pubkey> --role mirror

# Publish to grant read access
# (via HTTP API)
curl -X POST http://localhost:3000/api/v0/bucket/<bucket-id>/publish
```

### Clone a shared bucket

```bash
# On the receiving peer, use the ticket from the owner
jax-bucket bucket clone <ticket>

# The bucket will sync automatically
jax-bucket bucket ls <bucket-id>
```
