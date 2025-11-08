# Test Harness (testkit)

A lightweight test harness for multi-peer integration tests.

## Status

✅ **Implemented:**
- `TestPeer` wrapper for managing peer lifecycle
- `TestNetwork` coordinator for managing multiple peers
- In-memory bucket log provider integration
- Peer startup and shutdown management
- Helper methods for blob and bucket operations
- Eventually-consistent assertion helper

⚠️ **Known Issues:**
- Peer-to-peer blob downloads require additional setup for local peer discovery
- Peers currently try to use DHT/relay discovery which doesn't work well in local tests
- Need to provide direct `NodeAddr` with local socket addresses for downloads

## Usage

```rust
use common::testkit::TestNetwork;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_peers() -> anyhow::Result<()> {
    let mut net = TestNetwork::new();

    // Create and start peers
    net.add_peer("alice").await?;
    net.add_peer("bob").await?;

    // Access peers
    let alice = net.peer("alice").unwrap();
    let bob = net.peer("bob").unwrap();

    // Test operations
    let data = b"Hello!";
    let link = alice.put_blob(data).await?;
    assert!(alice.has_blob(&link).await?);

    // Eventual consistency helper
    net.eventually(Duration::from_secs(2), || async {
        bob.has_blob(&link).await
    }).await?;

    net.shutdown().await?;
    Ok(())
}
```

## API

### TestNetwork

- `new()` - Create a new test network
- `add_peer(name)` - Add and start a new peer
- `peer(name)` - Get a peer by name
- `shutdown()` - Stop all peers
- `eventually(timeout, condition)` - Poll until condition is true

### TestPeer

**Lifecycle:**
- `start()` - Start the peer (called automatically by `add_peer`)
- `stop()` - Stop the peer

**Identity:**
- `id()` - Get node ID
- `public_key()` - Get public key
- `socket_addr()` - Get socket address

**Blobs:**
- `put_blob(data)` - Store data and get link
- `get_blob(link)` - Retrieve data by link
- `has_blob(link)` - Check if blob exists
- `download_blob_from(peer, link)` - Download from specific peer

**Buckets:**
- `create_bucket(name)` - Create a bucket
- `commit_bucket(id, manifest, link, previous)` - Commit to bucket
- `bucket_height(id)` - Get current height
- `bucket_head(id, height)` - Get head at height
- `has_link(id, link)` - Check if link exists in log
- `sync_from(peer, bucket_id)` - Sync bucket from peer

## Next Steps

To make the test harness fully functional:

1. **Fix peer discovery**: Update `download_blob_from` to use `NodeAddr` with direct socket addresses instead of just `NodeId`
2. **Add examples**: Create more integration test examples showing bucket sync
3. **Helper methods**: Add utilities for creating test manifests and mounts
4. **Documentation**: Add more examples and troubleshooting guides

## Implementation Notes

- Uses `MemoryBucketLogProvider` for in-memory bucket logs
- Each peer gets its own temporary directory for blobs
- Peers are spawned using `peer::spawn` which handles router and job worker
- Shutdown is coordinated via watch channels
- Clones the peer after building so the original (with job_receiver) can be moved to spawn
