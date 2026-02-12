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
