mod directory;
mod file;
mod index;

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use uuid::Uuid;

use common::mount::NodeLink;

use crate::ServiceState;

pub use index::handler as index_handler;

/// Shared query parameters for gateway routes.
#[derive(Debug, Deserialize)]
pub struct GatewayQuery {
    /// Load a specific version by hash
    #[serde(default)]
    pub at: Option<String>,
    /// If true, serve the raw file with Content-Disposition: attachment
    #[serde(default)]
    pub download: Option<bool>,
    /// If true, recursively list all files under the path (deep listing)
    #[serde(default)]
    pub deep: Option<bool>,
    /// If true, return JSON instead of HTML
    #[serde(default)]
    pub json: Option<bool>,
}

pub fn router(state: ServiceState) -> Router<ServiceState> {
    Router::new()
        .route("/:bucket_id", get(root_handler))
        .route("/:bucket_id/", get(root_handler))
        .route("/:bucket_id/*file_path", get(handler))
        .with_state(state)
}

/// Handler for bucket root requests (no file path).
async fn root_handler(
    state: State<ServiceState>,
    Path(bucket_id): Path<Uuid>,
    query: Query<GatewayQuery>,
) -> Response {
    handler(state, Path((bucket_id, "/".to_string())), query).await
}

async fn handler(
    State(state): State<ServiceState>,
    Path((bucket_id, file_path)): Path<(Uuid, String)>,
    Query(query): Query<GatewayQuery>,
) -> Response {
    // Ensure path is absolute
    let absolute_path = if file_path.starts_with('/') {
        file_path
    } else {
        format!("/{}", file_path)
    };

    // Load mount - either from specific link or latest published version
    // Gateways always show the last published version, never HEAD
    let mount = if let Some(hash_str) = &query.at {
        match hash_str.parse::<common::linked_data::Hash>() {
            Ok(hash) => {
                let link = common::linked_data::Link::new(common::linked_data::LD_RAW_CODEC, hash);
                match common::mount::Mount::load(&link, state.peer().secret(), state.peer().blobs())
                    .await
                {
                    Ok(mount) => mount,
                    Err(e) => {
                        tracing::error!("Failed to load mount from link: {}", e);
                        return error_response("Failed to load historical version");
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to parse hash: {}", e);
                return error_response("Invalid hash format");
            }
        }
    } else {
        use common::bucket_log::BucketLogProvider;
        match state.peer().logs().latest_published(bucket_id).await {
            Ok(Some((published_link, _height))) => {
                match common::mount::Mount::load(
                    &published_link,
                    state.peer().secret(),
                    state.peer().blobs(),
                )
                .await
                {
                    Ok(mount) => mount,
                    Err(_e) => {
                        return syncing_response();
                    }
                }
            }
            _ => {
                return syncing_response();
            }
        }
    };

    let path_buf = std::path::PathBuf::from(&absolute_path);
    let is_root = absolute_path == "/";

    let node_link = if is_root {
        None
    } else {
        match mount.get(&path_buf).await {
            Ok(node) => Some(node),
            Err(e) => {
                tracing::error!("Failed to get path {}: {}", absolute_path, e);
                return not_found_response(&format!("Path not found: {}", absolute_path));
            }
        }
    };

    let is_directory = match &node_link {
        None => true,
        Some(NodeLink::Dir(_, _)) => true,
        Some(NodeLink::Data(_, _, _)) => false,
    };

    // Get bucket metadata from mount
    let inner = mount.inner().await;
    let bucket_name = inner.manifest().name().to_string();
    let bucket_id_str = bucket_id.to_string();
    let bucket_id_short = format!(
        "{}...{}",
        &bucket_id_str[..8],
        &bucket_id_str[bucket_id_str.len() - 4..]
    );
    let bucket_link = inner.link().hash().to_string();
    let bucket_link_short = format!(
        "{}...{}",
        &bucket_link[..8],
        &bucket_link[bucket_link.len() - 8..]
    );

    if is_directory {
        directory::handle(
            &mount,
            &path_buf,
            &absolute_path,
            &query,
            &bucket_id_str,
            &bucket_id_short,
            &bucket_name,
            &bucket_link,
            &bucket_link_short,
        )
        .await
    } else {
        file::handle(
            &mount,
            &path_buf,
            &absolute_path,
            &query,
            &bucket_id_str,
            &bucket_id_short,
            &bucket_name,
            &bucket_link,
            &bucket_link_short,
            node_link.unwrap(),
        )
        .await
    }
}

fn error_response(message: &str) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Error: {}", message),
    )
        .into_response()
}

fn syncing_response() -> Response {
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        [(axum::http::header::RETRY_AFTER, "5")],
        "Bucket is still syncing. Please try again in a moment.",
    )
        .into_response()
}

fn not_found_response(message: &str) -> Response {
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("Not found: {}", message),
    )
        .into_response()
}
