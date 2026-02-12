//! List mounts API endpoint

#[cfg(feature = "fuse")]
use axum::extract::State;
use axum::response::IntoResponse;
#[cfg(feature = "fuse")]
use axum::response::Response;
#[cfg(feature = "fuse")]
use axum::Json;
#[cfg(feature = "fuse")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "fuse")]
use super::create::MountInfo;
#[cfg(feature = "fuse")]
use crate::ServiceState;

#[cfg(feature = "fuse")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMountsResponse {
    pub mounts: Vec<MountInfo>,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
) -> Result<impl IntoResponse, ListMountsError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(ListMountsError::MountManagerUnavailable)?;

    let mounts = mount_manager.list_mounts().await?;
    let mount_infos: Vec<MountInfo> = mounts.into_iter().map(Into::into).collect();

    Ok((
        http::StatusCode::OK,
        Json(ListMountsResponse {
            mounts: mount_infos,
        }),
    )
        .into_response())
}

#[cfg(not(feature = "fuse"))]
pub async fn handler() -> impl IntoResponse {
    (
        http::StatusCode::NOT_IMPLEMENTED,
        "FUSE support not enabled",
    )
        .into_response()
}

#[cfg(feature = "fuse")]
#[derive(Debug, thiserror::Error)]
pub enum ListMountsError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount error: {0}")]
    Mount(#[from] crate::fuse::MountError),
}

#[cfg(feature = "fuse")]
impl IntoResponse for ListMountsError {
    fn into_response(self) -> Response {
        match self {
            ListMountsError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            ListMountsError::Mount(e) => (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Mount error: {}", e),
            )
                .into_response(),
        }
    }
}
