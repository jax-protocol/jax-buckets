//! ObjectStore - unified iroh-blobs Store implementation backed by SQLite + object storage.
//!
//! This module merges the BlobStore (content-addressed storage with SQLite metadata)
//! and the iroh-blobs Store adapter into a single type. It provides both direct
//! constructors and conversion to iroh_blobs::api::Store for P2P sync.

use std::ops::Deref;
use std::path::Path;

use bytes::Bytes;
use iroh_blobs::api::proto::Command;
use iroh_blobs::Hash;
use tracing::{debug, info, warn};

use crate::actor::{ObjectStoreActor, DEFAULT_MAX_IMPORT_SIZE};
use crate::database::{BlobState, Database};
use crate::error::Result;
use crate::storage::{ObjectStoreConfig, Storage};

/// Size threshold for generating BAO outboard data (16KB).
/// Blobs larger than this will have outboard verification data stored separately.
const OUTBOARD_THRESHOLD: usize = 16 * 1024;

/// Block size for BAO tree operations (matches iroh-blobs IROH_BLOCK_SIZE).
const IROH_BLOCK_SIZE: bao_tree::BlockSize = bao_tree::BlockSize::from_chunk_log(4);

/// Type alias for the irpc client
type ApiClient = irpc::Client<iroh_blobs::api::proto::Request>;

/// Internal BlobStore combining SQLite metadata with object storage.
///
/// This is used internally by ObjectStore and the actor.
#[derive(Debug, Clone)]
pub(crate) struct BlobStore {
    db: Database,
    storage: Storage,
}

impl BlobStore {
    /// Create a new BlobStore with a file-based SQLite database.
    pub async fn new(db_path: &Path, config: ObjectStoreConfig) -> Result<Self> {
        let db = Database::new(db_path).await?;
        let storage = Storage::new(config).await?;
        Ok(Self { db, storage })
    }

    /// Create a new BlobStore with an in-memory SQLite database.
    pub async fn in_memory(config: ObjectStoreConfig) -> Result<Self> {
        let db = Database::in_memory().await?;
        let storage = Storage::new(config).await?;
        Ok(Self { db, storage })
    }

    /// Create a new BlobStore backed by local filesystem.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `objects_path` - Directory for object storage
    pub async fn new_local(db_path: &Path, objects_path: &Path) -> Result<Self> {
        let config = ObjectStoreConfig::Local {
            path: objects_path.to_path_buf(),
        };
        Self::new(db_path, config).await
    }

    /// Create a fully ephemeral BlobStore (in-memory DB + in-memory object storage).
    pub async fn new_ephemeral() -> Result<Self> {
        Self::in_memory(ObjectStoreConfig::Memory).await
    }

    /// Close the database connection pool.
    #[allow(dead_code)]
    #[cfg(test)]
    pub async fn close(&self) {
        self.db.close().await;
    }

    /// Store data and return its content hash.
    ///
    /// For blobs larger than the outboard threshold, BAO outboard data is
    /// computed and stored alongside the blob data for verified streaming.
    pub async fn put(&self, data: Vec<u8>) -> Result<Hash> {
        let size = data.len();
        let hash = Hash::new(&data);
        let hash_str = hash.to_string();

        debug!(hash = %hash_str, size = size, "storing blob");

        let has_outboard = size > OUTBOARD_THRESHOLD;
        if has_outboard {
            let outboard =
                bao_tree::io::outboard::PreOrderMemOutboard::create(&data, IROH_BLOCK_SIZE);
            self.storage
                .put_outboard(&hash_str, Bytes::from(outboard.data))
                .await?;
        }
        self.storage.put_data(&hash_str, Bytes::from(data)).await?;
        self.db
            .insert_blob(&hash_str, size as i64, has_outboard)
            .await?;

        info!(hash = %hash_str, size = size, "blob stored successfully");
        Ok(hash)
    }

