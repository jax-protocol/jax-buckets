use iroh::protocol::Router;
use tokio::sync::watch::Receiver as WatchReceiver;

mod blobs_store;
mod jobs;
mod peer;
mod protocol;

pub use blobs_store::{BlobsStore, BlobsStoreError};
pub use jobs::{Job, JobDispatcher, JobReceiver};
pub use protocol::ALPN;

pub use iroh::NodeAddr;

pub use crate::peer::peer::{Peer, PeerBuilder};

/// Spawn the peer with protocol router and background job worker
///
/// This starts both the iroh protocol router (for handling incoming connections)
/// and the background job worker (for processing sync tasks and other jobs).
/// Both are gracefully shut down when the shutdown signal is received.
///
/// # Arguments
///
/// * `peer` - The peer instance to run
/// * `job_receiver` - The job receiver from PeerBuilder::build()
/// * `shutdown_rx` - Watch receiver for shutdown signal
///
/// # Example
///
/// ```ignore
/// let (peer, job_receiver) = PeerBuilder::new()
///     .log_provider(database)
///     .build()
///     .await;
///
/// let (shutdown_tx, shutdown_rx) = watch::channel(());
///
/// tokio::spawn(async move {
///     if let Err(e) = peer::spawn(peer, job_receiver, shutdown_rx).await {
///         tracing::error!("Peer failed: {}", e);
///     }
/// });
/// ```
pub async fn spawn<L>(
    peer: Peer<L>,
    job_receiver: jobs::JobReceiver,
    mut shutdown_rx: WatchReceiver<()>,
) -> Result<(), PeerError>
where
    L: crate::bucket_log::BucketLogProvider + Clone + Send + Sync + std::fmt::Debug + 'static,
    L::Error: std::fmt::Display + std::error::Error + Send + Sync + 'static,
{
    let node_id = peer.id();
    tracing::info!(peer_id = %node_id, "Starting peer");

    // Clone peer for the worker task
    let worker_peer = peer.clone();

    // Spawn the background job worker
    let worker_handle = tokio::spawn(async move {
        tracing::info!(peer_id = %node_id, "Starting background job worker");
        worker_peer.run_worker(job_receiver).await;
        tracing::info!(peer_id = %node_id, "Background job worker stopped");
    });

    // Build the protocol router with iroh-blobs and our custom protocol
    let inner_blobs = peer.blobs().inner.clone();
    let router_builder = Router::builder(peer.endpoint().clone())
        .accept(iroh_blobs::ALPN, inner_blobs)
        .accept(ALPN, peer.clone());

    let router = router_builder.spawn();

    tracing::info!(peer_id = %node_id, "Peer protocol router started");

    // Wait for shutdown signal
    let _ = shutdown_rx.changed().await;
    tracing::info!(peer_id = %node_id, "Shutdown signal received, stopping peer");

    // Shutdown the router (this closes the endpoint and stops accepting connections)
    router.shutdown().await.map_err(|e| PeerError::RouterShutdown(e.into()))?;

    // Wait for the worker to finish (it will stop when the job dispatcher is dropped)
    // We give it a reasonable timeout to finish processing current jobs
    let worker_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        worker_handle,
    ).await;

    match worker_result {
        Ok(Ok(())) => {
            tracing::info!(peer_id = %node_id, "Job worker stopped gracefully");
        }
        Ok(Err(e)) => {
            tracing::error!(peer_id = %node_id, error = %e, "Job worker panicked");
        }
        Err(_) => {
            tracing::warn!(peer_id = %node_id, "Job worker did not stop within timeout");
        }
    }

    tracing::info!(peer_id = %node_id, "Peer stopped");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("failed to shutdown router: {0}")]
    RouterShutdown(anyhow::Error),
}
