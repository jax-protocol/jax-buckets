//! FUSE mount IPC commands
//!
//! These commands use the daemon's HTTP API for mount management,
//! enabling both embedded and sidecar daemon modes.
//! The `is_fuse_available` and `mount_bucket`/`unmount_bucket` commands
//! perform local checks and orchestrate multiple API calls.

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::AppState;

/// Get the daemon API base URL
async fn get_daemon_url(state: &State<'_, AppState>) -> Result<String, String> {
    let inner = state.inner.read().await;
    let inner = inner.as_ref().ok_or("Daemon not connected")?;
    Ok(format!("http://localhost:{}", inner.api_port))
}

/// Get the platform-specific base mount directory
fn get_mount_base_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        std::path::PathBuf::from("/Volumes")
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(user) = std::env::var("USER") {
            let media_path = std::path::PathBuf::from(format!("/media/{}", user));
            if media_path.exists() {
                return media_path;
            }
            let run_media_path = std::path::PathBuf::from(format!("/run/media/{}", user));
            if run_media_path.exists() {
                return run_media_path;
            }
            media_path
        } else {
            std::path::PathBuf::from("/media")
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        std::path::PathBuf::from("/mnt")
    }
}

/// Generate a unique mount point for a bucket, handling naming conflicts
fn generate_mount_point(bucket_name: &str, existing_mounts: &[String]) -> std::path::PathBuf {
    let base_dir = get_mount_base_dir();
    let sanitized_name = sanitize_mount_name(bucket_name);

    let base_path = base_dir.join(&sanitized_name);
    if !existing_mounts.contains(&base_path.to_string_lossy().to_string()) && !base_path.exists() {
        return base_path;
    }

    for i in 2..100 {
        let numbered_path = base_dir.join(format!("{}-{}", sanitized_name, i));
        if !existing_mounts.contains(&numbered_path.to_string_lossy().to_string())
            && !numbered_path.exists()
        {
            return numbered_path;
        }
    }

    let uuid_suffix = Uuid::new_v4().to_string()[..8].to_string();
    base_dir.join(format!("{}-{}", sanitized_name, uuid_suffix))
}

/// Sanitize bucket name for use as mount point
fn sanitize_mount_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Create mount point directory with platform-specific privilege escalation
#[cfg(feature = "fuse")]
async fn create_mount_point(path: &std::path::Path) -> Result<(), String> {
    if std::fs::create_dir_all(path).is_ok() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"do shell script "mkdir -p '{}'" with administrator privileges"#,
            path.display()
        );

        let output = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to create mount point: {}", stderr))
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("pkexec")
            .args(["mkdir", "-p", &path.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to run pkexec: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Failed to create mount point: {}", stderr))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported platform for privilege escalation".to_string())
    }
}

/// Mount information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    pub mount_id: String,
    pub bucket_id: String,
    pub mount_point: String,
    pub enabled: bool,
    pub auto_mount: bool,
    pub read_only: bool,
    pub cache_size_mb: u32,
    pub cache_ttl_secs: u32,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to create a new mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMountRequest {
    pub bucket_id: String,
    pub mount_point: String,
    #[serde(default)]
    pub auto_mount: bool,
    #[serde(default)]
    pub read_only: bool,
    pub cache_size_mb: Option<u32>,
    pub cache_ttl_secs: Option<u32>,
}

/// Request to update a mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMountRequest {
    pub mount_point: Option<String>,
    pub enabled: Option<bool>,
    pub auto_mount: Option<bool>,
    pub read_only: Option<bool>,
    pub cache_size_mb: Option<u32>,
    pub cache_ttl_secs: Option<u32>,
}

// --- API response types matching daemon HTTP endpoints ---

