use crate::crypto::{PublicKey, SecretKey};

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use anyhow::{anyhow, Result};
use iroh::discovery::pkarr::dht::DhtDiscovery;
use iroh::{Endpoint, NodeId};
use uuid::Uuid;

pub use super::blobs_store::BlobsStore;

use crate::bucket_log::BucketLogProvider;
use crate::linked_data::Link;
use crate::mount::Manifest;

#[derive(Clone, Default)]
pub struct PeerBuilder<L: BucketLogProvider> {
    /// the socket addr to expose the peer on
    ///  if not set, an ephemeral port will be used
    socket_address: Option<SocketAddr>,
    /// the identity of the peer, as a SecretKey
    secret_key: Option<SecretKey>,
    /// pre-loaded blobs store (if provided, blobs_store_path is ignored)
    blobs_store: Option<BlobsStore>,
    log_provider: Option<L>,
}

// TODO (amiller68): proper errors
impl<L: BucketLogProvider> PeerBuilder<L> {
    pub fn new() -> Self {
        PeerBuilder {
            socket_address: None,
            secret_key: None,
            blobs_store: None,
            log_provider: None,
        }
    }

    pub fn socket_address(mut self, socket_addr: SocketAddr) -> Self {
        self.socket_address = Some(socket_addr);
        self
    }

    pub fn secret_key(mut self, secret_key: SecretKey) -> Self {
        self.secret_key = Some(secret_key);
        self
    }

    pub fn blobs_store(mut self, blobs: BlobsStore) -> Self {
        self.blobs_store = Some(blobs);
        self
    }

    pub fn log_provider(mut self, log_provider: L) -> Self {
        self.log_provider = Some(log_provider);
        self
    }

    pub async fn build(self) -> Peer<L> {
        // set the socket port to unspecified if not set
        let socket_addr = self
            .socket_address
            .unwrap_or_else(|| SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0));
        // generate a new secret key if not set
        let secret_key = self.secret_key.unwrap_or_else(SecretKey::generate);

        // get the blobs store, if not set use in memory
        let blobs_store = match self.blobs_store {
            Some(blobs) => blobs,
            None => BlobsStore::memory().await.unwrap(),
        };

        // setup our discovery mechanism for our peer
        let mainline_discovery = DhtDiscovery::builder()
            .secret_key(secret_key.0.clone())
            .build()
            .expect("failed to build mainline discovery");

        // Convert the SocketAddr to a SocketAddrV4
        let addr = SocketAddrV4::new(
            socket_addr
                .ip()
                .to_string()
                .parse::<Ipv4Addr>()
                .expect("failed to parse IP address"),
            socket_addr.port(),
        );

        // Create the endpoint with our key and discovery
        let endpoint = Endpoint::builder()
            .secret_key(secret_key.0.clone())
            .discovery(mainline_discovery)
            .bind_addr_v4(addr)
            .bind()
            .await
            .expect("failed to bind ephemeral endpoint");

        // get the log provider, must be set
        let log_provider = self.log_provider.expect("log_provider is required");

        // Create the job dispatcher and receiver
        let (jobs, job_receiver) = super::jobs::JobDispatcher::new();

        Peer {
            log_provider,
            socket_address: socket_addr,
            blobs_store,
            secret_key,
            endpoint,
            jobs,
            job_receiver: Some(job_receiver),
        }
    }
}

/// Overview of a peer's state, generic over a bucket log provider.
///  Provides everything that a peer needs in order to
///  load data, interact with peers, and manage buckets.
#[derive(Debug)]
pub struct Peer<L: BucketLogProvider> {
    log_provider: L,
    socket_address: SocketAddr,
    blobs_store: BlobsStore,
    secret_key: SecretKey,
    endpoint: Endpoint,
    jobs: super::jobs::JobDispatcher,
    job_receiver: Option<super::jobs::JobReceiver>,
}

impl<L: BucketLogProvider> Clone for Peer<L>
where
    L: Clone,
{
    fn clone(&self) -> Self {
        Self {
            log_provider: self.log_provider.clone(),
            socket_address: self.socket_address,
            blobs_store: self.blobs_store.clone(),
            secret_key: self.secret_key.clone(),
            endpoint: self.endpoint.clone(),
            jobs: self.jobs.clone(),
            // JobReceiver cannot be cloned - only the original peer can spawn worker
            job_receiver: None,
        }
    }
}

