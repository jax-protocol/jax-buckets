use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::database::Database;

/// Mount status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl MountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MountStatus::Stopped => "stopped",
            MountStatus::Starting => "starting",
            MountStatus::Running => "running",
            MountStatus::Stopping => "stopping",
            MountStatus::Error => "error",
        }
    }
}

impl std::str::FromStr for MountStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "stopped" => MountStatus::Stopped,
            "starting" => MountStatus::Starting,
            "running" => MountStatus::Running,
            "stopping" => MountStatus::Stopping,
            "error" => MountStatus::Error,
            _ => MountStatus::Stopped,
        })
    }
}

/// FUSE mount configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuseMount {
    pub mount_id: Uuid,
    pub bucket_id: Uuid,
    pub mount_point: String,
    pub enabled: bool,
    pub auto_mount: bool,
    pub read_only: bool,
    pub cache_size_mb: u32,
    pub cache_ttl_secs: u32,
    pub status: MountStatus,
    pub error_message: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Configuration for creating a new mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMountConfig {
    pub bucket_id: Uuid,
    pub mount_point: String,
    pub auto_mount: bool,
    pub read_only: bool,
    pub cache_size_mb: Option<u32>,
    pub cache_ttl_secs: Option<u32>,
}

/// Configuration for updating a mount
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateMountConfig {
    pub mount_point: Option<String>,
    pub enabled: Option<bool>,
    pub auto_mount: Option<bool>,
    pub read_only: Option<bool>,
    pub cache_size_mb: Option<u32>,
    pub cache_ttl_secs: Option<u32>,
}

impl Database {
    /// Create a new FUSE mount configuration
    pub async fn create_mount(&self, config: &CreateMountConfig) -> Result<FuseMount, sqlx::Error> {
        let mount_id = Uuid::new_v4();
        let mount_id_str = mount_id.to_string();
        let bucket_id_str = config.bucket_id.to_string();
        let cache_size = config.cache_size_mb.unwrap_or(100) as i64;
        let cache_ttl = config.cache_ttl_secs.unwrap_or(60) as i64;
        let auto_mount = config.auto_mount as i64;
        let read_only = config.read_only as i64;

        sqlx::query(
            r#"
            INSERT INTO fuse_mounts (
                mount_id, bucket_id, mount_point, auto_mount, read_only,
                cache_size_mb, cache_ttl_secs
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&mount_id_str)
        .bind(&bucket_id_str)
        .bind(&config.mount_point)
        .bind(auto_mount)
        .bind(read_only)
        .bind(cache_size)
        .bind(cache_ttl)
        .execute(&**self)
        .await?;

        // Return the created mount
        self.get_mount(&mount_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Get a FUSE mount by ID
    pub async fn get_mount(&self, mount_id: &Uuid) -> Result<Option<FuseMount>, sqlx::Error> {
        let mount_id_str = mount_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT
                mount_id,
                bucket_id,
                mount_point,
                enabled,
                auto_mount,
                read_only,
                cache_size_mb,
                cache_ttl_secs,
                status,
                error_message,
                created_at,
                updated_at
            FROM fuse_mounts
            WHERE mount_id = ?1
            "#,
        )
        .bind(&mount_id_str)
        .fetch_optional(&**self)
        .await?;

        Ok(row.map(|r| row_to_fuse_mount(&r)))
    }

    /// List all FUSE mounts
    pub async fn list_mounts(&self) -> Result<Vec<FuseMount>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                mount_id,
                bucket_id,
                mount_point,
                enabled,
                auto_mount,
                read_only,
                cache_size_mb,
                cache_ttl_secs,
                status,
                error_message,
                created_at,
                updated_at
            FROM fuse_mounts
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&**self)
        .await?;

        Ok(rows.iter().map(row_to_fuse_mount).collect())
    }

