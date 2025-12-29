use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use common::linked_data::Link;
use common::mount::MountError;

use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileRequest {
    pub bucket_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileResponse {
    pub success: bool,
    pub new_link: Link,
    pub ops_merged: usize,
    pub branches_merged: usize,
    pub message: String,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Json(request): Json<ReconcileRequest>,
) -> Result<impl IntoResponse, ReconcileError> {
    tracing::info!("Reconcile request for bucket {}", request.bucket_id);

    // Perform reconciliation
    let result = state.peer().reconcile_bucket(request.bucket_id).await?;

    // Insert merge log entries for each merged branch
    for branch in &result.merged_branches {
        if let Err(e) = state
            .database()
            .insert_merge_log(
                &request.bucket_id,
                &branch.link_from,
                branch.height_from,
                &result.canonical_link,
                result.canonical_height,
                &result.new_link,
                result.new_height,
                branch.ops_count as u32,
            )
            .await
        {
            tracing::warn!(
                "Failed to insert merge log entry for branch {:?}: {}",
                branch.link_from,
                e
            );
            // Continue even if logging fails - the merge already succeeded
        }
    }

    let branches_count = result.merged_branches.len();

    Ok((
        http::StatusCode::OK,
        axum::Json(ReconcileResponse {
            success: true,
            new_link: result.new_link,
            ops_merged: result.ops_merged,
            branches_merged: branches_count,
            message: format!(
                "Reconciled {} operations from {} branches onto canonical head",
                result.ops_merged, branches_count
            ),
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum ReconcileError {
    #[error("Mount error: {0}")]
    Mount(#[from] MountError),
}

impl IntoResponse for ReconcileError {
    fn into_response(self) -> axum::response::Response {
        let message = match &self {
            ReconcileError::Mount(e) => format!("{}", e),
        };

        (
            http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "success": false,
                "message": message
            })),
        )
            .into_response()
    }
}
