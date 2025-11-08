use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::PublicKey;
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
    /// The public key of the peer making the request
    pub requester_id: PublicKey,
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
pub struct Pong {
    /// The bucket ID being responded to
    pub bucket_id: Uuid,
    /// The sync status
    pub status: PPong,
    /// The public key of the peer responding
    pub responder_id: PublicKey,
}

impl Pong {
    /// Create a new pong message indicating bucket not found
    pub fn not_found(bucket_id: Uuid, responder_id: PublicKey) -> Self {
        Self {
            bucket_id,
            status: PPong::NotFound,
            responder_id,
        }
    }

    /// Create a new pong message indicating we are ahead
    pub fn ahead(bucket_id: Uuid, link: Link, height: u64, responder_id: PublicKey) -> Self {
        Self {
            bucket_id,
            status: PPong::Ahead(link, height),
            responder_id,
        }
    }

    /// Create a new pong message indicating we are behind
    pub fn behind(bucket_id: Uuid, link: Link, height: u64, responder_id: PublicKey) -> Self {
        Self {
            bucket_id,
            status: PPong::Behind(link, height),
            responder_id,
        }
    }

    /// Create a new pong message indicating we are in sync
    pub fn in_sync(bucket_id: Uuid, responder_id: PublicKey) -> Self {
        Self {
            bucket_id,
            status: PPong::InSync,
            responder_id,
        }
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
        let bucket_id = ping.bucket_id;
        let our_pub_key = PublicKey::from(*peer.secret().public());

        // Get our current height for this bucket
        let our_height = match logs.height(bucket_id).await {
            Ok(h) => h,
            Err(e) => {
                // Check if it's a HeadNotFound error (bucket doesn't exist)
                match e {
                    crate::bucket_log::BucketLogError::HeadNotFound(_) => {
                        return Pong::not_found(bucket_id, our_pub_key);
                    }
                    _ => {
                        tracing::error!("Unexpected error getting bucket height: {}", e);
                        panic!("Unexpected error getting bucket height: {}", e);
                    }
                }
            }
        };

        let head = logs
            .head(bucket_id, our_height)
            .await
            .expect("failed to get bucket head");

        // Compare heights and determine sync status
        if our_height < ping.height {
            Pong::behind(bucket_id, head, our_height, our_pub_key)
        } else if our_height == ping.height {
            // At same height, we're in sync
            Pong::in_sync(bucket_id, our_pub_key)
        } else {
            // We're ahead of the remote peer
            Pong::ahead(bucket_id, head, our_height, our_pub_key)
        }
    }

    /// Side effects after sending response
    ///
    /// This is called AFTER we've sent the pong back to the peer.
    /// Use this to trigger background operations without blocking the response.
    async fn after_response_sent<L: BucketLogProvider>(
        peer: &Peer<L>,
        ping: &Ping,
        pong: &Pong,
    ) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        match &pong.status {
            PPong::Behind(_, our_height) => {
                // We told them we're behind, so we should dispatch a sync job
                tracing::info!(
                    "We're behind peer for bucket {} (our height: {}, their height: {}), dispatching sync job",
                    ping.bucket_id,
                    our_height,
                    ping.height
                );

                // Dispatch sync job to background worker
                if let Err(e) = peer.jobs().dispatch_sync(
                    ping.bucket_id,
                    ping.link.clone(),
                    ping.height,
                    ping.requester_id.clone(),
                ) {
                    tracing::error!("Failed to dispatch sync job: {}", e);
                }
            }
            PPong::Ahead(_, our_height) => {
                // We told them we're ahead, they might fetch from us
                tracing::debug!(
                    "We're ahead of peer for bucket {} (our height: {}, their height: {})",
                    ping.bucket_id,
                    our_height,
                    ping.height
                );
                // Nothing to do - they'll fetch from us if they want
            }
            PPong::InSync => {
                tracing::debug!("In sync with peer for bucket {}", ping.bucket_id);
                // All good, nothing to do
            }
            PPong::NotFound => {
                tracing::debug!(
                    "We don't have bucket {} that peer is asking about",
                    ping.bucket_id
                );
                // They might announce the bucket to us or we might want to clone it
            }
        }
        Ok(())
    }

    // ========================================
    // INITIATOR SIDE: When we receive a pong
    // ========================================

    /// Handle pong response: decide what to do based on sync status
    async fn handle_response<L: BucketLogProvider>(peer: &Peer<L>, pong: &Pong) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        match &pong.status {
            PPong::NotFound => {
                tracing::info!(
                    "Remote peer {} doesn't have bucket {}",
                    pong.responder_id.to_hex(),
                    pong.bucket_id
                );
                // TODO: Could trigger an announce to share our bucket with them
                // e.g., announce_to_peer(...).await?;
            }
            PPong::Ahead(link, height) => {
                // Remote peer is ahead, dispatch a sync job
                tracing::info!(
                    "Remote peer {} is ahead for bucket {} at height {} with link {:?}, dispatching sync job",
                    pong.responder_id.to_hex(),
                    pong.bucket_id,
                    height,
                    link
                );

                // Dispatch sync job to background worker
                if let Err(e) = peer.jobs().dispatch_sync(
                    pong.bucket_id,
                    link.clone(),
                    *height,
                    pong.responder_id.clone(),
                ) {
                    tracing::error!("Failed to dispatch sync job: {}", e);
                }
            }
            PPong::Behind(link, height) => {
                tracing::info!(
                    "Remote peer {} is behind for bucket {} at height {} with link {:?}",
                    pong.responder_id.to_hex(),
                    pong.bucket_id,
                    height,
                    link
                );
                // Remote peer is behind, they might fetch from us
                // Nothing to do on our side
            }
            PPong::InSync => {
                tracing::info!(
                    "In sync with peer {} for bucket {}",
                    pong.responder_id.to_hex(),
                    pong.bucket_id
                );
                // All good
            }
        }
        Ok(())
    }
}
