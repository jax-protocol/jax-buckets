use std::fmt;

use clap::Args;
use owo_colors::OwoColorize;
use time::OffsetDateTime;
use uuid::Uuid;

use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::bucket::create::{CreateRequest, CreateResponse};

#[derive(Args, Debug, Clone)]
pub struct Create {
    /// Name of the bucket to create
    pub name: String,
}

#[derive(Debug)]
pub struct CreateOutput {
    pub name: String,
    pub bucket_id: Uuid,
    pub created_at: OffsetDateTime,
}

impl fmt::Display for CreateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} bucket {}",
            "Created".green().bold(),
            self.name.bold()
        )?;
        writeln!(f, "  {} {}", "id:".dimmed(), self.bucket_id)?;
        write!(f, "  {} {}", "at:".dimmed(), self.created_at)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for Create {
    type Error = CreateError;
    type Output = CreateOutput;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let request = CreateRequest {
            name: self.name.clone(),
        };
        let response: CreateResponse = client.call(request).await?;

        Ok(CreateOutput {
            name: response.name,
            bucket_id: response.bucket_id,
            created_at: response.created_at,
        })
    }
}
