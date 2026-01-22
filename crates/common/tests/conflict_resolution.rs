//! Integration tests for conflict resolution on divergent mounts
//!
//! These tests use actual Mount instances with blob storage to demonstrate
//! real-world conflict resolution scenarios. For PathOpLog unit tests,
//! see `src/mount/path_ops.rs`.
//!
//! # Philosophy
//!
//! Tests are written to read like plain English. Each test tells a story:
//! - WHO: Alice, Bob, Carol (peers)
//! - WHAT: creates files, makes changes, diverges
//! - WHEN: over multiple save cycles
//! - OUTCOME: conflicts detected and resolved
//!
//! # The Key Scenario
//!
//! Two peers diverge from a common ancestor for multiple save cycles,
//! then need to merge their ops_logs when syncing. This mirrors real-world
//! P2P sync where a peer discovers they're behind a longer canonical chain.

mod common;

use std::io::Cursor;
use std::path::PathBuf;

use ::common::crypto::SecretKey;
use ::common::mount::{merge_logs, ConflictFile, Mount, OpType, Resolution};
use ::common::peer::BlobsStore;
use tempfile::TempDir;
use uuid::Uuid;

// =============================================================================
// TEST SETUP HELPERS
// =============================================================================

/// A peer in our test scenarios with their own mount and identity
struct Peer {
    name: &'static str,
    mount: Mount,
    key: SecretKey,
}

impl Peer {
    /// Get the peer's public key for identification
    fn public_key(&self) -> ::common::crypto::PublicKey {
        self.key.public()
    }
}

/// Test scenario with shared infrastructure
struct TestScenario {
    blobs: BlobsStore,
    _temp: TempDir,
}

impl TestScenario {
    /// Create a new test scenario with blob storage
    async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let blob_path = temp_dir.path().join("blobs");
        let blobs = BlobsStore::fs(&blob_path).await.unwrap();
        Self {
            blobs,
            _temp: temp_dir,
        }
    }

    /// Create a new peer with their own mount
    async fn create_peer(&self, name: &'static str) -> Peer {
        let key = SecretKey::generate();
        let mount = Mount::init(Uuid::new_v4(), name.to_string(), &key, &self.blobs)
            .await
            .expect("Failed to create mount");
        Peer { name, mount, key }
    }

    /// Fork a peer's mount to create a second peer starting from the same state
    ///
    /// This simulates the real-world scenario where:
    /// 1. Alice has a bucket
    /// 2. Alice adds Bob as an owner
    /// 3. Alice saves (creating the "common ancestor")
    /// 4. Bob loads from that same saved state
    /// 5. Both now diverge from the same point
    async fn fork_peer(&self, original: &mut Peer, new_name: &'static str) -> Peer {
        // Create Bob's key
        let new_key = SecretKey::generate();

        // Alice adds Bob as an owner so he can load the mount
        original
            .mount
            .add_owner(new_key.public())
            .await
            .expect("Failed to add owner");

        // Save to create the common ancestor
        let (ancestor_link, _, _) = original
            .mount
            .save(&self.blobs, false)
            .await
            .expect("Failed to save ancestor");

        // Bob loads from the same ancestor - now both start from identical state
        let new_mount = Mount::load(&ancestor_link, &new_key, &self.blobs)
            .await
            .expect("Failed to load from ancestor");

        Peer {
            name: new_name,
            mount: new_mount,
            key: new_key,
        }
    }
}

// =============================================================================
// FILE OPERATION HELPERS (makes test code read like English)
// =============================================================================

impl Peer {
    /// Add a file with the given content
    async fn creates_file(&mut self, path: &str, content: &str) {
        self.mount
            .add(&PathBuf::from(path), Cursor::new(content.as_bytes().to_vec()))
            .await
            .unwrap_or_else(|e| panic!("{} failed to create {}: {}", self.name, path, e));
    }

    /// Create a directory
    async fn creates_directory(&mut self, path: &str) {
        self.mount
            .mkdir(&PathBuf::from(path))
            .await
            .unwrap_or_else(|e| panic!("{} failed to mkdir {}: {}", self.name, path, e));
    }

    /// Remove a file
    async fn removes_file(&mut self, path: &str) {
        self.mount
            .rm(&PathBuf::from(path))
            .await
            .unwrap_or_else(|e| panic!("{} failed to remove {}: {}", self.name, path, e));
    }

    /// Save the current state (creates a new manifest version)
    async fn saves(&mut self, blobs: &BlobsStore) {
        self.mount
            .save(blobs, false)
            .await
            .unwrap_or_else(|e| panic!("{} failed to save: {}", self.name, e));
    }

