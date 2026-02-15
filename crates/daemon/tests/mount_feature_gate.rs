//! Emit a warning when FUSE-dependent tests are skipped.
//!
//! The actual FUSE mount tests live in `mount_integration.rs` and are gated
//! behind `#[cfg(feature = "fuse")]`. This companion file ensures a visible
//! message is emitted when running `cargo test` without the `fuse` feature,
//! so the missing coverage is not silently hidden.

#[cfg(not(feature = "fuse"))]
#[test]
fn fuse_tests_skipped_warning() {
    eprintln!(
        "\n\x1b[33mwarning\x1b[0m: Skipping FUSE mount tests: built without `fuse` feature.\n\
         To run FUSE tests, build with: cargo test --features fuse\n"
    );
}
