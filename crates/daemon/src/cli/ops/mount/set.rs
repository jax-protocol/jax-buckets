use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Set {
    /// Mount ID to update
    pub id: Uuid,

    /// New mount point path
    #[arg(long)]
    pub mount_point: Option<String>,

    /// Enable/disable mount
    #[arg(long)]
    pub enabled: Option<bool>,

    /// Enable/disable auto-mount
    #[arg(long)]
    pub auto_mount: Option<bool>,

    /// Enable/disable read-only mode
    #[arg(long)]
    pub read_only: Option<bool>,

    /// Cache size in MB
    #[arg(long)]
    pub cache_size: Option<u32>,

    /// Cache TTL in seconds
    #[arg(long)]
    pub cache_ttl: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateMountRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    mount_point: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_mount: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_size_mb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_ttl_secs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateMountResponse {
    mount: MountInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MountInfo {
    mount_id: String,
    mount_point: String,
    enabled: bool,
    auto_mount: bool,
    read_only: bool,
    cache_size_mb: u32,
    cache_ttl_secs: u32,
}

#[async_trait::async_trait]
impl Op for Set {
    type Error = SetError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let client = ctx.client.clone();

        let request = UpdateMountRequest {
            mount_point: self.mount_point.clone(),
            enabled: self.enabled,
            auto_mount: self.auto_mount,
            read_only: self.read_only,
            cache_size_mb: self.cache_size,
            cache_ttl_secs: self.cache_ttl,
        };

        let url = client
            .base_url()
            .join(&format!("/api/v0/mounts/{}", self.id))
            .unwrap();

        let response: UpdateMountResponse = client
            .http_client()
            .patch(url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(format!(
            "Updated mount {}\n  mount_point: {}\n  enabled: {}\n  auto_mount: {}\n  read_only: {}\n  cache_size_mb: {}\n  cache_ttl_secs: {}",
            response.mount.mount_id,
            response.mount.mount_point,
            response.mount.enabled,
            response.mount.auto_mount,
            response.mount.read_only,
            response.mount.cache_size_mb,
            response.mount.cache_ttl_secs
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SetError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl std::fmt::Display for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount set {}", self.id)
    }
}
