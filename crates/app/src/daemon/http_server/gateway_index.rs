use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use uuid::Uuid;

use crate::ServiceState;

/// Bucket display info for the gateway homepage
#[derive(Debug, Clone)]
pub struct BucketDisplayInfo {
    pub id: Uuid,
    pub id_short: String,
    pub name: String,
    pub is_published: bool,
}

#[derive(Template)]
#[template(path = "pages/gateway/index.html")]
pub struct GatewayIndexTemplate {
    pub node_id: String,
    pub buckets: Vec<BucketDisplayInfo>,
}

/// Root page handler for the gateway.
/// Displays the gateway's public identity and available buckets.
pub async fn handler(State(state): State<ServiceState>) -> askama_axum::Response {
    let node_id = state.peer().id().to_string();

    // List all buckets
    let db_buckets = state
        .database()
        .list_buckets(None, None)
        .await
        .unwrap_or_default();

    // Get published status for each bucket
    let mut buckets = Vec::new();
    for b in db_buckets {
        let is_published = match state.peer().mount(b.id).await {
            Ok(mount) => mount.is_published().await,
            Err(_) => false,
        };

        let id_str = b.id.to_string();
        let id_short = format!("{}...{}", &id_str[..8], &id_str[id_str.len() - 4..]);

        buckets.push(BucketDisplayInfo {
            id: b.id,
            id_short,
            name: b.name,
            is_published,
        });
    }

    let template = GatewayIndexTemplate { node_id, buckets };

    template.into_response()
}