    /// Retrieve blob data by hash.
    pub async fn get(&self, hash: &Hash) -> Result<Option<Bytes>> {
        let hash_str = hash.to_string();
        if !self.db.has_blob(&hash_str).await? {
            return Ok(None);
        }
        self.storage.get_data(&hash_str).await
    }

    /// Delete a blob from the store.
    pub async fn delete(&self, hash: &Hash) -> Result<bool> {
        let hash_str = hash.to_string();

        let metadata = self.db.get_blob(&hash_str).await?;
        if metadata.is_none() {
            return Ok(false);
        }

        let metadata = metadata.unwrap();
        self.storage.delete_data(&hash_str).await?;
        if metadata.has_outboard {
            self.storage.delete_outboard(&hash_str).await?;
        }
        self.db.delete_blob(&hash_str).await?;

        info!(hash = %hash_str, "blob deleted");
        Ok(true)
    }

    /// Store data with pre-computed outboard and return its content hash.
    ///
    /// Used by import_bao which already has outboard data from the BAO stream.
    pub async fn put_with_outboard(&self, data: Vec<u8>, outboard: Vec<u8>) -> Result<Hash> {
        let size = data.len();
        let hash = Hash::new(&data);
        let hash_str = hash.to_string();

        debug!(hash = %hash_str, size = size, "storing blob with outboard");

        let has_outboard = !outboard.is_empty();
        self.storage.put_data(&hash_str, Bytes::from(data)).await?;
        if has_outboard {
            self.storage
                .put_outboard(&hash_str, Bytes::from(outboard))
                .await?;
        }
        self.db
            .insert_blob(&hash_str, size as i64, has_outboard)
            .await?;

        info!(hash = %hash_str, size = size, "blob with outboard stored successfully");
        Ok(hash)
    }

    /// Insert a partial blob record (import in progress).
    pub async fn insert_partial(&self, hash: &Hash, size: u64) -> Result<()> {
        let hash_str = hash.to_string();
        let has_outboard = size > OUTBOARD_THRESHOLD as u64;
        self.db
            .insert_partial_blob(&hash_str, size as i64, has_outboard)
            .await?;
        debug!(hash = %hash_str, size = size, "partial blob record created");
        Ok(())
    }

    /// Get the state of a blob (Complete, Partial, or None if not found).
    pub async fn get_state(&self, hash: &Hash) -> Result<Option<BlobState>> {
        let hash_str = hash.to_string();
        self.db.get_blob_state(&hash_str).await
    }

    /// List all blob hashes in the store.
    pub async fn list(&self) -> Result<Vec<Hash>> {
        let hash_strings = self.db.list_blobs().await?;
        let mut hashes = Vec::with_capacity(hash_strings.len());

        for s in hash_strings {
            match s.parse::<Hash>() {
                Ok(h) => hashes.push(h),
                Err(_) => {
                    warn!(hash = %s, "invalid hash in database, skipping");
                }
            }
        }

        Ok(hashes)
    }
}

/// ObjectStore provides an iroh-blobs compatible store backed by SQLite + object storage.
///
/// This store can be used with iroh-blobs' BlobsProtocol to enable P2P sync
/// while storing blobs in S3/MinIO/local filesystem/memory.
///
/// # Example
///
/// ```rust,no_run
/// use jax_object_store::ObjectStore;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a store with local filesystem storage
/// let store = ObjectStore::new_local(
///     Path::new("/tmp/blobs.db"),
///     Path::new("/tmp/blobs/objects"),
///     None, // use default max import size
/// ).await?;
///
/// // Convert to iroh_blobs::api::Store for use with BlobsProtocol
/// let iroh_store: iroh_blobs::api::Store = store.into();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ObjectStore {
    client: ApiClient,
}

impl ObjectStore {
    /// Create a new ObjectStore with the given configuration.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `config` - Object storage configuration (S3, MinIO, local, or memory)
    /// * `max_import_size` - Maximum blob size for BAO imports, or None for default (1GB)
    pub async fn new(
        db_path: &Path,
        config: ObjectStoreConfig,
        max_import_size: Option<u64>,
    ) -> Result<Self> {
        let store = BlobStore::new(db_path, config).await?;
        Ok(Self::from_blob_store(
            store,
            max_import_size.unwrap_or(DEFAULT_MAX_IMPORT_SIZE),
        ))
    }

