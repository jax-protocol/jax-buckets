use askama::Template;
use axum::response::{IntoResponse, Response};

use common::mount::NodeLink;

use super::GatewayQuery;

/// Template for file viewer
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

#[allow(clippy::too_many_arguments)]
pub async fn handle(
    mount: &common::mount::Mount,
    path_buf: &std::path::Path,
    absolute_path: &str,
    query: &GatewayQuery,
    bucket_id_str: &str,
    bucket_id_short: &str,
    bucket_name: &str,
    bucket_link: &str,
    bucket_link_short: &str,
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
            return error_response("Failed to read file");
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
            let html = markdown_to_html(&content_str);
            (html.into_bytes(), "text/html; charset=utf-8")
        } else {
            (file_data, "text/html; charset=utf-8")
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

    let back_url = format!("/gw/{}{}", bucket_id_str, get_parent_path(absolute_path));

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

/// Format a byte count into a human-readable string (e.g., "1.50 KB").
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

/// Converts markdown content to HTML.
pub fn markdown_to_html(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; max-width: 800px; margin: 40px auto; padding: 0 20px; line-height: 1.6; }}
        img {{ max-width: 100%; height: auto; }}
        code {{ background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }}
        pre {{ background: #f4f4f4; padding: 12px; border-radius: 5px; overflow-x: auto; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f4f4f4; }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
        html_output
    )
}

/// Get parent path for "Up" navigation.
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

/// Convert bytes to hex dump for binary files.
fn to_hex_dump(data: &[u8], max_bytes: usize) -> String {
    let bytes_to_show = data.len().min(max_bytes);
    let mut result = String::new();

    for (i, chunk) in data[..bytes_to_show].chunks(16).enumerate() {
        // Address
        result.push_str(&format!("{:08x}  ", i * 16));

        // Hex values
        for (j, byte) in chunk.iter().enumerate() {
            result.push_str(&format!("{:02x} ", byte));
            if j == 7 {
                result.push(' ');
            }
        }

        // Padding for incomplete lines
        for j in chunk.len()..16 {
            result.push_str("   ");
            if j == 7 {
                result.push(' ');
            }
        }

        result.push(' ');

        // ASCII representation
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

fn error_response(message: &str) -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Error: {}", message),
    )
        .into_response()
}
