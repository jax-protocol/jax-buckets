//! Bucket IPC commands
//!
//! All commands communicate with the daemon via its HTTP API,
//! enabling both embedded and sidecar daemon modes.

use serde::{Deserialize, Serialize};
use tauri::State;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::AppState;

/// Bucket information returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub bucket_id: Uuid,
    pub name: String,
    pub link_hash: String,
    pub height: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// File/directory entry returned by ls command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub mime_type: String,
    pub link_hash: String,
}

/// Result of reading a file with cat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatResult {
    pub content: Vec<u8>,
    pub mime_type: String,
    pub size: usize,
}

/// History entry for version list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub link_hash: String,
    pub height: u64,
    pub published: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Share info for a bucket peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInfo {
    pub public_key: String,
    pub role: String,
    pub is_self: bool,
}

/// Get the daemon API base URL
async fn get_daemon_url(state: &State<'_, AppState>) -> Result<String, String> {
    let inner = state.inner.read().await;
    let inner = inner.as_ref().ok_or("Daemon not connected")?;
    Ok(format!("http://localhost:{}", inner.api_port))
}

/// Shared HTTP client (reused across requests within the same command)
fn http_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Parse a bucket_id string into Uuid
fn parse_bucket_id(bucket_id: &str) -> Result<Uuid, String> {
    bucket_id
        .parse()
        .map_err(|e| format!("Invalid bucket ID: {}", e))
}

/// Helper to make a JSON POST request and parse the response
async fn post_json<Req: Serialize, Resp: serde::de::DeserializeOwned>(
    base_url: &str,
    path: &str,
    request: &Req,
) -> Result<Resp, String> {
    let url = format!("{}{}", base_url, path);
    let client = http_client();
    let response = client
        .post(&url)
        .json(request)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Daemon API error ({}): {}", status, body));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

// --- Daemon API response types (match the HTTP endpoint types) ---

#[derive(Deserialize)]
struct ApiListResponse {
    buckets: Vec<ApiBucketInfo>,
}

#[derive(Deserialize)]
struct ApiBucketInfo {
    bucket_id: Uuid,
    name: String,
    link: common::linked_data::Link,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Deserialize)]
struct ApiCreateResponse {
    bucket_id: Uuid,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Deserialize)]
struct ApiLsResponse {
    items: Vec<ApiPathInfo>,
}

#[derive(Deserialize)]
struct ApiPathInfo {
    path: String,
    name: String,
    link: common::linked_data::Link,
    is_dir: bool,
    mime_type: String,
}

#[derive(Deserialize)]
struct ApiCatResponse {
    #[allow(dead_code)]
    path: String,
    content: String,
    size: usize,
    mime_type: String,
}

#[derive(Deserialize)]
struct ApiHistoryResponse {
    #[allow(dead_code)]
    bucket_id: Uuid,
    entries: Vec<ApiHistoryEntry>,
}

#[derive(Deserialize)]
struct ApiHistoryEntry {
    link_hash: String,
    height: u64,
    published: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Deserialize)]
struct ApiIsPublishedResponse {
    #[allow(dead_code)]
    bucket_id: Uuid,
    published: bool,
}

#[derive(Deserialize)]
struct ApiSharesResponse {
    #[allow(dead_code)]
    bucket_id: Uuid,
    #[allow(dead_code)]
    self_key: String,
    shares: Vec<ApiShareInfo>,
}

#[derive(Deserialize)]
struct ApiShareInfo {
    public_key: String,
    role: String,
    is_self: bool,
}

/// List all buckets
#[tauri::command]
pub async fn list_buckets(state: State<'_, AppState>) -> Result<Vec<BucketInfo>, String> {
    let base_url = get_daemon_url(&state).await?;

    #[derive(Serialize)]
    struct ListReq {
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    }

    let resp: ApiListResponse =
        post_json(&base_url, "/api/v0/bucket/list", &ListReq { prefix: None, limit: None }).await?;

    Ok(resp
        .buckets
        .into_iter()
        .map(|b| BucketInfo {
            bucket_id: b.bucket_id,
            name: b.name,
            link_hash: b.link.to_string(),
            height: 0,
            created_at: b.created_at,
        })
        .collect())
}

