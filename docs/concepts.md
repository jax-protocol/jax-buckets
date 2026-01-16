# Concepts

This document explains the high-level architecture and key concepts in jax-bucket.

## Overview

jax-bucket organizes data into **Buckets** - encrypted containers that hold files and directories. Each bucket has:

- A unique identifier (UUID)
- A friendly name
- Encrypted content (files, directories)
- Access control (who can read/write)
- Version history

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI / HTTP API                        │
├─────────────────────────────────────────────────────────────┤
│                           Mount                              │
│              (Virtual filesystem abstraction)                │
├─────────────────────────────────────────────────────────────┤
│                          Manifest                            │
│           (Metadata, shares, version tracking)               │
├─────────────────────────────────────────────────────────────┤
│                          Crypto                              │
│        (Encryption, key sharing, authentication)             │
├─────────────────────────────────────────────────────────────┤
│                        BlobsStore                            │
│              (Content-addressed storage)                     │
├─────────────────────────────────────────────────────────────┤
│                           Peer                               │
│                (P2P networking via iroh)                     │
└─────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Bucket

A bucket is the top-level container for your data. It's similar to a folder or repository, but encrypted and distributed.

- Each bucket has a **Secret** - an AES-256 key that encrypts all content
- The secret is never stored directly; it's split into **Shares** for each principal
- Buckets sync automatically between peers that have access

### Mount

A mount is the runtime representation of a bucket. When you "mount" a bucket, you:

1. Load the bucket's manifest from storage
2. Decrypt the secret using your share
3. Gain access to the virtual filesystem

The mount provides file operations:
- `add` - Add files or directories
- `ls` - List directory contents
- `cat` - Read file contents
- `mkdir` - Create directories
- `mv` - Move/rename files
- `rm` - Remove files

### Manifest

The manifest is the encrypted metadata for a bucket:

```
Manifest
├── id: UUID           # Unique bucket identifier
├── name: String       # Friendly display name
├── shares: Map        # Access control (who can decrypt)
├── entry: Link        # Root directory of content
├── pins: Link         # Set of pinned content hashes
├── previous: Link?    # Link to previous version
└── version: Version   # Version metadata
```

Each time you modify a bucket, a new manifest is created with a `previous` link to the old one, creating an immutable version history.

### Share

A share grants a principal (user) access to a bucket. It contains:

- **Principal**: Public key + role (Owner or Mirror)
- **SecretShare**: The bucket's secret, encrypted to this principal's public key

```rust
Share {
    principal: Principal {
        role: Owner | Mirror,
        identity: PublicKey
    },
    share: Option<SecretShare>  // None for unpublished mirrors
}
```

### Principal Roles

**Owner**
- Full read/write access
- Can add/remove other principals
- Can publish to mirrors
- Always has a SecretShare

**Mirror**
- Read-only sync access
- Can sync blob data from the network
- Cannot decrypt until bucket is **published**
- SecretShare is `None` until published

### Publishing

Publishing grants mirrors the ability to decrypt:

1. Owner calls `publish` on a bucket
2. For each mirror, the bucket's secret is encrypted to their public key
3. The encrypted secret becomes their `SecretShare`
4. Mirrors can now mount and read the bucket

This separation allows:
- Adding mirrors before deciding to share content
- Revoking access by "unpublishing" (removing shares)
- Mirrors syncing encrypted blobs even before publication

## Cryptography

### Identity (Ed25519)

Each peer has an Ed25519 keypair:
- **SecretKey**: Private key, never shared
- **PublicKey**: Used to identify the peer and receive encrypted shares

### Content Encryption (AES-256-GCM)

Every bucket has a unique **Secret** (AES-256 key):
- Used to encrypt all content in the bucket
- Per-item encryption with unique nonces
- Authenticated encryption prevents tampering

### Key Sharing (ECDH + X25519)

To share a bucket's secret with another peer:

1. Convert Ed25519 keys to X25519 (Montgomery form)
2. Generate ephemeral keypair
3. Perform ECDH to derive shared secret
4. Wrap bucket secret with AES-KW
5. Package as SecretShare (ephemeral pubkey || wrapped secret)

The recipient reverses the process using their private key.

## Storage

### Content-Addressed (BlobsStore)

All data is stored by its hash (content-addressed):

```
BlobsStore
├── <hash1> → encrypted blob
├── <hash2> → encrypted blob
└── ...
```

Benefits:
- Automatic deduplication
- Integrity verification
- Immutable history (old versions remain accessible)

### Links (CID/IPLD)

References between data use **Links** (Content Identifiers):
- Self-describing hash format
- Includes codec and hash algorithm
- Enables content-addressed data graphs

## Networking

### Peer (iroh)

jax-bucket uses iroh for P2P networking:

- **Endpoint**: Network identity and connection management
- **Protocol Router**: Handles incoming connections
- **Blob Protocol**: Efficient blob transfer

### Sync

Peers sync buckets by:

1. Advertising bucket manifests they have
2. Requesting manifests and blobs from peers
3. Verifying integrity via content addressing
4. Resolving conflicts via version links

Sync happens automatically when:
- Daemon starts
- Changes are made locally
- Peer connections are established

## Data Flow Example

```
User: jax-bucket bucket add my-bucket photo.jpg

1. Load bucket manifest from BlobsStore
2. Decrypt manifest using user's share
3. Read photo.jpg from filesystem
4. Encrypt photo with bucket's secret
5. Store encrypted blob in BlobsStore (by hash)
6. Update directory entry with new link
7. Encrypt updated directory
8. Create new manifest with updated entry link
9. Store manifest in BlobsStore
10. Announce to connected peers
```
