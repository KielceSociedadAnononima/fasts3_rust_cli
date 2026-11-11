use fasts3_rust_cli::*;

/// Minimal valid 1x1 RGB PNG image bytes for real server thumbnail generation testing.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[tokio::test]
async fn test_live_fasts3_server_end_to_end() {
    let Ok(live_url) = std::env::var("FASTS3_LIVE_URL") else {
        return; // Skipped when no live server is configured
    };
    let boot_token =
        std::env::var("FASTS3_LIVE_BOOTSTRAP_TOKEN").expect("FASTS3_LIVE_BOOTSTRAP_TOKEN required");

    let temp_root = std::env::temp_dir().join(format!("fasts3_rust_live_e2e_{}", std::process::id()));
    let extra_disk_dir = temp_root.join("disk_rust");
    tokio::fs::create_dir_all(&extra_disk_dir).await.unwrap();

    let admin = FastS3Client::new(&live_url).unwrap();

    // 1. Bootstrap status + bootstrap login + setup + login + change password + verify
    let _status = admin.bootstrap_status().await.unwrap();
    let boot = admin
        .bootstrap_login_and_authenticate(&boot_token)
        .await
        .unwrap();
    assert!(boot.is_admin);

    let setup = admin
        .bootstrap_setup_and_authenticate(&BootstrapSetupRequest {
            token: boot_token.clone(),
            username: "rust_admin".to_string(),
            password: "RustPass123!".to_string(),
        })
        .await
        .unwrap();
    assert!(setup.is_admin);

    admin
        .login_and_authenticate("rust_admin", "RustPass123!")
        .await
        .unwrap();
    admin
        .change_password_and_authenticate(Some("RustPass123!"), "RustPass456!")
        .await
        .unwrap();

    let verified = admin.verify_auth().await.unwrap();
    assert!(verified.is_admin);
    assert_eq!(verified.username, "rust_admin");

    let fetched_boot = admin.get_bootstrap_token().await.unwrap();
    assert_eq!(fetched_boot.as_deref(), Some(boot_token.as_str()));

    // 2. Multi-disk + Bucket lifecycle
    let added_disk = admin
        .add_disk(
            &CreateDiskRequest::new("Rust Secondary Disk", extra_disk_dir.to_string_lossy())
                .with_id("rust_disk"),
        )
        .await
        .unwrap();
    assert_eq!(added_disk.id, "rust_disk");
    assert!(admin.list_disks().await.unwrap().len() >= 2);

    admin
        .create_bucket(
            &CreateBucketRequest::new("rust-e2e-bkt")
                .public(false)
                .disk_id("rust_disk")
                .quota(50 * 1024 * 1024),
        )
        .await
        .unwrap();

    let bkt = admin
        .get_bucket_by_name("rust-e2e-bkt")
        .await
        .unwrap()
        .unwrap();
    let mut upd = UpdateBucketRequest::from(&bkt);
    upd.quota = 60 * 1024 * 1024;
    admin.update_bucket(bkt.id, &upd).await.unwrap();

    // 3. User, Permissions, and API Key RBAC
    let worker = admin
        .create_user(&CreateUserRequest::new("rust_worker", "Worker123!", false))
        .await
        .unwrap();
    admin
        .update_user(
            worker.id,
            &UpdateUserRequest {
                password: Some("Worker456!".to_string()),
                is_admin: Some(false),
            },
        )
        .await
        .unwrap();

    let perm = admin
        .set_permission(&SetPermissionRequest {
            user_id: worker.id,
            bucket_name: "rust-e2e-bkt".to_string(),
            can_read: true,
            can_write: true,
            can_delete: true,
        })
        .await
        .unwrap();
    assert!(perm.can_write);
    assert!(!admin.list_permissions().await.unwrap().is_empty());
    assert_eq!(
        admin
            .get_bucket_permissions("rust-e2e-bkt")
            .await
            .unwrap()
            .len(),
        1
    );

    let created_key = admin
        .create_api_key("Rust Worker Key", Some(worker.id))
        .await
        .unwrap();

    // Use worker API Key client for file operations
    let worker_cli = FastS3Client::with_api_key(&live_url, &created_key.api_key).unwrap();
    let worker_verify = worker_cli.verify_auth().await.unwrap();
    assert!(!worker_verify.is_admin);
    assert_eq!(worker_verify.permissions.len(), 1);

    // 4. Upload PNG, JSON upload, directory upload, thumbnails, downloads, URL auth
    worker_cli
        .upload_bytes("rust-e2e-bkt", "images/dot.png", TINY_PNG, Some("image/png"))
        .await
        .unwrap();

    worker_cli
        .upload_json_bytes(
            "rust-e2e-bkt",
            "notes/hello world.txt",
            Some("hello world.txt"),
            b"Live Rust SDK",
        )
        .await
        .unwrap();

    let local_sync_src = temp_root.join("sync_src");
    tokio::fs::create_dir_all(local_sync_src.join("nested"))
        .await
        .unwrap();
    tokio::fs::write(local_sync_src.join("nested/item.txt"), b"Synced item")
        .await
        .unwrap();
    let synced_keys = worker_cli
        .upload_directory("rust-e2e-bkt", Some("batch"), &local_sync_src)
        .await
        .unwrap();
    assert_eq!(synced_keys, vec!["batch/nested/item.txt"]);

    let details = worker_cli
        .list_file_details("rust-e2e-bkt", None)
        .await
        .unwrap();
    assert_eq!(details.len(), 3);
    assert!(details.iter().any(|d| d.key == "images/dot.png" && d.is_image));

    let dl = worker_cli
        .download_file("rust-e2e-bkt", "notes/hello world.txt")
        .await
        .unwrap();
    assert_eq!(&dl.data[..], b"Live Rust SDK");

    let thumb_opts = ThumbnailOptions::new(64, 64);
    let thumb1 = worker_cli
        .get_thumbnail("rust-e2e-bkt", "images/dot.png", Some(&thumb_opts))
        .await
        .unwrap();
    assert_eq!(thumb1.content_type, "image/png");
    assert!(!thumb1.data.is_empty());

    let thumb2 = worker_cli
        .get_thumbnail_alt("rust-e2e-bkt", "images/dot.png", Some(&thumb_opts))
        .await
        .unwrap();
    assert!(!thumb2.data.is_empty());

    let dl_thumb = worker_cli
        .download_with_options(
            "rust-e2e-bkt",
            "images/dot.png",
            &DownloadOptions {
                thumbnail: true,
                width: Some(32),
                height: Some(32),
            },
        )
        .await
        .unwrap();
    assert_eq!(dl_thumb.content_type, "image/png");

    // Verify direct query-authenticated URL works in an unauthenticated HTTP GET
    let direct_url = worker_cli
        .file_url("rust-e2e-bkt", "notes/hello world.txt", true)
        .unwrap();
    let direct_resp = reqwest::get(&direct_url).await.unwrap();
    assert!(direct_resp.status().is_success());
    assert_eq!(direct_resp.bytes().await.unwrap().as_ref(), b"Live Rust SDK");

    let pulled_dir = temp_root.join("pulled");
    let pulled = worker_cli
        .download_prefix_to_dir("rust-e2e-bkt", Some("batch"), &pulled_dir)
        .await
        .unwrap();
    assert_eq!(pulled.len(), 1);

    // 5. Cleanup & AutoVacuum
    assert_eq!(worker_cli.delete_prefix("rust-e2e-bkt", "batch").await.unwrap(), 1);
    worker_cli
        .delete_file("rust-e2e-bkt", "notes/hello world.txt")
        .await
        .unwrap();
    worker_cli
        .delete_file("rust-e2e-bkt", "images/dot.png")
        .await
        .unwrap();

    let keys = admin.list_api_keys().await.unwrap();
    assert!(keys.iter().any(|k| k.id == created_key.id && k.last_used_at.is_some()));
    admin.delete_api_key(created_key.id).await.unwrap();

    admin
        .delete_bucket_permission("rust-e2e-bkt", worker.id)
        .await
        .unwrap();
    admin.delete_user(worker.id).await.unwrap();

    let vac = admin.trigger_vacuum().await.unwrap();
    assert!(vac.buckets_reconciled >= 1);
    let _vac_st = admin.vacuum_status().await.unwrap();

    admin.delete_bucket_by_name("rust-e2e-bkt").await.unwrap();
    admin.delete_disk("rust_disk").await.unwrap();

    let _ = tokio::fs::remove_dir_all(&temp_root).await;
}
