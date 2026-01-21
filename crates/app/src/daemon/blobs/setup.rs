//! Blob store setup logic.
//!
//! Currently all modes use iroh's FsStore. The Filesystem and S3 configs
//! are parsed and validated but the actual SQLite + object storage backend
//! is not yet implemented (requires iroh-blobs store trait implementation).

use std::path::Path;

use common::peer::BlobsStore;

use crate::state::BlobStoreConfig;

use super::BlobsSetupError;

/// Setup the blob store based on configuration.
///
/// NOTE: Currently all modes fall back to iroh's FsStore.
/// The new SQLite + object storage backend requires implementing
/// iroh-blobs store traits (see issues/sqlite-object-storage-blobs.md).
pub async fn setup_blobs_store(
    config: &BlobStoreConfig,
    jax_dir: &Path,
) -> Result<BlobsStore, BlobsSetupError> {
    // Determine the blobs path based on config
    let blobs_path = match config {
        BlobStoreConfig::Legacy => jax_dir.join("blobs"),
        BlobStoreConfig::Filesystem { path } => path.join("iroh-blobs"),
        BlobStoreConfig::S3 { .. } => jax_dir.join("blobs-cache"),
    };

    // Log what was configured vs what we're actually using
    match config {
        BlobStoreConfig::Legacy => {
            tracing::info!(path = %blobs_path.display(), "Using iroh blob store");
        }
        BlobStoreConfig::Filesystem { path } => {
            tracing::warn!(
                configured_path = %path.display(),
                actual_path = %blobs_path.display(),
                "Filesystem blob store not yet implemented, using iroh FsStore"
            );
        }
        BlobStoreConfig::S3 { url } => {
            tracing::warn!(
                url = %url,
                actual_path = %blobs_path.display(),
                "S3 blob store not yet implemented, using iroh FsStore"
            );
        }
    }

    let blobs = BlobsStore::fs(&blobs_path)
        .await
        .map_err(|e| BlobsSetupError::StoreError(e.to_string()))?;

    Ok(blobs)
}
