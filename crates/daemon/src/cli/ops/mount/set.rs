use clap::Args;
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{
    UpdateMountBody, UpdateMountRequest, UpdateMountResponse,
};

#[derive(Args, Debug, Clone)]
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

#[async_trait::async_trait]
impl Op for Set {
    type Error = SetError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        let request = UpdateMountRequest {
            mount_id: self.id,
            body: UpdateMountBody {
                mount_point: self.mount_point.clone(),
                enabled: self.enabled,
                auto_mount: self.auto_mount,
                read_only: self.read_only,
                cache_size_mb: self.cache_size,
                cache_ttl_secs: self.cache_ttl,
            },
        };

        let response: UpdateMountResponse = client.call(request).await?;

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
}

impl std::fmt::Display for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount set {}", self.id)
    }
}
