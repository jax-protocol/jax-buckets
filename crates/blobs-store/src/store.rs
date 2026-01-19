//! Main BlobStore API combining SQLite metadata with object storage.

use std::path::Path;

use bytes::Bytes;
use iroh_blobs::Hash;
use tracing::{debug, info, warn};

use crate::database::Database;
use crate::error::Result;
use crate::object_store::{ObjectStoreConfig, Storage};

/// Size threshold for generating BAO outboard data (16KB).
/// Blobs larger than this will have outboard verification data stored separately.
const OUTBOARD_THRESHOLD: usize = 16 * 1024;

/// Statistics from a recovery operation.
#[derive(Debug, Default)]
pub struct RecoveryStats {
    /// Number of blobs found in storage
    pub found: usize,
    /// Number of blobs added to metadata
    pub added: usize,
    /// Number of blobs that already existed in metadata
    pub existing: usize,
    /// Number of errors encountered
    pub errors: usize,
}

/// BlobStore provides content-addressed blob storage with SQLite metadata
/// and pluggable object storage backends (S3/MinIO/local/memory).
#[derive(Debug, Clone)]
pub struct BlobStore {
    db: Database,
    storage: Storage,
}

impl BlobStore {
    /// Create a new BlobStore with a file-based SQLite database.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `config` - Object storage configuration
    pub async fn new(db_path: &Path, config: ObjectStoreConfig) -> Result<Self> {
        let db = Database::new(db_path).await?;
        let storage = Storage::new(config).await?;
        Ok(Self { db, storage })
    }

    /// Create a new BlobStore with an in-memory SQLite database.
    /// Useful for testing or ephemeral storage.
    pub async fn in_memory(config: ObjectStoreConfig) -> Result<Self> {
        let db = Database::in_memory().await?;
        let storage = Storage::new(config).await?;
        Ok(Self { db, storage })
    }

    /// Create a new BlobStore backed by local filesystem.
    /// This creates both SQLite DB and object storage in the given directory.
    ///
    /// # Arguments
    /// * `data_dir` - Directory for all storage (db at data_dir/blobs.db, objects at data_dir/objects/)
    pub async fn new_local(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("blobs.db");
        let objects_path = data_dir.join("objects");

        let config = ObjectStoreConfig::Local { path: objects_path };
        Self::new(&db_path, config).await
    }

    /// Create a fully ephemeral BlobStore (in-memory DB + in-memory object storage).
    /// Data will be lost when the BlobStore is dropped.
    pub async fn new_ephemeral() -> Result<Self> {
        Self::in_memory(ObjectStoreConfig::Memory).await
    }

    /// Store data and return its content hash.
    ///
    /// The hash is computed using BLAKE3, matching iroh-blobs behavior.
    pub async fn put(&self, data: Vec<u8>) -> Result<Hash> {
        let size = data.len();
        let hash = Hash::new(&data);
        let hash_str = hash.to_string();

        debug!(hash = %hash_str, size = size, "storing blob");

        // Check if we need outboard data
        let has_outboard = size > OUTBOARD_THRESHOLD;

        // Store the data
        self.storage.put_data(&hash_str, Bytes::from(data)).await?;

        // Store metadata
        self.db
            .insert_blob(&hash_str, size as i64, has_outboard)
            .await?;

        info!(hash = %hash_str, size = size, "blob stored successfully");
        Ok(hash)
    }

    /// Retrieve blob data by hash.
    ///
    /// Returns `None` if the blob doesn't exist.
    pub async fn get(&self, hash: &Hash) -> Result<Option<Bytes>> {
        let hash_str = hash.to_string();

        // Check metadata first
        if !self.db.has_blob(&hash_str).await? {
            return Ok(None);
        }

        // Get from storage
        self.storage.get_data(&hash_str).await
    }

    /// Check if a blob exists in the store.
    pub async fn has(&self, hash: &Hash) -> Result<bool> {
        let hash_str = hash.to_string();
        self.db.has_blob(&hash_str).await
    }

