//! FUSE mount management API endpoints
//!
//! Provides REST endpoints for managing FUSE mounts:
//! - Create, list, get, update, delete mount configurations
//! - Start/stop mount operations

use axum::routing::{get, post};
use axum::Router;

use crate::ServiceState;

mod create;
mod delete_mount;
mod get;
mod list;
mod start;
mod stop;
mod update;

// Re-export request/response types for use by CLI and other clients
pub use create::{CreateMountRequest, CreateMountResponse, MountInfo};
pub use delete_mount::{DeleteMountRequest, DeleteMountResponse};
pub use get::{GetMountRequest, GetMountResponse};
pub use list::{ListMountsRequest, ListMountsResponse};
pub use start::{StartMountRequest, StartMountResponse};
pub use stop::{StopMountRequest, StopMountResponse};
pub use update::{UpdateMountBody, UpdateMountRequest, UpdateMountResponse};

pub fn router(state: ServiceState) -> Router<ServiceState> {
    Router::new()
        .route("/", post(create::handler).get(list::handler))
        .route(
            "/:id",
            get(get::handler)
                .patch(update::handler)
                .delete(delete_mount::handler),
        )
        .route("/:id/start", post(start::handler))
        .route("/:id/stop", post(stop::handler))
        .with_state(state)
}
