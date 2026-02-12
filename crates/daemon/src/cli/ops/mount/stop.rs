use clap::Args;
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{StopMountRequest, StopMountResponse};

#[derive(Args, Debug, Clone)]
pub struct Stop {
    /// Mount ID to stop
    pub id: Uuid,
}

#[async_trait::async_trait]
impl Op for Stop {
    type Error = StopError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        let request = StopMountRequest { mount_id: self.id };
        let response: StopMountResponse = client.call(request).await?;

        if response.stopped {
            Ok(format!("Stopped mount {}", self.id))
        } else {
            Ok(format!("Failed to stop mount {}", self.id))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StopError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount stop {}", self.id)
    }
}
