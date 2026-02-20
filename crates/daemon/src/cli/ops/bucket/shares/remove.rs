use std::fmt;

use clap::Args;
use owo_colors::OwoColorize;
use uuid::Uuid;

use jax_daemon::http_server::api::client::{resolve_bucket, ApiError};
use jax_daemon::http_server::api::v0::bucket::unshare::{UnshareRequest, UnshareResponse};

#[derive(Args, Debug, Clone)]
pub struct Remove {
    /// Bucket name or UUID
    pub bucket: String,

    /// Public key of the peer to remove (hex-encoded)
    #[arg(long)]
    pub peer_public_key: String,
}

#[derive(Debug)]
pub struct ShareRemoveOutput {
    pub bucket_id: Uuid,
    pub peer_key: String,
    pub new_link: String,
}

impl fmt::Display for ShareRemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} peer {} from bucket {}",
            "Removed".green().bold(),
            self.peer_key.bold(),
            self.bucket_id.bold()
        )?;
        write!(f, "  {} {}", "link:".dimmed(), self.new_link)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShareRemoveError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for Remove {
    type Error = ShareRemoveError;
    type Output = ShareRemoveOutput;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let bucket_id = resolve_bucket(&mut client, &self.bucket).await?;

        let request = UnshareRequest {
            bucket_id,
            peer_public_key: self.peer_public_key.clone(),
        };
        let response: UnshareResponse = client.call(request).await?;

        Ok(ShareRemoveOutput {
            bucket_id: response.bucket_id,
            peer_key: response.peer_public_key,
            new_link: response.new_bucket_link,
        })
    }
}
