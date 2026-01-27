//! Last-write-wins conflict resolver

use crate::crypto::PublicKey;

use super::types::{Conflict, Resolution};
use super::ConflictResolver;

/// Last-write-wins conflict resolution (default CRDT behavior)
///
/// The operation with the highest OpId wins:
/// 1. Higher Lamport timestamp wins
/// 2. If timestamps are equal, higher peer_id wins (lexicographic)
///
/// This is deterministic and matches the default PathOpLog behavior.
#[derive(Debug, Clone, Default)]
pub struct LastWriteWins;

impl LastWriteWins {
    /// Create a new LastWriteWins resolver
    pub fn new() -> Self {
        Self
    }
}

impl ConflictResolver for LastWriteWins {
    fn resolve(&self, conflict: &Conflict, _local_peer: &PublicKey) -> Resolution {
        if conflict.incoming.id > conflict.base.id {
            Resolution::UseIncoming
        } else {
            Resolution::UseBase
        }
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
    fn test_last_write_wins_incoming() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 1, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 2, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);
        let resolver = LastWriteWins::new();

        // Incoming has higher timestamp -> UseIncoming
        assert_eq!(resolver.resolve(&conflict, &peer1), Resolution::UseIncoming);
    }

    #[test]
    fn test_last_write_wins_base() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 2, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 1, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);
        let resolver = LastWriteWins::new();

        // Base has higher timestamp -> UseBase
        assert_eq!(resolver.resolve(&conflict, &peer1), Resolution::UseBase);
    }

    #[test]
    fn test_last_write_wins_tiebreak_by_peer_id() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 1, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 1, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);
        let resolver = LastWriteWins::new();

        let resolution = resolver.resolve(&conflict, &peer1);

        // Same timestamp, peer2 > peer1 (usually) -> check based on actual ordering
        if peer2 > peer1 {
            assert_eq!(resolution, Resolution::UseIncoming);
        } else {
            assert_eq!(resolution, Resolution::UseBase);
        }
    }
}
