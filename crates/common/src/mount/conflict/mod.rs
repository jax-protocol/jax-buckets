//! Conflict resolution for PathOpLog merges
//!
//! This module provides pluggable conflict resolution strategies for handling
//! concurrent edits from different peers. When two peers edit the same path
//! concurrently, the resolver determines how to reconcile the conflict.
//!
//! # Built-in Strategies
//!
//! - **[`LastWriteWins`]**: Higher timestamp wins (default CRDT behavior)
//! - **[`BaseWins`]**: Local operations win over incoming ones
//! - **[`ForkOnConflict`]**: Keep both versions, returning conflicts for manual resolution
//! - **[`ConflictFile`]**: Rename incoming to `<name>@<content-hash>` to preserve both versions
//!
//! # Custom Resolvers
//!
//! Implement the [`ConflictResolver`] trait to create custom resolution strategies.

mod base_wins;
mod conflict_file;
mod fork_on_conflict;
mod last_write_wins;
mod types;

pub use base_wins::BaseWins;
pub use conflict_file::ConflictFile;
pub use fork_on_conflict::ForkOnConflict;
pub use last_write_wins::LastWriteWins;
pub use types::{Conflict, MergeResult, Resolution, ResolvedConflict};

use std::path::PathBuf;

use crate::crypto::PublicKey;

use super::path_ops::{OpType, PathOperation};

/// Trait for conflict resolution strategies
///
/// Implementors define how to resolve conflicts when merging PathOpLogs
/// from different peers.
pub trait ConflictResolver: std::fmt::Debug + Send + Sync {
    /// Resolve a conflict between two operations on the same path
    ///
    /// # Arguments
    ///
    /// * `conflict` - The conflict to resolve
    /// * `local_peer` - The local peer's identity (useful for deterministic tie-breaking)
    ///
    /// # Returns
    ///
    /// The resolution decision
    fn resolve(&self, conflict: &Conflict, local_peer: &PublicKey) -> Resolution;
}

/// Check if two operations conflict
///
/// Two operations conflict if:
/// 1. They affect the same path
/// 2. They have different OpIds
/// 3. At least one is a destructive operation (Remove, Mv, or Add that overwrites)
pub fn operations_conflict(base: &PathOperation, incoming: &PathOperation) -> bool {
    // Same OpId means same operation, no conflict
    if base.id == incoming.id {
        return false;
    }

    // Must affect the same path
    if base.path != incoming.path {
        return false;
    }

    // Check if either operation is destructive
    let base_destructive = is_destructive(&base.op_type);
    let incoming_destructive = is_destructive(&incoming.op_type);

    // Conflict if either is destructive, or both are Add (concurrent creates)
    base_destructive
        || incoming_destructive
        || (matches!(base.op_type, OpType::Add) && matches!(incoming.op_type, OpType::Add))
}

/// Check if an operation type is destructive
fn is_destructive(op_type: &OpType) -> bool {
    matches!(op_type, OpType::Remove | OpType::Mv { .. })
}

/// Check if an operation at this path would conflict with a move operation
///
/// Move operations are special because they affect two paths: source and destination.
/// This checks if an operation conflicts with a move's source path.
pub fn conflicts_with_mv_source(op: &PathOperation, mv_from: &PathBuf) -> bool {
    // Check if op affects the mv source path
    &op.path == mv_from
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::SecretKey;
    use crate::mount::path_ops::OpId;

    fn make_peer_id(seed: u8) -> PublicKey {
        let mut seed_bytes = [0u8; 32];
        seed_bytes[0] = seed;
        let secret = SecretKey::from(seed_bytes);
        secret.public()
    }

    fn make_op(peer_id: PublicKey, timestamp: u64, op_type: OpType, path: &str) -> PathOperation {
        PathOperation {
            id: OpId { timestamp, peer_id },
            op_type,
            path: PathBuf::from(path),
            content_link: None,
            is_dir: false,
        }
    }

    #[test]
    fn test_conflict_detection() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let op1 = make_op(peer1, 1, OpType::Add, "file.txt");
        let op2 = make_op(peer2, 1, OpType::Add, "file.txt");

        // Same path, different peers, both Add -> conflict
        assert!(operations_conflict(&op1, &op2));
    }

    #[test]
    fn test_no_conflict_different_paths() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let op1 = make_op(peer1, 1, OpType::Add, "file1.txt");
        let op2 = make_op(peer2, 1, OpType::Add, "file2.txt");

        // Different paths -> no conflict
        assert!(!operations_conflict(&op1, &op2));
    }

    #[test]
    fn test_no_conflict_same_operation() {
        let peer1 = make_peer_id(1);

        let op1 = make_op(peer1, 1, OpType::Add, "file.txt");
        let op2 = op1.clone();

        // Same OpId -> no conflict (same operation)
        assert!(!operations_conflict(&op1, &op2));
    }

    #[test]
    fn test_conflict_add_vs_remove() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let op1 = make_op(peer1, 1, OpType::Add, "file.txt");
        let op2 = make_op(peer2, 1, OpType::Remove, "file.txt");

        // Add vs Remove on same path -> conflict
        assert!(operations_conflict(&op1, &op2));
    }

    #[test]
    fn test_conflict_mkdir_vs_remove() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let op1 = make_op(peer1, 1, OpType::Mkdir, "dir");
        let op2 = make_op(peer2, 1, OpType::Remove, "dir");

        // Mkdir vs Remove on same path -> conflict
        assert!(operations_conflict(&op1, &op2));
    }

    #[test]
    fn test_no_conflict_mkdir_vs_mkdir() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let op1 = make_op(peer1, 1, OpType::Mkdir, "dir");
        let op2 = make_op(peer2, 1, OpType::Mkdir, "dir");

        // Mkdir vs Mkdir is idempotent -> no conflict
        assert!(!operations_conflict(&op1, &op2));
    }

    #[test]
    fn test_conflict_is_concurrent() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 5, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 5, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);

        // Same timestamp -> concurrent
        assert!(conflict.is_concurrent());
    }

    #[test]
    fn test_conflict_not_concurrent() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 3, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 5, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);

        // Different timestamps -> not concurrent
        assert!(!conflict.is_concurrent());
    }

    #[test]
    fn test_crdt_winner() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 3, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 5, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming.clone());

        // Incoming has higher timestamp
        assert_eq!(conflict.crdt_winner().id, incoming.id);
    }

    #[test]
    fn test_merge_result() {
        let mut result = MergeResult::new();

        assert_eq!(result.operations_added, 0);
        assert!(!result.has_unresolved());
        assert_eq!(result.total_conflicts(), 0);

        // Add an unresolved conflict
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);
        let base = make_op(peer1, 1, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 1, OpType::Add, "file.txt");

        result
            .unresolved_conflicts
            .push(Conflict::new(PathBuf::from("file.txt"), base, incoming));

        assert!(result.has_unresolved());
        assert_eq!(result.total_conflicts(), 1);
    }
}
