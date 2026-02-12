use base64::Engine;
use reqwest::{header::HeaderMap, header::HeaderValue, Client};
use url::Url;
use uuid::Uuid;

use super::error::ApiError;
use super::ApiRequest;
use crate::http_server::api::v0::bucket::add_json::{AddJsonRequest, AddJsonResponse};
use crate::http_server::api::v0::bucket::cat::{CatRequest, CatResponse};
use crate::http_server::api::v0::bucket::delete::{DeleteRequest, DeleteResponse};
use crate::http_server::api::v0::bucket::list::{ListRequest, ListResponse};
use crate::http_server::api::v0::bucket::ls::{LsRequest, LsResponse};
use crate::http_server::api::v0::bucket::mkdir::{MkdirRequest, MkdirResponse};
use crate::http_server::api::v0::bucket::mv::{MvRequest, MvResponse};

#[derive(Debug, Clone)]
pub struct ApiClient {
    pub remote: Url,
    client: Client,
}

impl ApiClient {
    pub fn new(remote: &Url) -> Result<Self, ApiError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        let client = Client::builder().default_headers(default_headers).build()?;

        Ok(Self {
            remote: remote.clone(),
            client,
        })
    }

    pub async fn call<T: ApiRequest>(&mut self, request: T) -> Result<T::Response, ApiError> {
        let request_builder = request.build_request(&self.remote, &self.client);
        let response = request_builder.send().await?;

        if response.status().is_success() {
            Ok(response.json::<T::Response>().await?)
        } else {
            Err(ApiError::HttpStatus(
                response.status(),
                response.text().await?,
            ))
        }
    }

    /// Resolve a bucket name to a UUID
    /// Returns the first bucket with an exact name match
    pub async fn resolve_bucket_name(&mut self, name: &str) -> Result<Uuid, ApiError> {
        let request = ListRequest {
            prefix: Some(name.to_string()),
            limit: Some(100),
        };

        let response: ListResponse = self.call(request).await?;

        response
            .buckets
            .into_iter()
            .find(|b| b.name == name)
            .map(|b| b.bucket_id)
            .ok_or_else(|| {
                ApiError::HttpStatus(
                    reqwest::StatusCode::NOT_FOUND,
                    format!("Bucket not found: {}", name),
                )
            })
    }

    /// Get the base URL for API requests
    pub fn base_url(&self) -> &Url {
        &self.remote
    }

    /// Get the underlying HTTP client for custom requests
    pub fn http_client(&self) -> &Client {
        &self.client
    }

    // ==================== FUSE helper methods ====================

    /// List contents of a path in a bucket
    pub async fn ls(&mut self, bucket_id: Uuid, path: &str) -> Result<FuseLsResponse, ApiError> {
        let request = LsRequest {
            bucket_id,
            path: Some(path.to_string()),
            deep: Some(false),
        };

        let response: LsResponse = self.call(request).await?;

        Ok(FuseLsResponse {
            entries: response
                .items
                .into_iter()
                .map(|item| FuseDirEntry {
                    name: item.name,
                    entry_type: if item.is_dir {
                        "directory".to_string()
                    } else {
                        "file".to_string()
                    },
                    size: None, // LsResponse doesn't include size
                    mime_type: Some(item.mime_type),
                })
                .collect(),
        })
    }

    /// Read file content from a bucket
    pub async fn cat(&mut self, bucket_id: Uuid, path: &str) -> Result<FuseCatResponse, ApiError> {
        let request = CatRequest {
            bucket_id,
            path: path.to_string(),
            at: None,
            download: None,
        };

        let response: CatResponse = self.call(request).await?;

        // Decode base64 content
        let content = base64::engine::general_purpose::STANDARD
            .decode(&response.content)
            .map_err(|e| ApiError::Other(format!("Failed to decode content: {}", e)))?;

        Ok(FuseCatResponse {
            content,
            mime_type: response.mime_type,
            size: response.size,
        })
    }

    /// Add file content to a bucket (base64-encoded JSON request)
    pub async fn add_bytes(
        &mut self,
        bucket_id: Uuid,
        path: &str,
        content: Vec<u8>,
    ) -> Result<AddJsonResponse, ApiError> {
        let content_b64 = base64::engine::general_purpose::STANDARD.encode(&content);

        let request = AddJsonRequest {
            bucket_id,
            path: path.to_string(),
            content: content_b64,
        };

        self.call(request).await
    }

    /// Create a directory in a bucket
    pub async fn mkdir(&mut self, bucket_id: Uuid, path: &str) -> Result<MkdirResponse, ApiError> {
        let request = MkdirRequest {
            bucket_id,
            path: path.to_string(),
        };

        self.call(request).await
    }

    /// Delete a file or directory from a bucket
    pub async fn delete(
        &mut self,
        bucket_id: Uuid,
        path: &str,
    ) -> Result<DeleteResponse, ApiError> {
        let request = DeleteRequest {
            bucket_id,
            path: path.to_string(),
        };

        self.call(request).await
    }

    /// Move/rename a file or directory in a bucket
    pub async fn mv(
        &mut self,
        bucket_id: Uuid,
        source: &str,
        dest: &str,
    ) -> Result<MvResponse, ApiError> {
        let request = MvRequest {
            bucket_id,
            source_path: source.to_string(),
            dest_path: dest.to_string(),
        };

        self.call(request).await
    }
}

/// FUSE-friendly directory listing response
#[derive(Debug, Clone)]
pub struct FuseLsResponse {
    pub entries: Vec<FuseDirEntry>,
}

/// FUSE-friendly directory entry
#[derive(Debug, Clone)]
pub struct FuseDirEntry {
    pub name: String,
    pub entry_type: String,
    pub size: Option<u64>,
    pub mime_type: Option<String>,
}

/// FUSE-friendly cat response with decoded content
#[derive(Debug, Clone)]
pub struct FuseCatResponse {
    pub content: Vec<u8>,
    pub mime_type: String,
    pub size: usize,
}
