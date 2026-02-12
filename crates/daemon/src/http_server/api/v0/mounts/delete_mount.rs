//! Delete mount API endpoint

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
pub struct DeleteMountResponse {
    pub deleted: bool,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, DeleteMountError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(DeleteMountError::MountManagerUnavailable)?;

    let deleted = mount_manager.delete_mount(&id).await?;

    Ok((http::StatusCode::OK, Json(DeleteMountResponse { deleted })).into_response())
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
pub enum DeleteMountError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount error: {0}")]
    Mount(#[from] crate::fuse::MountError),
}

#[cfg(feature = "fuse")]
impl IntoResponse for DeleteMountError {
    fn into_response(self) -> Response {
        match self {
            DeleteMountError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            DeleteMountError::Mount(e) => (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Mount error: {}", e),
            )
                .into_response(),
        }
    }
}
