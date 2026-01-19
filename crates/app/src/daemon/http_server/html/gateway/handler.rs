use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use common::mount::NodeLink;

use crate::ServiceState;

use super::types::{
    DirectoryEntry, DirectoryListing, FileDisplayInfo, GatewayExplorerTemplate, GatewayQuery,
    GatewayViewerTemplate,
};
use super::utils::{
    build_path_segments, find_index_file, format_size, get_parent_path, markdown_to_html,
    rewrite_relative_urls, to_hex_dump, wants_json,
};

/// Handler for bucket root requests (no file path)
pub async fn root_handler(
    state: State<ServiceState>,
    Path(bucket_id): Path<Uuid>,
    query: Query<GatewayQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Delegate to main handler with "/" as the path
    handler(state, Path((bucket_id, "/".to_string())), query, headers).await
}

pub async fn handler(
    State(state): State<ServiceState>,
    Path((bucket_id, file_path)): Path<(Uuid, String)>,
    Query(query): Query<GatewayQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Extract host from request headers, fallback to localhost
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|h| {
            // Check if the host includes a scheme
            if h.starts_with("http://") || h.starts_with("https://") {
                h.to_string()
            } else {
                // Assume https for production, http for localhost
                if h.contains("localhost") || h.starts_with("127.0.0.1") {
                    format!("http://{}", h)
                } else {
                    format!("https://{}", h)
                }
            }
        })
        .unwrap_or_else(|| "http://localhost".to_string());

    // Ensure path is absolute
    let absolute_path = if file_path.starts_with('/') {
        file_path
    } else {
        format!("/{}", file_path)
    };

    // Load mount - either from specific link or current state
    let mount = if let Some(hash_str) = &query.at {
        // Parse the hash string and create a Link
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
        match state.peer().mount(bucket_id).await {
            Ok(mount) => mount,
            Err(common::prelude::MountError::LinkNotFound(_)) => {
                return syncing_response();
            }
            Err(e) => {
                tracing::error!("Failed to load mount: {}", e);
                return error_response("Failed to load bucket");
            }
        }
    };

    let path_buf = std::path::PathBuf::from(&absolute_path);

    // Handle root path specially - it's always a directory
    let is_root = absolute_path == "/";

    // Try to get the node to determine if it's a file or directory
    // Root path doesn't need mount.get() - it's implicitly a directory
    let node_link = if is_root {
        None // Will be treated as directory below
    } else {
        match mount.get(&path_buf).await {
            Ok(node) => Some(node),
            Err(e) => {
                tracing::error!("Failed to get path {}: {}", absolute_path, e);
                return not_found_response(&format!("Path not found: {}", absolute_path));
            }
        }
    };

    // Check if it's a directory (None means root, which is always a directory)
    let is_directory = match &node_link {
        None => true, // Root is always a directory
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
        handle_directory(
            &mount,
            &path_buf,
            &absolute_path,
            &query,
            &headers,
            &host,
            &bucket_id,
            &bucket_id_str,
            &bucket_id_short,
            &bucket_name,
            &bucket_link,
            &bucket_link_short,
        )
        .await
    } else {
        handle_file(
            &mount,
            &path_buf,
            &absolute_path,
            &query,
            &headers,
            &host,
            &bucket_id,
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

#[allow(clippy::too_many_arguments)]
async fn handle_directory(
    mount: &common::mount::Mount,
    path_buf: &std::path::Path,
    absolute_path: &str,
    query: &GatewayQuery,
    headers: &axum::http::HeaderMap,
    host: &str,
    bucket_id: &Uuid,
    bucket_id_str: &str,
    bucket_id_short: &str,
    bucket_name: &str,
    bucket_link: &str,
    bucket_link_short: &str,
) -> Response {
    // Check for index file first (unless JSON is explicitly requested)
    if !wants_json(headers) {
        if let Some((index_path, index_mime_type)) = find_index_file(mount, path_buf).await {
            // Serve the index file instead of directory listing
            let file_data = match mount.cat(&index_path).await {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Failed to read index file: {}", e);
                    return error_response("Failed to read index file");
                }
            };

            // Convert the index_path to string for URL rewriting
            let index_path_str = index_path.to_str().unwrap_or(absolute_path);

            // Handle different mime types
            let (final_content, final_mime_type) = if index_mime_type == "text/markdown" {
                // Convert markdown to HTML
                let content_str = String::from_utf8_lossy(&file_data);
                let html = markdown_to_html(&content_str);
                // Apply URL rewriting to the generated HTML
                let rewritten = rewrite_relative_urls(&html, index_path_str, bucket_id, host);
                (rewritten.into_bytes(), "text/html; charset=utf-8")
            } else if index_mime_type == "text/html" {
                // Apply URL rewriting to HTML
                let content_str = String::from_utf8_lossy(&file_data);
                let rewritten =
                    rewrite_relative_urls(&content_str, index_path_str, bucket_id, host);
                (rewritten.into_bytes(), "text/html; charset=utf-8")
            } else {
                // Serve text/plain as-is
                (file_data, "text/plain; charset=utf-8")
            };

            return (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, final_mime_type)],
                final_content,
            )
                .into_response();
        }
    }

    // List directory contents (deep or shallow based on query param)
    let wants_deep = query.deep.unwrap_or(false);
    let items_map = if wants_deep {
        match mount.ls_deep(path_buf).await {
            Ok(items) => items,
            Err(e) => {
                tracing::error!("Failed to deep list directory: {}", e);
                return error_response("Failed to list directory");
            }
        }
    } else {
        match mount.ls(path_buf).await {
            Ok(items) => items,
            Err(e) => {
                tracing::error!("Failed to list directory: {}", e);
                return error_response("Failed to list directory");
            }
        }
    };

    // Check if JSON is requested
    if wants_json(headers) {
        let entries: Vec<DirectoryEntry> = items_map
            .into_iter()
            .map(|(path, node_link)| {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let mime_type = match &node_link {
                    NodeLink::Dir(_, _) => "inode/directory".to_string(),
                    NodeLink::Data(_, _, data) => data
                        .mime()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                };

                DirectoryEntry {
                    name,
                    path: format!("/{}", path.display()),
                    mime_type,
                }
            })
            .collect();

        let listing = DirectoryListing {
            path: absolute_path.to_string(),
            entries,
        };

        return (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            serde_json::to_string_pretty(&listing).unwrap(),
        )
            .into_response();
    }

    // Render HTML explorer
    let items: Vec<FileDisplayInfo> = items_map
        .into_iter()
        .map(|(path, node_link)| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let (mime_type, is_dir) = match &node_link {
                NodeLink::Dir(_, _) => ("inode/directory".to_string(), true),
                NodeLink::Data(_, _, data) => (
                    data.mime()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                    false,
                ),
            };

            FileDisplayInfo {
                name,
                path: format!("/{}", path.display()),
                mime_type,
                is_dir,
            }
        })
        .collect();

    let template = GatewayExplorerTemplate {
        bucket_id: bucket_id_str.to_string(),
        bucket_id_short: bucket_id_short.to_string(),
        bucket_name: bucket_name.to_string(),
        bucket_link: bucket_link.to_string(),
        bucket_link_short: bucket_link_short.to_string(),
        path_segments: build_path_segments(absolute_path),
        items,
    };

    match template.render() {
        Ok(html) => (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to render explorer template: {}", e);
            error_response("Failed to render page")
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_file(
    mount: &common::mount::Mount,
    path_buf: &std::path::Path,
    absolute_path: &str,
    query: &GatewayQuery,
    headers: &axum::http::HeaderMap,
    host: &str,
    bucket_id: &Uuid,
    bucket_id_str: &str,
    bucket_id_short: &str,
    bucket_name: &str,
    bucket_link: &str,
    bucket_link_short: &str,
    node_link: NodeLink,
) -> Response {
    // Handle file - extract metadata from the node_link
    let file_metadata_data = match &node_link {
        NodeLink::Data(_, _, metadata) => metadata.clone(),
        _ => unreachable!("Already checked is_directory"),
    };

    let mime_type = file_metadata_data
        .mime()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    // Get filename
    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    // Check if raw download is requested
    let wants_download = query.download.unwrap_or(false);
    let wants_view = query.view.unwrap_or(false);

    // Read file data
    let file_data = match mount.cat(path_buf).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to read file: {}", e);
            return error_response("Failed to read file");
        }
    };

    // Calculate size from actual data
    let file_size = file_data.len() as u64;
    let size_formatted = format_size(file_size);

    // If download is requested, serve raw file
    if wants_download {
        return (
            axum::http::StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, mime_type.as_str()),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"{}\"", filename),
                ),
            ],
            file_data,
        )
            .into_response();
    }

    // If JSON is requested, return file metadata as JSON
    if wants_json(headers) {
        let metadata = serde_json::json!({
            "path": absolute_path,
            "name": filename,
            "mime_type": mime_type,
            "size": file_size,
            "size_formatted": size_formatted,
        });

        return (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
            .into_response();
    }

    // For HTML and Markdown files, render directly (unless ?view=true)
    let is_html = mime_type == "text/html";
    let is_markdown = mime_type == "text/markdown";

    if (is_html || is_markdown) && !wants_view {
        // Render the file directly
        let (final_content, final_mime_type) = if is_markdown {
            let content_str = String::from_utf8_lossy(&file_data);
            let html = markdown_to_html(&content_str);
            let rewritten = rewrite_relative_urls(&html, absolute_path, bucket_id, host);
            (rewritten.into_bytes(), "text/html; charset=utf-8")
        } else {
            let content_str = String::from_utf8_lossy(&file_data);
            let rewritten = rewrite_relative_urls(&content_str, absolute_path, bucket_id, host);
            (rewritten.into_bytes(), "text/html; charset=utf-8")
        };

        return (
            axum::http::StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, final_mime_type),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    &format!("inline; filename=\"{}\"", filename),
                ),
            ],
            final_content,
        )
            .into_response();
    }

    // Render file viewer UI
    let content = if mime_type.starts_with("text/")
        || mime_type == "application/json"
        || mime_type == "application/xml"
        || mime_type == "application/javascript"
    {
        // Text content - show as text
        String::from_utf8_lossy(&file_data).to_string()
    } else {
        // Binary content - show hex dump
        to_hex_dump(&file_data, 1024)
    };

    let back_url = format!("/gw/{}{}", bucket_id, get_parent_path(absolute_path));

    let template = GatewayViewerTemplate {
        bucket_id: bucket_id_str.to_string(),
        bucket_id_short: bucket_id_short.to_string(),
        bucket_name: bucket_name.to_string(),
        bucket_link: bucket_link.to_string(),
        bucket_link_short: bucket_link_short.to_string(),
        file_path: absolute_path.to_string(),
        file_name: filename,
        mime_type,
        size_formatted,
        content,
        back_url,
    };

    match template.render() {
        Ok(html) => (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to render viewer template: {}", e);
            error_response("Failed to render page")
        }
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
