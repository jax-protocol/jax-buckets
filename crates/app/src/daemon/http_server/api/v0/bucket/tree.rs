use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use common::bucket_log::BucketLogProvider;

use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeRequest {
    pub bucket_id: Uuid,
}

/// A node in the version tree
#[derive(Debug, Clone, Serialize)]
pub struct TreeNode {
    /// The link hash as string (unique identifier)
    pub id: String,
    /// The height in the chain
    pub height: u64,
    /// Whether this node is on the canonical chain
    pub is_canonical: bool,
    /// Whether this branch was merged
    pub is_merged: bool,
    /// The parent link hash (previous in chain)
    pub parent_id: Option<String>,
    /// The name at this version
    pub name: String,
    /// When this version was created
    pub created_at: String,
}

/// Information about a merge event for visualization
#[derive(Debug, Clone, Serialize)]
pub struct TreeMerge {
    /// The orphaned branch that was merged
    pub link_from: String,
    /// The canonical head it was merged onto
    pub link_onto: String,
    /// The resulting new head
    pub result_link: String,
    /// Height of the orphaned branch
    pub height_from: u64,
    /// Height merged onto
    pub height_onto: u64,
    /// Height of the result
    pub result_height: u64,
    /// Operations merged
    pub ops_merged: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreeResponse {
    /// All nodes in the tree
    pub nodes: Vec<TreeNode>,
    /// All merge events
    pub merges: Vec<TreeMerge>,
    /// The current canonical head
    pub canonical_head: String,
    /// The maximum height in the tree
    pub max_height: u64,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Json(request): Json<TreeRequest>,
) -> Result<impl IntoResponse, TreeError> {
    let bucket_id = request.bucket_id;

    // Get all log entries
    let all_entries = state
        .database()
        .get_all_bucket_logs(&bucket_id)
        .await
        .map_err(|e| TreeError::Database(e.to_string()))?;

    if all_entries.is_empty() {
        return Err(TreeError::NotFound("Bucket has no log entries".to_string()));
    }

    // Get canonical head
    let (canonical_link, _) = state
        .peer()
        .logs()
        .head(bucket_id, None)
        .await
        .map_err(|e| TreeError::Internal(format!("Failed to get head: {}", e)))?;

    let canonical_link_str = canonical_link.to_string();

    // Build canonical chain by walking backwards
    let entries_by_link: HashMap<String, _> = all_entries
        .iter()
        .map(|e| (e.current_link.to_string(), e))
        .collect();

    let mut canonical_chain: HashSet<String> = HashSet::new();
    let mut current = Some(canonical_link_str.clone());
    while let Some(link_str) = current {
        canonical_chain.insert(link_str.clone());
        current = entries_by_link
            .get(&link_str)
            .and_then(|e| e.previous_link.as_ref().map(|l| l.to_string()));
    }

    // Get merge log entries
    let merge_entries = state
        .database()
        .get_merge_log_entries(&bucket_id)
        .await
        .unwrap_or_default();

    let merged_links: HashSet<String> = merge_entries
        .iter()
        .map(|m| m.link_from.to_string())
        .collect();

    // Build nodes
    let max_height = all_entries.iter().map(|e| e.height).max().unwrap_or(0);

    let nodes: Vec<TreeNode> = all_entries
        .iter()
        .map(|e| {
            let link_str = e.current_link.to_string();
            let is_merged = merged_links.contains(&link_str);
            let is_canonical = canonical_chain.contains(&link_str);

            TreeNode {
                id: link_str,
                height: e.height,
                is_canonical,
                is_merged,
                parent_id: e.previous_link.as_ref().map(|l| l.to_string()),
                name: e.name.clone(),
                created_at: e
                    .created_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "unknown".to_string()),
            }
        })
        .collect();

    // Build merge info
    let merges: Vec<TreeMerge> = merge_entries
        .iter()
        .map(|m| TreeMerge {
            link_from: m.link_from.to_string(),
            link_onto: m.link_onto.to_string(),
            result_link: m.result_link.to_string(),
            height_from: m.height_from,
            height_onto: m.height_onto,
            result_height: m.result_height,
            ops_merged: m.ops_merged,
        })
        .collect();

    Ok((
        http::StatusCode::OK,
        axum::Json(TreeResponse {
            nodes,
            merges,
            canonical_head: canonical_link_str,
            max_height,
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for TreeError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            TreeError::Database(e) => (http::StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
            TreeError::NotFound(e) => (http::StatusCode::NOT_FOUND, e.clone()),
            TreeError::Internal(e) => (http::StatusCode::INTERNAL_SERVER_ERROR, e.clone()),
        };

        (
            status,
            axum::Json(serde_json::json!({
                "error": message
            })),
        )
            .into_response()
    }
}
