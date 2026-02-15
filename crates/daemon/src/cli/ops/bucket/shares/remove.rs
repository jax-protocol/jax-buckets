use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::bucket::unshare::{UnshareRequest, UnshareResponse};

#[derive(Debug, thiserror::Error)]
pub enum BucketUnshareError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for UnshareRequest {
    type Error = BucketUnshareError;
    type Output = String;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let response: UnshareResponse = client.call(self.clone()).await?;

        Ok(format!(
            "Peer {} removed from bucket {}. New bucket link: {}",
            response.peer_public_key, response.bucket_id, response.new_bucket_link
        ))
    }
}
