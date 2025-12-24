use std::path::PathBuf;

use url::Url;

use super::config::Config;
use crate::daemon::database::{Database, DatabaseSetupError};
use crate::state::{BlobStoreConfig, BLOBS_DIR_NAME};

use common::crypto::SecretKey;
use common::peer::{BlobsStore, Peer, PeerBuilder};

use super::sync_provider::{QueuedSyncConfig, QueuedSyncProvider};

/// Main service state - orchestrates all components
#[derive(Clone)]
pub struct State {
    database: Database,
    peer: Peer<Database>,
}

impl State {
    pub async fn from_config(config: &Config) -> Result<Self, StateSetupError> {
        // 1. Setup database
        let sqlite_database_url = match config.sqlite_path {
            Some(ref path) => {
                // check that the path exists
                if !path.exists() {
                    return Err(StateSetupError::DatabasePathDoesNotExist);
                }
                // parse the path into a URL
                Url::parse(&format!("sqlite://{}", path.display()))
                    .map_err(|_| StateSetupError::InvalidDatabaseUrl)
            }
            // otherwise just set up an in-memory database
            None => Url::parse("sqlite::memory:").map_err(|_| StateSetupError::InvalidDatabaseUrl),
        }?;
        tracing::info!("Database URL: {:?}", sqlite_database_url);
        let database = Database::connect(&sqlite_database_url).await?;

        // 2. Setup node secret
        let node_secret = config
            .node_secret
            .clone()
            .unwrap_or_else(SecretKey::generate);

        // 3. Setup blobs store based on configuration
        tracing::debug!("ServiceState::from_config - loading blobs store");
        let blobs = Self::setup_blobs_store(&config.blob_store, config.jax_dir.as_ref()).await?;
        tracing::debug!("ServiceState::from_config - blobs store loaded successfully");

        // 4. Build peer from the database as the log provider
        // TODO: Make queue size configurable via config

        // Create sync provider with worker
        let (sync_provider, job_receiver) = QueuedSyncProvider::new(QueuedSyncConfig::default());

        let mut peer_builder = PeerBuilder::new()
            .with_sync_provider(std::sync::Arc::new(sync_provider))
            .log_provider(database.clone())
            .blobs_store(blobs.clone())
            .secret_key(node_secret.clone());

        if let Some(addr) = config.node_listen_addr {
            peer_builder = peer_builder.socket_address(addr);
        }

        let peer = peer_builder.build().await;

        // Log the bound addresses
        let bound_addrs = peer.endpoint().bound_sockets();
        tracing::info!("Node id: {} (with JAX protocol)", peer.id());
        tracing::info!("Peer listening on: {:?}", bound_addrs);

        // Spawn the worker for the queued sync provider
        // The worker is managed outside the peer, like the database
        let peer_for_worker = peer.clone();
        let job_stream = job_receiver.into_async();
        tokio::spawn(async move {
            super::sync_provider::run_worker(peer_for_worker, job_stream).await;
        });

        Ok(Self { database, peer })
    }

    /// Setup the blobs store based on configuration
    async fn setup_blobs_store(
        blob_store_config: &BlobStoreConfig,
        jax_dir: Option<&PathBuf>,
    ) -> Result<BlobsStore, StateSetupError> {
        match blob_store_config {
            BlobStoreConfig::Legacy => {
                // Legacy mode: use iroh-blobs FsStore
                let blobs_path = jax_dir
                    .map(|dir| dir.join(BLOBS_DIR_NAME))
                    .unwrap_or_else(|| {
                        let temp_dir =
                            tempfile::tempdir().expect("failed to create temporary directory");
                        temp_dir.path().to_path_buf()
                    });
                tracing::info!("Using legacy FsStore at {:?}", blobs_path);
                BlobsStore::fs(&blobs_path)
                    .await
                    .map_err(|e| StateSetupError::BlobsStoreError(e.to_string()))
            }
            BlobStoreConfig::Filesystem { path } => {
                // Filesystem mode: use jax-blobs-store with local filesystem
                // For now, fall back to legacy FsStore until full integration
                let blobs_path = path.clone().unwrap_or_else(|| {
                    jax_dir
                        .map(|dir| dir.join(BLOBS_DIR_NAME))
                        .unwrap_or_else(|| {
                            let temp_dir =
                                tempfile::tempdir().expect("failed to create temporary directory");
                            temp_dir.path().to_path_buf()
                        })
                });
                tracing::info!(
                    "Filesystem blob store configured at {:?} (using legacy FsStore until full integration)",
                    blobs_path
                );
                // TODO: Use blobs_store::BlobStore::new_local() once integrated with iroh-blobs protocol
                BlobsStore::fs(&blobs_path)
                    .await
                    .map_err(|e| StateSetupError::BlobsStoreError(e.to_string()))
            }
            BlobStoreConfig::S3 {
                endpoint,
                access_key: _,
                secret_key: _,
                bucket,
                region,
                db_path: _,
            } => {
                // S3 mode: use jax-blobs-store with S3-compatible storage
                // For now, fall back to in-memory store until full integration
                tracing::warn!(
                    "S3 blob store configured (endpoint={}, bucket={}, region={:?}) but not yet fully integrated - using in-memory store",
                    endpoint,
                    bucket,
                    region
                );
                // TODO: Use blobs_store::BlobStore::new() with ObjectStoreConfig once integrated
                BlobsStore::memory()
                    .await
                    .map_err(|e| StateSetupError::BlobsStoreError(e.to_string()))
            }
        }
    }

    pub fn peer(&self) -> &Peer<Database> {
        &self.peer
    }

    pub fn node(&self) -> &Peer<Database> {
        // Alias for backwards compatibility
        &self.peer
    }

    pub fn database(&self) -> &Database {
        &self.database
    }
}

impl AsRef<Peer<Database>> for State {
    fn as_ref(&self) -> &Peer<Database> {
        &self.peer
    }
}

impl AsRef<Database> for State {
    fn as_ref(&self) -> &Database {
        self.database()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StateSetupError {
    #[error("Database path does not exist")]
    DatabasePathDoesNotExist,
    #[error("Database setup error")]
    DatabaseSetupError(#[from] DatabaseSetupError),
    #[error("Invalid database URL")]
    InvalidDatabaseUrl,
    #[error("Blobs store error: {0}")]
    BlobsStoreError(String),
}