impl<L: BucketLogProvider> Peer<L> {
    pub fn logs(&self) -> &L {
        &self.log_provider
    }

    pub fn blobs(&self) -> &BlobsStore {
        &self.blobs_store
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn secret(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn socket(&self) -> &SocketAddr {
        &self.socket_address
    }

    pub fn id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    pub fn jobs(&self) -> &super::jobs::JobDispatcher {
        &self.jobs
    }

    /// Extract the job receiver (internal use by peer::spawn)
    ///
    /// This can only be called once. Subsequent calls will return None.
    pub(super) fn take_job_receiver(&mut self) -> Option<super::jobs::JobReceiver> {
        self.job_receiver.take()
    }

    // ========================================
    // Peer-Centric Sync Functions
    // ========================================

    /// Sync a bucket from a peer
    ///
    /// This is the main entry point for syncing. It handles both cases:
    /// - Updating an existing bucket we already have
    /// - Cloning a new bucket we don't have yet
    pub async fn sync_from_peer(
        &self,
        bucket_id: Uuid,
        target_link: Link,
        target_height: u64,
        peer_id: &PublicKey,
    ) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        tracing::info!(
            "Syncing bucket {} from peer {} to link {:?} at height {}",
            bucket_id,
            peer_id.to_hex(),
            target_link,
            target_height
        );

        // Check if we have this bucket
        match self.log_provider.height(bucket_id).await {
            Ok(_height) => {
                // We have the bucket, sync it
                tracing::debug!("Bucket exists locally, performing update sync");
                self.sync_existing_bucket(bucket_id, target_link, peer_id)
                    .await
            }
            Err(crate::bucket_log::BucketLogError::HeadNotFound(_)) => {
                // We don't have the bucket, clone it
                tracing::debug!("Bucket not found locally, cloning from peer");
                self.clone_bucket_from_peer(bucket_id, target_link, peer_id)
                    .await
            }
            Err(e) => {
                tracing::error!("Error checking bucket height: {}", e);
                Err(anyhow!("Failed to check bucket height: {}", e))
            }
        }
    }

    /// Sync an existing bucket from a peer
    ///
    /// Downloads the manifest chain, finds common ancestor, and appends missing entries
    async fn sync_existing_bucket(
        &self,
        bucket_id: Uuid,
        target_link: Link,
        peer_id: &PublicKey,
    ) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        tracing::info!(
            "Syncing existing bucket {} to link {:?}",
            bucket_id,
            target_link
        );

        // Get our current state
        let our_height = self.log_provider.height(bucket_id).await?;
        let our_head = self.log_provider.head(bucket_id, our_height).await?;

        tracing::debug!(
            "Our current state: height={}, head={:?}",
            our_height,
            our_head
        );

        // Download manifest chain from peer (from target back to common ancestor)
        let manifests = self
            .download_manifest_chain(&target_link, peer_id, Some(&our_head))
            .await?;

        if manifests.is_empty() {
            tracing::info!("No new manifests to sync, already up to date");
            return Ok(());
        }

        tracing::info!("Downloaded {} manifests from peer", manifests.len());

        // Find common ancestor in our log
        let common_ancestor = self.find_common_ancestor(bucket_id, &manifests).await?;

        match common_ancestor {
            Some(ancestor_link) => {
                tracing::debug!("Found common ancestor: {:?}", ancestor_link);

                // Verify it's at our current head (single-hop update)
                if ancestor_link != our_head {
                    return Err(anyhow!(
                        "Fork detected: common ancestor {:?} != our head {:?}",
                        ancestor_link,
                        our_head
                    ));
                }

                // Append manifests that come after the ancestor
                self.apply_manifest_chain(bucket_id, &manifests, our_height)
                    .await?;

                tracing::info!("Successfully synced bucket {} from peer", bucket_id);
                Ok(())
            }
            None => {
                // No common ancestor found - this might be a fork or completely different chain
                Err(anyhow!(
                    "No common ancestor found for bucket {}. This might be a fork.",
                    bucket_id
                ))
            }
        }
    }

    /// Clone a bucket from a peer that we don't have locally
    ///
    /// Downloads the full manifest chain and verifies provenance before cloning
    async fn clone_bucket_from_peer(
        &self,
        bucket_id: Uuid,
        target_link: Link,
        peer_id: &PublicKey,
    ) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        tracing::info!(
            "Cloning bucket {} from peer {} with link {:?}",
            bucket_id,
            peer_id.to_hex(),
            target_link
        );

