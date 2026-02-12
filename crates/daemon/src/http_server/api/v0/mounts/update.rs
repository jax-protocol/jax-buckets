//! Update mount API endpoint

use axum::extract::Path;
use axum::response::IntoResponse;
#[cfg(feature = "fuse")]
use axum::response::Response;
use uuid::Uuid;

#[cfg(feature = "fuse")]
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

#[cfg(feature = "fuse")]
use super::create::MountInfo;
#[cfg(feature = "fuse")]
use crate::database::mount_queries::UpdateMountConfig;
#[cfg(feature = "fuse")]
use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMountRequest {
    pub mount_point: Option<String>,
    pub enabled: Option<bool>,
    pub auto_mount: Option<bool>,
    pub read_only: Option<bool>,
    pub cache_size_mb: Option<u32>,
    pub cache_ttl_secs: Option<u32>,
}

#[cfg(feature = "fuse")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMountResponse {
    pub mount: MountInfo,
}

#[cfg(feature = "fuse")]
pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMountRequest>,
) -> Result<impl IntoResponse, UpdateMountError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(UpdateMountError::MountManagerUnavailable)?;

    let config = UpdateMountConfig {
        mount_point: req.mount_point,
        enabled: req.enabled,
        auto_mount: req.auto_mount,
        read_only: req.read_only,
        cache_size_mb: req.cache_size_mb,
        cache_ttl_secs: req.cache_ttl_secs,
    };

    let mount = mount_manager
        .update_mount(&id, config)
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
    Json(_req): Json<UpdateMountRequest>,
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
