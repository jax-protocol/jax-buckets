use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use common::prelude::MountError;

use crate::http_server::api::client::ApiRequest;
use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct IsPublishedRequest {
    /// Bucket ID to check
    #[arg(long)]
    pub bucket_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsPublishedResponse {
    pub bucket_id: Uuid,
    pub published: bool,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Json(req): Json<IsPublishedRequest>,
) -> Result<impl IntoResponse, IsPublishedError> {
    let mount = state.peer().mount_for_read(req.bucket_id).await?;
    let published = mount.is_published().await;

    Ok((
        http::StatusCode::OK,
        Json(IsPublishedResponse {
            bucket_id: req.bucket_id,
            published,
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum IsPublishedError {
    #[error("Mount error: {0}")]
    Mount(#[from] MountError),
}

impl IntoResponse for IsPublishedError {
    fn into_response(self) -> Response {
        (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self),
        )
            .into_response()
    }
}

impl ApiRequest for IsPublishedRequest {
    type Response = IsPublishedResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v0/bucket/is-published").unwrap();
        client.post(full_url).json(&self)
    }
}
