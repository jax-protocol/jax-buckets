//! Core types for conflict resolution
//!
//! This module defines the fundamental types used in conflict detection
//! and resolution during PathOpLog merges.

use std::path::PathBuf;

use super::super::path_ops::PathOperation;

/// A detected conflict between two operations on the same path
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The path where the conflict occurred
    pub path: PathBuf,
    /// The local (base) operation
    pub base: PathOperation,
    /// The incoming (remote) operation
    pub incoming: PathOperation,
}

impl Conflict {
    /// Create a new conflict
    pub fn new(path: PathBuf, base: PathOperation, incoming: PathOperation) -> Self {
        Self {
            path,
            base,
            incoming,
        }
    }

    /* Getters */

    /// Check if both operations have the same timestamp (true concurrent edit)
    pub fn is_concurrent(&self) -> bool {
        self.base.id.timestamp == self.incoming.id.timestamp
    }

    /// Get the operation with the higher OpId (the "winner" by default CRDT rules)
    pub fn crdt_winner(&self) -> &PathOperation {
        if self.incoming.id > self.base.id {
            &self.incoming
        } else {
            &self.base
        }
    }
}

/// Resolution decision for a conflict
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Use the base (local) operation
    UseBase,
    /// Use the incoming (remote) operation
    UseIncoming,
    /// Keep both operations (fork state)
    KeepBoth,
    /// Skip both operations (neither is applied)
    SkipBoth,
    /// Rename the incoming operation to a new path (creates a conflict file)
    RenameIncoming {
        /// The new path for the incoming operation
        new_path: PathBuf,
    },
}

/// Result of a merge operation with conflict information
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Number of operations added from the incoming log
    pub operations_added: usize,
    /// Conflicts that were resolved
    pub conflicts_resolved: Vec<ResolvedConflict>,
    /// Conflicts that could not be auto-resolved (when using ForkOnConflict)
    pub unresolved_conflicts: Vec<Conflict>,
}

impl MergeResult {
    /// Create a new merge result
    pub fn new() -> Self {
        Self {
            operations_added: 0,
            conflicts_resolved: Vec::new(),
            unresolved_conflicts: Vec::new(),
        }
    }

    /* Getters */

    /// Check if there were any unresolved conflicts
    pub fn has_unresolved(&self) -> bool {
        !self.unresolved_conflicts.is_empty()
    }

    /// Total number of conflicts (resolved + unresolved)
    pub fn total_conflicts(&self) -> usize {
        self.conflicts_resolved.len() + self.unresolved_conflicts.len()
    }
}

impl Default for MergeResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A conflict that was resolved
#[derive(Debug, Clone)]
pub struct ResolvedConflict {
    /// The original conflict
    pub conflict: Conflict,
    /// How it was resolved
    pub resolution: Resolution,
}
