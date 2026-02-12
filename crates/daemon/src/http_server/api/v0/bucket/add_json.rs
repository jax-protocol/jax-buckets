//! JSON-based file add endpoint for FUSE writes
//!
//! This provides a simpler interface than the multipart add endpoint,
//! accepting base64-encoded content in a JSON request body.

use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;
use uuid::Uuid;

use common::prelude::{Link, MountError};

use crate::http_server::api::client::ApiRequest;
use crate::ServiceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddJsonRequest {
    /// Bucket ID to add file to
    pub bucket_id: Uuid,
    /// Absolute path in bucket where file should be created
    pub path: String,
    /// Base64-encoded file content
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddJsonResponse {
    pub path: String,
    pub size: usize,
    pub link: Link,
}

pub async fn handler(
    State(state): State<ServiceState>,
    Json(req): Json<AddJsonRequest>,
) -> Result<impl IntoResponse, AddJsonError> {
    let path = PathBuf::from(&req.path);

    if !path.is_absolute() {
        return Err(AddJsonError::InvalidPath(format!(
            "Path must be absolute: {}",
            req.path
        )));
    }

    // Decode base64 content
    let content = base64::engine::general_purpose::STANDARD
        .decode(&req.content)
        .map_err(|e| AddJsonError::InvalidContent(format!("Invalid base64: {}", e)))?;

    let size = content.len();

    // Load mount at current head
    let mut mount = state.peer().mount(req.bucket_id).await?;

    // Add file to mount
    mount.add(&path, Cursor::new(content)).await?;

    // Save mount
    let new_link = state.peer().save_mount(&mount, false).await?;

    tracing::debug!(
        "Added file {} ({} bytes) to bucket {}, new link: {}",
        req.path,
        size,
        req.bucket_id,
        new_link.hash()
    );

    Ok((
        http::StatusCode::OK,
        Json(AddJsonResponse {
            path: req.path,
            size,
            link: new_link,
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum AddJsonError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    #[error("Mount error: {0}")]
    Mount(#[from] MountError),
}

impl IntoResponse for AddJsonError {
    fn into_response(self) -> Response {
        match self {
            AddJsonError::InvalidPath(msg) | AddJsonError::InvalidContent(msg) => (
                http::StatusCode::BAD_REQUEST,
                format!("Bad request: {}", msg),
            )
                .into_response(),
            AddJsonError::Mount(_) => (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "Unexpected error".to_string(),
            )
                .into_response(),
        }
    }
}

impl ApiRequest for AddJsonRequest {
    type Response = AddJsonResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v0/bucket/add-json").unwrap();
        client.post(full_url).json(&self)
    }
}
