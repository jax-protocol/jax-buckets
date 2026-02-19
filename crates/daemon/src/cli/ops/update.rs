use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};

use clap::Args;

const GITHUB_REPO: &str = "jax-protocol/jax-fs";
const INSTALL_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/jax-protocol/jax-fs/main/install.sh";

/// Update jax to the latest release.
#[derive(Args, Debug, Clone)]
pub struct Update {
    /// Force update even if already on latest version
    #[arg(long)]
    force: bool,

    /// Install FUSE variant (macOS Apple Silicon only)
    #[arg(long)]
    fuse: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InstallMethod {
    /// Installed via install script to ~/.local/bin
    Script,
    /// Installed via cargo install to ~/.cargo/bin
    Cargo,
    /// Running from a source/target build directory
    Source,
    /// Unknown installation method
    Unknown,
}

impl InstallMethod {
    fn description(&self) -> &str {
        match self {
            InstallMethod::Script => "install script (~/.local/bin)",
            InstallMethod::Cargo => "cargo install (~/.cargo/bin)",
            InstallMethod::Source => "source build (target/)",
            InstallMethod::Unknown => "unknown",
        }
    }
}

fn detect_installation() -> InstallMethod {
    let Ok(exe_path) = std::env::current_exe() else {
        return InstallMethod::Unknown;
    };
    let path_str = exe_path.to_string_lossy();

    if path_str.contains("/.local/bin/") {
        InstallMethod::Script
    } else if path_str.contains("/.cargo/bin/") {
        InstallMethod::Cargo
    } else if path_str.contains("/target/") {
        InstallMethod::Source
    } else {
        InstallMethod::Unknown
    }
}

fn has_fuse_feature() -> bool {
    cfg!(feature = "fuse")
}

fn prompt_confirm(message: &str, default_yes: bool) -> bool {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    eprint!("{} {} ", message, suffix);
    let _ = io::stderr().flush();

    let stdin = io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return default_yes;
    }

    let answer = line.trim().to_lowercase();
    if answer.is_empty() {
        default_yes
    } else {
        answer == "y" || answer == "yes"
    }
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();
        (
            *parts.first().unwrap_or(&0),
            *parts.get(1).unwrap_or(&0),
            *parts.get(2).unwrap_or(&0),
        )
    };
    parse(latest) > parse(current)
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("{0}")]
    Failed(String),
}

#[async_trait::async_trait]
impl crate::cli::op::Op for Update {
    type Error = UpdateError;
    type Output = String;

    async fn execute(&self, _ctx: &crate::cli::op::OpContext) -> Result<Self::Output, Self::Error> {
        let install_method = detect_installation();
        let current_version = env!("CARGO_PKG_VERSION");
        let fuse_enabled = has_fuse_feature();

        eprintln!("Current version: {}", current_version);
        eprintln!("Installation: {}", install_method.description());
        eprintln!(
            "FUSE support: {}",
            if fuse_enabled { "enabled" } else { "disabled" }
        );
        eprintln!();

        // Fetch latest version
        eprintln!("Checking for updates...");
        let latest_version = fetch_latest_version()
            .await
            .map_err(|e| UpdateError::Failed(format!("Failed to check for updates: {}", e)))?;
        eprintln!("Latest version: {}", latest_version);
        eprintln!();

        let needs_update = is_newer_version(current_version, &latest_version);

        if !needs_update && !self.force {
            return Ok("Already up to date.".to_string());
        }

        if needs_update {
            eprintln!(
                "New version available: {} -> {}",
                current_version, latest_version
            );
        } else {
            eprintln!("Forcing update...");
        }

        match install_method {
            InstallMethod::Script => {
                run_install_script(self.fuse || fuse_enabled)
                    .map_err(|e| UpdateError::Failed(e.to_string()))?;
            }
            InstallMethod::Cargo | InstallMethod::Source => {
                eprintln!();
                eprintln!("You are running a local build.");

                if prompt_confirm(
                    "Switch to release binary and install to ~/.local/bin?",
                    true,
                ) {
                    run_install_script(self.fuse || fuse_enabled)
                        .map_err(|e| UpdateError::Failed(e.to_string()))?;
                } else {
                    eprintln!();
                    eprintln!("To update manually:");
                    eprintln!(
                        "  cargo install --git https://github.com/{} jax-daemon",
                        GITHUB_REPO
                    );
                    return Ok("Update cancelled.".to_string());
                }
            }
            InstallMethod::Unknown => {
                eprintln!();
                eprintln!("Could not detect installation method.");
                eprintln!();
                eprintln!("To install via script (recommended):");
                eprintln!("  curl -fsSL {} | sh", INSTALL_SCRIPT_URL);
                return Ok("Update cancelled — unknown installation method.".to_string());
            }
        }

        Ok("Updated successfully.".to_string())
    }
}

async fn fetch_latest_version() -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{}/releases", GITHUB_REPO);

    let output = Command::new("curl")
        .args(["-fsSL", &url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to fetch releases: {}", stderr));
    }

    let body = String::from_utf8_lossy(&output.stdout);

    // Find first jax-daemon-v* tag
    for line in body.lines() {
        let line = line.trim();
        if line.contains("\"tag_name\"") && line.contains("jax-daemon-v") {
            if let Some(start) = line.find("jax-daemon-v") {
                let rest = &line[start + "jax-daemon-v".len()..];
                if let Some(end) = rest.find('"') {
                    return Ok(rest[..end].to_string());
                }
            }
        }
    }

    Err("Could not find jax-daemon release version".to_string())
}

fn run_install_script(fuse: bool) -> Result<(), String> {
    eprintln!();
    eprintln!("Running install script...");
    eprintln!();

    let fuse_flag = if fuse { " --fuse" } else { "" };
    let cmd = format!("curl -fsSL {} | sh -s --{}", INSTALL_SCRIPT_URL, fuse_flag);

    let status = Command::new("sh")
        .args(["-c", &cmd])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to run install script: {}", e))?;

    if !status.success() {
        return Err("Install script failed".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.0", "0.1.1"));
        assert!(is_newer_version("0.1.9", "0.2.0"));
        assert!(is_newer_version("0.1.9", "1.0.0"));
        assert!(!is_newer_version("0.1.9", "0.1.9"));
        assert!(!is_newer_version("0.2.0", "0.1.9"));
    }

    #[test]
    fn test_has_fuse_feature() {
        // Just verify it compiles and returns a bool
        let _ = has_fuse_feature();
    }

    #[test]
    fn test_detect_installation() {
        // Should return some variant without panicking
        let _ = detect_installation();
    }
}