    /// Create a new ObjectStore backed by local filesystem.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `objects_path` - Directory for object storage
    /// * `max_import_size` - Maximum blob size for BAO imports, or None for default (1GB)
    pub async fn new_local(
        db_path: &Path,
        objects_path: &Path,
        max_import_size: Option<u64>,
    ) -> Result<Self> {
        let store = BlobStore::new_local(db_path, objects_path).await?;
        Ok(Self::from_blob_store(
            store,
            max_import_size.unwrap_or(DEFAULT_MAX_IMPORT_SIZE),
        ))
    }

    /// Create a new ObjectStore with S3/MinIO storage.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `endpoint` - S3 endpoint URL (e.g., "http://localhost:9000" for MinIO)
    /// * `access_key` - S3 access key ID
    /// * `secret_key` - S3 secret access key
    /// * `bucket` - S3 bucket name
    /// * `region` - Optional S3 region (defaults to "us-east-1")
    /// * `max_import_size` - Maximum blob size for BAO imports, or None for default (1GB)
    pub async fn new_s3(
        db_path: &Path,
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
        region: Option<&str>,
        max_import_size: Option<u64>,
    ) -> Result<Self> {
        let config = ObjectStoreConfig::S3 {
            endpoint: endpoint.to_string(),
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
            bucket: bucket.to_string(),
            region: region.map(|s| s.to_string()),
        };
        Self::new(db_path, config, max_import_size).await
    }

    /// Create a fully ephemeral ObjectStore (in-memory DB + in-memory object storage).
    ///
    /// Data will be lost when the ObjectStore is dropped. Useful for testing.
    pub async fn new_ephemeral() -> Result<Self> {
        let store = BlobStore::new_ephemeral().await?;
        Ok(Self::from_blob_store(store, DEFAULT_MAX_IMPORT_SIZE))
    }

    /// Create an ObjectStore from an existing BlobStore.
    fn from_blob_store(store: BlobStore, max_import_size: u64) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel::<Command>(256);
        let actor = ObjectStoreActor::new(store, rx, max_import_size);
        tokio::spawn(actor.run());
        let client: ApiClient = tx.into();
        Self { client }
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
        // and our ObjectStore::client is of type ApiClient. Since Store wraps
        // ApiClient with repr(transparent), they have the same memory layout.
        unsafe { std::mem::transmute::<ApiClient, iroh_blobs::api::Store>(self.client) }
    }

    /// Get a reference to the store as iroh_blobs::api::Store.
    fn as_iroh_store(&self) -> &iroh_blobs::api::Store {
        // SAFETY: Same reasoning as into_iroh_store - Store is repr(transparent)
        // over ApiClient, so &ApiClient can be safely reinterpreted as &Store.
        unsafe { std::mem::transmute::<&ApiClient, &iroh_blobs::api::Store>(&self.client) }
    }
}

/// Convert ObjectStore to iroh_blobs::api::Store.
///
/// This allows ObjectStore to be used with BlobsProtocol for P2P sync.
impl From<ObjectStore> for iroh_blobs::api::Store {
    fn from(value: ObjectStore) -> Self {
        value.into_iroh_store()
    }
}

/// Deref to iroh_blobs::api::Store for convenient API access.
impl Deref for ObjectStore {
    type Target = iroh_blobs::api::Store;

