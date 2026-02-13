use clap::Args;
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{StartMountRequest, StartMountResponse};

#[derive(Args, Debug, Clone)]
pub struct Start {
    /// Mount ID to start
    pub id: Uuid,
}

#[async_trait::async_trait]
impl Op for Start {
    type Error = StartError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        let request = StartMountRequest { mount_id: self.id };
        let response: StartMountResponse = client.call(request).await?;

        if response.started {
            Ok(format!("Started mount {}", self.id))
        } else {
            Ok(format!("Failed to start mount {}", self.id))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

impl std::fmt::Display for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount start {}", self.id)
    }
}
