use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use iroh::discovery::pkarr::dht::DhtDiscovery;
use iroh::Endpoint;

pub use super::blobs_store::BlobsStore;

use crate::bucket_log::BucketLogProvider;
use crate::crypto::SecretKey;

use super::peer_inner::Peer;
use super::sync::SyncProvider;

/// Marker type for builder state: needs sync provider to be configured
pub struct NeedsSyncProvider;

/// Marker type for builder state: ready to build
pub struct ReadyToBuild;

/// Peer builder with typestate pattern for compile-time enforcement
///
/// The builder enforces that a sync provider is configured before building.
#[derive(Clone)]
pub struct PeerBuilder<L: BucketLogProvider, State = ReadyToBuild> {
    /// the socket addr to expose the peer on
    ///  if not set, an ephemeral port will be used
    socket_address: Option<SocketAddr>,
    /// the identity of the peer, as a SecretKey
    secret_key: Option<SecretKey>,
    /// pre-loaded blobs store (if provided, blobs_store_path is ignored)
    blobs_store: Option<BlobsStore>,
    log_provider: Option<L>,
    /// Sync provider implementation (trait object for flexibility)
    sync_provider: Option<Arc<dyn SyncProvider<L>>>,
    /// State marker (zero-sized type for compile-time guarantees)
    _state: std::marker::PhantomData<State>,
}

// Common builder methods available in all states
impl<L: BucketLogProvider, State> PeerBuilder<L, State> {
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
}

// Initial construction - starts in NeedsSyncProvider state for explicit configuration
impl<L: BucketLogProvider> Default for PeerBuilder<L, NeedsSyncProvider> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: BucketLogProvider> PeerBuilder<L, NeedsSyncProvider> {
    /// Create a new peer builder
    ///
    /// You must call either `with_queued_sync()` or `with_sync_provider()` before building.
    pub fn new() -> Self {
        PeerBuilder {
            socket_address: None,
            secret_key: None,
            blobs_store: None,
            log_provider: None,
            sync_provider: None,
            _state: std::marker::PhantomData,
        }
    }

    /// Configure with a custom SyncProvider implementation
    ///
    /// This allows injecting custom sync providers for testing or alternative implementations.
    pub fn with_sync_provider(
        mut self,
        provider: Arc<dyn SyncProvider<L>>,
    ) -> PeerBuilder<L, ReadyToBuild> {
        self.sync_provider = Some(provider);

        PeerBuilder {
            socket_address: self.socket_address,
            secret_key: self.secret_key,
            blobs_store: self.blobs_store,
            log_provider: self.log_provider,
            sync_provider: self.sync_provider,
            _state: std::marker::PhantomData,
        }
    }
}

// Build only available in ReadyToBuild state
impl<L: BucketLogProvider> PeerBuilder<L, ReadyToBuild> {
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

        let sync_provider = self.sync_provider.expect("sync_provider must be set");

        Peer::new(
            log_provider,
            socket_addr,
            blobs_store,
            secret_key,
            endpoint,
            sync_provider,
        )
    }
}
