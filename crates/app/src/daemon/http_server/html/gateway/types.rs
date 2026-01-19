use askama::Template;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GatewayQuery {
    #[serde(default)]
    pub at: Option<String>,
    /// If true, serve the raw file with Content-Disposition: attachment
    #[serde(default)]
    pub download: Option<bool>,
    /// If true, show the file in viewer UI even if it's HTML/Markdown
    #[serde(default)]
    pub view: Option<bool>,
    /// If true, recursively list all files under the path (deep listing)
    #[serde(default)]
    pub deep: Option<bool>,
}

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