    /// Get the ops log for merging
    async fn ops_log(&self) -> ::common::mount::PathOpLog {
        self.mount.inner().await.ops_log().clone()
    }
}

// =============================================================================
// SCENARIO: Two peers create the same file with different content
// =============================================================================

/// Alice and Bob both create "notes.txt" offline with different content.
///
/// When they sync:
/// - Alice's version stays at "notes.txt" (she's the local peer)
/// - Bob's version is renamed to "notes@<content-hash>.txt"
///
/// This is the simplest conflict scenario: single file, single save cycle.
#[tokio::test]
async fn scenario_alice_and_bob_both_create_notes_txt() {
    // Setup: Create test infrastructure
    let scenario = TestScenario::new().await;

    // Alice creates her bucket
    let mut alice = scenario.create_peer("alice").await;

    // Bob forks from Alice's bucket (they start from the same ancestor)
    let mut bob = scenario.fork_peer(&mut alice, "bob").await;

    // -------------------------------------------------------------------------
    // Alice and Bob both create "notes.txt" with different content (offline)
    // -------------------------------------------------------------------------

    alice
        .creates_file("/notes.txt", "Alice's notes content")
        .await;

    bob.creates_file("/notes.txt", "Bob's notes content").await;

    // -------------------------------------------------------------------------
    // When Alice syncs with Bob, she merges his ops_log into hers
    // -------------------------------------------------------------------------

    let alice_log = alice.ops_log().await;
    let bob_log = bob.ops_log().await;

    let resolver = ConflictFile::new();
    let (merged_log, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice.public_key());

    // -------------------------------------------------------------------------
    // VERIFY: One conflict detected and resolved
    // -------------------------------------------------------------------------

    assert_eq!(results.len(), 1, "Should have one merge result");
    let result = &results[0];

    assert_eq!(
        result.total_conflicts(),
        1,
        "Should detect one conflict (both created notes.txt)"
    );
    assert_eq!(
        result.conflicts_resolved.len(),
        1,
        "Conflict should be resolved"
    );

    // The resolution should rename Bob's file to include a content hash
    let resolved = &result.conflicts_resolved[0];
    match &resolved.resolution {
        Resolution::RenameIncoming { new_path } => {
            let name = new_path.to_string_lossy();
            assert!(
                name.starts_with("notes@"),
                "Expected notes@<hash>.txt, got {}",
                name
            );
            assert!(name.ends_with(".txt"), "Expected .txt extension, got {}", name);
        }
        other => panic!("Expected RenameIncoming, got {:?}", other),
    }

    // -------------------------------------------------------------------------
    // VERIFY: Both files exist in the merged state
    // -------------------------------------------------------------------------

    let final_state = merged_log.resolve_all();

    assert!(
        final_state.contains_key(&PathBuf::from("notes.txt")),
        "Alice's notes.txt should exist at original path"
    );

    let add_ops: Vec<_> = merged_log
        .ops_in_order()
        .filter(|op| matches!(op.op_type, OpType::Add))
        .collect();
    assert_eq!(
        add_ops.len(),
        2,
        "Should have 2 Add operations (Alice's + Bob's renamed)"
    );
}

// =============================================================================
// SCENARIO: Multi-version divergence (the realistic case)
// =============================================================================

