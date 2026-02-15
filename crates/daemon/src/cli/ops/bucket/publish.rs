use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::bucket::publish::{PublishRequest, PublishResponse};

#[derive(Debug, thiserror::Error)]
pub enum BucketPublishError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for PublishRequest {
    type Error = BucketPublishError;
    type Output = String;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let response: PublishResponse = client.call(self.clone()).await?;

        Ok(format!(
            "Bucket {} published. New bucket link: {}",
            response.bucket_id, response.new_bucket_link
        ))
    }
}
