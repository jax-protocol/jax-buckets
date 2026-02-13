use clap::Args;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{ListMountsRequest, ListMountsResponse};

#[derive(Args, Debug, Clone)]
pub struct List {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[async_trait::async_trait]
impl Op for List {
    type Error = ListError;
    type Output = String;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let response: ListMountsResponse = client.call(ListMountsRequest {}).await?;

        if self.json {
            return Ok(serde_json::to_string_pretty(&response.mounts)?);
        }

        if response.mounts.is_empty() {
            return Ok("No mounts configured".to_string());
        }

        let mut output = String::new();
        output.push_str(&format!(
            "{:<36} {:<36} {:<30} {:<10} {:<5} {:<5}\n",
            "MOUNT ID", "BUCKET ID", "MOUNT POINT", "STATUS", "AUTO", "RO"
        ));
        output.push_str(&"-".repeat(140));
        output.push('\n');

        for mount in response.mounts {
            output.push_str(&format!(
                "{:<36} {:<36} {:<30} {:<10} {:<5} {:<5}\n",
                mount.mount_id,
                mount.bucket_id,
                truncate(&mount.mount_point, 28),
                mount.status,
                if mount.auto_mount { "yes" } else { "no" },
                if mount.read_only { "yes" } else { "no" },
            ));
        }

        Ok(output)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl std::fmt::Display for List {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mount list")
    }
}