/// The realistic scenario: peers diverge for MULTIPLE save cycles before syncing.
///
/// This simulates the DAG-like manifest chain where a peer discovers they're
/// behind a longer canonical chain and need to merge all accumulated operations.
///
/// Timeline:
/// ```text
/// v0 (common ancestor - empty bucket)
///  │
///  ├── Alice's chain (local):
///  │   v1: adds README.md, creates src/
///  │   v2: adds src/main.rs, src/utils.rs
///  │   v3: adds config.toml, updates README.md
///  │
///  └── Bob's chain (remote canonical):
///      v1': adds CONTRIBUTING.md, creates src/
///      v2': adds src/lib.rs, src/tests.rs
///      v3': adds config.toml (CONFLICT!), adds .gitignore
/// ```
///
/// When Alice syncs, she merges Bob's chain into hers. The only conflict
/// is `config.toml` - everything else merges cleanly because the paths
/// don't overlap.
#[tokio::test]
async fn scenario_alice_and_bob_diverge_for_three_versions() {
    // Setup: Create test infrastructure
    let scenario = TestScenario::new().await;

    // Alice creates her bucket (this is v0 - empty)
    let mut alice = scenario.create_peer("alice").await;

    // Bob forks from Alice (they share the same v0 ancestor)
    let mut bob = scenario.fork_peer(&mut alice, "bob").await;

    // =========================================================================
    // ALICE'S CHAIN: Three versions of changes
    // =========================================================================

    // Version 1: Alice sets up the project
    alice
        .creates_file("/README.md", "# My Project\n\nAlice's README")
        .await;
    alice.creates_directory("/src").await;
    alice.saves(&scenario.blobs).await;

    // Version 2: Alice adds source files
    alice
        .creates_file("/src/main.rs", "fn main() { println!(\"Alice\"); }")
        .await;
    alice
        .creates_file("/src/utils.rs", "pub fn helper() {}")
        .await;
    alice.saves(&scenario.blobs).await;

    // Version 3: Alice adds config and updates README
    alice
        .creates_file("/config.toml", "[alice]\nowner = true")
        .await;
    alice.removes_file("/README.md").await;
    alice
        .creates_file("/README.md", "# My Project v2\n\nUpdated by Alice")
        .await;
    alice.saves(&scenario.blobs).await;

    // =========================================================================
    // BOB'S CHAIN: Three versions, diverging from the same v0
    // =========================================================================

    // Version 1': Bob sets up differently
    bob.creates_file("/CONTRIBUTING.md", "# Contributing\n\nBob's guidelines")
        .await;
    bob.creates_directory("/src").await; // Same dir as Alice - idempotent
    bob.saves(&scenario.blobs).await;

    // Version 2': Bob adds his source files
    bob.creates_file("/src/lib.rs", "pub mod tests;").await;
    bob.creates_file("/src/tests.rs", "#[test] fn it_works() {}")
        .await;
    bob.saves(&scenario.blobs).await;

    // Version 3': Bob adds config (CONFLICTS with Alice!) and .gitignore
    bob.creates_file("/config.toml", "[bob]\ncontributor = true")
        .await;
    bob.creates_file("/.gitignore", "target/\n*.log").await;
    bob.saves(&scenario.blobs).await;

    // =========================================================================
    // MERGE: Alice discovers Bob's chain and merges it
    // =========================================================================

    let alice_log = alice.ops_log().await;
    let bob_log = bob.ops_log().await;

    // Both should have substantial operations (3 versions each)
    assert!(
        alice_log.len() >= 6,
        "Alice should have at least 6 ops across 3 saves, got {}",
        alice_log.len()
    );
    assert!(
        bob_log.len() >= 6,
        "Bob should have at least 6 ops across 3 saves, got {}",
        bob_log.len()
    );

    let resolver = ConflictFile::new();
    let (merged_log, results) = merge_logs(&[&alice_log, &bob_log], &resolver, &alice.public_key());

    // =========================================================================
    // VERIFY: config.toml is the only conflict
    // =========================================================================

    assert_eq!(results.len(), 1, "Should have one merge result");
    let result = &results[0];

    let config_conflicts: Vec<_> = result
        .conflicts_resolved
        .iter()
        .filter(|c| c.conflict.path.to_string_lossy().contains("config"))
        .collect();
    assert_eq!(
        config_conflicts.len(),
        1,
        "config.toml should be the only conflict"
    );

    // =========================================================================
    // VERIFY: All files from both chains exist in merged state
    // =========================================================================

    let final_state = merged_log.resolve_all();

    // Alice's files
    assert!(
        final_state.contains_key(&PathBuf::from("README.md")),
        "Alice's README.md should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/main.rs")),
        "Alice's main.rs should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/utils.rs")),
        "Alice's utils.rs should exist"
    );

    // Bob's files
    assert!(
        final_state.contains_key(&PathBuf::from("CONTRIBUTING.md")),
        "Bob's CONTRIBUTING.md should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/lib.rs")),
        "Bob's lib.rs should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from("src/tests.rs")),
        "Bob's tests.rs should exist"
    );
    assert!(
        final_state.contains_key(&PathBuf::from(".gitignore")),
        "Bob's .gitignore should exist"
    );

    // Shared: src/ directory (created by both, idempotent)
    assert!(
        final_state.contains_key(&PathBuf::from("src")),
        "src/ directory should exist"
    );

    // config.toml conflict: Alice keeps original, Bob's is renamed
    assert!(
        final_state.contains_key(&PathBuf::from("config.toml")),
        "Alice's config.toml should exist at original path"
    );

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
        "Bob's config.toml should be renamed to config@<hash>.toml"
    );

    // Total: should have all the files
    assert!(
        final_state.len() >= 10,
        "Should have at least 10 paths in final state, got {}",
        final_state.len()
    );
}
