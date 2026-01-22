//! Conflict-file resolver

use std::path::PathBuf;

use crate::crypto::PublicKey;

use super::super::path_ops::OpType;
use super::types::{Conflict, Resolution};
use super::ConflictResolver;

/// Conflict-file resolution (recommended for peer sync)
///
/// When a conflict is detected, renames the incoming file to include a version
/// suffix based on the content hash: `<name>@<short-hash>.<ext>`.
///
/// This preserves both versions:
/// - The base (local) operation wins and keeps the original path
/// - The incoming operation is renamed to a conflict file with its content version
///
/// Users can then manually review and resolve the conflict files.
///
/// # Example
///
/// If `document.txt` conflicts and incoming has content hash `abc123...`:
/// - Local version stays at `document.txt`
/// - Incoming version becomes `document@abc123de.txt`
#[derive(Debug, Clone, Default)]
pub struct ConflictFile {
    /// Number of hex characters to use from the hash (default: 8)
    pub hash_length: usize,
}

impl ConflictFile {
    /// Create a new ConflictFile resolver with default settings
    pub fn new() -> Self {
        Self { hash_length: 8 }
    }

    /// Create a new ConflictFile resolver with custom hash length
    pub fn with_hash_length(hash_length: usize) -> Self {
        Self { hash_length }
    }

    /// Generate a conflict filename using a version string
    ///
    /// Format: `<stem>@<version>.<ext>` or `<stem>@<version>` if no extension
    pub fn conflict_path(path: &std::path::Path, version: &str) -> PathBuf {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = path.extension().and_then(|e| e.to_str());

        let conflict_name = match ext {
            Some(ext) => format!("{}@{}.{}", stem, version, ext),
            None => format!("{}@{}", stem, version),
        };

        match path.parent() {
            Some(parent) if parent != std::path::Path::new("") => parent.join(conflict_name),
            _ => PathBuf::from(conflict_name),
        }
    }
}

impl ConflictResolver for ConflictFile {
    fn resolve(&self, conflict: &Conflict, _local_peer: &PublicKey) -> Resolution {
        // Only create conflict files for Add operations (file content conflicts)
        // For Remove or Mv, use standard CRDT resolution
        match (&conflict.base.op_type, &conflict.incoming.op_type) {
            (OpType::Add, OpType::Add) => {
                // Both are adds - create a conflict file for incoming
                // Use the content link hash as the version identifier
                let version = match &conflict.incoming.content_link {
                    Some(link) => {
                        let hash_str = link.hash().to_string();
                        // Take first N characters of the hash
                        hash_str.chars().take(self.hash_length).collect::<String>()
                    }
                    // Fallback to timestamp if no content link (shouldn't happen for Add)
                    None => conflict.incoming.id.timestamp.to_string(),
                };
                let new_path = Self::conflict_path(&conflict.incoming.path, &version);
                Resolution::RenameIncoming { new_path }
            }
            _ => {
                // For other conflicts (Remove, Mv), fall back to last-write-wins
                if conflict.incoming.id > conflict.base.id {
                    Resolution::UseIncoming
                } else {
                    Resolution::UseBase
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::SecretKey;
    use crate::linked_data::Link;
    use crate::mount::path_ops::{OpId, OpType, PathOperation};

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

    fn make_op_with_link(
        peer_id: PublicKey,
        timestamp: u64,
        op_type: OpType,
        path: &str,
        hash_seed: u8,
    ) -> PathOperation {
        // Create a deterministic hash from the seed
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = hash_seed;
        let hash = iroh_blobs::Hash::from_bytes(hash_bytes);
        let link = Link::new(crate::linked_data::LD_RAW_CODEC, hash);

        PathOperation {
            id: OpId { timestamp, peer_id },
            op_type,
            path: PathBuf::from(path),
            content_link: Some(link),
            is_dir: false,
        }
    }

    #[test]
    fn test_conflict_file_path_with_extension() {
        let path = PathBuf::from("document.txt");
        let result = ConflictFile::conflict_path(&path, "abc12345");
        assert_eq!(result, PathBuf::from("document@abc12345.txt"));
    }

    #[test]
    fn test_conflict_file_path_without_extension() {
        let path = PathBuf::from("README");
        let result = ConflictFile::conflict_path(&path, "abc12345");
        assert_eq!(result, PathBuf::from("README@abc12345"));
    }

    #[test]
    fn test_conflict_file_path_nested() {
        let path = PathBuf::from("docs/notes/file.md");
        let result = ConflictFile::conflict_path(&path, "v42");
        assert_eq!(result, PathBuf::from("docs/notes/file@v42.md"));
    }

    #[test]
    fn test_conflict_file_resolver_add_vs_add() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        // Create ops with content links - hash_seed determines the hash
        let base = make_op_with_link(peer1, 1, OpType::Add, "file.txt", 0xAA);
        let incoming = make_op_with_link(peer2, 100, OpType::Add, "file.txt", 0xBB);

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming.clone());
        let resolver = ConflictFile::new();

        let resolution = resolver.resolve(&conflict, &peer1);

        // Should rename incoming to conflict file using content hash
        match resolution {
            Resolution::RenameIncoming { new_path } => {
                // The hash starts with 0xBB, so first 8 chars of hex representation
                let expected_version: String = incoming
                    .content_link
                    .unwrap()
                    .hash()
                    .to_string()
                    .chars()
                    .take(8)
                    .collect();
                assert_eq!(
                    new_path,
                    PathBuf::from(format!("file@{}.txt", expected_version))
                );
            }
            _ => panic!("Expected RenameIncoming, got {:?}", resolution),
        }
    }

    #[test]
    fn test_conflict_file_resolver_add_vs_remove() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 1, OpType::Add, "file.txt");
        let incoming = make_op(peer2, 100, OpType::Remove, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);
        let resolver = ConflictFile::new();

        // Non-Add conflicts fall back to last-write-wins
        // incoming (ts=100) > base (ts=1)
        assert_eq!(resolver.resolve(&conflict, &peer1), Resolution::UseIncoming);
    }

