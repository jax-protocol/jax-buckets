use std::collections::HashSet;
use std::fmt::{Debug, Display};

use async_trait::async_trait;
use uuid::Uuid;

use crate::linked_data::Link;

/// Entry representing a non-canonical (orphaned) branch in the bucket log.
/// These are manifest entries that are not ancestors of the canonical head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanedBranch {
    /// The link to this orphaned manifest
    pub link: Link,
    /// The height at which this branch exists
    pub height: u64,
    /// The previous link this branch points to
    pub previous: Option<Link>,
}

/// Entry in the bucket log used for orphan detection
#[derive(Debug, Clone)]
pub struct BucketLogEntry {
    pub current_link: Link,
    pub previous_link: Option<Link>,
    pub height: u64,
}

// TODO (amiller68): it might be easier to design this to work
//  with dependency injection over a generic type

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum BucketLogError<T> {
    /// The bucket log is empty
    #[error("unhandled bucket log provider error: {0}")]
    Provider(#[from] T),
    /// The bucket log is empty
    #[error("head not found at height {0}")]
    HeadNotFound(u64),
    /// An append causes a conflict with the current of the
    ///  log i.e. same link at the same height
    #[error("conflict with current log entry")]
    Conflict,
    /// An append does not implement a valid link structure
    ///  st the previous link pointed at by the new log does
    ///  not exist in the log at the expected height --
    ///  current, previous, height
    #[error("invalid append: {0}, {1}, {2}")]
    InvalidAppend(Link, Link, u64),
}

#[async_trait]
pub trait BucketLogProvider: Send + Sync + std::fmt::Debug + Clone + 'static {
    type Error: Display + Debug;

    async fn exists(&self, id: Uuid) -> Result<bool, BucketLogError<Self::Error>>;

    /// Get the possible heads for a bucket
    ///  based on passed height
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    /// * `height` - The height to query the candidate heads for
    ///
    /// # Returns
    /// * `Ok(Vec<Link>)` - The candidate heads for the bucket
    /// * `Err(Self::Error)` - An error occurred while fetching the candidate heads
    async fn heads(&self, id: Uuid, height: u64) -> Result<Vec<Link>, BucketLogError<Self::Error>>;

    // NOTE (amiller68): maybe name is more of a
    //  implementation detail or product concern,
    //  but maybe its not such thing to mandate a
    //  cache for.
    /// Append a version of the bucket to the log
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    /// * `name` - The friendly name for the bucket
    /// * `current` - The current link of the record
    /// * `previous` - The previous link of the record
    /// * `height` - The reported depth of the bucket version within the chain
    ///
    /// Should fail with the following errors to be considered
    ///  correct:
    /// * `Err(BucketLogError::Conflict)` - The append causes a conflict with the current log
    /// * `Err(BucketLogError::InvalidHeight)` - The height is not greater than the previous height
    async fn append(
        &self,
        id: Uuid,
        name: String,
        current: Link,
        // NOTE (amiller68): this should *only*
        //  be null for the genesis of a bucket
        previous: Option<Link>,
        height: u64,
    ) -> Result<(), BucketLogError<Self::Error>>;

    /// Return the greatest height of the bucket version within the chain
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    ///
    /// # Returns
    /// * `Result<u64, BucketLogError<Self::Error>>` - The height of the bucket version within the chain
    ///
    /// NOTE: while this returns a BucketLogError, it should only ever return a BucketLogError::NotFound
    ///  or ProviderError
    async fn height(&self, id: Uuid) -> Result<u64, BucketLogError<Self::Error>>;

    /// Check if a link exists within a bucket
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    /// * `link` - The link to check for existence as current
    ///
    /// # Returns
    /// * `Result<Vec<u64>, BucketLogError<Self::Error>>`
    ///     The heights at which the link exists within the bucket
    async fn has(&self, id: Uuid, link: Link) -> Result<Vec<u64>, BucketLogError<Self::Error>>;

    /// Get the peers canonical head based on its log entries
    async fn head(
        &self,
        id: Uuid,
        height: Option<u64>,
    ) -> Result<(Link, u64), BucketLogError<Self::Error>> {
        let height = height.unwrap_or(self.height(id).await?);
        let heads = self.heads(id, height).await?;
        Ok((
            heads
                .into_iter()
                .max()
                .ok_or(BucketLogError::HeadNotFound(height))?,
            height,
        ))
    }

    /// List all bucket IDs that have log entries
    ///
    /// # Returns
    /// * `Ok(Vec<Uuid>)` - The list of bucket IDs
    /// * `Err(BucketLogError)` - An error occurred while fetching bucket IDs
    async fn list_buckets(&self) -> Result<Vec<Uuid>, BucketLogError<Self::Error>>;

    /// Get all log entries for a bucket
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    ///
    /// # Returns
    /// * `Ok(Vec<BucketLogEntry>)` - All log entries for the bucket
    /// * `Err(BucketLogError)` - An error occurred while fetching entries
    async fn all_entries(
        &self,
        id: Uuid,
    ) -> Result<Vec<BucketLogEntry>, BucketLogError<Self::Error>>;

    /// Find all branches that are not ancestors of the canonical head.
    ///
    /// These are manifest entries in the log that diverged from the main chain
    /// and contain potentially unmerged operations.
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    ///
    /// # Returns
    /// * `Ok(Vec<OrphanedBranch>)` - All orphaned branches found
    /// * `Err(BucketLogError)` - An error occurred
    async fn find_orphaned_branches(
        &self,
        id: Uuid,
    ) -> Result<Vec<OrphanedBranch>, BucketLogError<Self::Error>> {
        self.find_orphaned_branches_excluding(id, &HashSet::new())
            .await
    }

    /// Find all branches that are not ancestors of the canonical head,
    /// excluding branches that have already been merged.
    ///
    /// This is the main implementation of orphaned branch detection.
    /// Use this when you have access to a merge log that tracks which
    /// branches have already been reconciled.
    ///
    /// # Arguments
    /// * `id` - The UUID of the bucket
    /// * `already_merged` - Set of links that have already been merged and should be excluded
    ///
    /// # Returns
    /// * `Ok(Vec<OrphanedBranch>)` - All orphaned branches found (excluding already merged)
    /// * `Err(BucketLogError)` - An error occurred
    async fn find_orphaned_branches_excluding(
        &self,
        id: Uuid,
        already_merged: &HashSet<Link>,
    ) -> Result<Vec<OrphanedBranch>, BucketLogError<Self::Error>> {
        // 1. Get canonical head
        let (canonical_link, _) = self.head(id, None).await?;

        // 2. Get all entries for this bucket
        let all_entries = self.all_entries(id).await?;

        // 3. Build canonical chain by walking previous links
        let mut canonical_chain: HashSet<Link> = HashSet::new();
        let mut current = Some(canonical_link);
        while let Some(link) = current {
            canonical_chain.insert(link.clone());
            // Find entry with this link and get its previous
            current = all_entries
                .iter()
                .find(|e| e.current_link == link)
                .and_then(|e| e.previous_link.clone());
        }

        // 4. Find entries NOT in canonical chain AND NOT already merged
        Ok(all_entries
            .into_iter()
            .filter(|e| {
                !canonical_chain.contains(&e.current_link)
                    && !already_merged.contains(&e.current_link)
            })
            .map(|e| OrphanedBranch {
                link: e.current_link,
                height: e.height,
                previous: e.previous_link,
            })
            .collect())
    }
}
