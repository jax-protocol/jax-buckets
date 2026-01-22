//! S3Store - iroh-blobs Store implementation backed by SQLite + S3.
//!
//! This module provides an iroh-blobs compatible store that uses our BlobStore
//! (SQLite metadata + S3/MinIO/local object storage) as the backend.

use std::ops::Deref;
use std::path::Path;

use iroh_blobs::api::proto::Command;

use crate::actor::S3Actor;
use crate::error::Result;
use crate::object_store::ObjectStoreConfig;
use crate::store::BlobStore;

/// Type alias for the irpc client
type ApiClient = irpc::Client<iroh_blobs::api::proto::Request>;

/// S3Store provides an iroh-blobs compatible store backed by SQLite + S3.
///
/// This store can be used with iroh-blobs' BlobsProtocol to enable P2P sync
/// while storing blobs in S3/MinIO/object storage.
///
/// # Example
///
/// ```rust,no_run
/// use jax_blobs_store::{S3Store, ObjectStoreConfig};
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a store with local filesystem storage
/// let store = S3Store::new_local(Path::new("/tmp/blobs")).await?;
///
/// // Convert to iroh_blobs::api::Store for use with BlobsProtocol
/// let iroh_store: iroh_blobs::api::Store = store.into();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct S3Store {
    client: ApiClient,
}

impl S3Store {
    /// Create a new S3Store with the given configuration.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `config` - Object storage configuration (S3, MinIO, local, or memory)
    pub async fn new(db_path: &Path, config: ObjectStoreConfig) -> Result<Self> {
        let store = BlobStore::new(db_path, config).await?;
        Ok(Self::from_blob_store(store))
    }

    /// Create a new S3Store backed by local filesystem.
    ///
    /// This creates both SQLite DB and object storage in the given directory.
    ///
    /// # Arguments
    /// * `data_dir` - Directory for all storage (db at data_dir/blobs.db, objects at data_dir/objects/)
    pub async fn new_local(data_dir: &Path) -> Result<Self> {
        let store = BlobStore::new_local(data_dir).await?;
        Ok(Self::from_blob_store(store))
    }

    /// Create a new S3Store with S3/MinIO storage.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `endpoint` - S3 endpoint URL (e.g., "http://localhost:9000" for MinIO)
    /// * `access_key` - S3 access key ID
    /// * `secret_key` - S3 secret access key
    /// * `bucket` - S3 bucket name
    /// * `region` - Optional S3 region (defaults to "us-east-1")
    pub async fn new_s3(
        db_path: &Path,
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
        region: Option<&str>,
    ) -> Result<Self> {
        let config = ObjectStoreConfig::S3 {
            endpoint: endpoint.to_string(),
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            bucket: bucket.to_string(),
            region: region.map(|s| s.to_string()),
        };
        Self::new(db_path, config).await
    }

    /// Create a fully ephemeral S3Store (in-memory DB + in-memory object storage).
    ///
    /// Data will be lost when the S3Store is dropped. Useful for testing.
    pub async fn new_ephemeral() -> Result<Self> {
        let store = BlobStore::new_ephemeral().await?;
        Ok(Self::from_blob_store(store))
    }

    /// Create an S3Store from an existing BlobStore.
    fn from_blob_store(store: BlobStore) -> Self {
        // Create channel for commands
        let (tx, rx) = tokio::sync::mpsc::channel::<Command>(256);

        // Spawn actor
        let actor = S3Actor::new(store, rx);
        tokio::spawn(actor.run());

        // Create client from sender
        let client: ApiClient = tx.into();
        Self { client }
    }

    /// Get the underlying ApiClient.
    pub fn client(&self) -> &ApiClient {
        &self.client
    }

    /// Convert to an iroh_blobs::api::Store.
    ///
    /// This method uses unsafe transmute because:
    /// - Store is repr(transparent) over ApiClient
    /// - ApiClient = irpc::Client<proto::Request>
    /// - Our client field is the same type
    /// - Therefore the memory layouts are identical
    pub fn into_iroh_store(self) -> iroh_blobs::api::Store {
        // SAFETY: iroh_blobs::api::Store is repr(transparent) over ApiClient
        // and our S3Store::client is of type ApiClient. Since Store wraps
        // ApiClient with repr(transparent), they have the same memory layout.
        unsafe { std::mem::transmute::<ApiClient, iroh_blobs::api::Store>(self.client) }
    }

