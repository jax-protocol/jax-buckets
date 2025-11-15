//! Bucket synchronization job and execution logic
//!
//! This module contains the logic for syncing buckets between peers.

use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::bucket_log::BucketLogProvider;
use crate::crypto::PublicKey;
use crate::linked_data::Link;
use crate::mount::Manifest;
use crate::peer::Peer;

use super::{DownloadPinsJob, SyncJob};

/// Target peer and state for bucket synchronization
#[derive(Debug, Clone)]
pub struct SyncTarget {
    /// Link to the target bucket state
    pub link: Link,
    /// Height of the target bucket
    pub height: u64,
    /// Public key of the peer to sync from
    pub peer_id: PublicKey,
}

/// Sync bucket job definition
#[derive(Debug, Clone)]
pub struct SyncBucketJob {
    pub bucket_id: Uuid,
    pub target: SyncTarget,
}

/// Execute a bucket sync job
///
/// This is the main entry point for syncing. It handles both cases:
/// - Updating an existing bucket we already have
/// - Cloning a new bucket we don't have yet
pub async fn execute<L>(peer: &Peer<L>, job: SyncBucketJob) -> Result<()>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    tracing::info!(
        "Syncing bucket {} from peer {} to link {:?} at height {}",
        job.bucket_id,
        job.target.peer_id.to_hex(),
        job.target.link,
        job.target.height
    );

    let exists: bool = peer.logs().exists(job.bucket_id).await?;

    let common_ancestor = if exists {
        // find a common ancestor between our log and the
        //  link the peer advertised to us
        find_common_ancestor(peer, job.bucket_id, &job.target.link, &job.target.peer_id).await?
    } else {
        None
    };

    // TODO (amiller68): between finding the common ancestor and downloading the manifest chain
    //  there are redundant operations. We should optimize this.

    // if we know the bucket exists, but we did not find a common ancestor
    //  then we have diverged / are not talking about the same bucket
    // for now just log a warning and do nothing
    if exists && common_ancestor.is_none() {
        tracing::warn!(
            "Bucket {} diverged from peer {:?}",
            job.bucket_id,
            job.target.peer_id
        );
        return Ok(());
    }

    // Determine between what links we should download manifests for
    let stop_link = if let Some(ancestor) = common_ancestor {
        Some(&ancestor.0.clone())
    } else {
        // No common ancestor - we'll sync everything from the target back to genesis
        tracing::info!(
            "No common ancestor for bucket {}, syncing from genesis",
            job.bucket_id
        );
        None
    };

    // now we know there is a valid list of manifests we should
    //  fetch and apply to our log

    // Download manifest chain from peer (from target back to common ancestor)
    let manifests =
        download_manifest_chain(peer, &job.target.link, stop_link, &job.target.peer_id).await?;

    // TODO (amiller68): maybe theres an optimization here in that we should know
    //  we can exit earlier by virtue of finding a common ancestor which is just
    //  our current head
    if manifests.is_empty() {
        tracing::info!("No new manifests to sync, already up to date");
        return Ok(());
    };

    // Just check we are still included in the shares at the end of this,
    //  if not we wouldn't have gotten the ping, but we might as well just
    //  check
    if !verify_provenance(peer, &manifests.last().unwrap().0)? {
        tracing::warn!("Provenance verification failed: our key not in bucket shares");
        return Ok(());
    }

    // apply the updates to the bucket
    apply_manifest_chain(peer, job.bucket_id, &manifests).await?;

    Ok(())
}

/// Download a chain of manifests from a peer
///
/// Walks backwards through the manifest chain via `previous` links.
/// Stops when it reaches `stop_at` link (common ancestor) or genesis (no previous).
///
/// Returns manifests in order from oldest to newest with their links and heights.
async fn download_manifest_chain<L>(
    peer: &Peer<L>,
    start_link: &Link,
    stop_link: Option<&Link>,
    // TODO (amiller68): this could use multi-peer download
    peer_id: &PublicKey,
) -> Result<Vec<(Manifest, Link)>>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    tracing::debug!(
        "Downloading manifest chain from {:?}, stop_at: {:?}",
        start_link,
        stop_link
    );

    let mut manifests = Vec::new();
    let mut current_link = start_link.clone();

    // Download manifests walking backwards
    loop {
        // Download the manifest blob from peer
        peer.blobs()
            .download_hash(current_link.hash(), vec![*peer_id], peer.endpoint())
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to download manifest {:?} from peer: {}",
                    current_link,
                    e
                )
            })?;

        // Read and decode the manifest
        let manifest: Manifest = peer.blobs().get_cbor(&current_link.hash()).await?;

        // Check if we should stop
        if let Some(stop_link) = stop_link {
            if &current_link == stop_link {
                tracing::debug!("Reached stop_at link, stopping download");
                break;
            }
        }

        manifests.push((manifest.clone(), current_link.clone()));

        // Check for previous link
        match manifest.previous() {
            Some(prev_link) => {
                current_link = prev_link.clone();
            }
            None => {
                // Reached genesis, stop
                tracing::debug!("Reached genesis manifest, stopping download");
                break;
            }
        }
    }

    // Reverse to get oldest-to-newest order
    manifests.reverse();

    tracing::debug!("Downloaded {} manifests", manifests.len());
    Ok(manifests)
}

