use axum::extract::Path;
#[cfg(feature = "fuse")]
use axum::extract::State;
use axum::response::IntoResponse;
#[cfg(feature = "fuse")]
use axum::response::Response;
#[cfg(feature = "fuse")]
use axum::Json;
#[cfg(feature = "fuse")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "fuse")]
use super::create::MountInfo;
#[cfg(feature = "fuse")]
use crate::ServiceState;

#[cfg(feature = "fuse")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMountResponse {
    pub mount: MountInfo,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, GetMountError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(GetMountError::MountManagerUnavailable)?;

    let mount = mount_manager
        .get_mount(&id)
        .await?
        .ok_or(GetMountError::NotFound(id))?;

    Ok((
        http::StatusCode::OK,
        Json(GetMountResponse {
            mount: mount.into(),
        }),
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
pub enum GetMountError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount not found: {0}")]
    NotFound(Uuid),
    #[error("Mount error: {0}")]
    Mount(#[from] crate::fuse::MountError),
}

#[cfg(feature = "fuse")]
impl IntoResponse for GetMountError {
    fn into_response(self) -> Response {
        match self {
            GetMountError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            GetMountError::NotFound(id) => (
                http::StatusCode::NOT_FOUND,
                format!("Mount not found: {}", id),
            )
                .into_response(),
            GetMountError::Mount(e) => (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Mount error: {}", e),
            )
                .into_response(),
        }
    }
}