    /// Get a reference to the store as iroh_blobs::api::Store.
    ///
    /// This method uses unsafe transmute for the same reasons as into_iroh_store.
    pub fn as_iroh_store(&self) -> &iroh_blobs::api::Store {
        // SAFETY: Same reasoning as into_iroh_store - Store is repr(transparent)
        // over ApiClient, so &ApiClient can be safely reinterpreted as &Store.
        unsafe { std::mem::transmute::<&ApiClient, &iroh_blobs::api::Store>(&self.client) }
    }
}

/// Convert S3Store to iroh_blobs::api::Store.
///
/// This allows S3Store to be used with BlobsProtocol for P2P sync.
impl From<S3Store> for iroh_blobs::api::Store {
    fn from(value: S3Store) -> Self {
        value.into_iroh_store()
    }
}

/// Get a reference to the iroh_blobs::api::Store.
impl AsRef<iroh_blobs::api::Store> for S3Store {
    fn as_ref(&self) -> &iroh_blobs::api::Store {
        self.as_iroh_store()
    }
}

/// Deref to iroh_blobs::api::Store for convenient API access.
impl Deref for S3Store {
    type Target = iroh_blobs::api::Store;

    fn deref(&self) -> &Self::Target {
        self.as_iroh_store()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh_blobs::api::blobs::BlobStatus;

    #[tokio::test]
    async fn test_ephemeral_store() {
        let store = S3Store::new_ephemeral().await.unwrap();

        // Test basic operations through the iroh-blobs API
        let data = b"hello world".to_vec();
        let tt = store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        // Check status
        let status = store.status(hash).await.unwrap();
        assert!(matches!(status, BlobStatus::Complete { size: 11 }));

        // Get bytes back
        let retrieved = store.get_bytes(hash).await.unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_local_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = S3Store::new_local(temp_dir.path()).await.unwrap();

        let data = b"test local storage".to_vec();
        let tt = store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        let status = store.status(hash).await.unwrap();
        assert!(matches!(status, BlobStatus::Complete { .. }));

        let retrieved = store.get_bytes(hash).await.unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_list_blobs() {
        let store = S3Store::new_ephemeral().await.unwrap();

        // Add some data
        let _tt1 = store
            .add_bytes(b"blob one".to_vec())
            .temp_tag()
            .await
            .unwrap();
        let _tt2 = store
            .add_bytes(b"blob two".to_vec())
            .temp_tag()
            .await
            .unwrap();

        // List blobs
        use n0_future::StreamExt;
        let stream = store.list().stream().await.unwrap();
        let blobs: Vec<_> = stream.collect().await;
        assert_eq!(blobs.len(), 2);
    }

    #[tokio::test]
    async fn test_tags() {
        let store = S3Store::new_ephemeral().await.unwrap();

        let data = b"tagged blob".to_vec();
        let tt = store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        // Create a tag
        let tag = store.tags().create(tt.hash_and_format()).await.unwrap();

        // List tags
        use n0_future::StreamExt;
        let stream = store.tags().list().await.unwrap();
        let tags: Vec<_> = stream.collect().await;
        assert_eq!(tags.len(), 1);
        // Results are wrapped in Result, unwrap them
        let first_tag = tags[0].as_ref().unwrap();
        assert_eq!(first_tag.name, tag);
        assert_eq!(first_tag.hash, hash);
    }

    #[tokio::test]
    async fn test_convert_to_iroh_store() {
        let s3_store = S3Store::new_ephemeral().await.unwrap();

        // Convert to iroh_blobs::api::Store
        let iroh_store: iroh_blobs::api::Store = s3_store.into();

        // Use through iroh-blobs API
        let data = b"test via iroh store".to_vec();
        let tt = iroh_store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        let retrieved = iroh_store.get_bytes(hash).await.unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }
}
