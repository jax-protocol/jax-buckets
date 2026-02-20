use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::http_server::api::client::ApiRequest;
use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct HistoryRequest {
    /// Bucket ID to get history for
    #[arg(long)]
    pub bucket_id: Uuid,

    /// Page number (0-indexed)
    #[serde(default)]
    #[arg(long)]
    pub page: Option<u32>,

    /// Number of entries per page (default 50)
    #[serde(default)]
    #[arg(long)]
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub bucket_id: Uuid,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub link_hash: String,
    pub height: u64,
    pub published: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Json(req): Json<HistoryRequest>,
) -> Result<impl IntoResponse, HistoryError> {
    let page = req.page.unwrap_or(0);
    let page_size = req.page_size.unwrap_or(50);

    let entries = state
        .database()
        .get_bucket_logs(&req.bucket_id, page, page_size)
        .await
        .map_err(|e| HistoryError::Database(e.to_string()))?;

    let history_entries: Vec<HistoryEntry> = entries
        .into_iter()
        .map(|e| HistoryEntry {
            link_hash: e.current_link.to_string(),
            height: e.height,
            published: e.published,
            created_at: e.created_at,
        })
        .collect();

    Ok((
        http::StatusCode::OK,
        Json(HistoryResponse {
            bucket_id: req.bucket_id,
            entries: history_entries,
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Database error: {0}")]
    Database(String),
}

impl IntoResponse for HistoryError {
    fn into_response(self) -> Response {
        (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self),
        )
            .into_response()
    }
}

impl ApiRequest for HistoryRequest {
    type Response = HistoryResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v0/bucket/history").unwrap();
        client.post(full_url).json(&self)
    }
}
