use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use common::mount::PrincipalRole;
use common::prelude::{Link, MountError};
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::http_server::api::client::ApiRequest;
use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct StatRequest {
    /// Bucket ID to get stats for
    #[arg(long)]
    pub bucket_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatResponse {
    pub bucket_id: Uuid,
    pub name: String,
    pub height: u64,
    pub link: Link,
    pub published: bool,
    pub peers: Vec<StatPeerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatPeerInfo {
    pub public_key: String,
    pub role: String,
    pub is_self: bool,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Json(req): Json<StatRequest>,
) -> Result<impl IntoResponse, StatError> {
    let mount = state.peer().mount_for_read(req.bucket_id).await?;
    let inner = mount.inner().await;
    let manifest = inner.manifest();

    let self_key = state.peer().secret().public().to_hex();

    let peers: Vec<StatPeerInfo> = manifest
        .shares()
        .iter()
        .map(|(key_hex, share)| {
            let role = match share.role() {
                PrincipalRole::Owner => "Owner",
                PrincipalRole::Mirror => "Mirror",
            };
            StatPeerInfo {
                public_key: key_hex.clone(),
                role: role.to_string(),
                is_self: *key_hex == self_key,
            }
        })
        .collect();

    Ok((
        http::StatusCode::OK,
        Json(StatResponse {
            bucket_id: req.bucket_id,
            name: manifest.name().to_string(),
            height: inner.height(),
            link: mount.link().await,
            published: manifest.is_published(),
            peers,
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum StatError {
    #[error("Mount error: {0}")]
    Mount(#[from] MountError),
}

impl IntoResponse for StatError {
    fn into_response(self) -> Response {
        (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self),
        )
            .into_response()
    }
}

impl ApiRequest for StatRequest {
    type Response = StatResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v0/bucket/stat").unwrap();
        client.post(full_url).json(&self)
    }
}
