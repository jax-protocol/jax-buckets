//! FUSE mount IPC commands
//!
//! These commands provide mount management through Tauri IPC.
//! They access the MountManager through ServiceState (when fuse feature is enabled).

use serde::{Deserialize, Serialize};
use tauri::State;
#[cfg(feature = "fuse")]
use uuid::Uuid;

use crate::AppState;

/// Get the platform-specific base mount directory
#[cfg(feature = "fuse")]
fn get_mount_base_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        std::path::PathBuf::from("/Volumes")
    }

    #[cfg(target_os = "linux")]
    {
        // Prefer /media/$USER, fall back to /run/media/$USER
        if let Ok(user) = std::env::var("USER") {
            let media_path = std::path::PathBuf::from(format!("/media/{}", user));
            if media_path.exists() {
                return media_path;
            }
            let run_media_path = std::path::PathBuf::from(format!("/run/media/{}", user));
            if run_media_path.exists() {
                return run_media_path;
            }
            // Default to /media/$USER
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
#[cfg(feature = "fuse")]
fn generate_mount_point(bucket_name: &str, existing_mounts: &[String]) -> std::path::PathBuf {
    let base_dir = get_mount_base_dir();
    let sanitized_name = sanitize_mount_name(bucket_name);

    // Check if base name is available
    let base_path = base_dir.join(&sanitized_name);
    if !existing_mounts.contains(&base_path.to_string_lossy().to_string())
        && !base_path.exists()
    {
        return base_path;
    }

    // Try numbered suffixes
    for i in 2..100 {
        let numbered_path = base_dir.join(format!("{}-{}", sanitized_name, i));
        if !existing_mounts.contains(&numbered_path.to_string_lossy().to_string())
            && !numbered_path.exists()
        {
            return numbered_path;
        }
    }

    // Fallback: use UUID suffix
    let uuid_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    base_dir.join(format!("{}-{}", sanitized_name, uuid_suffix))
}

/// Sanitize bucket name for use as mount point
#[cfg(feature = "fuse")]
fn sanitize_mount_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Create mount point directory with platform-specific privilege escalation
#[cfg(feature = "fuse")]
async fn create_mount_point(path: &std::path::Path) -> Result<(), String> {
    // First try without elevation
    if std::fs::create_dir_all(path).is_ok() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        // Use AppleScript to create with admin privileges
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
        // Use pkexec for privilege escalation
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

/// List all mounts
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn list_mounts(state: State<'_, AppState>) -> Result<Vec<MountInfo>, String> {
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    let mounts = manager
        .list_mounts()
        .await
        .map_err(|e| format!("Failed to list mounts: {}", e))?;

    Ok(mounts.into_iter().map(fuse_mount_to_info).collect())
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn list_mounts(_state: State<'_, AppState>) -> Result<Vec<MountInfo>, String> {
    Ok(Vec::new()) // FUSE not enabled
}

/// Create a new mount
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn create_mount(
    state: State<'_, AppState>,
    request: CreateMountRequest,
) -> Result<MountInfo, String> {
    use jax_daemon::mount_queries::CreateMountConfig;

    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let bucket_id = Uuid::parse_str(&request.bucket_id)
        .map_err(|e| format!("Invalid bucket ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    let config = CreateMountConfig {
        bucket_id,
        mount_point: request.mount_point,
        auto_mount: request.auto_mount,
        read_only: request.read_only,
        cache_size_mb: request.cache_size_mb,
        cache_ttl_secs: request.cache_ttl_secs,
    };

    let mount = manager
        .create_mount(config)
        .await
        .map_err(|e| format!("Failed to create mount: {}", e))?;

    Ok(fuse_mount_to_info(mount))
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
pub async fn get_mount(
    state: State<'_, AppState>,
    mount_id: String,
) -> Result<MountInfo, String> {
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    let mount = manager
        .get_mount(&id)
        .await
        .map_err(|e| format!("Failed to get mount: {}", e))?
        .ok_or("Mount not found")?;

    Ok(fuse_mount_to_info(mount))
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
    use jax_daemon::mount_queries::UpdateMountConfig;

    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    let config = UpdateMountConfig {
        mount_point: request.mount_point,
        enabled: request.enabled,
        auto_mount: request.auto_mount,
        read_only: request.read_only,
        cache_size_mb: request.cache_size_mb,
        cache_ttl_secs: request.cache_ttl_secs,
    };

    let mount = manager
        .update_mount(&id, config)
        .await
        .map_err(|e| format!("Failed to update mount: {}", e))?
        .ok_or("Mount not found")?;

    Ok(fuse_mount_to_info(mount))
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
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    manager
        .delete_mount(&id)
        .await
        .map_err(|e| format!("Failed to delete mount: {}", e))
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
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    manager
        .start_mount(&id)
        .await
        .map_err(|e| format!("Failed to start mount: {}", e))?;

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
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let id = Uuid::parse_str(&mount_id).map_err(|e| format!("Invalid mount ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    manager
        .stop_mount(&id)
        .await
        .map_err(|e| format!("Failed to stop mount: {}", e))?;

    Ok(true)
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn stop_mount(_state: State<'_, AppState>, _mount_id: String) -> Result<bool, String> {
    Err("FUSE support not enabled".to_string())
}

/// Check if FUSE support is available on the host system
#[tauri::command]
pub async fn is_fuse_available() -> Result<bool, String> {
    #[cfg(not(feature = "fuse"))]
    {
        return Ok(false);
    }

    #[cfg(feature = "fuse")]
    {
        Ok(check_fuse_installed())
    }
}

/// Check if FUSE is installed on the host system
#[cfg(feature = "fuse")]
fn check_fuse_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Check for macFUSE
        std::path::Path::new("/Library/Filesystems/macfuse.fs").exists()
            || std::path::Path::new("/Library/Filesystems/osxfuse.fs").exists()
    }

    #[cfg(target_os = "linux")]
    {
        // Check for /dev/fuse
        std::path::Path::new("/dev/fuse").exists()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

#[cfg(feature = "fuse")]
fn fuse_mount_to_info(m: jax_daemon::mount_queries::FuseMount) -> MountInfo {
    MountInfo {
        mount_id: m.mount_id.to_string(),
        bucket_id: m.bucket_id.to_string(),
        mount_point: m.mount_point,
        enabled: m.enabled,
        auto_mount: m.auto_mount,
        read_only: m.read_only,
        cache_size_mb: m.cache_size_mb,
        cache_ttl_secs: m.cache_ttl_secs,
        status: m.status.as_str().to_string(),
        error_message: m.error_message,
        created_at: m.created_at.to_string(),
        updated_at: m.updated_at.to_string(),
    }
}

/// Mount a bucket with automatic mount point selection (desktop app simplified API)
///
/// This command:
/// 1. Gets the bucket name
/// 2. Generates a mount point in /Volumes (macOS) or /media/$USER (Linux)
/// 3. Handles naming conflicts automatically
/// 4. Requests elevated permissions if needed
/// 5. Creates the mount and starts it
#[tauri::command]
#[cfg(feature = "fuse")]
pub async fn mount_bucket(
    state: State<'_, AppState>,
    bucket_id: String,
) -> Result<MountInfo, String> {
    use jax_daemon::mount_queries::CreateMountConfig;

    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let bucket_uuid =
        Uuid::parse_str(&bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    // Get bucket info to get the name
    let bucket_name = daemon
        .service
        .database()
        .get_bucket_info(&bucket_uuid)
        .await
        .map_err(|e| format!("Failed to get bucket info: {}", e))?
        .map(|info| info.name)
        .unwrap_or_else(|| "bucket".to_string());

    // Get existing mounts to check for conflicts
    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    let existing_mounts: Vec<String> = manager
        .list_mounts()
        .await
        .map_err(|e| format!("Failed to list mounts: {}", e))?
        .into_iter()
        .map(|m| m.mount_point)
        .collect();

    // Generate unique mount point
    let mount_point = generate_mount_point(&bucket_name, &existing_mounts);
    let mount_point_str = mount_point.to_string_lossy().to_string();

    // Create mount point directory (with privilege escalation if needed)
    create_mount_point(&mount_point).await?;

    // Create mount config
    let config = CreateMountConfig {
        bucket_id: bucket_uuid,
        mount_point: mount_point_str,
        auto_mount: false,
        read_only: false,
        cache_size_mb: None,
        cache_ttl_secs: None,
    };

    let mount = manager
        .create_mount(config)
        .await
        .map_err(|e| format!("Failed to create mount: {}", e))?;

    let mount_id = mount.mount_id;

    // Start the mount
    manager
        .start_mount(&mount_id)
        .await
        .map_err(|e| format!("Failed to start mount: {}", e))?;

    // Get updated mount info
    let mount = manager
        .get_mount(&mount_id)
        .await
        .map_err(|e| format!("Failed to get mount: {}", e))?
        .ok_or("Mount not found after creation")?;

    Ok(fuse_mount_to_info(mount))
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
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let bucket_uuid =
        Uuid::parse_str(&bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    // Find mount for this bucket
    let mounts = manager
        .list_mounts()
        .await
        .map_err(|e| format!("Failed to list mounts: {}", e))?;

    let mount = mounts
        .into_iter()
        .find(|m| m.bucket_id == bucket_uuid)
        .ok_or("No mount found for this bucket")?;

    // Stop the mount
    manager
        .stop_mount(&mount.mount_id)
        .await
        .map_err(|e| format!("Failed to stop mount: {}", e))?;

    // Optionally delete the mount config
    manager
        .delete_mount(&mount.mount_id)
        .await
        .map_err(|e| format!("Failed to delete mount: {}", e))?;

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
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;

    let bucket_uuid =
        Uuid::parse_str(&bucket_id).map_err(|e| format!("Invalid bucket ID: {}", e))?;

    let mount_manager = daemon.service.mount_manager().read().await;
    let manager = mount_manager
        .as_ref()
        .ok_or("Mount manager not available")?;

    let mounts = manager
        .list_mounts()
        .await
        .map_err(|e| format!("Failed to list mounts: {}", e))?;

    let mount = mounts
        .into_iter()
        .find(|m| m.bucket_id == bucket_uuid && m.status.as_str() == "running");

    Ok(mount.map(fuse_mount_to_info))
}

#[tauri::command]
#[cfg(not(feature = "fuse"))]
pub async fn is_bucket_mounted(
    _state: State<'_, AppState>,
    _bucket_id: String,
) -> Result<Option<MountInfo>, String> {
    Ok(None)
}
