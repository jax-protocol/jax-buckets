use jax_daemon::http_server::api::client::ApiError;
use jax_daemon::http_server::api::v0::bucket::latest_published::{
    LatestPublishedRequest, LatestPublishedResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum BucketIsPublishedError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for LatestPublishedRequest {
    type Error = BucketIsPublishedError;
    type Output = String;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let mut client = ctx.client.clone();
        let response: LatestPublishedResponse = client.call(self.clone()).await?;

        match (response.link, response.height) {
            (Some(link), Some(height)) => Ok(format!(
                "Bucket {} is published at height {}. Link: {}",
                response.bucket_id, height, link
            )),
            _ => Ok(format!("Bucket {} is not published.", response.bucket_id)),
        }
    }
}