/// Create a new bucket
#[tauri::command]
pub async fn create_bucket(state: State<'_, AppState>, name: String) -> Result<BucketInfo, String> {
    let base_url = get_daemon_url(&state).await?;

    #[derive(Serialize)]
    struct CreateReq {
        name: String,
    }

    let resp: ApiCreateResponse =
        post_json(&base_url, "/api/v0/bucket", &CreateReq { name }).await?;

    Ok(BucketInfo {
        bucket_id: resp.bucket_id,
        name: resp.name,
        link_hash: String::new(),
        height: 0,
        created_at: resp.created_at,
    })
}

/// Delete a file or directory (or entire bucket at path "/")
#[tauri::command]
pub async fn delete_bucket(state: State<'_, AppState>, bucket_id: String) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct DeleteReq {
        bucket_id: Uuid,
        path: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/delete",
        &DeleteReq {
            bucket_id: bucket_uuid,
            path: "/".to_string(),
        },
    )
    .await?;

    Ok(())
}

/// List directory contents
#[tauri::command]
pub async fn ls(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct LsReq {
        bucket_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        deep: Option<bool>,
    }

    let resp: ApiLsResponse = post_json(
        &base_url,
        "/api/v0/bucket/ls",
        &LsReq {
            bucket_id: bucket_uuid,
            path: Some(path),
            deep: None,
        },
    )
    .await?;

    Ok(resp
        .items
        .into_iter()
        .map(|item| FileEntry {
            path: item.path,
            name: item.name,
            is_dir: item.is_dir,
            mime_type: item.mime_type,
            link_hash: item.link.to_string(),
        })
        .collect())
}

/// Read file contents
#[tauri::command]
pub async fn cat(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
) -> Result<CatResult, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct CatReq {
        bucket_id: Uuid,
        path: String,
    }

    let resp: ApiCatResponse = post_json(
        &base_url,
        "/api/v0/bucket/cat",
        &CatReq {
            bucket_id: bucket_uuid,
            path,
        },
    )
    .await?;

    // Decode base64 content
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD
        .decode(&resp.content)
        .map_err(|e| format!("Failed to decode content: {}", e))?;

    Ok(CatResult {
        content,
        mime_type: resp.mime_type,
        size: resp.size,
    })
}

/// Upload a file via multipart
#[tauri::command]
pub async fn add_file(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let form = reqwest::multipart::Form::new()
        .text("bucket_id", bucket_uuid.to_string())
        .text("mount_path", path)
        .part(
            "file",
            reqwest::multipart::Part::bytes(data).file_name(file_name),
        );

    let url = format!("{}/api/v0/bucket/add", base_url);
    let client = http_client();
    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to add file ({}): {}", status, body));
    }

    Ok(())
}

/// Update (overwrite) a file via multipart
#[tauri::command]
pub async fn update_file(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let form = reqwest::multipart::Form::new()
        .text("bucket_id", bucket_uuid.to_string())
        .text("mount_path", path)
        .part(
            "file",
            reqwest::multipart::Part::bytes(data).file_name(file_name),
        );

    let url = format!("{}/api/v0/bucket/update", base_url);
    let client = http_client();
    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to update file ({}): {}", status, body));
    }

    Ok(())
}

/// Rename a file or directory
#[tauri::command]
pub async fn rename_path(
    state: State<'_, AppState>,
    bucket_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct RenameReq {
        bucket_id: Uuid,
        old_path: String,
        new_path: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/rename",
        &RenameReq {
            bucket_id: bucket_uuid,
            old_path,
            new_path,
        },
    )
    .await?;

    Ok(())
}

