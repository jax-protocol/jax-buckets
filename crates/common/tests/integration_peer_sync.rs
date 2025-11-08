#![cfg(feature = "testkit")]
//! Integration test demonstrating peer-to-peer sync using the testkit

use anyhow::Result;
use common::testkit::TestNetwork;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_two_peers_basic_communication() -> Result<()> {
    // Initialize tracing for test visibility
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    tracing::info!("=== Starting two-peer basic communication test ===");

    let mut net = TestNetwork::new();

    // Create two peers
    tracing::info!("Creating alice and bob peers");
    net.add_peer("alice").await?;
    net.add_peer("bob").await?;

    // Introduce peers to each other for local discovery
    net.introduce_all_peers()?;

    tracing::info!(
        "Alice ID: {}, Bob ID: {}",
        net.peer("alice").unwrap().id(),
        net.peer("bob").unwrap().id()
    );

    // Alice puts some data
    tracing::info!("Alice putting test data into blobs");
    let data = b"Hello from Alice!";
    let link = net.peer("alice").unwrap().put_blob(data).await?;
    tracing::info!("Alice stored data with link: {}", link);

    // Verify alice has it
    assert!(net.peer("alice").unwrap().has_blob(&link).await?);
    tracing::info!("Verified alice has the blob");

    // Bob downloads from Alice
    tracing::info!("Bob downloading blob from alice");
    {
        let alice = net.peer("alice").unwrap();
        let bob = net.peer("bob").unwrap();
        bob.download_blob_from(alice, &link).await?;
    }

    // Verify bob has it
    let link_clone = link.clone();
    net.eventually(Duration::from_secs(2), || async {
        net.peer("bob").unwrap().has_blob(&link_clone).await
    })
    .await?;

    tracing::info!("Verified bob has the blob");

    // Verify content matches
    let bob_data = net.peer("bob").unwrap().get_blob(&link).await?;
    assert_eq!(bob_data.as_slice(), data);
    tracing::info!("Verified bob's data matches alice's");

    net.shutdown().await?;
    tracing::info!("=== Test completed successfully ===");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn test_three_peer_cascade_sync() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    tracing::info!("=== Starting three-peer cascade sync test ===");

    let mut net = TestNetwork::new();

    // Create three peers
    net.add_peer("alice").await?;
    net.add_peer("bob").await?;
    net.add_peer("carol").await?;

    // Introduce all peers to each other
    net.introduce_all_peers()?;

    // Alice creates data
    let data = b"Data from Alice";
    let link = net.peer("alice").unwrap().put_blob(data).await?;
    tracing::info!("Alice created blob: {}", link);

    // Bob gets it from Alice
    {
        let alice = net.peer("alice").unwrap();
        let bob = net.peer("bob").unwrap();
        bob.download_blob_from(alice, &link).await?;
    }

    let link_clone = link.clone();
    net.eventually(Duration::from_secs(2), || async {
        net.peer("bob").unwrap().has_blob(&link_clone).await
    })
    .await?;
    tracing::info!("Bob synced from Alice");

    // Carol gets it from Bob (not Alice!)
    {
        let bob = net.peer("bob").unwrap();
        let carol = net.peer("carol").unwrap();
        carol.download_blob_from(bob, &link).await?;
    }

    let link_clone = link.clone();
    net.eventually(Duration::from_secs(2), || async {
        net.peer("carol").unwrap().has_blob(&link_clone).await
    })
    .await?;
    tracing::info!("Carol synced from Bob");

    // Verify all have the same data
    let bob_data = net.peer("bob").unwrap().get_blob(&link).await?;
    let carol_data = net.peer("carol").unwrap().get_blob(&link).await?;
    assert_eq!(bob_data.as_slice(), data);
    assert_eq!(carol_data.as_slice(), data);
    tracing::info!("All peers have identical data");

    net.shutdown().await?;
    tracing::info!("=== Test completed successfully ===");

    Ok(())
}
