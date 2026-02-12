use clap::Args;
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{CreateMountRequest, CreateMountResponse};

#[derive(Args, Debug, Clone)]
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

        let request = CreateMountRequest {
            bucket_id,
            mount_point: self.path.clone(),
            auto_mount: self.auto_mount,
            read_only: self.read_only,
            cache_size_mb: Some(self.cache_size),
            cache_ttl_secs: Some(self.cache_ttl),
        };

        let response: CreateMountResponse = client.call(request).await?;

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
}

impl std::fmt::Display for Add {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount add {} {}", self.bucket, self.path)
    }
}