    /// Delete a blob from the store.
    ///
    /// Returns `true` if the blob existed and was deleted.
    pub async fn delete(&self, hash: &Hash) -> Result<bool> {
        let hash_str = hash.to_string();

        // Check if blob exists
        let metadata = self.db.get_blob(&hash_str).await?;
        if metadata.is_none() {
            return Ok(false);
        }

        let metadata = metadata.unwrap();

        // Delete from storage
        self.storage.delete_data(&hash_str).await?;
        if metadata.has_outboard {
            self.storage.delete_outboard(&hash_str).await?;
        }

        // Delete metadata
        self.db.delete_blob(&hash_str).await?;

        info!(hash = %hash_str, "blob deleted");
        Ok(true)
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

    /// Get the number of blobs in the store.
    pub async fn count(&self) -> Result<u64> {
        let count = self.db.count_blobs().await?;
        Ok(count as u64)
    }

    /// Get the total size of all blobs in the store.
    pub async fn total_size(&self) -> Result<u64> {
        let size = self.db.total_size().await?;
        Ok(size as u64)
    }

    /// Recover metadata from object storage.
    ///
    /// This scans the object storage for blobs and ensures they have
    /// corresponding metadata entries. Useful for recovery scenarios
    /// or when the SQLite database is lost/corrupted.
    pub async fn recover_from_storage(&self) -> Result<RecoveryStats> {
        info!("starting recovery from object storage");

        let mut stats = RecoveryStats::default();

        // List all hashes in storage
        let hashes = self.storage.list_data_hashes().await?;
        stats.found = hashes.len();

        for hash_str in hashes {
            // Check if metadata exists
            if self.db.has_blob(&hash_str).await? {
                stats.existing += 1;
                continue;
            }

            // Get the data to compute size
            match self.storage.get_data(&hash_str).await {
                Ok(Some(data)) => {
                    let size = data.len();
                    let has_outboard = size > OUTBOARD_THRESHOLD;

                    // Insert metadata
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

        info!(
            found = stats.found,
            added = stats.added,
            existing = stats.existing,
            errors = stats.errors,
            "recovery complete"
        );

        Ok(stats)
    }

    /// Get the object storage configuration.
    pub fn storage_config(&self) -> &ObjectStoreConfig {
        self.storage.config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ephemeral_store() {
        let store = BlobStore::new_ephemeral().await.unwrap();

        // Put some data
        let data = b"hello world".to_vec();
        let hash = store.put(data.clone()).await.unwrap();

        // Verify it exists
        assert!(store.has(&hash).await.unwrap());

        // Get it back
        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());

        // List should include it
        let list = store.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], hash);

        // Count and size
        assert_eq!(store.count().await.unwrap(), 1);
        assert_eq!(store.total_size().await.unwrap(), data.len() as u64);

        // Delete
        assert!(store.delete(&hash).await.unwrap());
        assert!(!store.has(&hash).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_local_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = BlobStore::new_local(temp_dir.path()).await.unwrap();

        let data = b"test local storage".to_vec();
        let hash = store.put(data.clone()).await.unwrap();

        // Verify files exist
        assert!(temp_dir.path().join("blobs.db").exists());
        assert!(temp_dir
            .path()
            .join("objects")
            .join("data")
            .join(hash.to_string())
            .exists());

        // Get works
        let retrieved = store.get(&hash).await.unwrap().unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_recovery() {
        // Create a store and add some data
        let temp_dir = tempfile::tempdir().unwrap();

        let hash1;
        let hash2;

        {
            let store = BlobStore::new_local(temp_dir.path()).await.unwrap();
            hash1 = store.put(b"blob one".to_vec()).await.unwrap();
            hash2 = store.put(b"blob two".to_vec()).await.unwrap();
        }

        // Delete the database to simulate loss
        tokio::fs::remove_file(temp_dir.path().join("blobs.db"))
            .await
            .unwrap();

        // Recreate the store (new empty database)
        let store = BlobStore::new_local(temp_dir.path()).await.unwrap();

        // Initially nothing in metadata
        assert_eq!(store.count().await.unwrap(), 0);

        // Run recovery
        let stats = store.recover_from_storage().await.unwrap();
        assert_eq!(stats.found, 2);
        assert_eq!(stats.added, 2);
        assert_eq!(stats.existing, 0);
        assert_eq!(stats.errors, 0);

        // Now metadata is restored
        assert!(store.has(&hash1).await.unwrap());
        assert!(store.has(&hash2).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 2);

        // Running recovery again should find existing
        let stats2 = store.recover_from_storage().await.unwrap();
        assert_eq!(stats2.found, 2);
        assert_eq!(stats2.added, 0);
        assert_eq!(stats2.existing, 2);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = BlobStore::new_ephemeral().await.unwrap();
        // Create a hash for data that was never stored
        let fake_hash = Hash::new(b"this data was never stored");

        assert!(!store.has(&fake_hash).await.unwrap());
        assert!(store.get(&fake_hash).await.unwrap().is_none());
        assert!(!store.delete(&fake_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_multiple_blobs() {
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

        // All exist
        for hash in &hashes {
            assert!(store.has(hash).await.unwrap());
        }

        // Delete middle one
        assert!(store.delete(&hashes[1]).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 2);
        assert!(!store.has(&hashes[1]).await.unwrap());

        // Others still exist
        assert!(store.has(&hashes[0]).await.unwrap());
        assert!(store.has(&hashes[2]).await.unwrap());
    }
}