    fn deref(&self) -> &Self::Target {
        self.as_iroh_store()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh_blobs::api::blobs::BlobStatus;

    /// Statistics from a recovery operation.
    #[derive(Debug, Default)]
    struct RecoveryStats {
        found: usize,
        added: usize,
        existing: usize,
        errors: usize,
    }

    /// Test-only methods on BlobStore for verifying internal state.
    impl BlobStore {
        async fn get_outboard(&self, hash: &Hash) -> Result<Option<Bytes>> {
            let hash_str = hash.to_string();
            self.storage.get_outboard(&hash_str).await
        }

        async fn has(&self, hash: &Hash) -> Result<bool> {
            let hash_str = hash.to_string();
            self.db.has_blob(&hash_str).await
        }

        async fn count(&self) -> Result<u64> {
            let count = self.db.count_blobs().await?;
            Ok(count as u64)
        }

        async fn total_size(&self) -> Result<u64> {
            let size = self.db.total_size().await?;
            Ok(size as u64)
        }

        async fn recover_from_storage(&self) -> Result<RecoveryStats> {
            use futures::TryStreamExt;

            let mut stats = RecoveryStats::default();
            let mut stream = std::pin::pin!(self.storage.list_data_hashes_stream());

            while let Some(hash_str) = stream.try_next().await? {
                stats.found += 1;

                if stats.found % 1000 == 0 {
                    info!(
                        found = stats.found,
                        added = stats.added,
                        "recovery progress"
                    );
                }

                if self.db.has_blob(&hash_str).await? {
                    stats.existing += 1;
                    continue;
                }

                match self.storage.get_data(&hash_str).await {
                    Ok(Some(data)) => {
                        let size = data.len();
                        let has_outboard = size > OUTBOARD_THRESHOLD;

                        if let Err(e) = self
                            .db
                            .insert_blob(&hash_str, size as i64, has_outboard)
                            .await
                        {
                            warn!(hash = %hash_str, error = %e, "failed to insert recovered blob metadata");
                            stats.errors += 1;
                        } else {
                            debug!(hash = %hash_str, size = size, "recovered blob metadata");
                            stats.added += 1;
                        }
                    }
                    Ok(None) => {
                        warn!(hash = %hash_str, "blob listed but not found in storage");
                        stats.errors += 1;
                    }
                    Err(e) => {
                        warn!(hash = %hash_str, error = %e, "failed to read blob during recovery");
                        stats.errors += 1;
                    }
                }
            }

            Ok(stats)
        }
    }

    #[tokio::test]
    async fn test_ephemeral_store() {
        let store = ObjectStore::new_ephemeral().await.unwrap();

        let data = b"hello world".to_vec();
        let tt = store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        let status = store.status(hash).await.unwrap();
        assert!(matches!(status, BlobStatus::Complete { size: 11 }));

        let retrieved = store.get_bytes(hash).await.unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_local_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("blobs.db");
        let objects_path = temp_dir.path().join("objects");
        let store = ObjectStore::new_local(&db_path, &objects_path, None)
            .await
            .unwrap();

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
        let store = ObjectStore::new_ephemeral().await.unwrap();

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

        use n0_future::StreamExt;
        let stream = store.list().stream().await.unwrap();
        let blobs: Vec<_> = stream.collect().await;
        assert_eq!(blobs.len(), 2);
    }

    #[tokio::test]
    async fn test_tags() {
        let store = ObjectStore::new_ephemeral().await.unwrap();

        let data = b"tagged blob".to_vec();
        let tt = store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        let tag = store.tags().create(tt.hash_and_format()).await.unwrap();

        use n0_future::StreamExt;
        let stream = store.tags().list().await.unwrap();
        let tags: Vec<_> = stream.collect().await;
        assert_eq!(tags.len(), 1);
        let first_tag = tags[0].as_ref().unwrap();
        assert_eq!(first_tag.name, tag);
        assert_eq!(first_tag.hash, hash);
    }

    #[tokio::test]
    async fn test_convert_to_iroh_store() {
        let obj_store = ObjectStore::new_ephemeral().await.unwrap();

        let iroh_store: iroh_blobs::api::Store = obj_store.into();

        let data = b"test via iroh store".to_vec();
        let tt = iroh_store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        let retrieved = iroh_store.get_bytes(hash).await.unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_blob_store_ephemeral() {
        let store = BlobStore::new_ephemeral().await.unwrap();

        let data = b"hello world".to_vec();
        let hash = store.put(data.clone()).await.unwrap();

        assert!(store.has(&hash).await.unwrap());

        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());

        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], hash);

