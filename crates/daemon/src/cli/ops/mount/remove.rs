use std::fmt;

use clap::Args;
use owo_colors::OwoColorize;
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{DeleteMountRequest, DeleteMountResponse};

#[derive(Args, Debug, Clone)]
pub struct Remove {
    /// Mount ID to remove
    pub id: Uuid,

    /// Force removal without confirmation
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug)]
pub struct RemoveOutput {
    pub mount_id: Uuid,
    pub deleted: bool,
}

impl fmt::Display for RemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.deleted {
            write!(
                f,
                "{} mount {}",
                "Removed".green().bold(),
                self.mount_id.bold()
            )
        } else {
            write!(f, "Mount {} {}", self.mount_id.bold(), "not found".red())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl Op for Remove {
    type Error = RemoveError;
    type Output = RemoveOutput;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        let request = DeleteMountRequest { mount_id: self.id };
        let response: DeleteMountResponse = client.call(request).await?;

        Ok(RemoveOutput {
            mount_id: self.id,
            deleted: response.deleted,
        })
    }
}
