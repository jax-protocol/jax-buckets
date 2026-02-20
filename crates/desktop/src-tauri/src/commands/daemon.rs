//! Daemon status IPC commands
//!
//! These commands use the daemon's HTTP API or local filesystem reads,
//! enabling both embedded and sidecar daemon modes.

use serde::{Deserialize, Serialize};
use tauri::State;

use jax_daemon::http_server::health::identity::IdentityRequest;

use crate::AppState;

/// Daemon status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub api_port: u16,
    pub gateway_port: u16,
    pub node_id: Option<String>,
    pub mode: String,
}

/// Get daemon status
#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<DaemonStatus, String> {
    let inner = state.inner.read().await;

    match inner.as_ref() {
        Some(daemon) => {
            let node_id = {
                let mut client = daemon.client.clone();
                client
                    .call(IdentityRequest {})
                    .await
                    .ok()
                    .map(|r| r.node_id)
            };

            Ok(DaemonStatus {
                running: node_id.is_some(),
                api_port: daemon.api_port,
                gateway_port: daemon.gateway_port,
                node_id,
                mode: daemon.mode.to_string(),
            })
        }
        None => Ok(DaemonStatus {
            running: false,
            api_port: 0,
            gateway_port: 0,
            node_id: None,
            mode: "disconnected".to_string(),
        }),
    }
}

/// Get node identity (public key)
#[tauri::command]
pub async fn get_identity(state: State<'_, AppState>) -> Result<String, String> {
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not connected")?;
    let mut client = daemon.client.clone();
    let resp = client
        .call(IdentityRequest {})
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.node_id)
}

/// Configuration info for the Settings page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub jax_dir: String,
    pub db_path: String,
    pub config_path: String,
    pub blob_store: String,
}

/// Get configuration info (reads from local filesystem - no HTTP needed)
#[tauri::command]
pub async fn get_config_info(state: State<'_, AppState>) -> Result<ConfigInfo, String> {
    let inner = state.inner.read().await;
    let daemon = inner.as_ref().ok_or("Daemon not connected")?;
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
