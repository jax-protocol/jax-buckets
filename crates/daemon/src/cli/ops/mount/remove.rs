use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Remove {
    /// Mount ID to remove
    pub id: Uuid,

    /// Force removal without confirmation
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeleteMountResponse {
    deleted: bool,
}

#[async_trait::async_trait]
impl Op for Remove {
    type Error = RemoveError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let client = ctx.client.clone();

        // TODO: Add confirmation prompt if not force

        let url = client
            .base_url()
            .join(&format!("/api/v0/mounts/{}", self.id))
            .unwrap();

        let response: DeleteMountResponse = client
            .http_client()
            .delete(url)
            .send()
            .await?
            .json()
            .await?;

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
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount remove {}", self.id)
    }
}
