//! SQLite + Object Storage Blob Store
//!
//! This crate provides a blob store implementation that uses SQLite for metadata
//! and pluggable object storage (S3/MinIO/local filesystem/memory) for blob data.
//!
//! # Features
//!
//! - Content-addressed storage using BLAKE3 hashes (compatible with iroh-blobs)
//! - SQLite for fast metadata queries
//! - Multiple storage backends: S3, MinIO, local filesystem, in-memory
//! - Recovery support: rebuild metadata from object storage
//!
//! # Example
//!
//! ```rust,no_run
//! use jax_blobs_store::{BlobStore, ObjectStoreConfig};
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), jax_blobs_store::BlobStoreError> {
//! // Create a local file-based store
//! let store = BlobStore::new_local(Path::new("/tmp/blobs")).await?;
//!
//! // Store some data
//! let hash = store.put(b"hello world".to_vec()).await?;
//! println!("Stored blob with hash: {}", hash);
//!
//! // Retrieve it
//! let data = store.get(&hash).await?.unwrap();
//! println!("Retrieved: {:?}", data);
//!
//! // For S3/MinIO:
//! let s3_config = ObjectStoreConfig::S3 {
//!     endpoint: "http://localhost:9000".to_string(),
//!     access_key: "minioadmin".to_string(),
//!     secret_key: "minioadmin".to_string(),
//!     bucket: "jax-blobs".to_string(),
//!     region: None,
//! };
//! let s3_store = BlobStore::new(
//!     Path::new("/tmp/blobs.db"),
//!     s3_config,
//! ).await?;
//! # Ok(())
//! # }
//! ```

mod database;
mod error;
mod object_store;
mod store;

pub use database::{BlobMetadata, BlobState, Database};
pub use error::{BlobStoreError, Result};
pub use object_store::{ObjectStoreConfig, Storage};
pub use store::{BlobStore, RecoveryStats};
