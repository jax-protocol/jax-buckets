//! Integration tests for conflict resolution on divergent mounts
//!
//! These tests demonstrate real-world scenarios where multiple peers
//! make concurrent changes and need to merge their PathOpLogs.

mod common;

use std::io::Cursor;
use std::path::PathBuf;

use ::common::crypto::SecretKey;
use ::common::mount::{merge_logs, ConflictFile, LastWriteWins, OpType, PathOpLog, Resolution};

/// Helper to make a deterministic peer ID from a seed byte
fn make_peer_id(seed: u8) -> ::common::crypto::PublicKey {
    let mut seed_bytes = [0u8; 32];
    seed_bytes[0] = seed;
    let secret = SecretKey::from(seed_bytes);
    secret.public()
}

// =============================================================================
// SCENARIO: Two peers create the same file with different content
// =============================================================================

/// When Alice and Bob both create "notes.txt" offline with different content,
/// merging with ConflictFile resolver should:
/// - Keep Alice's version at "notes.txt"
/// - Rename Bob's version to "notes@<hash>.txt"
#[tokio::test]
async fn scenario_two_peers_same_file_conflict_file_resolver() {
    use ::common::mount::Mount;

    // Setup: Create Alice's mount
    let (mut alice_mount, blobs, alice_key, _temp) = common::setup_test_env().await;

    // Add Bob as an owner so he can load the mount
    let bob_key = SecretKey::generate();
    alice_mount.add_owner(bob_key.public()).await.unwrap();

    // Save to create the common ancestor (v0)
    let (ancestor_link, _, _) = alice_mount.save(&blobs, false).await.unwrap();

    // Bob loads from the same ancestor - now both start from identical state
    let mut bob_mount = Mount::load(&ancestor_link, &bob_key, &blobs)
        .await
        .expect("Bob should be able to load as owner");

    // Alice adds her version of notes.txt (diverging from ancestor)
    alice_mount
        .add(
            &PathBuf::from("/notes.txt"),
            Cursor::new(b"Alice's notes content".to_vec()),
        )
        .await
        .unwrap();

    // Bob adds his version of notes.txt (also diverging from same ancestor)
    bob_mount
        .add(
            &PathBuf::from("/notes.txt"),
            Cursor::new(b"Bob's notes content".to_vec()),
        )
        .await
        .unwrap();

    // Get both ops logs - these now represent divergent changes from same ancestor
    let alice_ops_log = alice_mount.inner().await.ops_log().clone();
    let bob_ops_log = bob_mount.inner().await.ops_log().clone();

    // Merge using ConflictFile resolver
    let resolver = ConflictFile::new();
    let local_peer = alice_key.public();
    let (merged_log, results) = merge_logs(&[&alice_ops_log, &bob_ops_log], &resolver, &local_peer);

    // Should have one merge result
    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should have detected and resolved one conflict
    assert_eq!(result.total_conflicts(), 1);
    assert_eq!(result.conflicts_resolved.len(), 1);

    // The resolution should be RenameIncoming
    let resolved = &result.conflicts_resolved[0];
    match &resolved.resolution {
        Resolution::RenameIncoming { new_path } => {
            // Should have @hash in the filename
            let name = new_path.to_string_lossy();
            assert!(
                name.starts_with("notes@"),
                "Expected notes@hash.txt, got {}",
                name
            );
            assert!(
                name.ends_with(".txt"),
                "Expected .txt extension, got {}",
                name
            );
        }
        other => panic!("Expected RenameIncoming, got {:?}", other),
    }

    // The merged log should have operations for both the original and conflict file
    let resolved_paths = merged_log.resolve_all();
    assert!(
        resolved_paths.contains_key(&PathBuf::from("notes.txt")),
        "Original file should exist"
    );

    // Should have 2 Add operations total (original + conflict file)
    let add_ops: Vec<_> = merged_log
        .ops_in_order()
        .filter(|op| matches!(op.op_type, OpType::Add))
        .collect();
    assert_eq!(add_ops.len(), 2, "Should have 2 Add operations");
}

// =============================================================================
// SCENARIO: Two peers add different files (no conflict)
// =============================================================================

