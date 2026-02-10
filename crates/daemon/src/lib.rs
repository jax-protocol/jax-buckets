// Re-export daemon functionality for Tauri and other consumers
pub mod daemon;
pub mod state;

pub use daemon::process::spawn_service;
pub use daemon::{start_service, ServiceConfig, ServiceState, ShutdownHandle};
pub use state::{AppConfig, AppState, BlobStoreConfig, StateError};