/// Move a file or directory
#[tauri::command]
pub async fn move_path(
    state: State<'_, AppState>,
    bucket_id: String,
    source_path: String,
    dest_path: String,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct MvReq {
        bucket_id: Uuid,
        source_path: String,
        dest_path: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/mv",
        &MvReq {
            bucket_id: bucket_uuid,
            source_path,
            dest_path,
        },
    )
    .await?;

    Ok(())
}

/// Share a bucket with a peer
#[tauri::command]
pub async fn share_bucket(
    state: State<'_, AppState>,
    bucket_id: String,
    peer_public_key: String,
    role: String,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct ShareReq {
        bucket_id: Uuid,
        peer_public_key: String,
        role: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/share",
        &ShareReq {
            bucket_id: bucket_uuid,
            peer_public_key,
            role,
        },
    )
    .await?;

    Ok(())
}

/// Remove a share from a bucket
#[tauri::command]
pub async fn remove_share(
    state: State<'_, AppState>,
    bucket_id: String,
    peer_public_key: String,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct UnshareReq {
        bucket_id: Uuid,
        peer_public_key: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/unshare",
        &UnshareReq {
            bucket_id: bucket_uuid,
            peer_public_key,
        },
    )
    .await?;

    Ok(())
}

/// Check if the current HEAD of a bucket is published
#[tauri::command]
pub async fn is_published(state: State<'_, AppState>, bucket_id: String) -> Result<bool, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct IsPublishedReq {
        bucket_id: Uuid,
    }

    let resp: ApiIsPublishedResponse = post_json(
        &base_url,
        "/api/v0/bucket/is-published",
        &IsPublishedReq {
            bucket_id: bucket_uuid,
        },
    )
    .await?;

    Ok(resp.published)
}

/// Publish a bucket
#[tauri::command]
pub async fn publish_bucket(state: State<'_, AppState>, bucket_id: String) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct PublishReq {
        bucket_id: Uuid,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/publish",
        &PublishReq {
            bucket_id: bucket_uuid,
        },
    )
    .await?;

    Ok(())
}

/// Ping a peer
#[tauri::command]
pub async fn ping_peer(
    state: State<'_, AppState>,
    bucket_id: String,
    peer_public_key: String,
) -> Result<String, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct PingReq {
        bucket_id: Uuid,
        peer_public_key: String,
    }

    #[derive(Deserialize)]
    struct PingResp {
        message: String,
    }

    let resp: PingResp = post_json(
        &base_url,
        "/api/v0/bucket/ping",
        &PingReq {
            bucket_id: bucket_uuid,
            peer_public_key,
        },
    )
    .await?;

    Ok(resp.message)
}

/// Upload native files from disk via multipart (avoids large IPC transfers)
#[tauri::command]
pub async fn upload_native_files(
    state: State<'_, AppState>,
    bucket_id: String,
    mount_path: String,
    file_paths: Vec<String>,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    for file_path in file_paths {
        let path = std::path::Path::new(&file_path);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        let data = tokio::fs::read(&file_path)
            .await
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        let dest_path = if mount_path.ends_with('/') {
            format!("{}{}", mount_path, file_name)
        } else {
            format!("{}/{}", mount_path, file_name)
        };

        let form = reqwest::multipart::Form::new()
            .text("bucket_id", bucket_uuid.to_string())
            .text("mount_path", dest_path)
            .part(
                "file",
                reqwest::multipart::Part::bytes(data).file_name(file_name),
            );

        let url = format!("{}/api/v0/bucket/add", base_url);
        let client = http_client();
        let response = client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to upload file ({}): {}", status, body));
        }
    }

    Ok(())
}

/// Create a directory
#[tauri::command]
pub async fn mkdir(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct MkdirReq {
        bucket_id: Uuid,
        path: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/mkdir",
        &MkdirReq {
            bucket_id: bucket_uuid,
            path,
        },
    )
    .await?;

    Ok(())
}