    /// Update a FUSE mount configuration
    pub async fn update_mount(
        &self,
        mount_id: &Uuid,
        config: &UpdateMountConfig,
    ) -> Result<Option<FuseMount>, sqlx::Error> {
        let mount_id_str = mount_id.to_string();

        // First check if mount exists
        let existing = self.get_mount(mount_id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        // Build update with coalesced values
        let mount_point = config.mount_point.as_ref().unwrap_or(&existing.mount_point);
        let enabled = config.enabled.unwrap_or(existing.enabled) as i64;
        let auto_mount = config.auto_mount.unwrap_or(existing.auto_mount) as i64;
        let read_only = config.read_only.unwrap_or(existing.read_only) as i64;
        let cache_size = config.cache_size_mb.unwrap_or(existing.cache_size_mb) as i64;
        let cache_ttl = config.cache_ttl_secs.unwrap_or(existing.cache_ttl_secs) as i64;

        sqlx::query(
            r#"
            UPDATE fuse_mounts
            SET mount_point = ?1,
                enabled = ?2,
                auto_mount = ?3,
                read_only = ?4,
                cache_size_mb = ?5,
                cache_ttl_secs = ?6,
                updated_at = CURRENT_TIMESTAMP
            WHERE mount_id = ?7
            "#,
        )
        .bind(mount_point)
        .bind(enabled)
        .bind(auto_mount)
        .bind(read_only)
        .bind(cache_size)
        .bind(cache_ttl)
        .bind(&mount_id_str)
        .execute(&**self)
        .await?;

        self.get_mount(mount_id).await
    }

    /// Delete a FUSE mount
    pub async fn delete_mount(&self, mount_id: &Uuid) -> Result<bool, sqlx::Error> {
        let mount_id_str = mount_id.to_string();

        let result = sqlx::query(
            r#"
            DELETE FROM fuse_mounts
            WHERE mount_id = ?1
            "#,
        )
        .bind(&mount_id_str)
        .execute(&**self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update the status of a FUSE mount
    pub async fn update_mount_status(
        &self,
        mount_id: &Uuid,
        status: MountStatus,
        error_message: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let mount_id_str = mount_id.to_string();
        let status_str = status.as_str();

        let result = sqlx::query(
            r#"
            UPDATE fuse_mounts
            SET status = ?1,
                error_message = ?2,
                updated_at = CURRENT_TIMESTAMP
            WHERE mount_id = ?3
            "#,
        )
        .bind(status_str)
        .bind(error_message)
        .bind(&mount_id_str)
        .execute(&**self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get all mounts configured for auto-mount
    pub async fn get_auto_mount_list(&self) -> Result<Vec<FuseMount>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                mount_id,
                bucket_id,
                mount_point,
                enabled,
                auto_mount,
                read_only,
                cache_size_mb,
                cache_ttl_secs,
                status,
                error_message,
                created_at,
                updated_at
            FROM fuse_mounts
            WHERE auto_mount = 1 AND enabled = 1
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&**self)
        .await?;

        Ok(rows.iter().map(row_to_fuse_mount).collect())
    }

    /// Get mounts by bucket ID
    pub async fn get_mounts_by_bucket(
        &self,
        bucket_id: &Uuid,
    ) -> Result<Vec<FuseMount>, sqlx::Error> {
        let bucket_id_str = bucket_id.to_string();

        let rows = sqlx::query(
            r#"
            SELECT
                mount_id,
                bucket_id,
                mount_point,
                enabled,
                auto_mount,
                read_only,
                cache_size_mb,
                cache_ttl_secs,
                status,
                error_message,
                created_at,
                updated_at
            FROM fuse_mounts
            WHERE bucket_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(&bucket_id_str)
        .fetch_all(&**self)
        .await?;

        Ok(rows.iter().map(row_to_fuse_mount).collect())
    }
}

fn row_to_fuse_mount(row: &sqlx::sqlite::SqliteRow) -> FuseMount {
    FuseMount {
        mount_id: Uuid::parse_str(row.get::<String, _>("mount_id").as_str())
            .expect("invalid mount_id UUID in database"),
        bucket_id: Uuid::parse_str(row.get::<String, _>("bucket_id").as_str())
            .expect("invalid bucket_id UUID in database"),
        mount_point: row.get("mount_point"),
        enabled: row.get::<i64, _>("enabled") != 0,
        auto_mount: row.get::<i64, _>("auto_mount") != 0,
        read_only: row.get::<i64, _>("read_only") != 0,
        cache_size_mb: row.get::<i64, _>("cache_size_mb") as u32,
        cache_ttl_secs: row.get::<i64, _>("cache_ttl_secs") as u32,
        status: row.get::<String, _>("status").parse().unwrap(),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
