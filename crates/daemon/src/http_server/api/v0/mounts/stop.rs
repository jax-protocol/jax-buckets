//! Stop mount API endpoint

use axum::extract::Path;
use axum::response::IntoResponse;
#[cfg(feature = "fuse")]
use axum::response::Response;
use uuid::Uuid;

#[cfg(feature = "fuse")]
use axum::extract::State;
#[cfg(feature = "fuse")]
use axum::Json;
#[cfg(feature = "fuse")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "fuse")]
use crate::ServiceState;

#[cfg(feature = "fuse")]
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

    mount_manager.stop_mount(&id).await?;

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
