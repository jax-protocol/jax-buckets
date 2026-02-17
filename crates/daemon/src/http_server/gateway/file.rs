use askama::Template;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use uuid::Uuid;

use common::mount::{Mount, NodeLink};

/// Query parameters for file requests.
#[derive(Debug, Deserialize)]
pub struct FileQuery {
    /// If true, serve the raw file with Content-Disposition: attachment
    #[serde(default)]
    pub download: Option<bool>,
    /// If true, return file metadata as JSON
    #[serde(default)]
    pub json: Option<bool>,
}

/// Template for file viewer.
#[derive(Template)]
#[template(path = "pages/gateway/viewer.html")]
pub struct GatewayViewerTemplate {
    pub bucket_id: String,
    pub bucket_id_short: String,
    pub bucket_name: String,
    pub bucket_link: String,
    pub bucket_link_short: String,
    pub file_path: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_formatted: String,
    pub content: String,
    pub back_url: String,
}

pub async fn handler(
    mount: &Mount,
    path_buf: &std::path::Path,
    absolute_path: &str,
    query: &FileQuery,
    host: &str,
    bucket_id: &Uuid,
    meta: &super::BucketMeta<'_>,
    node_link: NodeLink,
) -> Response {
    let file_metadata_data = match &node_link {
        NodeLink::Data(_, _, metadata) => metadata.clone(),
        _ => unreachable!("Already checked is_directory"),
    };

    let mime_type = file_metadata_data
        .mime()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let wants_download = query.download.unwrap_or(false);
    let wants_json = query.json.unwrap_or(false);

    // Read file data
    let file_data = match mount.cat(path_buf).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to read file: {}", e);
            return super::error_response("Failed to read file");
        }
    };

    let size_formatted = format_bytes(file_data.len());

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

    // If JSON is requested, return file metadata
    if wants_json {
        let metadata = serde_json::json!({
            "path": absolute_path,
            "name": filename,
            "mime_type": mime_type,
            "size": file_data.len(),
            "size_formatted": size_formatted,
        });

        return (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
            .into_response();
    }

    // For HTML and Markdown files, render directly
    let is_html = mime_type == "text/html";
    let is_markdown = mime_type == "text/markdown";

    if is_html || is_markdown {
        let (final_content, final_mime_type) = if is_markdown {
            let content_str = String::from_utf8_lossy(&file_data);
            let html = super::markdown_to_html(&content_str);
            let rewritten = super::rewrite_relative_urls(&html, absolute_path, bucket_id, host);
            (rewritten.into_bytes(), "text/html; charset=utf-8")
        } else {
            let content_str = String::from_utf8_lossy(&file_data);
            let rewritten =
                super::rewrite_relative_urls(&content_str, absolute_path, bucket_id, host);
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
        String::from_utf8_lossy(&file_data).to_string()
    } else {
        to_hex_dump(&file_data, 1024)
    };

    let back_url = format!("/gw/{}{}", bucket_id, get_parent_path(absolute_path));

    let template = GatewayViewerTemplate {
        bucket_id: meta.id_str.to_string(),
        bucket_id_short: meta.id_short.to_string(),
        bucket_name: meta.name.to_string(),
        bucket_link: meta.link.to_string(),
        bucket_link_short: meta.link_short.to_string(),
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
            super::error_response("Failed to render page")
        }
    }
}

fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f64 = bytes as f64;
    let k = 1024_f64;
    let i = (bytes_f64.log(k).floor() as usize).min(UNITS.len() - 1);
    let size = bytes_f64 / k.powi(i as i32);

    format!("{:.2} {}", size, UNITS[i])
}

fn get_parent_path(path: &str) -> String {
    if path == "/" {
        return "/".to_string();
    }

    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) => "/".to_string(),
        Some(pos) => trimmed[..pos].to_string(),
        None => "/".to_string(),
    }
}

fn to_hex_dump(data: &[u8], max_bytes: usize) -> String {
    let bytes_to_show = data.len().min(max_bytes);
    let mut result = String::new();

    for (i, chunk) in data[..bytes_to_show].chunks(16).enumerate() {
        result.push_str(&format!("{:08x}  ", i * 16));

        for (j, byte) in chunk.iter().enumerate() {
            result.push_str(&format!("{:02x} ", byte));
            if j == 7 {
                result.push(' ');
            }
        }

        for j in chunk.len()..16 {
            result.push_str("   ");
            if j == 7 {
                result.push(' ');
            }
        }

        result.push(' ');

        for byte in chunk {
            if *byte >= 32 && *byte < 127 {
                result.push(*byte as char);
            } else {
                result.push('.');
            }
        }

        result.push('\n');
    }

    if data.len() > max_bytes {
        result.push_str(&format!("\n... ({} more bytes)\n", data.len() - max_bytes));
    }

    result
}
