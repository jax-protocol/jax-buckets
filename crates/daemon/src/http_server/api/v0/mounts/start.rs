//! Start mount API endpoint

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

/// Request to start a mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMountRequest {
    pub mount_id: Uuid,
}

/// Response indicating mount was started
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMountResponse {
    pub started: bool,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StartMountError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(StartMountError::MountManagerUnavailable)?;

    mount_manager.start_mount(&id).await?;

    Ok((
        http::StatusCode::OK,
        Json(StartMountResponse { started: true }),
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
pub enum StartMountError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount error: {0}")]
    Mount(#[from] crate::fuse::MountError),
}

#[cfg(feature = "fuse")]
impl IntoResponse for StartMountError {
    fn into_response(self) -> Response {
        match self {
            StartMountError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            StartMountError::Mount(e) => {
                (http::StatusCode::BAD_REQUEST, format!("Mount error: {}", e)).into_response()
            }
        }
    }
}

// Client implementation - builds request for this operation
impl ApiRequest for StartMountRequest {
    type Response = StartMountResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url
            .join(&format!("/api/v0/mounts/{}/start", self.mount_id))
            .unwrap();
        client.post(full_url)
    }
}
