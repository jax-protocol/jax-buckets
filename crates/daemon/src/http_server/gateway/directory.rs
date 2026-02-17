use askama::Template;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use uuid::Uuid;

use common::mount::NodeLink;

use super::{BucketMeta, GatewayQuery};

/// Path segment for breadcrumb navigation
#[derive(Debug, Clone)]
pub struct PathSegment {
    pub name: String,
    pub path: String,
}

/// File display info for directory listings
#[derive(Debug, Clone)]
pub struct FileDisplayInfo {
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub is_dir: bool,
}

/// Template for directory explorer
#[derive(Template)]
#[template(path = "pages/gateway/explorer.html")]
pub struct GatewayExplorerTemplate {
    pub bucket_id: String,
    pub bucket_id_short: String,
    pub bucket_name: String,
    pub bucket_link: String,
    pub bucket_link_short: String,
    pub path_segments: Vec<PathSegment>,
    pub items: Vec<FileDisplayInfo>,
}

#[derive(Debug, Serialize)]
pub struct DirectoryListing {
    pub path: String,
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Serialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub mime_type: String,
}

pub async fn handle(
    mount: &common::mount::Mount,
    path_buf: &std::path::Path,
    absolute_path: &str,
    query: &GatewayQuery,
    bucket_id: &Uuid,
    meta: &BucketMeta,
) -> Response {
    let wants_json = query.json.unwrap_or(false);

    // Check for index file first (unless JSON is explicitly requested)
    if !wants_json {
        if let Some((index_path, index_mime_type)) = find_index_file(mount, path_buf).await {
            let file_data = match mount.cat(&index_path).await {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Failed to read index file: {}", e);
                    return super::error_response("Failed to read index file");
                }
            };

            let index_path_str = index_path.to_str().unwrap_or(absolute_path);

            let (final_content, final_mime_type) = if index_mime_type == "text/markdown" {
                let content_str = String::from_utf8_lossy(&file_data);
                let html = markdown_to_html(&content_str);
                let rewritten = rewrite_relative_urls(&html, index_path_str, bucket_id);
                (rewritten.into_bytes(), "text/html; charset=utf-8")
            } else if index_mime_type == "text/html" {
                let content_str = String::from_utf8_lossy(&file_data);
                let rewritten = rewrite_relative_urls(&content_str, index_path_str, bucket_id);
                (rewritten.into_bytes(), "text/html; charset=utf-8")
            } else {
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
                return super::error_response("Failed to list directory");
            }
        }
    } else {
        match mount.ls(path_buf).await {
            Ok(items) => items,
            Err(e) => {
                tracing::error!("Failed to list directory: {}", e);
                return super::error_response("Failed to list directory");
            }
        }
    };

    if wants_json {
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
        bucket_id: meta.bucket_id_str.clone(),
        bucket_id_short: meta.bucket_id_short.clone(),
        bucket_name: meta.bucket_name.clone(),
        bucket_link: meta.bucket_link.clone(),
        bucket_link_short: meta.bucket_link_short.clone(),
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
            super::error_response("Failed to render page")
        }
    }
}

/// Attempts to find an index file in a directory
async fn find_index_file(
    mount: &common::mount::Mount,
    dir_path: &std::path::Path,
) -> Option<(std::path::PathBuf, String)> {
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

/// Build path segments for breadcrumb navigation
fn build_path_segments(path: &str) -> Vec<PathSegment> {
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

/// Rewrites relative URLs in content to absolute gateway URLs
pub(super) fn rewrite_relative_urls(content: &str, current_path: &str, bucket_id: &Uuid) -> String {
    use regex::Regex;
    use std::sync::LazyLock;

    static HTML_ATTR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?P<attr>(?:href|src|action|data|srcset))=["'](?P<url>\.{0,2}/[^"']+)["']"#)
            .unwrap()
    });

    static MARKDOWN_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"\]\((?P<url>\.{0,2}/[^)]+)\)"#).unwrap());

    let current_dir = if current_path == "/" {
        "".to_string()
    } else {
        std::path::Path::new(current_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    };

    let content = HTML_ATTR_REGEX.replace_all(content, |caps: &regex::Captures| {
        let attr = &caps["attr"];
        let url = &caps["url"];
        let absolute_url = resolve_relative_url(url, &current_dir, bucket_id);
        format!(r#"{}="{}""#, attr, absolute_url)
    });

    let content = MARKDOWN_LINK_REGEX.replace_all(&content, |caps: &regex::Captures| {
        let url = &caps["url"];
        let absolute_url = resolve_relative_url(url, &current_dir, bucket_id);
        format!("]({})", absolute_url)
    });

    content.to_string()
}

/// Resolves a relative URL to an absolute gateway URL
fn resolve_relative_url(relative_url: &str, current_dir: &str, bucket_id: &Uuid) -> String {
    let path = if let Some(stripped) = relative_url.strip_prefix("./") {
        format!("{}/{}", current_dir, stripped)
    } else if let Some(stripped) = relative_url.strip_prefix("../") {
        let parent = std::path::Path::new(current_dir)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        format!("{}/{}", parent, stripped)
    } else if relative_url.starts_with('/') {
        relative_url.to_string()
    } else {
        format!("{}/{}", current_dir, relative_url)
    };

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
    format!("/gw/{}/{}", bucket_id, normalized_str)
}

/// Converts markdown content to HTML
pub(super) fn markdown_to_html(markdown: &str) -> String {
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