    #[test]
    fn test_conflict_file_resolver_remove_vs_add() {
        let peer1 = make_peer_id(1);
        let peer2 = make_peer_id(2);

        let base = make_op(peer1, 100, OpType::Remove, "file.txt");
        let incoming = make_op(peer2, 1, OpType::Add, "file.txt");

        let conflict = Conflict::new(PathBuf::from("file.txt"), base, incoming);
        let resolver = ConflictFile::new();

        // Non-Add conflicts fall back to last-write-wins
        // base (ts=100) > incoming (ts=1)
        assert_eq!(resolver.resolve(&conflict, &peer1), Resolution::UseBase);
    }

    // =========================================================================
    // READABLE SCENARIO TESTS
    // These tests demonstrate real-world conflict scenarios in plain English.
    // =========================================================================

    /// Scenario: Alice and Bob both create a file called "notes.txt" offline.
    /// When they sync, we keep Alice's version at the original path and
    /// rename Bob's to include his content's hash, like "notes@a1b2c3d4.txt".
    #[test]
    fn scenario_two_peers_create_same_file_offline() {
        // Setup: Alice and Bob are peers
        let alice = make_peer_id(1);
        let bob = make_peer_id(2);

        // Alice creates "notes.txt" with some content (hash seed 0x11)
        let alice_creates_notes = make_op_with_link(
            alice,
            1000, // timestamp
            OpType::Add,
            "notes.txt",
            0x11, // produces hash starting with "11000000..."
        );

        // Bob also creates "notes.txt" with different content (hash seed 0x22)
        let bob_creates_notes = make_op_with_link(
            bob,
            1001, // slightly later timestamp
            OpType::Add,
            "notes.txt",
            0x22, // produces hash starting with "22000000..."
        );

        // When we merge Bob's changes into Alice's log, there's a conflict
        let conflict = Conflict::new(
            PathBuf::from("notes.txt"),
            alice_creates_notes,       // base: Alice's local version
            bob_creates_notes.clone(), // incoming: Bob's version to merge
        );

        // Use ConflictFile resolver - it creates conflict files for Add vs Add
        let resolver = ConflictFile::new();
        let resolution = resolver.resolve(&conflict, &alice);

        // Result: Bob's file gets renamed to include his content hash
        match resolution {
            Resolution::RenameIncoming { new_path } => {
                // The new path should be "notes@<first-8-chars-of-hash>.txt"
                let path_str = new_path.to_string_lossy();
                assert!(path_str.starts_with("notes@"), "Should have @ separator");
                assert!(path_str.ends_with(".txt"), "Should keep extension");
                assert!(path_str.contains("22"), "Should contain Bob's hash prefix");
            }
            other => panic!("Expected RenameIncoming, got {:?}", other),
        }
    }

