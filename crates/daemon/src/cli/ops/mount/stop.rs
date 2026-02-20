use std::fmt;

use clap::Args;
use owo_colors::OwoColorize;
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{StopMountRequest, StopMountResponse};

#[derive(Args, Debug, Clone)]
pub struct Stop {
    /// Mount ID to stop
    pub id: Uuid,
}

#[derive(Debug)]
pub struct StopOutput {
    pub mount_id: Uuid,
    pub stopped: bool,
}

impl fmt::Display for StopOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.stopped {
            write!(
                f,
                "{} mount {}",
                "Stopped".green().bold(),
                self.mount_id.bold()
            )
        } else {
            write!(
                f,
                "{} to stop mount {}",
                "Failed".red().bold(),
                self.mount_id.bold()
            )
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StopError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl Op for Stop {
    type Error = StopError;
    type Output = StopOutput;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        let request = StopMountRequest { mount_id: self.id };
        let response: StopMountResponse = client.call(request).await?;

        Ok(StopOutput {
            mount_id: self.id,
            stopped: response.stopped,
        })
    }
}
