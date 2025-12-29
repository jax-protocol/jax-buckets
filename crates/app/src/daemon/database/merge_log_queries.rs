use time::OffsetDateTime;
use uuid::Uuid;

use crate::daemon::database::{types::DCid, Database};
use common::prelude::Link;

/// Merge log entry for tracking reconciled branches
#[derive(Debug, Clone)]
pub struct MergeLogEntry {
    /// The bucket this merge belongs to
    pub bucket_id: Uuid,
    /// The orphaned branch that was merged (source)
    pub link_from: Link,
    /// Height of the orphaned branch
    pub height_from: u64,
    /// The canonical head we merged onto (target)
    pub link_onto: Link,
    /// Height of the canonical head before merge
    pub height_onto: u64,
    /// The new head created after merge (result)
    pub result_link: Link,
    /// Height of the new head (should be height_onto + 1)
    pub result_height: u64,
    /// Number of operations merged from this branch
    pub ops_merged: u32,
    /// When this merge occurred
    pub merged_at: OffsetDateTime,
}

impl Database {
    /// Insert a merge log entry after successful reconciliation
    ///
    /// Records full context about a merge:
    /// - link_from/height_from: the orphaned branch that was merged
    /// - link_onto/height_onto: the canonical head before merge
    /// - result_link/result_height: the new head after merge
    pub async fn insert_merge_log(
        &self,
        bucket_id: &Uuid,
        link_from: &Link,
        height_from: u64,
        link_onto: &Link,
        height_onto: u64,
        result_link: &Link,
        result_height: u64,
        ops_merged: u32,
    ) -> Result<(), sqlx::Error> {
        let bucket_id_str = bucket_id.to_string();
        let link_from_dcid: DCid = link_from.clone().into();
        let link_onto_dcid: DCid = link_onto.clone().into();
        let result_link_dcid: DCid = result_link.clone().into();
        let height_from_i64 = height_from as i64;
        let height_onto_i64 = height_onto as i64;
        let result_height_i64 = result_height as i64;
        let ops_i32 = ops_merged as i32;

        sqlx::query!(
            r#"
            INSERT INTO merge_log (
                bucket_id,
                link_from, height_from,
                link_onto, height_onto,
                result_link, result_height,
                ops_merged
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            bucket_id_str,
            link_from_dcid,
            height_from_i64,
            link_onto_dcid,
            height_onto_i64,
            result_link_dcid,
            result_height_i64,
            ops_i32
        )
        .execute(&**self)
        .await?;

        Ok(())
    }

    /// Get all merged branch links (link_from) for a bucket
    ///
    /// Used to filter orphaned branches - if a branch's link appears here,
    /// it has already been merged and shouldn't be shown as orphaned.
    pub async fn get_merged_links_from(&self, bucket_id: &Uuid) -> Result<Vec<Link>, sqlx::Error> {
        let bucket_id_str = bucket_id.to_string();

        let rows = sqlx::query!(
            r#"
            SELECT link_from as "link_from!: DCid"
            FROM merge_log
            WHERE bucket_id = ?1
            "#,
            bucket_id_str
        )
        .fetch_all(&**self)
        .await?;

        Ok(rows.into_iter().map(|r| r.link_from.into()).collect())
    }

    /// Get all merge log entries for a bucket
    ///
    /// Returns full merge history for display in tree view or merge log UI.
    pub async fn get_merge_log_entries(
        &self,
        bucket_id: &Uuid,
    ) -> Result<Vec<MergeLogEntry>, sqlx::Error> {
        let bucket_id_str = bucket_id.to_string();

        let rows = sqlx::query!(
            r#"
            SELECT
                bucket_id as "bucket_id!",
                link_from as "link_from!: DCid",
                height_from as "height_from!",
                link_onto as "link_onto!: DCid",
                height_onto as "height_onto!",
                result_link as "result_link!: DCid",
                result_height as "result_height!",
                ops_merged as "ops_merged!",
                merged_at as "merged_at!"
            FROM merge_log
            WHERE bucket_id = ?1
            ORDER BY merged_at DESC
            "#,
            bucket_id_str
        )
        .fetch_all(&**self)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MergeLogEntry {
                bucket_id: Uuid::parse_str(&r.bucket_id)
                    .expect("invalid bucket_id UUID in database"),
                link_from: r.link_from.into(),
                height_from: r.height_from as u64,
                link_onto: r.link_onto.into(),
                height_onto: r.height_onto as u64,
                result_link: r.result_link.into(),
                result_height: r.result_height as u64,
                ops_merged: r.ops_merged as u32,
                merged_at: r.merged_at,
            })
            .collect())
    }
}
