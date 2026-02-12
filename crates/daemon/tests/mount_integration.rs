//! Integration tests for FUSE mount management
//!
//! These tests verify the mount configuration and lifecycle management without
//! requiring actual FUSE mounting (which requires privileges).

#![cfg(feature = "fuse")]

use tempfile::TempDir;
use uuid::Uuid;

use jax_daemon::mount_queries::{CreateMountConfig, MountStatus, UpdateMountConfig};
use jax_daemon::Database;

/// Create a test database
async fn setup_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create an empty db file
    std::fs::File::create(&db_path).unwrap();

    let db_url = format!("sqlite://{}", db_path.display());
    let db = Database::connect(&url::Url::parse(&db_url).unwrap())
        .await
        .unwrap();

    (db, temp_dir)
}

#[tokio::test]
async fn test_create_and_get_mount() {
    let (db, temp_dir) = setup_test_db().await;
    let mount_point = temp_dir.path().join("mount").to_string_lossy().to_string();
    std::fs::create_dir_all(&mount_point).unwrap();

    // Create a bucket first (mounts require a bucket)
    let bucket_id = Uuid::new_v4();

    let config = CreateMountConfig {
        bucket_id,
        mount_point: mount_point.clone(),
        auto_mount: false,
        read_only: false,
        cache_size_mb: Some(50),
        cache_ttl_secs: Some(30),
    };

    let mount = db.create_mount(&config).await.unwrap();

    assert_eq!(mount.bucket_id, bucket_id);
    assert_eq!(mount.mount_point, mount_point);
    assert!(!mount.auto_mount);
    assert!(!mount.read_only);
    assert_eq!(mount.cache_size_mb, 50);
    assert_eq!(mount.cache_ttl_secs, 30);
    assert_eq!(mount.status, MountStatus::Stopped);
    assert!(mount.enabled);

    // Get the mount by ID
    let retrieved = db.get_mount(&mount.mount_id).await.unwrap().unwrap();
    assert_eq!(retrieved.mount_id, mount.mount_id);
    assert_eq!(retrieved.bucket_id, bucket_id);
}

#[tokio::test]
async fn test_list_mounts() {
    let (db, temp_dir) = setup_test_db().await;

    // Create multiple mounts
    for i in 0..3 {
        let mount_point = temp_dir
            .path()
            .join(format!("mount{}", i))
            .to_string_lossy()
            .to_string();
        std::fs::create_dir_all(&mount_point).unwrap();

        let config = CreateMountConfig {
            bucket_id: Uuid::new_v4(),
            mount_point,
            auto_mount: i == 1, // Only middle one is auto-mount
            read_only: false,
            cache_size_mb: None,
            cache_ttl_secs: None,
        };
        db.create_mount(&config).await.unwrap();
    }

    let mounts = db.list_mounts().await.unwrap();
    assert_eq!(mounts.len(), 3);
}

#[tokio::test]
async fn test_update_mount() {
    let (db, temp_dir) = setup_test_db().await;
    let mount_point = temp_dir.path().join("mount").to_string_lossy().to_string();
    std::fs::create_dir_all(&mount_point).unwrap();

    let config = CreateMountConfig {
        bucket_id: Uuid::new_v4(),
        mount_point,
        auto_mount: false,
        read_only: false,
        cache_size_mb: None,
        cache_ttl_secs: None,
    };

    let mount = db.create_mount(&config).await.unwrap();
    assert!(!mount.auto_mount);
    assert!(!mount.read_only);

    // Update the mount
    let update = UpdateMountConfig {
        mount_point: None,
        enabled: Some(false),
        auto_mount: Some(true),
        read_only: Some(true),
        cache_size_mb: Some(200),
        cache_ttl_secs: Some(120),
    };

    let updated = db
        .update_mount(&mount.mount_id, &update)
        .await
        .unwrap()
        .unwrap();
    assert!(!updated.enabled);
    assert!(updated.auto_mount);
    assert!(updated.read_only);
    assert_eq!(updated.cache_size_mb, 200);
    assert_eq!(updated.cache_ttl_secs, 120);
}

#[tokio::test]
async fn test_delete_mount() {
    let (db, temp_dir) = setup_test_db().await;
    let mount_point = temp_dir.path().join("mount").to_string_lossy().to_string();
    std::fs::create_dir_all(&mount_point).unwrap();

    let config = CreateMountConfig {
        bucket_id: Uuid::new_v4(),
        mount_point,
        auto_mount: false,
        read_only: false,
        cache_size_mb: None,
        cache_ttl_secs: None,
    };

    let mount = db.create_mount(&config).await.unwrap();

    // Verify mount exists
    assert!(db.get_mount(&mount.mount_id).await.unwrap().is_some());

    // Delete the mount
    let deleted = db.delete_mount(&mount.mount_id).await.unwrap();
    assert!(deleted);

    // Verify mount is gone
    assert!(db.get_mount(&mount.mount_id).await.unwrap().is_none());

    // Deleting again should return false
    let deleted_again = db.delete_mount(&mount.mount_id).await.unwrap();
    assert!(!deleted_again);
}