/// When Alice adds "alice.txt" and Bob adds "bob.txt", there should be
/// no conflict and both files should be present after merge.
#[tokio::test]
async fn scenario_two_peers_different_files_no_conflict() {
    let alice = make_peer_id(1);
    let bob = make_peer_id(2);

    // Alice's log with her file
    let mut alice_log = PathOpLog::new();
    alice_log.record(alice, OpType::Add, "alice.txt", None, false);
    alice_log.record(alice, OpType::Mkdir, "docs", None, true);

    // Bob's log with his file
    let mut bob_log = PathOpLog::new();
    bob_log.record(bob, OpType::Add, "bob.txt", None, false);
    bob_log.record(bob, OpType::Add, "readme.md", None, false);

    // Merge with LastWriteWins (shouldn't matter, no conflicts)
    let resolver = LastWriteWins::new();
    let (merged, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice);

    // One merge operation
    assert_eq!(results.len(), 1);

    // No conflicts
    assert_eq!(results[0].total_conflicts(), 0);

    // All files present
    assert_eq!(merged.len(), 4);
    assert!(merged.resolve_path("alice.txt").is_some());
    assert!(merged.resolve_path("bob.txt").is_some());
    assert!(merged.resolve_path("readme.md").is_some());
    assert!(merged.resolve_path("docs").is_some());
}

// =============================================================================
// SCENARIO: Three-way merge (Alice, Bob, Carol all diverge)
// =============================================================================

/// When Alice, Bob, and Carol all create "report.txt" with different content,
/// merging should create conflict files for each divergent version.
#[tokio::test]
async fn scenario_three_way_merge_same_file() {
    let alice = make_peer_id(1);
    let bob = make_peer_id(2);
    let carol = make_peer_id(3);

    // Create deterministic content links
    let make_link = |seed: u8| {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = seed;
        let hash = iroh_blobs::Hash::from_bytes(hash_bytes);
        ::common::linked_data::Link::new(::common::linked_data::LD_RAW_CODEC, hash)
    };

    // Alice's log
    let mut alice_log = PathOpLog::new();
    alice_log.record(
        alice,
        OpType::Add,
        "report.txt",
        Some(make_link(0xAA)),
        false,
    );

    // Bob's log
    let mut bob_log = PathOpLog::new();
    bob_log.record(bob, OpType::Add, "report.txt", Some(make_link(0xBB)), false);

    // Carol's log
    let mut carol_log = PathOpLog::new();
    carol_log.record(
        carol,
        OpType::Add,
        "report.txt",
        Some(make_link(0xCC)),
        false,
    );

    // Merge all three with ConflictFile resolver
    let resolver = ConflictFile::new();
    let (merged, results) = merge_logs(&[&alice_log, &bob_log, &carol_log], &resolver, &alice);

    // Two merge operations
    assert_eq!(results.len(), 2);

    // Each merge should have one conflict
    assert_eq!(
        results[0].total_conflicts(),
        1,
        "Bob merge should have 1 conflict"
    );
    assert_eq!(
        results[1].total_conflicts(),
        1,
        "Carol merge should have 1 conflict"
    );

    // Merged log should have 3 operations (Alice's original + 2 conflict files)
    assert_eq!(merged.len(), 3);

    // Original file should exist
    assert!(merged.resolve_path("report.txt").is_some());

    // Count files with @ in the name (conflict files)
    let resolved = merged.resolve_all();
    let conflict_files: Vec<_> = resolved
        .keys()
        .filter(|p| p.to_string_lossy().contains('@'))
        .collect();
    assert_eq!(conflict_files.len(), 2, "Should have 2 conflict files");
}

// =============================================================================
// SCENARIO: Add vs Remove conflict
// =============================================================================

/// When Alice adds a file and Bob removes it (or vice versa),
/// LastWriteWins should resolve based on timestamp.
#[tokio::test]
async fn scenario_add_vs_remove_conflict() {
    let alice = make_peer_id(1);
    let bob = make_peer_id(2);

    // Alice adds a file at timestamp 1
    let mut alice_log = PathOpLog::new();
    alice_log.record(alice, OpType::Add, "file.txt", None, false);

    // Bob removes the same file at timestamp 2 (simulating later action)
    let mut bob_log = PathOpLog::new();
    bob_log.record(bob, OpType::Add, "dummy.txt", None, false); // ts=1
    bob_log.record(bob, OpType::Remove, "file.txt", None, false); // ts=2

    // Merge with LastWriteWins
    let resolver = LastWriteWins::new();
    let (merged, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice);

    // Should have one merge result
    assert_eq!(results.len(), 1);

    // Should have detected a conflict
    assert_eq!(results[0].total_conflicts(), 1);

    // Bob's remove should win (ts=2 > ts=1)
    let resolved = &results[0].conflicts_resolved[0];
    assert_eq!(resolved.resolution, Resolution::UseIncoming);

    // File should be removed in final state
    let final_state = merged.resolve_all();
    assert!(
        !final_state.contains_key(&PathBuf::from("file.txt")),
        "file.txt should be removed"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("dummy.txt")),
        "dummy.txt should exist"
    );
}

