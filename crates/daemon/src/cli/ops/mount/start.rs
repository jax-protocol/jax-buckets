use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct Start {
    /// Mount ID to start
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StartMountResponse {
    started: bool,
}

#[async_trait::async_trait]
impl Op for Start {
    type Error = StartError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let client = ctx.client.clone();

        let url = client
            .base_url()
            .join(&format!("/api/v0/mounts/{}/start", self.id))
            .unwrap();

        let response: StartMountResponse =
            client.http_client().post(url).send().await?.json().await?;

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
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl std::fmt::Display for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount start {}", self.id)
    }
}
