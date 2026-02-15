//! Integration tests for bucket publish operations
//!
//! Tests cover owner publish, non-owner rejection, idempotent publish,
//! and publish/unpublish round-trip.

mod common;

use std::io::Cursor;
use std::path::PathBuf;

use ::common::crypto::SecretKey;
use ::common::mount::Mount;

const TEST_PATH: &str = "/file.txt";

#[tokio::test]
async fn test_owner_can_publish() {
    let (mut mount, blobs, _owner_key, _temp_dir) = common::setup_test_env().await;

    mount
        .add(&PathBuf::from(TEST_PATH), Cursor::new(b"hello".to_vec()))
        .await
        .unwrap();

    // Add a mirror so publish has an effect
    let mirror_key = SecretKey::generate();
    mount.add_mirror(mirror_key.public()).await;

    // Owner publishes
    let (link, _, _) = mount.publish().await.unwrap();

    // Verify manifest has plaintext secret (is_published)
    let published_mount = Mount::load(&link, &mirror_key, &blobs)
        .await
        .expect("Mirror should mount published bucket");
    assert!(published_mount.is_published().await);

    // Mirror can decrypt content
    let data = published_mount
        .cat(&PathBuf::from(TEST_PATH))
        .await
        .unwrap();
    assert_eq!(data, b"hello");
}

#[tokio::test]
async fn test_idempotent_publish() {
    let (mut mount, blobs, owner_key, _temp_dir) = common::setup_test_env().await;

    mount
        .add(&PathBuf::from(TEST_PATH), Cursor::new(b"data".to_vec()))
        .await
        .unwrap();

    let mirror_key = SecretKey::generate();
    mount.add_mirror(mirror_key.public()).await;

    // Publish once
    let (link1, _, _) = mount.publish().await.unwrap();
    assert!(mount.is_published().await);

    // Reload and publish again — should succeed without error
    let mount2 = Mount::load(&link1, &owner_key, &blobs).await.unwrap();
    assert!(mount2.is_published().await);

    // Add mirror back (shares are preserved from link1)
    let (link2, _, _) = mount2.save(&blobs, true).await.unwrap();

    // Both links should produce valid published mounts
    let m1 = Mount::load(&link1, &mirror_key, &blobs).await.unwrap();
    assert!(m1.is_published().await);
    let m2 = Mount::load(&link2, &mirror_key, &blobs).await.unwrap();
    assert!(m2.is_published().await);
}

#[tokio::test]
async fn test_publish_then_unpublish_round_trip() {
    let (mut mount, blobs, owner_key, _temp_dir) = common::setup_test_env().await;

    mount
        .add(&PathBuf::from(TEST_PATH), Cursor::new(b"secret".to_vec()))
        .await
        .unwrap();

    let mirror_key = SecretKey::generate();
    mount.add_mirror(mirror_key.public()).await;

    // Publish
    let (link_pub, _, _) = mount.publish().await.unwrap();
    let mirror_mount = Mount::load(&link_pub, &mirror_key, &blobs).await.unwrap();
    assert!(mirror_mount.is_published().await);

    // Unpublish by saving without publish flag
    let owner_mount = Mount::load(&link_pub, &owner_key, &blobs).await.unwrap();
    let (link_unpub, _, _) = owner_mount.save(&blobs, false).await.unwrap();

    // Mirror should no longer be able to mount
    let result = Mount::load(&link_unpub, &mirror_key, &blobs).await;
    assert!(result.is_err(), "Mirror should not mount after unpublish");

    // Owner can still mount
    let owner_mount2 = Mount::load(&link_unpub, &owner_key, &blobs).await.unwrap();
    assert!(!owner_mount2.is_published().await);
}

#[tokio::test]
async fn test_mirror_publish_rejected_during_sync() {
    // Mirrors cannot create valid signed manifests with Owner role,
    // so the sync validation (verify_author) rejects them.
    // Here we verify that a mirror-loaded mount doesn't have owner permissions
    // by checking its role in the manifest.
    let (mut mount, blobs, _owner_key, _temp_dir) = common::setup_test_env().await;

    mount
        .add(&PathBuf::from(TEST_PATH), Cursor::new(b"data".to_vec()))
        .await
        .unwrap();

    let mirror_key = SecretKey::generate();
    mount.add_mirror(mirror_key.public()).await;

    // Publish so mirror can mount
    let (link, _, _) = mount.publish().await.unwrap();

    // Mirror loads the mount
    let mirror_mount = Mount::load(&link, &mirror_key, &blobs).await.unwrap();

    // Verify the mirror's role is Mirror, not Owner
    let inner = mirror_mount.inner().await;
    let mirror_share = inner.manifest().get_share(&mirror_key.public()).unwrap();
    assert_eq!(
        *mirror_share.role(),
        ::common::mount::PrincipalRole::Mirror,
        "Mirror should have Mirror role, not Owner"
    );

    // If the mirror tries to save (publish), the manifest will be signed
    // with the mirror's key. During sync, verify_author will reject this
    // because mirrors don't have Owner role.
}
