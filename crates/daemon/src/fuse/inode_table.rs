//! Bidirectional inode ↔ path mapping for FUSE filesystem
//!
//! FUSE uses 64-bit inode numbers to identify files and directories.
//! This module provides efficient bidirectional mapping between inodes and paths.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bidirectional mapping between inodes and paths
#[derive(Debug)]
pub struct InodeTable {
    /// Path to inode mapping
    path_to_inode: HashMap<String, u64>,
    /// Inode to path mapping
    inode_to_path: HashMap<u64, String>,
    /// Next available inode number (starts at 2, as 1 is reserved for root)
    next_inode: AtomicU64,
}

impl Default for InodeTable {
    fn default() -> Self {
        Self::new()
    }
}

impl InodeTable {
    /// Root inode number (always 1 in FUSE)
    pub const ROOT_INODE: u64 = 1;

    /// Create a new inode table with root pre-registered
    pub fn new() -> Self {
        let mut table = Self {
            path_to_inode: HashMap::new(),
            inode_to_path: HashMap::new(),
            next_inode: AtomicU64::new(2), // Start at 2, 1 is root
        };

        // Register root
        table
            .path_to_inode
            .insert("/".to_string(), Self::ROOT_INODE);
        table
            .inode_to_path
            .insert(Self::ROOT_INODE, "/".to_string());

        table
    }

    /// Get or create an inode for a path
    pub fn get_or_create(&mut self, path: &str) -> u64 {
        let normalized = Self::normalize_path(path);

        if let Some(&inode) = self.path_to_inode.get(&normalized) {
            return inode;
        }

        let inode = self.next_inode.fetch_add(1, Ordering::SeqCst);
        self.path_to_inode.insert(normalized.clone(), inode);
        self.inode_to_path.insert(inode, normalized);
        inode
    }

    /// Get the inode for a path if it exists
    pub fn get_inode(&self, path: &str) -> Option<u64> {
        let normalized = Self::normalize_path(path);
        self.path_to_inode.get(&normalized).copied()
    }

    /// Get the path for an inode if it exists
    pub fn get_path(&self, inode: u64) -> Option<&str> {
        self.inode_to_path.get(&inode).map(String::as_str)
    }

    /// Remove an inode and its path mapping
    pub fn remove(&mut self, inode: u64) -> Option<String> {
        if let Some(path) = self.inode_to_path.remove(&inode) {
            self.path_to_inode.remove(&path);
            Some(path)
        } else {
            None
        }
    }

    /// Remove by path and return the inode
    pub fn remove_by_path(&mut self, path: &str) -> Option<u64> {
        let normalized = Self::normalize_path(path);
        if let Some(inode) = self.path_to_inode.remove(&normalized) {
            self.inode_to_path.remove(&inode);
            Some(inode)
        } else {
            None
        }
    }

    /// Rename a path (move inode to new path)
    pub fn rename(&mut self, old_path: &str, new_path: &str) -> Option<u64> {
        let old_normalized = Self::normalize_path(old_path);
        let new_normalized = Self::normalize_path(new_path);

        if let Some(inode) = self.path_to_inode.remove(&old_normalized) {
            self.inode_to_path.insert(inode, new_normalized.clone());
            self.path_to_inode.insert(new_normalized, inode);
            Some(inode)
        } else {
            None
        }
    }

    /// Clear all mappings except root
    pub fn clear(&mut self) {
        self.path_to_inode.clear();
        self.inode_to_path.clear();
        self.next_inode.store(2, Ordering::SeqCst);

        // Re-register root
        self.path_to_inode.insert("/".to_string(), Self::ROOT_INODE);
        self.inode_to_path.insert(Self::ROOT_INODE, "/".to_string());
    }

    /// Normalize a path to a consistent format
    fn normalize_path(path: &str) -> String {
        let path = path.trim();

        // Handle empty or root
        if path.is_empty() || path == "/" {
            return "/".to_string();
        }

        // Ensure leading slash, no trailing slash
        let mut normalized = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized.pop();
        }

        normalized
    }

    /// Get the parent path of a given path
    pub fn parent_path(path: &str) -> String {
        let normalized = Self::normalize_path(path);
        if normalized == "/" {
            return "/".to_string();
        }

        match normalized.rfind('/') {
            Some(0) => "/".to_string(),
            Some(pos) => normalized[..pos].to_string(),
            None => "/".to_string(),
        }
    }

    /// Get the filename component of a path
    pub fn filename(path: &str) -> &str {
        let normalized = path.trim();
        if normalized == "/" || normalized.is_empty() {
            return "";
        }

        match normalized.rfind('/') {
            Some(pos) => &normalized[pos + 1..],
            None => normalized,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_inode() {
        let table = InodeTable::new();
        assert_eq!(table.get_inode("/"), Some(InodeTable::ROOT_INODE));
        assert_eq!(table.get_path(InodeTable::ROOT_INODE), Some("/"));
    }

    #[test]
    fn test_get_or_create() {
        let mut table = InodeTable::new();

        let inode1 = table.get_or_create("/foo");
        let inode2 = table.get_or_create("/foo");
        let inode3 = table.get_or_create("/bar");

        assert_eq!(inode1, inode2);
        assert_ne!(inode1, inode3);
        assert_ne!(inode1, InodeTable::ROOT_INODE);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(InodeTable::normalize_path(""), "/");
        assert_eq!(InodeTable::normalize_path("/"), "/");
        assert_eq!(InodeTable::normalize_path("foo"), "/foo");
        assert_eq!(InodeTable::normalize_path("/foo"), "/foo");
        assert_eq!(InodeTable::normalize_path("/foo/"), "/foo");
        assert_eq!(InodeTable::normalize_path("/foo/bar"), "/foo/bar");
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(InodeTable::parent_path("/"), "/");
        assert_eq!(InodeTable::parent_path("/foo"), "/");
        assert_eq!(InodeTable::parent_path("/foo/bar"), "/foo");
        assert_eq!(InodeTable::parent_path("/foo/bar/baz"), "/foo/bar");
    }

    #[test]
    fn test_filename() {
        assert_eq!(InodeTable::filename("/"), "");
        assert_eq!(InodeTable::filename("/foo"), "foo");
        assert_eq!(InodeTable::filename("/foo/bar"), "bar");
        assert_eq!(InodeTable::filename("/foo/bar.txt"), "bar.txt");
    }

    #[test]
    fn test_remove() {
        let mut table = InodeTable::new();
        let inode = table.get_or_create("/foo");

        assert!(table.get_inode("/foo").is_some());
        table.remove(inode);
        assert!(table.get_inode("/foo").is_none());
    }

    #[test]
    fn test_rename() {
        let mut table = InodeTable::new();
        let inode = table.get_or_create("/old");

        table.rename("/old", "/new");

        assert!(table.get_inode("/old").is_none());
        assert_eq!(table.get_inode("/new"), Some(inode));
        assert_eq!(table.get_path(inode), Some("/new"));
    }

    #[test]
    fn test_clear() {
        let mut table = InodeTable::new();
        table.get_or_create("/foo");
        table.get_or_create("/bar");

        table.clear();

        assert_eq!(table.get_inode("/"), Some(InodeTable::ROOT_INODE));
        assert!(table.get_inode("/foo").is_none());
        assert!(table.get_inode("/bar").is_none());
    }
}
