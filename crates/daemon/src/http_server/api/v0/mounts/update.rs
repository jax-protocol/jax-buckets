//! Update mount API endpoint

use axum::extract::Path;
use axum::response::IntoResponse;
#[cfg(feature = "fuse")]
use axum::response::Response;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "fuse")]
use axum::extract::State;
use axum::Json;

use super::create::MountInfo;
use crate::http_server::api::client::ApiRequest;
#[cfg(feature = "fuse")]
use crate::ServiceState;

/// Request body for updating a mount (used by handler)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMountBody {
    pub mount_point: Option<String>,
    pub enabled: Option<bool>,
    pub auto_mount: Option<bool>,
    pub read_only: Option<bool>,
    pub cache_size_mb: Option<u32>,
    pub cache_ttl_secs: Option<u32>,
}

/// Full request for updating a mount (used by client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMountRequest {
    pub mount_id: Uuid,
    #[serde(flatten)]
    pub body: UpdateMountBody,
}

/// Response containing the updated mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMountResponse {
    pub mount: MountInfo,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMountBody>,
) -> Result<impl IntoResponse, UpdateMountError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(UpdateMountError::MountManagerUnavailable)?;

    let mount = mount_manager
        .update(
            &id,
            req.mount_point.as_deref(),
            req.enabled,
            req.auto_mount,
            req.read_only,
            req.cache_size_mb,
            req.cache_ttl_secs,
        )
        .await?
        .ok_or(UpdateMountError::NotFound(id))?;

    Ok((
        http::StatusCode::OK,
        Json(UpdateMountResponse {
            mount: mount.into(),
        }),
    )
        .into_response())
}

#[cfg(not(feature = "fuse"))]
pub async fn handler(
    Path(_id): Path<Uuid>,
    Json(_req): Json<UpdateMountBody>,
) -> impl IntoResponse {
    (
        http::StatusCode::NOT_IMPLEMENTED,
        "FUSE support not enabled",
    )
        .into_response()
}

#[cfg(feature = "fuse")]
#[derive(Debug, thiserror::Error)]
pub enum UpdateMountError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount not found: {0}")]
    NotFound(Uuid),
    #[error("Mount error: {0}")]
    Mount(#[from] crate::fuse::MountError),
}

#[cfg(feature = "fuse")]
impl IntoResponse for UpdateMountError {
    fn into_response(self) -> Response {
        match self {
            UpdateMountError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            UpdateMountError::NotFound(id) => (
                http::StatusCode::NOT_FOUND,
                format!("Mount not found: {}", id),
            )
                .into_response(),
            UpdateMountError::Mount(e) => {
                (http::StatusCode::BAD_REQUEST, format!("Mount error: {}", e)).into_response()
            }
        }
    }
}

// Client implementation - builds request for this operation
impl ApiRequest for UpdateMountRequest {
    type Response = UpdateMountResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url
            .join(&format!("/api/v0/mounts/{}", self.mount_id))
            .unwrap();
        client.patch(full_url).json(&self.body)
    }
}
