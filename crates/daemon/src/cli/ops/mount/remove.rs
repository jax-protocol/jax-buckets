use clap::Args;
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

#[async_trait::async_trait]
impl Op for Remove {
    type Error = RemoveError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();

        // TODO: Add confirmation prompt if not force

        let request = DeleteMountRequest { mount_id: self.id };
        let response: DeleteMountResponse = client.call(request).await?;

        if response.deleted {
            Ok(format!("Removed mount {}", self.id))
        } else {
            Ok(format!("Mount {} not found", self.id))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount remove {}", self.id)
    }
}