    /// Scenario: File naming format examples.
    /// Shows exactly what conflict filenames look like.
    #[test]
    fn scenario_conflict_file_naming_examples() {
        // Example 1: Simple text file
        // "report.txt" with hash "abc123def456..." becomes "report@abc123de.txt"
        let path1 = ConflictFile::conflict_path(&PathBuf::from("report.txt"), "abc123de");
        assert_eq!(path1.to_string_lossy(), "report@abc123de.txt");

        // Example 2: File without extension
        // "Makefile" with hash "deadbeef..." becomes "Makefile@deadbeef"
        let path2 = ConflictFile::conflict_path(&PathBuf::from("Makefile"), "deadbeef");
        assert_eq!(path2.to_string_lossy(), "Makefile@deadbeef");

        // Example 3: Nested path
        // "src/lib/utils.rs" with hash "cafebabe..." becomes "src/lib/utils@cafebabe.rs"
        let path3 = ConflictFile::conflict_path(&PathBuf::from("src/lib/utils.rs"), "cafebabe");
        assert_eq!(path3.to_string_lossy(), "src/lib/utils@cafebabe.rs");
    }

    /// Scenario: Different conflict types get different resolutions.
    /// Only Add-vs-Add creates conflict files. Other conflicts use last-write-wins.
    #[test]
    fn scenario_different_conflict_types() {
        let alice = make_peer_id(1);
        let bob = make_peer_id(2);
        let resolver = ConflictFile::new();

        // Case 1: Add vs Add -> Creates conflict file (both users want the file)
        let alice_adds = make_op_with_link(alice, 1, OpType::Add, "file.txt", 0xAA);
        let bob_adds = make_op_with_link(bob, 2, OpType::Add, "file.txt", 0xBB);
        let add_conflict = Conflict::new(PathBuf::from("file.txt"), alice_adds, bob_adds);
        assert!(matches!(
            resolver.resolve(&add_conflict, &alice),
            Resolution::RenameIncoming { .. }
        ));

        // Case 2: Add vs Remove -> Last-write-wins (one wants it, one doesn't)
        let alice_adds = make_op(alice, 1, OpType::Add, "file.txt");
        let bob_removes = make_op(bob, 2, OpType::Remove, "file.txt");
        let mixed_conflict = Conflict::new(PathBuf::from("file.txt"), alice_adds, bob_removes);
        // Bob's remove (ts=2) wins over Alice's add (ts=1)
        assert_eq!(
            resolver.resolve(&mixed_conflict, &alice),
            Resolution::UseIncoming
        );

        // Case 3: Remove vs Remove -> Last-write-wins (both want it gone, no conflict file needed)
        let alice_removes = make_op(alice, 1, OpType::Remove, "file.txt");
        let bob_removes = make_op(bob, 2, OpType::Remove, "file.txt");
        let both_remove = Conflict::new(PathBuf::from("file.txt"), alice_removes, bob_removes);
        // Bob's remove (ts=2) wins
        assert_eq!(
            resolver.resolve(&both_remove, &alice),
            Resolution::UseIncoming
        );
    }

    /// Scenario: Custom hash length.
    /// You can configure how many characters of the hash to use.
    #[test]
    fn scenario_custom_hash_length() {
        let alice = make_peer_id(1);
        let bob = make_peer_id(2);

        let alice_adds = make_op_with_link(alice, 1, OpType::Add, "doc.md", 0xAA);
        let bob_adds = make_op_with_link(bob, 2, OpType::Add, "doc.md", 0xBB);
        let conflict = Conflict::new(PathBuf::from("doc.md"), alice_adds, bob_adds);

        // Default: 8 characters
        let resolver_8 = ConflictFile::new();
        if let Resolution::RenameIncoming { new_path } = resolver_8.resolve(&conflict, &alice) {
            let name = new_path.file_stem().unwrap().to_string_lossy();
            let hash_part = name.split('@').nth(1).unwrap();
            assert_eq!(hash_part.len(), 8, "Default should use 8 hash chars");
        }

        // Custom: 4 characters (shorter, but more collision risk)
        let resolver_4 = ConflictFile::with_hash_length(4);
        if let Resolution::RenameIncoming { new_path } = resolver_4.resolve(&conflict, &alice) {
            let name = new_path.file_stem().unwrap().to_string_lossy();
            let hash_part = name.split('@').nth(1).unwrap();
            assert_eq!(hash_part.len(), 4, "Custom should use 4 hash chars");
        }

        // Custom: 16 characters (longer, less collision risk)
        let resolver_16 = ConflictFile::with_hash_length(16);
        if let Resolution::RenameIncoming { new_path } = resolver_16.resolve(&conflict, &alice) {
            let name = new_path.file_stem().unwrap().to_string_lossy();
            let hash_part = name.split('@').nth(1).unwrap();
            assert_eq!(hash_part.len(), 16, "Custom should use 16 hash chars");
        }
    }
}
