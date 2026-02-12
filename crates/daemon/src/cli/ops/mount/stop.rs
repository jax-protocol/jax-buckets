use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Stop {
    /// Mount ID to stop
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StopMountResponse {
    stopped: bool,
}

#[async_trait::async_trait]
impl Op for Stop {
    type Error = StopError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let client = ctx.client.clone();

        let url = client
            .base_url()
            .join(&format!("/api/v0/mounts/{}/stop", self.id))
            .unwrap();

        let response: StopMountResponse =
            client.http_client().post(url).send().await?.json().await?;

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
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount stop {}", self.id)
    }
}