#[derive(Debug, Deserialize)]
struct ApiMountInfo {
    mount_id: Uuid,
    bucket_id: Uuid,
    mount_point: String,
    enabled: bool,
    auto_mount: bool,
    read_only: bool,
    cache_size_mb: u32,
    cache_ttl_secs: u32,
    status: String,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<ApiMountInfo> for MountInfo {
    fn from(m: ApiMountInfo) -> Self {
        Self {
            mount_id: m.mount_id.to_string(),
            bucket_id: m.bucket_id.to_string(),
            mount_point: m.mount_point,
            enabled: m.enabled,
            auto_mount: m.auto_mount,
            read_only: m.read_only,
            cache_size_mb: m.cache_size_mb,
            cache_ttl_secs: m.cache_ttl_secs,
            status: m.status,
            error_message: m.error_message,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Deserialize)]
struct ApiListMountsResponse {
    mounts: Vec<ApiMountInfo>,
}

#[derive(Deserialize)]
struct ApiCreateMountResponse {
    mount: ApiMountInfo,
}

#[derive(Deserialize)]
struct ApiGetMountResponse {
    mount: ApiMountInfo,
}

#[derive(Deserialize)]
struct ApiUpdateMountResponse {
    mount: ApiMountInfo,
}

#[derive(Deserialize)]
struct ApiDeleteMountResponse {
    deleted: bool,
}

#[derive(Deserialize)]
struct ApiStartMountResponse {
    #[allow(dead_code)]
    started: bool,
}

#[derive(Deserialize)]
struct ApiStopMountResponse {
    #[allow(dead_code)]
    stopped: bool,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// List all mounts
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn list_mounts(state: State<'_, AppState>) -> Result<Vec<MountInfo>, String> {
    let base_url = get_daemon_url(&state).await?;
    let url = format!("{}/api/v0/mounts/", base_url);

    let client = http_client();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to list mounts ({}): {}", status, body));
    }

    let resp: ApiListMountsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(resp.mounts.into_iter().map(Into::into).collect())
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn list_mounts(_state: State<'_, AppState>) -> Result<Vec<MountInfo>, String> {
    Ok(Vec::new())
}

/// Create a new mount
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn create_mount(
    state: State<'_, AppState>,
    request: CreateMountRequest,
) -> Result<MountInfo, String> {
    let base_url = get_daemon_url(&state).await?;
    let url = format!("{}/api/v0/mounts/", base_url);

    let bucket_id =
        Uuid::parse_str(&request.bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    #[derive(Serialize)]
    struct ApiCreateReq {
        bucket_id: Uuid,
        mount_point: String,
        auto_mount: bool,
        read_only: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_size_mb: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_ttl_secs: Option<u32>,
    }

    let client = http_client();
    let response = client
        .post(&url)
        .json(&ApiCreateReq {
            bucket_id,
            mount_point: request.mount_point,
            auto_mount: request.auto_mount,
            read_only: request.read_only,
            cache_size_mb: request.cache_size_mb,
            cache_ttl_secs: request.cache_ttl_secs,
        })
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to create mount ({}): {}", status, body));
    }

    let resp: ApiCreateMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(resp.mount.into())
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn create_mount(
    _state: State<'_, AppState>,
    _request: CreateMountRequest,
) -> Result<MountInfo, String> {
    Err("FUSE support not enabled".to_string())
}

/// Get a mount by ID
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn get_mount(state: State<'_, AppState>, mount_id: String) -> Result<MountInfo, String> {
    let base_url = get_daemon_url(&state).await?;
    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;
    let url = format!("{}/api/v0/mounts/{}", base_url, id);

    let client = http_client();
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to get mount ({}): {}", status, body));
    }

    let resp: ApiGetMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(resp.mount.into())
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn get_mount(
    _state: State<'_, AppState>,
    _mount_id: String,
) -> Result<MountInfo, String> {
    Err("FUSE support not enabled".to_string())
}

/// Update a mount
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn update_mount(
    state: State<'_, AppState>,
    mount_id: String,
    request: UpdateMountRequest,
) -> Result<MountInfo, String> {
    let base_url = get_daemon_url(&state).await?;
    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;
    let url = format!("{}/api/v0/mounts/{}", base_url, id);

    let client = http_client();
    let response = client
        .patch(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to update mount ({}): {}", status, body));
    }

    let resp: ApiUpdateMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(resp.mount.into())
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn update_mount(
    _state: State<'_, AppState>,
    _mount_id: String,
    _request: UpdateMountRequest,
) -> Result<MountInfo, String> {
    Err("FUSE support not enabled".to_string())
}

/// Delete a mount
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn delete_mount(state: State<'_, AppState>, mount_id: String) -> Result<bool, String> {
    let base_url = get_daemon_url(&state).await?;
    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;
    let url = format!("{}/api/v0/mounts/{}", base_url, id);

    let client = http_client();
    let response = client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to delete mount ({}): {}", status, body));
    }

    let resp: ApiDeleteMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(resp.deleted)
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn delete_mount(_state: State<'_, AppState>, _mount_id: String) -> Result<bool, String> {
    Err("FUSE support not enabled".to_string())
}

/// Start a mount (spawn FUSE process)
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn start_mount(state: State<'_, AppState>, mount_id: String) -> Result<bool, String> {
    let base_url = get_daemon_url(&state).await?;
    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;
    let url = format!("{}/api/v0/mounts/{}/start", base_url, id);

    let client = http_client();
    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to start mount ({}): {}", status, body));
    }

    let _resp: ApiStartMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(true)
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn start_mount(_state: State<'_, AppState>, _mount_id: String) -> Result<bool, String> {
    Err("FUSE support not enabled".to_string())
}

/// Stop a mount
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn stop_mount(state: State<'_, AppState>, mount_id: String) -> Result<bool, String> {
    let base_url = get_daemon_url(&state).await?;
    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;
    let url = format!("{}/api/v0/mounts/{}/stop", base_url, id);

    let client = http_client();
    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to stop mount ({}): {}", status, body));
    }

    let _resp: ApiStopMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(true)
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn stop_mount(_state: State<'_, AppState>, _mount_id: String) -> Result<bool, String> {
    Err("FUSE support not enabled".to_string())
}

/// Check if FUSE support is available on the host system (local check, no HTTP)
#[tauri::command]
pub async fn is_fuse_available() -> Result<bool, String> {
    #[cfg(not(feature = "fuse"))]
    {
        Ok(false)
    }

    #[cfg(feature = "fuse")]
    {
        Ok(check_fuse_installed())
    }
}

#[cfg(feature = "fuse")]
fn check_fuse_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::path::Path::new("/Library/Filesystems/macfuse.fs").exists()
            || std::path::Path::new("/Library/Filesystems/osxfuse.fs").exists()
    }

    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/dev/fuse").exists()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

/// Mount a bucket with automatic mount point selection (desktop app simplified API)
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn mount_bucket(
    state: State<'_, AppState>,
    bucket_id: String,
) -> Result<MountInfo, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid =
        Uuid::parse_str(&bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    // Get bucket name via list endpoint
    #[derive(Serialize)]
    struct ListReq {
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    }

    #[derive(Deserialize)]
    struct ListResp {
        buckets: Vec<ListBucketInfo>,
    }

    #[derive(Deserialize)]
    struct ListBucketInfo {
        bucket_id: Uuid,
        name: String,
    }

    let client = http_client();
    let list_resp: ListResp = {
        let response = client
            .post(format!("{}/api/v0/bucket/list", base_url))
            .json(&ListReq {
                prefix: None,
                limit: None,
            })
            .send()
            .await
            .map_err(|e| format!("Failed to list buckets: {}", e))?;
        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse bucket list: {}", e))?
    };

    let bucket_name = list_resp
        .buckets
        .into_iter()
        .find(|b| b.bucket_id == bucket_uuid)
        .map(|b| b.name)
        .unwrap_or_else(|| "bucket".to_string());

    // Get existing mounts to check for conflicts
    let mounts_resp: ApiListMountsResponse = {
        let response = client
            .get(format!("{}/api/v0/mounts/", base_url))
            .send()
            .await
            .map_err(|e| format!("Failed to list mounts: {}", e))?;
        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse mounts list: {}", e))?
    };

    let existing_mounts: Vec<String> = mounts_resp
        .mounts
        .iter()
        .map(|m| m.mount_point.clone())
        .collect();

    // Generate unique mount point
    let mount_point = generate_mount_point(&bucket_name, &existing_mounts);
    let mount_point_str = mount_point.to_string_lossy().to_string();

    // Create mount point directory
    create_mount_point(&mount_point).await?;

    // Create mount via API
    #[derive(Serialize)]
    struct CreateReq {
        bucket_id: Uuid,
        mount_point: String,
        auto_mount: bool,
        read_only: bool,
    }

    let create_resp: ApiCreateMountResponse = {
        let response = client
            .post(format!("{}/api/v0/mounts/", base_url))
            .json(&CreateReq {
                bucket_id: bucket_uuid,
                mount_point: mount_point_str,
                auto_mount: false,
                read_only: false,
            })
            .send()
            .await
            .map_err(|e| format!("Failed to create mount: {}", e))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Failed to create mount: {}", body));
        }

        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse create mount response: {}", e))?
    };

    let mount_id = create_resp.mount.mount_id;

    // Start the mount
    let start_url = format!("{}/api/v0/mounts/{}/start", base_url, mount_id);
    let response = client
        .post(&start_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start mount: {}", e))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to start mount: {}", body));
    }

    // Get updated mount info
    let get_url = format!("{}/api/v0/mounts/{}", base_url, mount_id);
    let response = client
        .get(&get_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get mount: {}", e))?;

    let get_resp: ApiGetMountResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse mount info: {}", e))?;

    Ok(get_resp.mount.into())
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn mount_bucket(
    _state: State<'_, AppState>,
    _bucket_id: String,
) -> Result<MountInfo, String> {
    Err("FUSE support not enabled".to_string())
}

/// Unmount a bucket by bucket ID (finds and stops the mount)
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn unmount_bucket(state: State<'_, AppState>, bucket_id: String) -> Result<bool, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid =
        Uuid::parse_str(&bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    let client = http_client();

    // List mounts to find the one for this bucket
    let mounts_resp: ApiListMountsResponse = {
        let response = client
            .get(format!("{}/api/v0/mounts/", base_url))
            .send()
            .await
            .map_err(|e| format!("Failed to list mounts: {}", e))?;
        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse mounts list: {}", e))?
    };

    let mount = mounts_resp
        .mounts
        .into_iter()
        .find(|m| m.bucket_id == bucket_uuid)
        .ok_or("No mount found for this bucket")?;

    // Stop the mount
    let stop_url = format!("{}/api/v0/mounts/{}/stop", base_url, mount.mount_id);
    let response = client
        .post(&stop_url)
        .send()
        .await
        .map_err(|e| format!("Failed to stop mount: {}", e))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to stop mount: {}", body));
    }

    // Delete the mount config
    let delete_url = format!("{}/api/v0/mounts/{}", base_url, mount.mount_id);
    let response = client
        .delete(&delete_url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete mount: {}", e))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to delete mount: {}", body));
    }

    Ok(true)
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn unmount_bucket(
    _state: State<'_, AppState>,
    _bucket_id: String,
) -> Result<bool, String> {
    Err("FUSE support not enabled".to_string())
}

/// Check if a bucket is currently mounted
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn is_bucket_mounted(
    state: State<'_, AppState>,
    bucket_id: String,
) -> Result<Option<MountInfo>, String> {
    let base_url = get_daemon_url(&state).await?;
    let bucket_uuid =
        Uuid::parse_str(&bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    let client = http_client();
    let response = client
        .get(format!("{}/api/v0/mounts/", base_url))
        .send()
        .await
        .map_err(|e| format!("Failed to list mounts: {}", e))?;

    let resp: ApiListMountsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse mounts list: {}", e))?;

    let mount = resp
        .mounts
        .into_iter()
        .find(|m| m.bucket_id == bucket_uuid && m.status == "running");

    Ok(mount.map(Into::into))
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn is_bucket_mounted(
    _state: State<'_, AppState>,
    _bucket_id: String,
) -> Result<Option<MountInfo>, String> {
    Ok(None)
}
