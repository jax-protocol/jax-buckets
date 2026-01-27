//! Fork-on-conflict resolver

use crate::crypto::PublicKey;

use super::types::{Conflict, Resolution};
use super::ConflictResolver;

/// Fork-on-conflict resolution
///
/// Keeps both operations in the log when a conflict is detected.
/// Conflicts are tracked and returned for manual resolution later.
///
/// This is useful when automatic conflict resolution is not acceptable
/// and users need to manually choose which version to keep.
#[derive(Debug, Clone, Default)]
pub struct ForkOnConflict;

impl ForkOnConflict {
    /// Create a new ForkOnConflict resolver
    pub fn new() -> Self {
        Self
    }
}

impl ConflictResolver for ForkOnConflict {
    fn resolve(&self, _conflict: &Conflict, _local_peer: &PublicKey) -> Resolution {
        Resolution::KeepBoth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::SecretKey;
    use crate::mount::path_ops::{OpId, OpType, PathOperation};
    use std::path::PathBuf;

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
    fn test_fork_on_conflict() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 1, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 2, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);
        let resolver = ForkOnConflict::new();

        // ForkOnConflict always returns KeepBoth
        assert_eq!(resolver.resolve(&conflict, &peer1), Resolution::KeepBoth);
    }
}