// =============================================================================
// SCENARIO: Concurrent mkdir (idempotent, no conflict)
// =============================================================================

/// When both Alice and Bob create the same directory, it should be idempotent
/// and not create a conflict file.
#[tokio::test]
async fn scenario_concurrent_mkdir_idempotent() {
    let alice = make_peer_id(1);
    let bob = make_peer_id(2);

    // Alice creates a directory
    let mut alice_log = PathOpLog::new();
    alice_log.record(alice, OpType::Mkdir, "shared", None, true);

    // Bob also creates the same directory
    let mut bob_log = PathOpLog::new();
    bob_log.record(bob, OpType::Mkdir, "shared", None, true);

    // Merge with ConflictFile resolver
    let resolver = ConflictFile::new();
    let (merged, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice);

    // Should have one merge result
    assert_eq!(results.len(), 1);

    // Mkdir vs Mkdir is idempotent, no conflict
    assert_eq!(
        results[0].total_conflicts(),
        0,
        "Mkdir should be idempotent"
    );

    // Both operations should be in the log
    assert_eq!(merged.len(), 2);

    // Directory should resolve
    let resolved = merged.resolve_path("shared");
    assert!(resolved.is_some());
    assert!(matches!(resolved.unwrap().op_type, OpType::Mkdir));
}

// =============================================================================
// SCENARIO: Nested file conflicts
// =============================================================================

/// When conflicts happen in nested directories, paths should be handled correctly.
#[tokio::test]
async fn scenario_nested_directory_conflicts() {
    let alice = make_peer_id(1);
    let bob = make_peer_id(2);

    let make_link = |seed: u8| {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = seed;
        let hash = iroh_blobs::Hash::from_bytes(hash_bytes);
        ::common::linked_data::Link::new(::common::linked_data::LD_RAW_CODEC, hash)
    };

    // Alice creates directory structure with a file
    let mut alice_log = PathOpLog::new();
    alice_log.record(alice, OpType::Mkdir, "docs", None, true);
    alice_log.record(alice, OpType::Mkdir, "docs/notes", None, true);
    alice_log.record(
        alice,
        OpType::Add,
        "docs/notes/meeting.md",
        Some(make_link(0xAA)),
        false,
    );

    // Bob creates the same file with different content
    let mut bob_log = PathOpLog::new();
    bob_log.record(bob, OpType::Mkdir, "docs", None, true);
    bob_log.record(bob, OpType::Mkdir, "docs/notes", None, true);
    bob_log.record(
        bob,
        OpType::Add,
        "docs/notes/meeting.md",
        Some(make_link(0xBB)),
        false,
    );

    // Merge with ConflictFile resolver
    let resolver = ConflictFile::new();
    let (merged, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice);

    // Should have conflicts only on the file, not the directories
    let file_conflicts: Vec<_> = results[0]
        .conflicts_resolved
        .iter()
        .filter(|c| c.conflict.path.to_string_lossy().contains("meeting"))
        .collect();
    assert_eq!(file_conflicts.len(), 1, "Should have 1 file conflict");

    // Check the conflict file path is nested correctly
    let resolved = &file_conflicts[0];
    match &resolved.resolution {
        Resolution::RenameIncoming { new_path } => {
            let path_str = new_path.to_string_lossy();
            assert!(
                path_str.starts_with("docs/notes/meeting@"),
                "Conflict file should be in docs/notes/, got {}",
                path_str
            );
            assert!(path_str.ends_with(".md"), "Should keep .md extension");
        }
        _ => panic!("Expected RenameIncoming"),
    }

    // Original file should exist at nested path
    assert!(merged.resolve_path("docs/notes/meeting.md").is_some());
}

// =============================================================================
// SCENARIO: Multi-version divergence (the real-world case)
// =============================================================================