/// Delete a file or directory
#[tauri::command]
pub async fn delete_path(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
) -> Result<(), String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct DeleteReq {
        bucket_id: Uuid,
        path: String,
    }

    let _: serde_json::Value = post_json(
        &base_url,
        "/api/v0/bucket/delete",
        &DeleteReq {
            bucket_id: bucket_uuid,
            path,
        },
    )
    .await?;

    Ok(())
}

/// Get bucket version history
#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
    bucket_id: String,
    page: Option<u32>,
) -> Result<Vec<HistoryEntry>, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct HistoryReq {
        bucket_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        page: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        page_size: Option<u32>,
    }

    let resp: ApiHistoryResponse = post_json(
        &base_url,
        "/api/v0/bucket/history",
        &HistoryReq {
            bucket_id: bucket_uuid,
            page,
            page_size: Some(50),
        },
    )
    .await?;

    Ok(resp
        .entries
        .into_iter()
        .map(|e| HistoryEntry {
            link_hash: e.link_hash,
            height: e.height,
            published: e.published,
            created_at: e.created_at,
        })
        .collect())
}

/// List directory contents at a specific version
#[tauri::command]
pub async fn ls_at_version(
    state: State<'_, AppState>,
    bucket_id: String,
    link_hash: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct LsReq {
        bucket_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        deep: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        at: Option<String>,
    }

    let resp: ApiLsResponse = post_json(
        &base_url,
        "/api/v0/bucket/ls",
        &LsReq {
            bucket_id: bucket_uuid,
            path: Some(path),
            deep: None,
            at: Some(link_hash),
        },
    )
    .await?;

    Ok(resp
        .items
        .into_iter()
        .map(|item| FileEntry {
            path: item.path,
            name: item.name,
            is_dir: item.is_dir,
            mime_type: item.mime_type,
            link_hash: item.link.to_string(),
        })
        .collect())
}

/// Get all shares for a bucket
#[tauri::command]
pub async fn get_bucket_shares(
    state: State<'_, AppState>,
    bucket_id: String,
) -> Result<Vec<ShareInfo>, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct SharesReq {
        bucket_id: Uuid,
    }

    let resp: ApiSharesResponse = post_json(
        &base_url,
        "/api/v0/bucket/shares",
        &SharesReq {
            bucket_id: bucket_uuid,
        },
    )
    .await?;

    Ok(resp
        .shares
        .into_iter()
        .map(|s| ShareInfo {
            public_key: s.public_key,
            role: s.role,
            is_self: s.is_self,
        })
        .collect())
}

/// Export a file to the native filesystem
#[tauri::command]
pub async fn export_file(
    state: State<'_, AppState>,
    bucket_id: String,
    path: String,
    dest_path: String,
) -> Result<(), String> {
    // Read file content via cat, then write to local filesystem
    let result = cat(state, bucket_id, path).await?;

    tokio::fs::write(&dest_path, &result.content)
        .await
        .map_err(|e| format!("Failed to write file '{}': {}", dest_path, e))?;

    Ok(())
}

/// Read file contents at a specific version
#[tauri::command]
pub async fn cat_at_version(
    state: State<'_, AppState>,
    bucket_id: String,
    link_hash: String,
    path: String,
) -> Result<CatResult, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid = parse_bucket_id(&bucket_id)?;

    #[derive(Serialize)]
    struct CatReq {
        bucket_id: Uuid,
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        at: Option<String>,
    }

    let resp: ApiCatResponse = post_json(
        &base_url,
        "/api/v0/bucket/cat",
        &CatReq {
            bucket_id: bucket_uuid,
            path,
            at: Some(link_hash),
        },
    )
    .await?;

    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD
        .decode(&resp.content)
        .map_err(|e| format!("Failed to decode content: {}", e))?;

    Ok(CatResult {
        content,
        mime_type: resp.mime_type,
        size: resp.size,
    })
}
