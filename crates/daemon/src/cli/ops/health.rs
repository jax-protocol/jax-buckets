use std::fmt;
use std::path::PathBuf;

use clap::Args;
use owo_colors::OwoColorize;

use jax_daemon::state::AppState;

#[derive(Args, Debug, Clone)]
pub struct Health;

#[derive(Debug)]
pub struct ConfigInfo {
    pub directory: PathBuf,
    pub api_port: u16,
    pub gateway_port: u16,
}

#[derive(Debug)]
pub enum EndpointStatus {
    Ok,
    Unhealthy(String),
    NotReachable,
}

#[derive(Debug)]
pub struct DaemonInfo {
    pub url: String,
    pub livez: EndpointStatus,
    pub readyz: EndpointStatus,
}

#[derive(Debug)]
pub struct HealthOutput {
    pub config: Option<ConfigInfo>,
    pub config_error: Option<String>,
    pub daemon: DaemonInfo,
}

impl fmt::Display for HealthOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", "Config".bold())?;
        match &self.config {
            Some(info) => {
                writeln!(
                    f,
                    "  {} {}",
                    "directory:".dimmed(),
                    info.directory.display()
                )?;
                writeln!(f, "  {} {}", "config.toml:".dimmed(), "OK".green())?;
                writeln!(f, "  {} {}", "db.sqlite:".dimmed(), "OK".green())?;
                writeln!(f, "  {} {}", "key.pem:".dimmed(), "OK".green())?;
                writeln!(f, "  {} {}", "blobs/:".dimmed(), "OK".green())?;
                writeln!(f, "  {} {}", "api_port:".dimmed(), info.api_port)?;
                writeln!(f, "  {} {}", "gateway_port:".dimmed(), info.gateway_port)?;
            }
            None => {
                if let Some(err) = &self.config_error {
                    writeln!(f, "  {} {}", "error:".red(), err)?;
                }
            }
        }

        writeln!(f)?;
        writeln!(f, "{} ({}):", "Daemon".bold(), self.daemon.url)?;

        let status_str = |s: &EndpointStatus| -> String {
            match s {
                EndpointStatus::Ok => "OK".green().to_string(),
                EndpointStatus::Unhealthy(code) => format!("{} ({})", "UNHEALTHY".red(), code),
                EndpointStatus::NotReachable => "NOT REACHABLE".red().to_string(),
            }
        };

        writeln!(
            f,
            "  {} {}",
            "livez:".dimmed(),
            status_str(&self.daemon.livez)
        )?;
        write!(
            f,
            "  {} {}",
            "readyz:".dimmed(),
            status_str(&self.daemon.readyz)
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HealthError {
    #[error("Health check failed: {0}")]
    Failed(String),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for Health {
    type Error = HealthError;
    type Output = HealthOutput;

    async fn execute(&self, ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let (config, config_error) = match AppState::load(ctx.config_path.clone()) {
            Ok(state) => (
                Some(ConfigInfo {
                    directory: state.jax_dir,
                    api_port: state.config.api_port,
                    gateway_port: state.config.gateway_port,
                }),
                None,
            ),
            Err(e) => (None, Some(e.to_string())),
        };

        let base = ctx.client.base_url();
        let client = ctx.client.http_client();

        let livez_url = format!("{}/_status/livez", base.as_str().trim_end_matches('/'));
        let livez = match client.get(&livez_url).send().await {
            Ok(resp) if resp.status().is_success() => EndpointStatus::Ok,
            Ok(resp) => EndpointStatus::Unhealthy(resp.status().to_string()),
            Err(_) => EndpointStatus::NotReachable,
        };

        let readyz_url = format!("{}/_status/readyz", base.as_str().trim_end_matches('/'));
        let readyz = match client.get(&readyz_url).send().await {
            Ok(resp) if resp.status().is_success() => EndpointStatus::Ok,
            Ok(resp) => EndpointStatus::Unhealthy(resp.status().to_string()),
            Err(_) => EndpointStatus::NotReachable,
        };

        Ok(HealthOutput {
            config,
            config_error,
            daemon: DaemonInfo {
                url: base.to_string(),
                livez,
                readyz,
            },
        })
    }
}