        // Download the full manifest chain (all the way to genesis)
        let manifests = self
            .download_manifest_chain(&target_link, peer_id, None)
            .await?;

        if manifests.is_empty() {
            return Err(anyhow!("No manifests downloaded from peer"));
        }

        tracing::info!("Downloaded {} manifests from peer", manifests.len());

        // Get the latest manifest to verify provenance
        let latest_manifest = manifests
            .last()
            .ok_or_else(|| anyhow!("No manifests in chain"))?;

        // Verify we're in the shares (provenance check)
        if !self.verify_provenance(&latest_manifest.0)? {
            return Err(anyhow!(
                "Provenance verification failed: our key not in bucket shares"
            ));
        }

        tracing::debug!("Provenance verified, our key is in bucket shares");

        // Append entire chain starting from genesis (height 0)
        self.apply_manifest_chain(bucket_id, &manifests, 0).await?;

        tracing::info!("Successfully cloned bucket {} from peer", bucket_id);
        Ok(())
    }

    /// Download a chain of manifests from a peer
    ///
    /// Walks backwards through the manifest chain via `previous` links.
    /// Stops when it reaches `stop_at` link (common ancestor) or genesis (no previous).
    ///
    /// Returns manifests in order from oldest to newest with their links and heights.
    async fn download_manifest_chain(
        &self,
        start_link: &Link,
        peer_id: &PublicKey,
        stop_at: Option<&Link>,
    ) -> Result<Vec<(Manifest, Link, u64)>> {
        tracing::debug!(
            "Downloading manifest chain from {:?}, stop_at: {:?}",
            start_link,
            stop_at
        );

        let mut manifests = Vec::new();
        let mut current_link = start_link.clone();
        let mut current_height = 0u64; // Will be calculated in reverse

        // Download manifests walking backwards
        loop {
            // Download the manifest blob from peer
            // Convert our PublicKey to iroh's PublicKey, then to NodeId
            let iroh_pub_key: iroh::PublicKey = (*peer_id).into();
            let node_id = NodeId::from(iroh_pub_key);
            self.blobs_store
                .download_hash(current_link.hash().clone(), vec![node_id], &self.endpoint)
                .await
                .map_err(|e| {
                    anyhow!(
                        "Failed to download manifest {:?} from peer: {}",
                        current_link,
                        e
                    )
                })?;

            // Read and decode the manifest
            let manifest: Manifest = self.blobs_store.get_cbor(&current_link.hash()).await?;

            // Store manifest with its link and height
            manifests.push((manifest.clone(), current_link.clone(), current_height));

            // Check if we should stop
            if let Some(stop_link) = stop_at {
                if &current_link == stop_link {
                    tracing::debug!("Reached stop_at link, stopping download");
                    break;
                }
            }

            // Check for previous link
            match manifest.previous() {
                Some(prev_link) => {
                    current_link = prev_link.clone();
                    current_height += 1;
                }
                None => {
                    // Reached genesis, stop
                    tracing::debug!("Reached genesis manifest, stopping download");
                    break;
                }
            }
        }

        // Reverse to get oldest-to-newest order and recalculate heights
        manifests.reverse();
        let base_height = if manifests.is_empty() {
            0
        } else {
            current_height
        };

        // Fix heights to be correct (oldest is lowest)
        for (i, (_, _, height)) in manifests.iter_mut().enumerate() {
            *height = base_height - i as u64;
        }

        tracing::debug!("Downloaded {} manifests", manifests.len());
        Ok(manifests)
    }

    /// Find common ancestor between peer's manifests and our local log
    ///
    /// Iterates through peer manifests from oldest to newest and checks if each
    /// link exists in our log. Returns the first (oldest) link found.
    async fn find_common_ancestor(
        &self,
        bucket_id: Uuid,
        peer_manifests: &[(Manifest, Link, u64)],
    ) -> Result<Option<Link>>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        tracing::debug!(
            "Finding common ancestor for {} peer manifests",
            peer_manifests.len()
        );

        for (_manifest, link, _height) in peer_manifests.iter() {
            // Check if this link exists in our log
            match self.log_provider.has(bucket_id, link.clone()).await {
                Ok(heights) if !heights.is_empty() => {
                    tracing::debug!("Found common ancestor at heights {:?}: {:?}", heights, link);
                    return Ok(Some(link.clone()));
                }
                Ok(_) => {
                    // Link not in our log, continue searching
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Error checking for link in log: {}", e);
                    continue;
                }
            }
        }

        // No common ancestor found
        tracing::debug!("No common ancestor found in peer manifests");
        Ok(None)
    }

    /// Apply a chain of manifests to the log
    ///
    /// Appends each manifest to the log in order, starting from `start_height`.
    async fn apply_manifest_chain(
        &self,
        bucket_id: Uuid,
        manifests: &[(Manifest, Link, u64)],
        start_height: u64,
    ) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        tracing::info!(
            "Applying {} manifests to log starting at height {}",
            manifests.len(),
            start_height
        );

        for (i, (manifest, link, _expected_height)) in manifests.iter().enumerate() {
            let height = start_height + i as u64 + 1;
            let previous = manifest.previous().clone();

            tracing::debug!(
                "Appending manifest to log: height={}, link={:?}, previous={:?}",
                height,
                link,
                previous
            );

            self.log_provider
                .append(
                    bucket_id,
                    manifest.name().to_string(),
                    link.clone(),
                    previous,
                    height,
                )
                .await
                .map_err(|e| anyhow!("Failed to append manifest at height {}: {}", height, e))?;
        }

        tracing::info!("Successfully applied {} manifests to log", manifests.len());
        Ok(())
    }

    /// Verify that our PublicKey is in the manifest's shares
    fn verify_provenance(&self, manifest: &Manifest) -> Result<bool> {
        let our_pub_key = PublicKey::from(*self.secret_key.public());
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

    // ========================================
    // Background Job Worker
    // ========================================

    /// Schedule periodic pings to peers for all our buckets
    ///
    /// This queries all buckets, extracts peer IDs from manifest shares,
    /// and dispatches ping jobs for each peer.
    async fn schedule_periodic_pings(&self)
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        // Get all bucket IDs
        let bucket_ids = match self.log_provider.list_buckets().await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!("Failed to list buckets for periodic pings: {}", e);
                return;
            }
        };

        tracing::debug!("Scheduling periodic pings for {} buckets", bucket_ids.len());

        // For each bucket, dispatch pings to peers in shares
        for bucket_id in bucket_ids {
            if let Err(e) = self.ping_bucket_peers(bucket_id).await {
                tracing::warn!(
                    "Failed to ping peers for bucket {}: {}",
                    bucket_id,
                    e
                );
            }
        }
    }

    /// Ping all peers listed in a bucket's manifest shares
    async fn ping_bucket_peers(&self, bucket_id: Uuid) -> Result<()>
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        // Get current head link
        let head_link = self.log_provider.current_head(bucket_id).await
            .map_err(|e| anyhow!("Failed to get head for bucket {}: {}", bucket_id, e))?;

        // Load manifest from blobs store
        let manifest_bytes = self.blobs_store.get(&head_link.hash()).await
            .map_err(|e| anyhow!("Failed to load manifest: {}", e))?;

        let manifest: crate::prelude::Manifest = serde_ipld_dagcbor::from_slice(&manifest_bytes)
            .map_err(|e| anyhow!("Failed to deserialize manifest: {}", e))?;

        // Extract our own key to skip ourselves
        let our_key = crate::crypto::PublicKey::from(*self.secret_key.public()).to_hex();

        // For each peer in shares, dispatch a ping job
        for (peer_key_hex, _share) in manifest.shares() {
            if peer_key_hex == &our_key {
                continue; // Skip ourselves
            }

            let peer_id = crate::crypto::PublicKey::from_hex(peer_key_hex)
                .map_err(|e| anyhow!("Invalid peer key in shares: {}", e))?;

            // Dispatch ping job
            if let Err(e) = self.jobs.dispatch(super::jobs::Job::PingPeer {
                bucket_id,
                peer_id,
            }) {
                tracing::warn!(
                    "Failed to dispatch ping job for bucket {} peer {}: {}",
                    bucket_id,
                    peer_key_hex,
                    e
                );
            }
        }

        Ok(())
    }

    /// Handle a ping peer job by sending a ping and processing the response
    async fn handle_ping_peer(&self, bucket_id: Uuid, peer_id: crate::crypto::PublicKey)
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        use super::protocol::{Ping, PingHandler};
        use super::protocol::bidirectional::BidirectionalHandler;

        // Get our bucket state
        let our_link = match self.log_provider.current_head(bucket_id).await {
            Ok(link) => link,
            Err(e) => {
                tracing::warn!(
                    "Failed to get head for bucket {} when pinging peer {}: {}",
                    bucket_id,
                    peer_id.to_hex(),
                    e
                );
                return;
            }
        };

        let our_height = match self.log_provider.height(bucket_id).await {
            Ok(height) => height,
            Err(e) => {
                tracing::warn!(
                    "Failed to get height for bucket {} when pinging peer {}: {}",
                    bucket_id,
                    peer_id.to_hex(),
                    e
                );
                return;
            }
        };

        // Construct ping
        let ping = Ping {
            bucket_id,
            link: our_link,
            height: our_height,
            requester_id: crate::crypto::PublicKey::from(*self.secret_key.public()),
        };

        // Convert peer_id to NodeAddr
        let iroh_node_id: iroh::NodeId = peer_id.into();
        let node_addr = iroh::NodeAddr::new(iroh_node_id);

        // Send ping
        match PingHandler::send_to_peer::<L>(&self.endpoint, &node_addr, ping).await {
            Ok(pong) => {
                tracing::debug!(
                    "Received pong from peer {} for bucket {}",
                    peer_id.to_hex(),
                    bucket_id
                );

                // Handle response (dispatches sync jobs if needed)
                if let Err(e) = PingHandler::handle_response(self, &pong).await {
                    tracing::error!(
                        "Failed to handle pong from peer {} for bucket {}: {}",
                        peer_id.to_hex(),
                        bucket_id,
                        e
                    );
                }
            }
            Err(e) => {
                tracing::debug!(
                    "Failed to ping peer {} for bucket {}: {}",
                    peer_id.to_hex(),
                    bucket_id,
                    e
                );
            }
        }
    }

    /// Run the background job worker
    ///
    /// This consumes the job receiver and processes jobs until the receiver is closed.
    /// Typically this should be spawned in a background task:
    ///
    /// ```ignore
    /// let (peer, job_receiver) = PeerBuilder::new()
    ///     .log_provider(database)
    ///     .build()
    ///     .await;
    ///
    /// // Spawn the worker
    /// tokio::spawn(async move {
    ///     peer.clone().run_worker(job_receiver).await;
    /// });
    /// ```
    pub async fn run_worker(self, job_receiver: super::jobs::JobReceiver)
    where
        L::Error: std::error::Error + Send + Sync + 'static,
    {
        use super::jobs::Job;
        use futures::StreamExt;
        use tokio::time::{interval, Duration};

        tracing::info!("Starting background job worker for peer {}", self.id());

        // Convert to async stream for efficient async processing
        let mut stream = job_receiver.into_async();

        // Create interval timer for periodic pings (every 60 seconds)
        let mut ping_interval = interval(Duration::from_secs(60));
        ping_interval.tick().await; // Skip first immediate tick

        loop {
            tokio::select! {
                // Process incoming jobs from the queue
                Some(job) = stream.next() => {
                    match job {
                        Job::SyncBucket {
                            bucket_id,
                            target_link,
                            target_height,
                            peer_id,
                        } => {
                            tracing::info!(
                                "Processing sync job: bucket_id={}, peer_id={}, height={}",
                                bucket_id,
                                peer_id.to_hex(),
                                target_height
                            );

                            if let Err(e) = self
                                .sync_from_peer(bucket_id, target_link, target_height, &peer_id)
                                .await
                            {
                                tracing::error!(
                                    "Sync job failed for bucket {} from peer {}: {}",
                                    bucket_id,
                                    peer_id.to_hex(),
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Sync job completed successfully for bucket {} from peer {}",
                                    bucket_id,
                                    peer_id.to_hex()
                                );
                            }
                        }
                        Job::PingPeer { bucket_id, peer_id } => {
                            tracing::debug!(
                                "Processing ping job: bucket_id={}, peer_id={}",
                                bucket_id,
                                peer_id.to_hex()
                            );
                            self.handle_ping_peer(bucket_id, peer_id).await;
                        }
                    }
                }

                // Periodic ping scheduler
                _ = ping_interval.tick() => {
                    tracing::debug!("Running periodic ping scheduler");
                    self.schedule_periodic_pings().await;
                }

                // Stream closed (all senders dropped)
                else => {
                    tracing::info!("Job queue closed, shutting down worker");
                    break;
                }
            }
        }

        tracing::info!("Background job worker shutting down for peer {}", self.id());
    }
}