#[tokio::test]
async fn test_update_mount_status() {
    let (db, temp_dir) = setup_test_db().await;
    let mount_point = temp_dir.path().join("mount").to_string_lossy().to_string();
    std::fs::create_dir_all(&mount_point).unwrap();

    let config = CreateMountConfig {
        bucket_id: Uuid::new_v4(),
        mount_point,
        auto_mount: false,
        read_only: false,
        cache_size_mb: None,
        cache_ttl_secs: None,
    };

    let mount = db.create_mount(&config).await.unwrap();
    assert_eq!(mount.status, MountStatus::Stopped);

    // Update to starting
    db.update_mount_status(&mount.mount_id, MountStatus::Starting, None)
        .await
        .unwrap();
    let mount = db.get_mount(&mount.mount_id).await.unwrap().unwrap();
    assert_eq!(mount.status, MountStatus::Starting);
    assert!(mount.error_message.is_none());

    // Update to running
    db.update_mount_status(&mount.mount_id, MountStatus::Running, None)
        .await
        .unwrap();
    let mount = db.get_mount(&mount.mount_id).await.unwrap().unwrap();
    assert_eq!(mount.status, MountStatus::Running);

    // Update to error with message
    db.update_mount_status(&mount.mount_id, MountStatus::Error, Some("Test error"))
        .await
        .unwrap();
    let mount = db.get_mount(&mount.mount_id).await.unwrap().unwrap();
    assert_eq!(mount.status, MountStatus::Error);
    assert_eq!(mount.error_message.as_deref(), Some("Test error"));
}

#[tokio::test]
async fn test_get_auto_mount_list() {
    let (db, temp_dir) = setup_test_db().await;

    // Create mounts with different auto_mount settings
    for (i, (auto_mount, enabled)) in [(true, true), (true, false), (false, true), (false, false)]
        .iter()
        .enumerate()
    {
        let mount_point = temp_dir
            .path()
            .join(format!("mount{}", i))
            .to_string_lossy()
            .to_string();
        std::fs::create_dir_all(&mount_point).unwrap();

        let config = CreateMountConfig {
            bucket_id: Uuid::new_v4(),
            mount_point,
            auto_mount: *auto_mount,
            read_only: false,
            cache_size_mb: None,
            cache_ttl_secs: None,
        };

        let mount = db.create_mount(&config).await.unwrap();

        // Disable the mount if needed
        if !enabled {
            db.update_mount(
                &mount.mount_id,
                &UpdateMountConfig {
                    enabled: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
    }

    // Only mounts with auto_mount=true AND enabled=true should be returned
    let auto_mounts = db.get_auto_mount_list().await.unwrap();
    assert_eq!(auto_mounts.len(), 1);
    assert!(auto_mounts[0].auto_mount);
    assert!(auto_mounts[0].enabled);
}

#[tokio::test]
async fn test_get_mounts_by_bucket() {
    let (db, temp_dir) = setup_test_db().await;
    let bucket_id = Uuid::new_v4();

    // Create multiple mounts for the same bucket
    for i in 0..2 {
        let mount_point = temp_dir
            .path()
            .join(format!("mount_b1_{}", i))
            .to_string_lossy()
            .to_string();
        std::fs::create_dir_all(&mount_point).unwrap();

        let config = CreateMountConfig {
            bucket_id,
            mount_point,
            auto_mount: false,
            read_only: false,
            cache_size_mb: None,
            cache_ttl_secs: None,
        };
        db.create_mount(&config).await.unwrap();
    }

    // Create a mount for a different bucket
    let other_bucket = Uuid::new_v4();
    let other_mount_point = temp_dir
        .path()
        .join("mount_other")
        .to_string_lossy()
        .to_string();
    std::fs::create_dir_all(&other_mount_point).unwrap();

    let config = CreateMountConfig {
        bucket_id: other_bucket,
        mount_point: other_mount_point,
        auto_mount: false,
        read_only: false,
        cache_size_mb: None,
        cache_ttl_secs: None,
    };
    db.create_mount(&config).await.unwrap();

    // Get mounts by bucket
    let bucket_mounts = db.get_mounts_by_bucket(&bucket_id).await.unwrap();
    assert_eq!(bucket_mounts.len(), 2);
    for mount in &bucket_mounts {
        assert_eq!(mount.bucket_id, bucket_id);
    }

    let other_mounts = db.get_mounts_by_bucket(&other_bucket).await.unwrap();
    assert_eq!(other_mounts.len(), 1);
    assert_eq!(other_mounts[0].bucket_id, other_bucket);
}
