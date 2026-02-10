//! Daemon status IPC commands

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

/// Daemon status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub api_port: u16,
    pub gateway_port: u16,
    pub node_id: Option<String>,
}

/// Get daemon status
#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<DaemonStatus, String> {
    let inner = state.inner.read().await;

    match inner.as_ref() {
        Some(daemon) => Ok(DaemonStatus {
            running: true,
            api_port: daemon.api_port,
            gateway_port: daemon.gateway_port,
            node_id: Some(daemon.service.peer().id().to_string()),
        }),
        None => Ok(DaemonStatus {
            running: false,
            api_port: 0,
            gateway_port: 0,
            node_id: None,
        }),
    }
}

/// Get node identity (public key)
#[tauri::command]
pub async fn get_identity(state: State<'_, AppState>) -> Result<String, String> {
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;
    Ok(daemon.service.peer().id().to_string())
}

/// Configuration info for the Settings page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub jax_dir: String,
    pub db_path: String,
    pub config_path: String,
    pub blob_store: String,
}

/// Get configuration info
#[tauri::command]
pub async fn get_config_info(state: State<'_, AppState>) -> Result<ConfigInfo, String> {
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not started")?;
    let jax_dir = &daemon.jax_dir;

    Ok(ConfigInfo {
        jax_dir: jax_dir.display().to_string(),
        db_path: jax_dir.join("db.sqlite").display().to_string(),
        config_path: jax_dir.join("config.toml").display().to_string(),
        blob_store: if jax_dir.join("blobs-store").exists() {
            "filesystem"
        } else {
            "legacy"
        }
        .to_string(),
    })
}
