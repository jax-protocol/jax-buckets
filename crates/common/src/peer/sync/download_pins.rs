//! Pins download job and execution logic
//!
//! This module contains the logic for downloading pinned content from peers.

use anyhow::Result;

use crate::bucket_log::BucketLogProvider;
use crate::crypto::PublicKey;
use crate::linked_data::Link;
use crate::peer::Peer;

/// Download pins job definition
#[derive(Debug, Clone)]
pub struct DownloadPinsJob {
    pub pins_link: Link,
    pub peer_ids: Vec<PublicKey>,
}

/// Execute a pins download job
///
/// This downloads the hash list from the specified peers.
pub async fn execute<L>(peer: &Peer<L>, job: DownloadPinsJob) -> Result<()>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    peer.blobs()
        .download_hash_list(job.pins_link.hash(), job.peer_ids, peer.endpoint())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to download pins: {}", e))
}
