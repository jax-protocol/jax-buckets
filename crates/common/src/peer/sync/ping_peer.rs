//! Peer ping job and execution logic
//!
//! This module contains the logic for pinging peers to check sync status.

use anyhow::Result;
use uuid::Uuid;

use crate::bucket_log::BucketLogProvider;
use crate::crypto::PublicKey;
use crate::peer::protocol::bidirectional::BidirectionalHandler;
use crate::peer::protocol::{Ping, PingMessage};
use crate::peer::Peer;

/// Ping peer job definition
#[derive(Debug, Clone)]
pub struct PingPeerJob {
    pub bucket_id: Uuid,
    pub peer_id: PublicKey,
}

/// Execute a ping peer job
///
/// This sends a ping to the specified peer with our current bucket state
/// and processes the response.
pub async fn execute<L>(peer: &Peer<L>, job: PingPeerJob) -> Result<()>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    tracing::info!(
        "Processing ping job: bucket_id={}, peer_id={}",
        job.bucket_id,
        job.peer_id.to_hex()
    );

    // Get our bucket state
    let (our_link, our_height) = match peer.log_provider().head(job.bucket_id, None).await {
        Ok((link, height)) => (link, height),
        Err(e) => {
            tracing::warn!(
                "Failed to get head for bucket {} when pinging peer {}: {}",
                job.bucket_id,
                job.peer_id.to_hex(),
                e
            );
            return Ok(());
        }
    };

    // Construct ping
    let ping = PingMessage {
        bucket_id: job.bucket_id,
        link: our_link,
        height: our_height,
    };

    // Send ping
    tracing::info!("Sending ping to peer {}", job.peer_id.to_hex());
    match Ping::send::<L>(peer, &job.peer_id, ping).await {
        Ok(pong) => {
            tracing::info!(
                "Received pong from peer {} for bucket {} | {:?}",
                job.peer_id.to_hex(),
                job.bucket_id,
                pong
            );
            Ok(())
        }
        Err(e) => {
            tracing::debug!(
                "Failed to ping peer {} for bucket {}: {}",
                job.peer_id.to_hex(),
                job.bucket_id,
                e
            );
            Err(anyhow::anyhow!(
                "Ping job failed for bucket {} to peer {}: {}",
                job.bucket_id,
                job.peer_id.to_hex(),
                e
            ))
        }
    }
}
