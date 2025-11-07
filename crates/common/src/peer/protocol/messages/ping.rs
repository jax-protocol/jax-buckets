use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::linked_data::Link;

use crate::bucket_log::BucketLogProvider;
use crate::peer::protocol::bidirectional::BidirectionalHandler;
use crate::peer::Peer;

/// Request to ping a peer and check bucket sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping {
    /// The bucket ID to check
    pub bucket_id: Uuid,
    /// The current link the requesting peer has for this bucket
    pub link: Link,
    /// The height of the link we are responding to
    pub height: u64,
}

/// Sync status between two peers for a bucket
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PPong {
    /// The peer does not have this bucket at all
    NotFound,
    /// We are ahead of the current peer's history,
    ///  report where we are
    Ahead(Link, u64),
    /// We are behind, report where we are
    Behind(Link, u64),
    /// Both agree on the current link (in sync)
    InSync,
}

/// Response to a ping request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong(pub PPong);

impl Pong {
    /// Create a new pong message
    pub fn not_found() -> Self {
        Self(PPong::NotFound)
    }

    /// Create a new pong message indicating we are ahead
    pub fn ahead(link: Link, height: u64) -> Self {
        Self(PPong::Ahead(link, height))
    }

    /// Create a new pong message indicating we are behind
    pub fn behind(link: Link, height: u64) -> Self {
        Self(PPong::Behind(link, height))
    }

    /// Create a new pong message indicating we are in sync
    pub fn in_sync() -> Self {
        Self(PPong::InSync)
    }
}

/// Ping handler implementing the bidirectional handler pattern
///
/// This demonstrates the complete protocol flow in one place:
/// - Responder: what to send back + side effects after sending
/// - Initiator: what to do with the response
pub struct PingHandler;

impl BidirectionalHandler for PingHandler {
    type Request = Ping;
    type Response = Pong;

    // ========================================
    // RESPONDER SIDE: When we receive a ping
    // ========================================

    /// Generate response: compare our state with peer's state
    async fn handle_request<L: BucketLogProvider>(peer: &Peer<L>, ping: &Ping) -> Pong {
        let logs = peer.logs();

        // Get our current height for this bucket
        let our_height = match logs.height(ping.bucket_id).await {
            Ok(h) => h,
            Err(e) => {
                // Check if it's a HeadNotFound error (bucket doesn't exist)
                match e {
                    crate::bucket_log::BucketLogError::HeadNotFound(_) => {
                        return Pong::not_found();
                    }
                    _ => {
                        tracing::error!("Unexpected error getting bucket height: {}", e);
                        panic!("Unexpected error getting bucket height: {}", e);
                    }
                }
            }
        };

        let head = logs
            .head(ping.bucket_id, our_height)
            .await
            .expect("failed to get bucket head");

        // Compare heights and determine sync status
        if our_height < ping.height {
            Pong::behind(head, our_height)
        } else if our_height == ping.height {
            // At same height, we're in sync
            Pong::in_sync()
        } else {
            // We're ahead of the remote peer
            Pong::ahead(head, our_height)
        }
    }

    /// Side effects after sending response
    ///
    /// This is called AFTER we've sent the pong back to the peer.
    /// Use this to trigger background operations without blocking the response.
    async fn after_response_sent<L: BucketLogProvider>(
        _peer: &Peer<L>,
        ping: &Ping,
        pong: &Pong,
    ) -> Result<()> {
        match &pong.0 {
            PPong::Behind(_, our_height) => {
                // We told them we're behind, so we might want to prepare to fetch from them
                tracing::debug!(
                    "After responding 'behind' to ping for bucket {} (our height: {})",
                    ping.bucket_id,
                    our_height
                );
                // TODO: Could spawn a background task to sync from this peer
                // e.g., tokio::spawn(async { fetch_missing_data(...) });
            }
            PPong::Ahead(_, our_height) => {
                // We told them we're ahead, they might fetch from us
                tracing::debug!(
                    "After responding 'ahead' to ping for bucket {} (our height: {})",
                    ping.bucket_id,
                    our_height
                );
                // Nothing to do - they'll fetch from us if they want
            }
            PPong::InSync => {
                tracing::debug!("After responding 'in sync' to ping for bucket {}", ping.bucket_id);
                // All good, nothing to do
            }
            PPong::NotFound => {
                tracing::debug!("After responding 'not found' to ping for bucket {}", ping.bucket_id);
                // They might announce the bucket to us
            }
        }
        Ok(())
    }

    // ========================================
    // INITIATOR SIDE: When we receive a pong
    // ========================================

    /// Handle pong response: decide what to do based on sync status
    async fn handle_response<L: BucketLogProvider>(_peer: &Peer<L>, pong: &Pong) -> Result<()> {
        match &pong.0 {
            PPong::NotFound => {
                tracing::info!("Remote peer doesn't have this bucket");
                // TODO: Could trigger an announce to share our bucket with them
                // e.g., announce_to_peer(...).await?;
            }
            PPong::Ahead(link, height) => {
                tracing::info!(
                    "Remote peer is ahead at height {} with link {:?}",
                    height,
                    link
                );
                // TODO: Could spawn background sync task to fetch missing data
                // e.g., tokio::spawn(async { sync_from_peer(...) });
            }
            PPong::Behind(link, height) => {
                tracing::info!(
                    "Remote peer is behind at height {} with link {:?}",
                    height,
                    link
                );
                // Remote peer is behind, they might fetch from us
                // Nothing to do on our side
            }
            PPong::InSync => {
                tracing::info!("Peers are in sync");
                // All good
            }
        }
        Ok(())
    }
}
