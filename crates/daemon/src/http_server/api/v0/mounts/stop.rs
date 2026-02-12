//! Stop mount API endpoint

use axum::extract::Path;
use axum::response::IntoResponse;
#[cfg(feature = "fuse")]
use axum::response::Response;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "fuse")]
use axum::extract::State;
#[cfg(feature = "fuse")]
use axum::Json;

use crate::http_server::api::client::ApiRequest;
#[cfg(feature = "fuse")]
use crate::ServiceState;

/// Request to stop a mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopMountRequest {
    pub mount_id: Uuid,
}

/// Response indicating mount was stopped
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopMountResponse {
    pub stopped: bool,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StopMountError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(StopMountError::MountManagerUnavailable)?;

    mount_manager.stop(&id).await?;

    Ok((
        http::StatusCode::OK,
        Json(StopMountResponse { stopped: true }),
    )
        .into_response())
}

#[cfg(not(feature = "fuse"))]
pub async fn handler(Path(_id): Path<Uuid>) -> impl IntoResponse {
    (
        http::StatusCode::NOT_IMPLEMENTED,
        "FUSE support not enabled",
    )
        .into_response()
}

#[cfg(feature = "fuse")]
#[derive(Debug, thiserror::Error)]
pub enum StopMountError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount error: {0}")]
    Mount(#[from] crate::fuse::MountError),
}

#[cfg(feature = "fuse")]
impl IntoResponse for StopMountError {
    fn into_response(self) -> Response {
        match self {
            StopMountError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            StopMountError::Mount(e) => {
                (http::StatusCode::BAD_REQUEST, format!("Mount error: {}", e)).into_response()
            }
        }
    }
}

// Client implementation - builds request for this operation
impl ApiRequest for StopMountRequest {
    type Response = StopMountResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url
            .join(&format!("/api/v0/mounts/{}/stop", self.mount_id))
            .unwrap();
        client.post(full_url)
    }
}
