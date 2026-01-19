use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

use super::types::PathSegment;

// Lazy static regex patterns for URL rewriting
static HTML_ATTR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?P<attr>(?:href|src|action|data|srcset))=["'](?P<url>\.{0,2}/[^"']+)["']"#)
        .unwrap()
});

static MARKDOWN_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\]\((?P<url>\.{0,2}/[^)]+)\)"#).unwrap());

/// Rewrites relative URLs in content to absolute gateway URLs
pub fn rewrite_relative_urls(
    content: &str,
    current_path: &str,
    bucket_id: &Uuid,
    host: &str,
) -> String {
    let current_dir = if current_path == "/" {
        "".to_string()
    } else {
        std::path::Path::new(current_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    };

    // Rewrite HTML attributes (href, src, etc.)
    let content = HTML_ATTR_REGEX.replace_all(content, |caps: &regex::Captures| {
        let attr = &caps["attr"];
        let url = &caps["url"];
        let absolute_url = resolve_relative_url(url, &current_dir, bucket_id, host);
        format!(r#"{}="{}""#, attr, absolute_url)
    });

    // Rewrite Markdown links
    let content = MARKDOWN_LINK_REGEX.replace_all(&content, |caps: &regex::Captures| {
        let url = &caps["url"];
        let absolute_url = resolve_relative_url(url, &current_dir, bucket_id, host);
        format!("]({})", absolute_url)
    });

    content.to_string()
}

/// Resolves a relative URL to an absolute gateway URL
fn resolve_relative_url(
    relative_url: &str,
    current_dir: &str,
    bucket_id: &Uuid,
    host: &str,
) -> String {
    let path = if let Some(stripped) = relative_url.strip_prefix("./") {
        // Current directory reference
        format!("{}/{}", current_dir, stripped)
    } else if let Some(stripped) = relative_url.strip_prefix("../") {
        // Parent directory reference
        let parent = std::path::Path::new(current_dir)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        format!("{}/{}", parent, stripped)
    } else if relative_url.starts_with('/') {
        // Already absolute path within bucket
        relative_url.to_string()
    } else {
        // Relative path without ./ prefix
        format!("{}/{}", current_dir, relative_url)
    };

    // Normalize the path and ensure it starts with /
    let normalized = std::path::PathBuf::from(&path).components().fold(
        std::path::PathBuf::new(),
        |mut acc, component| {
            match component {
                std::path::Component::ParentDir => {
                    acc.pop();
                }
                std::path::Component::Normal(part) => {
                    acc.push(part);
                }
                _ => {}
            }
            acc
        },
    );

    let normalized_str = normalized.to_str().unwrap_or("");
    format!(
        "{}/gw/{}/{}",
        host.trim_end_matches('/'),
        bucket_id,
        normalized_str
    )
}

/// Converts markdown content to HTML
pub fn markdown_to_html(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Wrap in basic HTML structure
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

/// Attempts to find an index file in a directory
/// Returns (file_path, mime_type) if found
pub async fn find_index_file(
    mount: &common::mount::Mount,
    dir_path: &std::path::Path,
) -> Option<(std::path::PathBuf, String)> {
    // Priority order: index.html, index.htm, index.md, index.txt
    let candidates = [
        ("index.html", "text/html"),
        ("index.htm", "text/html"),
        ("index.md", "text/markdown"),
        ("index.txt", "text/plain"),
    ];

    for (filename, mime_type) in &candidates {
        let index_path = dir_path.join(filename);
        if mount.get(&index_path).await.is_ok() {
            return Some((index_path, mime_type.to_string()));
        }
    }

    None
}

/// Check if the Accept header indicates JSON is preferred
pub fn wants_json(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|h| h.to_str().ok())
        .map(|accept| {
            // Check if application/json is present and has higher priority than text/html
            // Simple heuristic: if application/json appears before text/html or text/html is absent
            let json_pos = accept.find("application/json");
            let html_pos = accept.find("text/html");
            match (json_pos, html_pos) {
                (Some(j), Some(h)) => j < h,
                (Some(_), None) => true,
                _ => false,
            }
        })
        .unwrap_or(false)
}

/// Build path segments for breadcrumb navigation
pub fn build_path_segments(path: &str) -> Vec<PathSegment> {
    if path == "/" {
        return vec![];
    }

    let mut segments = Vec::new();
    let mut current_path = String::new();

    for part in path.trim_start_matches('/').split('/') {
        if !part.is_empty() {
            current_path = format!("{}/{}", current_path, part);
            segments.push(PathSegment {
                name: part.to_string(),
                path: current_path.clone(),
            });
        }
    }

    segments
}

/// Get parent path for "Up" navigation
pub fn get_parent_path(path: &str) -> String {
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

/// Format file size for display
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Convert bytes to hex dump for binary files
pub fn to_hex_dump(data: &[u8], max_bytes: usize) -> String {
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