/// Find common ancestor by downloading manifests from peer
///
/// Starting from `start_link`, walks backwards through the peer's manifest chain,
/// downloading each manifest and checking if it exists in our local log.
/// Returns the first (most recent) link and height found in our log.
///
/// # Arguments
///
/// * `peer` - The peer instance with access to logs and blobs
/// * `bucket_id` - The bucket to check against our local log
/// * `link` - The starting point on the peer's chain (typically their head)
/// * `peer_id` - The peer to download manifests from
///
/// # Returns
///
/// * `Ok(Some((link, height)))` - Found common ancestor with its link and height
/// * `Ok(None)` - No common ancestor found (reached genesis without intersection)
/// * `Err(_)` - Download or log access error
async fn find_common_ancestor<L>(
    peer: &Peer<L>,
    bucket_id: Uuid,
    link: &Link,
    peer_id: &PublicKey,
) -> Result<Option<(Link, u64)>>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    tracing::debug!(
        "Finding common ancestor starting from {:?} with peer {}",
        link,
        peer_id.to_hex()
    );

    let mut current_link = link.clone();
    let mut manifests_checked = 0;

    loop {
        manifests_checked += 1;
        tracing::debug!(
            "Checking manifest {} at link {:?}",
            manifests_checked,
            current_link
        );

        // TODO (amiller68): this should build in memory
        //  but for now we just download it
        // Download the manifest from peer
        peer.blobs()
            .download_hash(current_link.hash(), vec![*peer_id], peer.endpoint())
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to download manifest {:?} from peer: {}",
                    current_link,
                    e
                )
            })?;

        // Read and decode the manifest
        let manifest: Manifest = peer.blobs().get_cbor(&current_link.hash()).await?;
        let height = manifest.height();

        // Check if this link exists in our local log
        match peer.logs().has(bucket_id, current_link.clone()).await {
            Ok(heights) if !heights.is_empty() => {
                tracing::info!(
                    "Found common ancestor at link {:?} with height {} (in our log at heights {:?})",
                    current_link,
                    height,
                    heights
                );
                return Ok(Some((current_link, height)));
            }
            Ok(_) => {
                // Link not in our log, check previous
                tracing::debug!("Link {:?} not in our log, checking previous", current_link);
            }
            Err(e) => {
                tracing::warn!("Error checking for link in log: {}", e);
                // Continue checking previous links despite error
            }
        }

        // Move to previous link
        match manifest.previous() {
            Some(prev_link) => {
                current_link = prev_link.clone();
            }
            None => {
                // Reached genesis without finding common ancestor
                tracing::debug!(
                    "Reached genesis after checking {} manifests, no common ancestor found",
                    manifests_checked
                );
                return Ok(None);
            }
        }
    }
}

/// Apply a chain of manifests to the log
///
/// Appends each manifest to the log in order, starting from `start_height`.
async fn apply_manifest_chain<L>(
    peer: &Peer<L>,
    bucket_id: Uuid,
    manifests: &[(Manifest, Link)],
) -> Result<()>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    tracing::info!("Applying {} manifests to log", manifests.len(),);

    if let Some((_i, (manifest, link))) = manifests.iter().enumerate().next() {
        let previous = manifest.previous().clone();
        let height = manifest.height();

        tracing::info!(
            "Appending manifest to log: height={}, link={:?}, previous={:?}",
            height,
            link,
            previous
        );

        peer.logs()
            .append(
                bucket_id,
                manifest.name().to_string(),
                link.clone(),
                previous,
                height,
            )
            .await
            .map_err(|e| anyhow!("Failed to append manifest at height {}: {}", height, e))?;

        let pins_link = manifest.pins().clone();
        let peer_ids = manifest
            .shares()
            .iter()
            .map(|share| share.1.principal().identity)
            .collect();
        return peer
            .dispatch(SyncJob::DownloadPins(DownloadPinsJob {
                pins_link,
                peer_ids,
            }))
            .await;
    }

    tracing::info!("Successfully applied {} manifests to log", manifests.len());
    Ok(())
}

/// Verify that our PublicKey is in the manifest's shares
fn verify_provenance<L>(peer: &Peer<L>, manifest: &Manifest) -> Result<bool>
where
    L: BucketLogProvider + Clone + Send + Sync + 'static,
    L::Error: std::error::Error + Send + Sync + 'static,
{
    let our_pub_key = PublicKey::from(*peer.secret().public());
    let our_pub_key_hex = our_pub_key.to_hex();

    // Check if our key is in the shares
    let is_authorized = manifest
        .shares()
        .iter()
        .any(|(key_hex, _share)| key_hex == &our_pub_key_hex);

    tracing::debug!(
        "Provenance check: our_key={}, authorized={}",
        our_pub_key_hex,
        is_authorized
    );

    Ok(is_authorized)
}