/// Simulate the realistic scenario where peers diverge for MULTIPLE save cycles.
///
/// This is the core use case: Alice and Bob start from a common ancestor,
/// then each works offline making multiple changes across multiple saves.
/// When they finally sync, their ops_logs contain many accumulated operations.
///
/// Timeline:
/// ```text
/// v0 (common ancestor - empty bucket)
///  |
///  +-- Alice's branch:
///  |   v1: adds README.md, creates src/
///  |   v2: adds src/main.rs, src/utils.rs
///  |   v3: adds config.toml, modifies README.md
///  |
///  +-- Bob's branch:
///      v1': adds CONTRIBUTING.md, creates src/
///      v2': adds src/lib.rs, src/tests.rs
///      v3': adds config.toml (CONFLICT!), adds .gitignore
/// ```
#[tokio::test]
async fn scenario_multi_version_divergence() {
    use ::common::mount::Mount;

    // Setup: Create Alice's mount (this is v0 - empty bucket)
    let (mut alice_mount, blobs, alice_key, _temp) = common::setup_test_env().await;

    // Add Bob as an owner so he can load
    let bob_key = SecretKey::generate();
    alice_mount.add_owner(bob_key.public()).await.unwrap();

    // Save v0 - the common ancestor (empty bucket with just the structure)
    let (ancestor_link, _, _) = alice_mount.save(&blobs, false).await.unwrap();

    // Bob loads from v0 - same starting point
    let mut bob_mount = Mount::load(&ancestor_link, &bob_key, &blobs)
        .await
        .expect("Bob should load from ancestor");

    // =========================================================================
    // Alice's work session - multiple saves accumulating operations
    // =========================================================================

    // Alice v1: initial project setup
    alice_mount
        .add(
            &PathBuf::from("/README.md"),
            Cursor::new(b"# My Project\n\nAlice's README".to_vec()),
        )
        .await
        .unwrap();
    alice_mount.mkdir(&PathBuf::from("/src")).await.unwrap();
    let (_alice_v1, _, _) = alice_mount.save(&blobs, false).await.unwrap();

    // Alice v2: add source files
    alice_mount
        .add(
            &PathBuf::from("/src/main.rs"),
            Cursor::new(b"fn main() { println!(\"Alice\"); }".to_vec()),
        )
        .await
        .unwrap();
    alice_mount
        .add(
            &PathBuf::from("/src/utils.rs"),
            Cursor::new(b"pub fn helper() {}".to_vec()),
        )
        .await
        .unwrap();
    let (_alice_v2, _, _) = alice_mount.save(&blobs, false).await.unwrap();

    // Alice v3: add config and update README
    alice_mount
        .add(
            &PathBuf::from("/config.toml"),
            Cursor::new(b"[alice]\nowner = true".to_vec()),
        )
        .await
        .unwrap();
    alice_mount.rm(&PathBuf::from("/README.md")).await.unwrap();
    alice_mount
        .add(
            &PathBuf::from("/README.md"),
            Cursor::new(b"# My Project v2\n\nUpdated by Alice".to_vec()),
        )
        .await
        .unwrap();
    let (_alice_v3, _, _) = alice_mount.save(&blobs, false).await.unwrap();

    // =========================================================================
    // Bob's work session - also multiple saves, diverging from same v0
    // =========================================================================

    // Bob v1': different initial setup
    bob_mount
        .add(
            &PathBuf::from("/CONTRIBUTING.md"),
            Cursor::new(b"# Contributing\n\nBob's guidelines".to_vec()),
        )
        .await
        .unwrap();
    bob_mount.mkdir(&PathBuf::from("/src")).await.unwrap(); // Same dir as Alice - idempotent
    let (_bob_v1, _, _) = bob_mount.save(&blobs, false).await.unwrap();

    // Bob v2': add his source files
    bob_mount
        .add(
            &PathBuf::from("/src/lib.rs"),
            Cursor::new(b"pub mod tests;".to_vec()),
        )
        .await
        .unwrap();
    bob_mount
        .add(
            &PathBuf::from("/src/tests.rs"),
            Cursor::new(b"#[test] fn it_works() {}".to_vec()),
        )
        .await
        .unwrap();
    let (_bob_v2, _, _) = bob_mount.save(&blobs, false).await.unwrap();

    // Bob v3': add config (CONFLICT with Alice!) and .gitignore
    bob_mount
        .add(
            &PathBuf::from("/config.toml"),
            Cursor::new(b"[bob]\ncontributor = true".to_vec()),
        )
        .await
        .unwrap();
    bob_mount
        .add(
            &PathBuf::from("/.gitignore"),
            Cursor::new(b"target/\n*.log".to_vec()),
        )
        .await
        .unwrap();
    let (_bob_v3, _, _) = bob_mount.save(&blobs, false).await.unwrap();

    // =========================================================================
    // Now merge - both have accumulated 3 versions worth of operations
    // =========================================================================

    let alice_ops_log = alice_mount.inner().await.ops_log().clone();
    let bob_ops_log = bob_mount.inner().await.ops_log().clone();

    // Verify we have substantial operation counts before merge
    assert!(
        alice_ops_log.len() >= 6,
        "Alice should have at least 6 ops (README, mkdir, main.rs, utils.rs, config, README update), got {}",
        alice_ops_log.len()
    );
    assert!(
        bob_ops_log.len() >= 6,
        "Bob should have at least 6 ops, got {}",
        bob_ops_log.len()
    );

    // Merge with ConflictFile resolver
    let resolver = ConflictFile::new();
    let local_peer = alice_key.public();
    let (merged_log, results) = merge_logs(&[&alice_ops_log, &bob_ops_log], &resolver, &local_peer);

    // =========================================================================
    // Verify the merge results
    // =========================================================================

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Should have conflicts:
    // - config.toml: Alice and Bob both added (CONFLICT -> creates conflict file)
    // - src/: Both created, but mkdir is idempotent (no conflict)
    let config_conflicts: Vec<_> = result
        .conflicts_resolved
        .iter()
        .filter(|c| c.conflict.path.to_string_lossy().contains("config"))
        .collect();
    assert_eq!(
        config_conflicts.len(),
        1,
        "Should have exactly one config.toml conflict"
    );

    // Verify final state has all files from both branches
    let final_state = merged_log.resolve_all();

    // Alice's files
    assert!(
        final_state.contains_key(&PathBuf::from("README.md")),
        "Alice's README.md should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/main.rs")),
        "Alice's src/main.rs should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/utils.rs")),
        "Alice's src/utils.rs should exist"
    );

    // Bob's files
    assert!(
        final_state.contains_key(&PathBuf::from("CONTRIBUTING.md")),
        "Bob's CONTRIBUTING.md should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/lib.rs")),
        "Bob's src/lib.rs should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/tests.rs")),
        "Bob's src/tests.rs should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from(".gitignore")),
        "Bob's .gitignore should exist"
    );

    // Shared directory (idempotent)
    assert!(
        final_state.contains_key(&PathBuf::from("src")),
        "src/ directory should exist"
    );

    // config.toml - Alice's version at original path
    assert!(
        final_state.contains_key(&PathBuf::from("config.toml")),
        "Original config.toml should exist"
    );

    // Bob's config.toml as conflict file
    let conflict_configs: Vec<_> = final_state
        .keys()
        .filter(|p| {
            let s = p.to_string_lossy();
            s.starts_with("config@") && s.ends_with(".toml")
        })
        .collect();
    assert_eq!(
        conflict_configs.len(),
        1,
        "Should have one config conflict file for Bob's version"
    );

    // Total file count: Alice (4) + Bob (4) + shared src (1) + conflict file (1) = depends on removes
    // Actually: README, src, main.rs, utils.rs, config.toml from Alice
    //           CONTRIBUTING, src (idempotent), lib.rs, tests.rs, .gitignore, config@hash.toml from Bob
    // Minus the README remove/re-add (still 1 README in final state)
    assert!(
        final_state.len() >= 10,
        "Should have at least 10 resolved paths, got {}",
        final_state.len()
    );
}