        assert_eq!(store.count().await.unwrap(), 1);
        assert_eq!(store.total_size().await.unwrap(), data.len() as u64);

        assert!(store.delete(&hash).await.unwrap());
        assert!(!store.has(&hash).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_blob_store_local() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("blobs.db");
        let objects_path = temp_dir.path().join("objects");
        let store = BlobStore::new_local(&db_path, &objects_path).await.unwrap();

        let data = b"test local storage".to_vec();
        let hash = store.put(data.clone()).await.unwrap();

        assert!(db_path.exists());
        assert!(objects_path.join("data").join(hash.to_string()).exists());

        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_blob_store_recovery() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("blobs.db");
        let objects_path = temp_dir.path().join("objects");

        let hash1;
        let hash2;

        {
            let store = BlobStore::new_local(&db_path, &objects_path).await.unwrap();
            hash1 = store.put(b"blob one".to_vec()).await.unwrap();
            hash2 = store.put(b"blob two".to_vec()).await.unwrap();
            store.close().await;
        }

        tokio::fs::remove_file(&db_path).await.unwrap();

        let store = BlobStore::new_local(&db_path, &objects_path).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);

        let stats = store.recover_from_storage().await.unwrap();
        assert_eq!(stats.found, 2);
        assert_eq!(stats.added, 2);
        assert_eq!(stats.existing, 0);
        assert_eq!(stats.errors, 0);

        assert!(store.has(&hash1).await.unwrap());
        assert!(store.has(&hash2).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 2);

