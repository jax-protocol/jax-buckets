use clap::Args;

use crate::daemon::{spawn_gateway_service, spawn_service, ServiceConfig};
use crate::state::AppState;

#[derive(Args, Debug, Clone)]
pub struct Daemon {
    /// Run only the gateway server (no HTML UI, no API server)
    #[arg(long)]
    pub gateway_only: bool,

    /// API hostname to use for HTML UI (default: http://localhost:<api_port>)
    #[arg(long)]
    pub api_hostname: Option<String>,

    /// Enable gateway server (uses port from config, default 9090)
    #[arg(long)]
    pub gateway: bool,

    /// Override gateway port (implies --gateway)
    #[arg(long)]
    pub gateway_port: Option<u16>,

    /// Gateway URL for share/download links (e.g., https://gateway.example.com)
    #[arg(long)]
    pub gateway_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("state error: {0}")]
    StateError(#[from] crate::state::StateError),

    #[error("daemon failed: {0}")]
    Failed(String),
}

#[async_trait::async_trait]
impl crate::op::Op for Daemon {
    type Error = DaemonError;
    type Output = String;

    async fn execute(&self, ctx: &crate::op::OpContext) -> Result<Self::Output, Self::Error> {
        // Load state from config path (or default ~/.jax)
        let state = AppState::load(ctx.config_path.clone())?;

        // Load the secret key
        let secret_key = state.load_key()?;

        // Build node listen address from peer_port if configured
        let node_listen_addr = state.config.peer_port.map(|port| {
            format!("0.0.0.0:{}", port)
                .parse()
                .expect("Failed to parse peer listen address")
        });

        // Gateway-only mode: run just the gateway server
        if self.gateway_only {
            let gateway_port = self.gateway_port.unwrap_or(state.config.gateway_port);

            let config = ServiceConfig {
                node_listen_addr,
                node_secret: Some(secret_key),
                node_blobs_store_path: Some(state.blobs_path),
                html_listen_addr: Some(
                    format!("0.0.0.0:{}", gateway_port)
                        .parse()
                        .expect("Failed to parse gateway listen address"),
                ),
                api_listen_addr: None,
                sqlite_path: Some(state.db_path),
                log_level: tracing::Level::DEBUG,
                api_hostname: None,
                gateway_port: None,
                gateway_url: None,
            };

            spawn_gateway_service(&config).await;
            return Ok("gateway ended".to_string());
        }

        // Determine gateway port: --gateway-port overrides, --gateway uses config
        let gateway_port = if let Some(port) = self.gateway_port {
            Some(port)
        } else if self.gateway {
            Some(state.config.gateway_port)
        } else {
            None
        };

        // Build daemon config with persistent paths
        let config = ServiceConfig {
            node_listen_addr,
            node_secret: Some(secret_key),
            node_blobs_store_path: Some(state.blobs_path),
            html_listen_addr: state.config.html_listen_addr.parse().ok(),
            api_listen_addr: state.config.api_listen_addr.parse().ok(),
            sqlite_path: Some(state.db_path),
            log_level: tracing::Level::DEBUG,
            api_hostname: self.api_hostname.clone(),
            gateway_port,
            gateway_url: self.gateway_url.clone(),
        };

        spawn_service(&config).await;
        Ok("daemon ended".to_string())
    }
}