// =============================================================================
// SCENARIO: Simple divergent work (single-version, for comparison)
// =============================================================================

/// Simpler scenario for comparison - single save on each side.
#[tokio::test]
async fn scenario_single_version_divergent_work() {
    let alice = make_peer_id(1);
    let bob = make_peer_id(2);

    let make_link = |seed: u8| {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[0] = seed;
        let hash = iroh_blobs::Hash::from_bytes(hash_bytes);
        ::common::linked_data::Link::new(::common::linked_data::LD_RAW_CODEC, hash)
    };

    // Alice's single work session
    let mut alice_log = PathOpLog::new();
    alice_log.record(alice, OpType::Mkdir, "src", None, true);
    alice_log.record(
        alice,
        OpType::Add,
        "src/main.rs",
        Some(make_link(0x01)),
        false,
    );
    alice_log.record(
        alice,
        OpType::Add,
        "config.toml",
        Some(make_link(0xAA)),
        false,
    );

    // Bob's single work session
    let mut bob_log = PathOpLog::new();
    bob_log.record(bob, OpType::Mkdir, "src", None, true);
    bob_log.record(bob, OpType::Add, "src/lib.rs", Some(make_link(0x03)), false);
    bob_log.record(
        bob,
        OpType::Add,
        "config.toml",
        Some(make_link(0xBB)),
        false,
    );

    // Merge
    let resolver = ConflictFile::new();
    let (merged, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice);

    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].conflicts_resolved.len(),
        1,
        "Only config.toml should conflict"
    );

    let final_state = merged.resolve_all();
    assert!(final_state.contains_key(&PathBuf::from("src")));
    assert!(final_state.contains_key(&PathBuf::from("src/main.rs")));
    assert!(final_state.contains_key(&PathBuf::from("src/lib.rs")));
    assert!(final_state.contains_key(&PathBuf::from("config.toml")));
}
