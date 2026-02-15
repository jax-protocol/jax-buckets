//! Cache stats API endpoint

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fuse::CacheStats;
use crate::ServiceState;

/// Response containing cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    pub mount_id: Uuid,
    pub stats: CacheStats,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, CacheStatsError> {
    let mount_manager = state.mount_manager().read().await;
    let mount_manager = mount_manager
        .as_ref()
        .ok_or(CacheStatsError::MountManagerUnavailable)?;

    let cache = mount_manager
        .get_mount_cache(&id)
        .await
        .ok_or(CacheStatsError::NotRunning(id))?;

    Ok((
        http::StatusCode::OK,
        Json(CacheStatsResponse {
            mount_id: id,
            stats: cache.stats(),
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum CacheStatsError {
    #[error("Mount manager unavailable")]
    MountManagerUnavailable,
    #[error("Mount not running: {0}")]
    NotRunning(Uuid),
}

impl IntoResponse for CacheStatsError {
    fn into_response(self) -> Response {
        match self {
            CacheStatsError::MountManagerUnavailable => (
                http::StatusCode::SERVICE_UNAVAILABLE,
                "Mount manager not available",
            )
                .into_response(),
            CacheStatsError::NotRunning(id) => (
                http::StatusCode::NOT_FOUND,
                format!("Mount not running or not found: {}", id),
            )
                .into_response(),
        }
    }
}
