use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::Extension;
use tracing::instrument;
use uuid::Uuid;

use common::linked_data::BlockEncoded;
use common::prelude::Manifest;

use crate::daemon::http_server::Config;
use crate::ServiceState;

use super::history::ManifestShare;

#[derive(Template)]
#[template(path = "pages/buckets/tree.html")]
pub struct BucketTreeTemplate {
    pub bucket_id: String,
    pub bucket_id_short: String,
    pub bucket_name: String,
    pub bucket_link: String,
    pub bucket_link_short: String,
    pub bucket_data_formatted: String,
    pub manifest_height: u64,
    pub manifest_version: String,
    pub manifest_entry_link: String,
    pub manifest_pins_link: String,
    pub manifest_previous_link: Option<String>,
    pub manifest_shares: Vec<ManifestShare>,
    pub api_url: String,
    pub read_only: bool,
    pub current_path: String,
    pub file_metadata: Option<super::file_explorer::FileMetadata>,
    pub path_segments: Vec<super::file_explorer::PathSegment>,
}

fn shorten_link(link: &str) -> String {
    if link.len() > 16 {
        format!("{}...{}", &link[..8], &link[link.len() - 8..])
    } else {
        link.to_string()
    }
}

#[instrument(skip(state, config))]
pub async fn handler(
    State(state): State<ServiceState>,
    Extension(config): Extension<Config>,
    Path(bucket_id): Path<Uuid>,
) -> askama_axum::Response {
    // Get bucket info
    let bucket = match state.database().get_bucket_info(&bucket_id).await {
        Ok(Some(bucket)) => bucket,
        Ok(None) => return error_response("Bucket not found"),
        Err(e) => return error_response(&format!("Database error: {}", e)),
    };

    // Format bucket link for display
    let bucket_link = bucket.link.hash().to_string();
    let bucket_link_short = shorten_link(&bucket_link);

    // Format bucket ID for display
    let bucket_id_str = bucket_id.to_string();
    let bucket_id_short = shorten_link(&bucket_id_str);

    // Load the full bucket data from blobs
    let blobs = state.node().blobs();
    let (
        bucket_data_formatted,
        manifest_height,
        manifest_version,
        manifest_entry_link,
        manifest_pins_link,
        manifest_previous_link,
        manifest_shares,
    ) = match blobs.get(&bucket.link.hash()).await {
        Ok(data) => match Manifest::decode(&data) {
            Ok(bucket_data) => {
                let formatted = serde_json::to_string_pretty(&bucket_data)
                    .unwrap_or_else(|_| format!("{:#?}", bucket_data));

                let height = bucket_data.height();
                let version = format!("{:?}", bucket_data.version());
                let entry_link = bucket_data.entry().hash().to_string();
                let pins_link = bucket_data.pins().hash().to_string();
                let previous = bucket_data
                    .previous()
                    .as_ref()
                    .map(|l| l.hash().to_string());
                let shares: Vec<ManifestShare> = bucket_data
                    .shares()
                    .iter()
                    .map(|(pub_key, share)| ManifestShare {
                        public_key: pub_key.clone(),
                        role: format!("{:?}", share.principal().role),
                    })
                    .collect();

                (
                    formatted, height, version, entry_link, pins_link, previous, shares,
                )
            }
            Err(e) => (
                format!("Error: {}", e),
                0,
                String::new(),
                String::new(),
                String::new(),
                None,
                Vec::new(),
            ),
        },
        Err(e) => (
            format!("Error: {}", e),
            0,
            String::new(),
            String::new(),
            String::new(),
            None,
            Vec::new(),
        ),
    };

    // Get API URL from config
    let api_url = config
        .api_url
        .clone()
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    let template = BucketTreeTemplate {
        bucket_id: bucket_id.to_string(),
        bucket_id_short,
        bucket_name: bucket.name,
        bucket_link,
        bucket_link_short,
        bucket_data_formatted,
        manifest_height,
        manifest_version,
        manifest_entry_link,
        manifest_pins_link,
        manifest_previous_link,
        manifest_shares,
        api_url,
        read_only: false,
        current_path: "/".to_string(),
        file_metadata: None,
        path_segments: vec![],
    };

    template.into_response()
}

fn error_response(message: &str) -> askama_axum::Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Error: {}", message),
    )
        .into_response()
}
