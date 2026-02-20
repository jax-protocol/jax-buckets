use std::fmt;

use clap::Args;
use comfy_table::Table;

use crate::cli::op::{Op, OpContext};
use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::mounts::{ListMountsRequest, ListMountsResponse, MountInfo};

#[derive(Args, Debug, Clone)]
pub struct List;

#[derive(Debug)]
pub struct ListOutput {
    pub mounts: Vec<MountInfo>,
}

impl fmt::Display for ListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mounts.is_empty() {
            return write!(f, "No mounts configured");
        }

        let mut table = Table::new();
        table.set_header(vec![
            "MOUNT ID",
            "BUCKET ID",
            "PATH",
            "STATUS",
            "AUTO",
            "RO",
        ]);
        for mount in &self.mounts {
            table.add_row(vec![
                mount.mount_id.to_string(),
                mount.bucket_id.to_string(),
                mount.mount_point.clone(),
                mount.status.clone(),
                if mount.auto_mount { "yes" } else { "no" }.to_string(),
                if mount.read_only { "yes" } else { "no" }.to_string(),
            ]);
        }
        write!(f, "{table}")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl Op for List {
    type Error = ListError;
    type Output = ListOutput;

    async fn execute(&self, ctx: &OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let response: ListMountsResponse = client.call(ListMountsRequest {}).await?;

        Ok(ListOutput {
            mounts: response.mounts,
        })
    }
}