        let stats2 = store.recover_from_storage().await.unwrap();
        assert_eq!(stats2.found, 2);
        assert_eq!(stats2.added, 0);
        assert_eq!(stats2.existing, 2);
    }

    #[tokio::test]
    async fn test_blob_store_get_nonexistent() {
        let store = BlobStore::new_ephemeral().await.unwrap();
        let fake_hash = Hash::new(b"this data was never stored");

        assert!(!store.has(&fake_hash).await.unwrap());
        assert!(store.get(&fake_hash).await.unwrap().is_none());
        assert!(!store.delete(&fake_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_blob_store_multiple_blobs() {
        let store = BlobStore::new_ephemeral().await.unwrap();

        let blobs: Vec<Vec<u8>> = vec![
            b"first blob".to_vec(),
            b"second blob".to_vec(),
            b"third blob".to_vec(),
        ];

        let mut hashes = Vec::new();
        for data in &blobs {
            let hash = store.put(data.clone()).await.unwrap();
            hashes.push(hash);
        }

        assert_eq!(store.count().await.unwrap(), 3);

        for hash in &hashes {
            assert!(store.has(hash).await.unwrap());
        }

        assert!(store.delete(&hashes[1]).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 2);
        assert!(!store.has(&hashes[1]).await.unwrap());

        assert!(store.has(&hashes[0]).await.unwrap());
        assert!(store.has(&hashes[2]).await.unwrap());
    }

    #[tokio::test]
    async fn test_partial_blob_state_tracking() {
        let store = BlobStore::new_ephemeral().await.unwrap();
        let hash = Hash::new(b"some data");

        // Insert partial record
        store.insert_partial(&hash, 1024).await.unwrap();

        // State should be partial
        let state = store.get_state(&hash).await.unwrap();
        assert_eq!(state, Some(BlobState::Partial));

        // get() should return None since it checks complete state only
        assert!(store.get(&hash).await.unwrap().is_none());

        // Partial blobs should not appear in list
        let list = store.list().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_put_with_outboard() {
        let store = BlobStore::new_ephemeral().await.unwrap();

        // Create data large enough to have outboard
        let data = vec![42u8; 32 * 1024]; // 32KB > OUTBOARD_THRESHOLD
        let outboard_data = bao_tree::io::outboard::PreOrderMemOutboard::create(
            &data,
            bao_tree::BlockSize::from_chunk_log(4),
        );

        let hash = store
            .put_with_outboard(data.clone(), outboard_data.data.clone())
            .await
            .unwrap();

        // Should be complete
        let state = store.get_state(&hash).await.unwrap();
        assert_eq!(state, Some(BlobState::Complete));

        // Data should be retrievable
        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());

        // Outboard should be stored
        let retrieved_outboard = store.get_outboard(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved_outboard.as_ref(), outboard_data.data.as_slice());
    }

    #[tokio::test]
    async fn test_partial_then_complete() {
        let store = BlobStore::new_ephemeral().await.unwrap();
        let data = b"test data for partial then complete".to_vec();
        let hash = Hash::new(&data);

        // Start as partial
        store
            .insert_partial(&hash, data.len() as u64)
            .await
            .unwrap();
        assert_eq!(
            store.get_state(&hash).await.unwrap(),
            Some(BlobState::Partial)
        );

        // Complete the import via put (simulates import_bao completing)
        let stored_hash = store.put(data.clone()).await.unwrap();
        assert_eq!(hash, stored_hash);

        // Should now be complete
        assert_eq!(
            store.get_state(&hash).await.unwrap(),
            Some(BlobState::Complete)
        );

        // Data should be retrievable
        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_put_stores_outboard_for_large_blobs() {
        let store = BlobStore::new_ephemeral().await.unwrap();

        // Small blob (< 16KB threshold) should NOT have outboard
        let small_data = b"small".to_vec();
        let small_hash = store.put(small_data).await.unwrap();
        assert!(store.get_outboard(&small_hash).await.unwrap().is_none());

        // Large blob (> 16KB threshold) should have outboard
        let large_data = vec![0u8; 32 * 1024];
        let large_hash = store.put(large_data).await.unwrap();
        assert!(store.get_outboard(&large_hash).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_export_bao_verified_streaming() {
        use n0_future::StreamExt;

        let store = ObjectStore::new_ephemeral().await.unwrap();

        // Store a blob
        let data = b"hello world verified streaming test".to_vec();
        let tt = store.add_bytes(data.clone()).temp_tag().await.unwrap();
        let hash = tt.hash();

        // Export BAO and collect items
        let bao_reader = store.export_bao(hash, bao_tree::ChunkRanges::all());
        let items: Vec<_> = bao_reader.stream().collect().await;

        // Should have Size, then parent/leaf items, then Done
        assert!(!items.is_empty(), "BAO export produced no items");

        // First item should be Size
        let first = items.first().unwrap();
        assert!(
            matches!(first, bao_tree::io::mixed::EncodedItem::Size(_)),
            "first item should be Size, got: {first:?}"
        );

        // Last item should be Done
        let last = items.last().unwrap();
        assert!(matches!(last, bao_tree::io::mixed::EncodedItem::Done));

        // Collect all leaf data
        let mut reconstructed = vec![0u8; data.len()];
        for item in &items {
            if let bao_tree::io::mixed::EncodedItem::Leaf(leaf) = item {
                let start = leaf.offset as usize;
                let end = start + leaf.data.len();
                reconstructed[start..end].copy_from_slice(&leaf.data);
            }
        }
        assert_eq!(reconstructed, data);
    }

    #[tokio::test]
    async fn test_blob_status_complete_and_not_found() {
        let store = ObjectStore::new_ephemeral().await.unwrap();

        // Store a blob normally (complete)
        let data = b"complete blob".to_vec();
        let tt = store.add_bytes(data).temp_tag().await.unwrap();
        let hash = tt.hash();

        let status = store.status(hash).await.unwrap();
        assert!(matches!(status, BlobStatus::Complete { size: 13 }));

        // Non-existent blob
        let fake_hash = Hash::new(b"nonexistent");
        let status = store.status(fake_hash).await.unwrap();
        assert!(matches!(status, BlobStatus::NotFound));
    }
}
