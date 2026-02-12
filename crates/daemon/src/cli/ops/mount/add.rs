use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Add {
    /// Bucket name or ID
    pub bucket: String,

    /// Mount point path
    pub path: String,

    /// Auto-mount on daemon startup
    #[arg(long)]
    pub auto_mount: bool,

    /// Mount as read-only
    #[arg(long)]
    pub read_only: bool,

    /// Cache size in MB
    #[arg(long, default_value = "100")]
    pub cache_size: u32,

    /// Cache TTL in seconds
    #[arg(long, default_value = "60")]
    pub cache_ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateMountRequest {
    bucket_id: Uuid,
    mount_point: String,
    auto_mount: bool,
    read_only: bool,
    cache_size_mb: u32,
    cache_ttl_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateMountResponse {
    mount: MountInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MountInfo {
    mount_id: String,
    bucket_id: String,
    mount_point: String,
    status: String,
}

#[async_trait::async_trait]
impl Op for Add {
    type Error = AddError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        // Resolve bucket name to ID if needed
        let bucket_id = if let Ok(uuid) = Uuid::parse_str(&self.bucket) {
            uuid
        } else {
            client.resolve_bucket_name(&self.bucket).await?
        };

        // Create mount request
        let request = CreateMountRequest {
            bucket_id,
            mount_point: self.path.clone(),
            auto_mount: self.auto_mount,
            read_only: self.read_only,
            cache_size_mb: self.cache_size,
            cache_ttl_secs: self.cache_ttl,
        };

        let url = client.base_url().join("/api/v0/mounts/").unwrap();
        let response: CreateMountResponse = client
            .http_client()
            .post(url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(format!(
            "Created mount {} for bucket {} at {}\nStatus: {}",
            response.mount.mount_id,
            response.mount.bucket_id,
            response.mount.mount_point,
            response.mount.status
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AddError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl std::fmt::Display for Add {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount add {} {}", self.bucket, self.path)
    }
}
