use iroh::protocol::Router;
use tokio::sync::watch::Receiver as WatchReceiver;

mod blobs_store;
mod peer_builder;
mod peer_inner;
mod protocol;
pub mod sync;

pub use blobs_store::{BlobsStore, BlobsStoreError};
pub use protocol::{PingReplyStatus, ALPN};
pub use sync::{SyncJob, SyncProvider, SyncTarget};

pub use iroh::NodeAddr;

pub use peer_builder::PeerBuilder;
pub use peer_inner::Peer;

/// Spawn the peer with protocol router
///
/// This starts the iroh protocol router for handling incoming connections.
/// The peer's sync provider is responsible for managing its own background workers.
///
/// # Arguments
///
/// * `peer` - The peer instance to run
/// * `shutdown_rx` - Watch receiver for shutdown signal
pub async fn spawn<L>(peer: Peer<L>, mut shutdown_rx: WatchReceiver<()>) -> Result<(), PeerError>
where
    L: crate::bucket_log::BucketLogProvider + Clone + Send + Sync + std::fmt::Debug + 'static,
    L::Error: std::fmt::Display + std::error::Error + Send + Sync + 'static,
{
    let node_id = peer.id();
    tracing::info!(peer_id = %node_id, "Starting peer");

    // Extract what we need for the router
    let inner_blobs = peer.blobs().inner.clone();
    let endpoint = peer.endpoint().clone();
    let peer_for_router = peer.clone();

    // Build the protocol router with iroh-blobs and our custom protocol
    let router_builder = Router::builder(endpoint)
        .accept(iroh_blobs::ALPN, inner_blobs)
        .accept(ALPN, peer_for_router);

    let router = router_builder.spawn();

    tracing::info!(peer_id = %node_id, "Peer protocol router started");

    // Wait for shutdown signal
    let _ = shutdown_rx.changed().await;
    tracing::info!(peer_id = %node_id, "Shutdown signal received, stopping peer");

    // Shutdown the router (this closes the endpoint and stops accepting connections)
    router
        .shutdown()
        .await
        .map_err(|e| PeerError::RouterShutdown(e.into()))?;

    tracing::info!(peer_id = %node_id, "Peer stopped");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("failed to shutdown router: {0}")]
    RouterShutdown(anyhow::Error),
}
