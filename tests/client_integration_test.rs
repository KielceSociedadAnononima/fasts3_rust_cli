use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fasts3_rust_cli::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    query: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct MockResponse {
    status: u16,
    content_type: String,
    extra_headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl MockResponse {
    fn json(status: u16, val: serde_json::Value) -> Self {
        Self {
            status,
            content_type: "application/json".to_string(),
            extra_headers: Vec::new(),
            body: serde_json::to_vec(&val).unwrap(),
        }
    }

    fn binary(
        status: u16,
        content_type: &str,
        body: Vec<u8>,
        extra_headers: Vec<(String, String)>,
    ) -> Self {
        Self {
            status,
            content_type: content_type.to_string(),
            extra_headers,
            body,
        }
    }
}

async fn read_http_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 2048];
    let header_end = loop {
        let n = stream.read(&mut tmp).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };

    let header_str = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = header_str.split("\r\n");
    let req_line = lines.next()?;
    let mut parts = req_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.to_string(), String::new()),
    };

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let mut body = buf[header_end..].to_vec();
    if let Some(te) = headers.get("transfer-encoding")
        && te.to_ascii_lowercase().contains("chunked")
    {
        // Decode HTTP/1.1 chunked body
        while !body.windows(5).any(|w| w == b"0\r\n\r\n") {
            let n = stream.read(&mut tmp).await.ok()?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&tmp[..n]);
        }
        let mut decoded = Vec::new();
        let mut idx = 0;
        while idx < body.len() {
            let Some(crlf) = body[idx..].windows(2).position(|w| w == b"\r\n") else {
                break;
            };
            let size_hex = String::from_utf8_lossy(&body[idx..idx + crlf]);
            let size = usize::from_str_radix(size_hex.trim(), 16).unwrap_or(0);
            idx += crlf + 2;
            if size == 0 {
                break;
            }
            decoded.extend_from_slice(&body[idx..idx + size]);
            idx += size + 2;
        }
        body = decoded;
    } else if let Some(cl) = headers.get("content-length")
        && let Ok(expected_len) = cl.parse::<usize>()
    {
        while body.len() < expected_len {
            let n = stream.read(&mut tmp).await.ok()?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&tmp[..n]);
        }
        body.truncate(expected_len);
    }

    Some(RecordedRequest {
        method,
        path,
        query,
        headers,
        body,
    })
}

async fn spawn_mock_server<F>(handler: F) -> (String, Arc<Mutex<Vec<RecordedRequest>>>)
where
    F: Fn(&RecordedRequest) -> MockResponse + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let recorded_clone = recorded.clone();
    let handler = Arc::new(handler);

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let handler = handler.clone();
            let recorded = recorded_clone.clone();
            tokio::spawn(async move {
                if let Some(req) = read_http_request(&mut stream).await {
                    let resp = handler(&req);
                    recorded.lock().unwrap().push(req);

                    let reason = match resp.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        403 => "Forbidden",
                        404 => "Not Found",
                        409 => "Conflict",
                        _ => "Server Response",
                    };

                    let mut header_block = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                        resp.status,
                        reason,
                        resp.content_type,
                        resp.body.len()
                    );
                    for (k, v) in resp.extra_headers {
                        header_block.push_str(&format!("{k}: {v}\r\n"));
                    }
                    header_block.push_str("\r\n");

                    let _ = stream.write_all(header_block.as_bytes()).await;
                    let _ = stream.write_all(&resp.body).await;
                    let _ = stream.flush().await;
                }
            });
        }
    });

    (format!("http://{}", addr), recorded)
}

#[tokio::test]
async fn test_auth_users_apikeys_permissions_disks_and_vacuum() {
    let (base_url, recorded) = spawn_mock_server(|req| {
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/api/auth/bootstrap-status") => MockResponse::json(
                200,
                serde_json::json!({
                    "is_configured": false,
                    "message": "Pending setup"
                }),
            ),
            ("POST", "/api/auth/bootstrap-login") => MockResponse::json(
                200,
                serde_json::json!({
                    "status": "authenticated",
                    "is_admin": true,
                    "username": "bootstrap_admin",
                    "token": "fasts3_sec_boot_123"
                }),
            ),
            ("POST", "/api/auth/bootstrap-setup") => MockResponse::json(
                200,
                serde_json::json!({
                    "status": "configured",
                    "message": "Configured",
                    "username": "admin",
                    "user_id": 1,
                    "is_admin": true,
                    "basic_auth": "Basic YWRtaW46U3VwZXJQYXNzIQ=="
                }),
            ),
            ("POST", "/api/auth/login") => MockResponse::json(
                200,
                serde_json::json!({
                    "status": "authenticated",
                    "is_admin": true,
                    "username": "admin",
                    "basic_auth": "Basic YWRtaW46U3VwZXJQYXNzIQ=="
                }),
            ),
            ("POST", "/api/auth/change-password") => MockResponse::json(
                200,
                serde_json::json!({
                    "status": "success",
                    "message": "Updated",
                    "username": "admin",
                    "basic_auth": "Basic YWRtaW46TmV3UGFzcyEyMw=="
                }),
            ),
            ("GET", "/api/auth/verify") => MockResponse::json(
                200,
                serde_json::json!({
                    "status": "authenticated",
                    "is_admin": true,
                    "username": "admin",
                    "user_id": 1,
                    "is_bootstrap": false
                }),
            ),
            ("GET", "/api/auth/bootstrap-token") => MockResponse::json(
                200,
                serde_json::json!({
                    "bootstrap_token": "fasts3_sec_boot_123"
                }),
            ),
            ("POST", "/api/users") => MockResponse::json(
                201,
                serde_json::json!({
                    "id": 2,
                    "username": "alice",
                    "is_admin": false,
                    "created_at": "2026-11-11T10:00:00Z"
                }),
            ),
            ("GET", "/api/users") => MockResponse::json(
                200,
                serde_json::json!([
                    {"id": 1, "username": "admin", "is_admin": true, "created_at": "2026-11-11T09:00:00Z"},
                    {"id": 2, "username": "alice", "is_admin": false, "created_at": "2026-11-11T10:00:00Z"}
                ]),
            ),
            ("PUT", "/api/users/2") => MockResponse::json(
                200,
                serde_json::json!({
                    "id": 2,
                    "username": "alice",
                    "is_admin": true,
                    "created_at": "2026-11-11T10:00:00Z"
                }),
            ),
            ("DELETE", "/api/users/2") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "User deleted successfully"}),
            ),
            ("POST", "/api/api-keys") => MockResponse::json(
                201,
                serde_json::json!({
                    "id": 10,
                    "user_id": 2,
                    "username": "alice",
                    "is_admin": false,
                    "name": "CI Key",
                    "key_prefix": "fs3_abcd",
                    "api_key": "fs3_abcd1234567890",
                    "created_at": "2026-11-11T10:05:00Z"
                }),
            ),
            ("GET", "/api/api-keys") => MockResponse::json(
                200,
                serde_json::json!([{
                    "id": 10,
                    "user_id": 2,
                    "username": "alice",
                    "is_admin": false,
                    "name": "CI Key",
                    "key_prefix": "fs3_abcd",
                    "created_at": "2026-11-11T10:05:00Z",
                    "last_used_at": "2026-11-11T10:06:00Z"
                }]),
            ),
            ("DELETE", "/api/api-keys/10") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "API Key revocada exitosamente"}),
            ),
            ("POST", "/api/permissions") => MockResponse::json(
                200,
                serde_json::json!({
                    "id": 5,
                    "user_id": 2,
                    "username": "alice",
                    "bucket_name": "invoices",
                    "can_read": true,
                    "can_write": true,
                    "can_delete": false
                }),
            ),
            ("GET", "/api/permissions") => MockResponse::json(
                200,
                serde_json::json!([{
                    "id": 5,
                    "user_id": 2,
                    "username": "alice",
                    "bucket_name": "invoices",
                    "can_read": true,
                    "can_write": true,
                    "can_delete": false
                }]),
            ),
            ("GET", "/api/buckets/invoices/permissions") => MockResponse::json(
                200,
                serde_json::json!([{
                    "id": 5,
                    "user_id": 2,
                    "username": "alice",
                    "bucket_name": "invoices",
                    "can_read": true,
                    "can_write": true,
                    "can_delete": false
                }]),
            ),
            ("DELETE", "/api/buckets/invoices/permissions/2") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "Bucket permission removed successfully"}),
            ),
            ("DELETE", "/api/permissions/5") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "Permission removed successfully"}),
            ),
            ("POST", "/api/disks") => MockResponse::json(
                201,
                serde_json::json!({
                    "id": "nvme2",
                    "name": "Fast NVMe",
                    "path": "/mnt/nvme2",
                    "is_default": false,
                    "created_at": "2026-11-11T11:00:00Z",
                    "total_space": 1_000_000_000u64,
                    "available_space": 800_000_000u64,
                    "used_space": 200_000_000u64,
                    "fasts3_used_space": 4096u64
                }),
            ),
            ("GET", "/api/disks") => MockResponse::json(
                200,
                serde_json::json!([{
                    "id": "nvme2",
                    "name": "Fast NVMe",
                    "path": "/mnt/nvme2",
                    "is_default": false,
                    "created_at": "2026-11-11T11:00:00Z",
                    "total_space": 1_000_000_000u64,
                    "available_space": 800_000_000u64,
                    "used_space": 200_000_000u64,
                    "fasts3_used_space": 4096u64
                }]),
            ),
            ("DELETE", "/api/disks/nvme2") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "Disk deleted successfully"}),
            ),
            ("GET", "/api/vacuum/status") | ("POST", "/api/vacuum") => MockResponse::json(
                200,
                serde_json::json!({
                    "files_deleted": 3,
                    "bytes_reclaimed": 16384,
                    "buckets_reconciled": 2,
                    "last_run_timestamp": 1794400000u64,
                    "duration_ms": 14
                }),
            ),
            _ => MockResponse::json(404, serde_json::json!({"error": "Not found", "status": 404})),
        }
    })
    .await;

    let client = FastS3Client::new(&base_url).unwrap();

    // 1. Bootstrap status & login
    let status = client.bootstrap_status().await.unwrap();
    assert!(!status.is_configured);

    let boot_login = client
        .bootstrap_login_and_authenticate("fasts3_sec_boot_123")
        .await
        .unwrap();
    assert!(boot_login.is_admin);
    assert_eq!(
        client.credentials(),
        Credentials::Bearer("fasts3_sec_boot_123".to_string())
    );

    // 2. Bootstrap setup & login & change password & verify
    let setup = client
        .bootstrap_setup_and_authenticate(&BootstrapSetupRequest {
            token: "fasts3_sec_boot_123".to_string(),
            username: "admin".to_string(),
            password: "SuperPass!".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(setup.user_id, 1);
    assert_eq!(
        client.credentials(),
        Credentials::basic("admin", "SuperPass!")
    );

    let login = client
        .login_and_authenticate("admin", "SuperPass!")
        .await
        .unwrap();
    assert!(login.is_admin);
    assert_eq!(login.user_id, None); // Config admin omits user_id

    let pwd = client
        .change_password_and_authenticate(Some("SuperPass!"), "NewPass!23")
        .await
        .unwrap();
    assert_eq!(pwd.status, "success");
    assert_eq!(
        client.credentials(),
        Credentials::basic("admin", "NewPass!23")
    );

    let verified = client.verify_auth().await.unwrap();
    assert!(verified.is_admin);
    assert_eq!(verified.username, "admin");

    let boot_tok = client.get_bootstrap_token().await.unwrap();
    assert_eq!(boot_tok.as_deref(), Some("fasts3_sec_boot_123"));

    // 3. Users CRUD
    let alice = client
        .create_user(&CreateUserRequest::new("alice", "alice123", false))
        .await
        .unwrap();
    assert_eq!(alice.id, 2);

    let found_alice = client.get_user_by_username("alice").await.unwrap().unwrap();
    assert_eq!(found_alice.id, 2);

    let updated_alice = client
        .update_user(
            2,
            &UpdateUserRequest {
                password: None,
                is_admin: Some(true),
            },
        )
        .await
        .unwrap();
    assert!(updated_alice.is_admin);

    // 4. API Keys
    let created_key = client.create_api_key("CI Key", Some(2)).await.unwrap();
    assert!(created_key.api_key.starts_with("fs3_"));
    let keys = client.list_api_keys().await.unwrap();
    assert_eq!(keys.len(), 1);
    client.delete_api_key(10).await.unwrap();

    // 5. Permissions
    let perm = client
        .set_permission(&SetPermissionRequest {
            user_id: 2,
            bucket_name: "invoices".to_string(),
            can_read: true,
            can_write: true,
            can_delete: false,
        })
        .await
        .unwrap();
    assert_eq!(perm.id, 5);
    assert_eq!(client.list_permissions().await.unwrap().len(), 1);
    assert_eq!(
        client.get_bucket_permissions("invoices").await.unwrap().len(),
        1
    );
    client.delete_bucket_permission("invoices", 2).await.unwrap();
    client.delete_permission(5).await.unwrap();
    client.delete_user(2).await.unwrap();

    // 6. Disks
    let disk = client
        .add_disk(&CreateDiskRequest::new("Fast NVMe", "/mnt/nvme2").with_id("nvme2"))
        .await
        .unwrap();
    assert_eq!(disk.id, "nvme2");
    assert_eq!(client.list_disks().await.unwrap().len(), 1);
    client.delete_disk("nvme2").await.unwrap();

    // 7. Vacuum
    let vac_status = client.vacuum_status().await.unwrap();
    assert_eq!(vac_status.files_deleted, 3);
    let vac_run = client.trigger_vacuum().await.unwrap();
    assert_eq!(vac_run.bytes_reclaimed, 16384);

    assert!(recorded.lock().unwrap().len() >= 20);
}

#[tokio::test]
async fn test_buckets_files_streaming_thumbnails_and_errors() {
    let (base_url, recorded) = spawn_mock_server(|req| {
        match (req.method.as_str(), req.path.as_str()) {
            ("POST", "/api/buckets") => MockResponse::json(
                201,
                serde_json::json!({"status": "success", "message": "Bucket created successfully"}),
            ),
            ("GET", "/api/buckets") => MockResponse::json(
                200,
                serde_json::json!([{
                    "id": 42,
                    "name": "media",
                    "is_public": true,
                    "disk_id": "default",
                    "disk_path": "/storage",
                    "quota": 50_000_000u64,
                    "used": 128u64
                }]),
            ),
            ("PUT", "/api/buckets/42") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "Bucket updated successfully"}),
            ),
            ("DELETE", "/api/buckets/42/media") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "Bucket deleted successfully"}),
            ),
            ("PUT", "/api/buckets/media/files/docs/hello%20world.txt") => MockResponse::json(
                201,
                serde_json::json!({"status": "success", "message": "File uploaded successfully"}),
            ),
            ("POST", "/api/buckets/media/files/upload") => MockResponse::json(
                201,
                serde_json::json!({"status": "success", "message": "File uploaded successfully"}),
            ),
            ("GET", "/api/buckets/media/files") => MockResponse::json(
                200,
                serde_json::json!(["docs/hello world.txt"]),
            ),
            ("GET", "/api/buckets/media/files/details") => MockResponse::json(
                200,
                serde_json::json!([{
                    "key": "docs/hello world.txt",
                    "name": "hello world.txt",
                    "size": 11,
                    "mime_type": "text/plain",
                    "extension": "txt",
                    "is_image": false
                }]),
            ),
            ("GET", "/api/buckets/media/files/docs/hello%20world.txt") => MockResponse::binary(
                200,
                "text/plain",
                b"Hello World".to_vec(),
                vec![(
                    "Content-Disposition".to_string(),
                    "inline; filename=\"hello world.txt\"".to_string(),
                )],
            ),
            ("GET", "/api/buckets/media/files/pics/logo.png/thumbnail")
            | ("GET", "/api/buckets/media/thumbnail/pics/logo.png") => {
                MockResponse::binary(200, "image/png", vec![0x89, b'P', b'N', b'G'], Vec::new())
            }
            ("DELETE", "/api/buckets/media/files/docs/hello%20world.txt") => MockResponse::json(
                200,
                serde_json::json!({"status": "success", "message": "File deleted successfully"}),
            ),
            ("PUT", "/api/buckets/media/files/over_quota.bin") => MockResponse::json(
                403,
                serde_json::json!({"error": "Storage quota exceeded", "status": 403}),
            ),
            _ => MockResponse::json(404, serde_json::json!({"error": "Not found", "status": 404})),
        }
    })
    .await;

    let client = FastS3Client::with_api_key(&base_url, "fs3_test_key_999").unwrap();

    // Bucket lifecycle
    client
        .create_bucket(
            &CreateBucketRequest::new("media")
                .public(true)
                .quota(50_000_000),
        )
        .await
        .unwrap();

    let bucket = client.get_bucket_by_name("media").await.unwrap().unwrap();
    assert_eq!(bucket.id, 42);

    let mut update_req = UpdateBucketRequest::from(&bucket);
    update_req.quota = 100_000_000;
    client.update_bucket(bucket.id, &update_req).await.unwrap();

    // File upload (raw bytes & local file stream & JSON base64)
    client
        .upload_bytes(
            "media",
            "docs/hello world.txt",
            "Hello World",
            Some("text/plain"),
        )
        .await
        .unwrap();

    let temp_dir = std::env::temp_dir().join(format!("fasts3_rust_cli_test_{}", std::process::id()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();
    let local_src = temp_dir.join("src_hello.txt");
    tokio::fs::write(&local_src, b"Hello World").await.unwrap();

    client
        .upload_file("media", "docs/hello world.txt", &local_src)
        .await
        .unwrap();

    client
        .upload_json_bytes(
            "media",
            "docs/hello world.txt",
            Some("hello world.txt"),
            b"Hello World",
        )
        .await
        .unwrap();

    // Listing & details
    let filter = FileListFilter::new()
        .with_subpath("docs")
        .with_name("hello");
    let keys = client.list_files("media", Some(&filter)).await.unwrap();
    assert_eq!(keys, vec!["docs/hello world.txt"]);

    let details = client
        .list_file_details("media", Some(&filter))
        .await
        .unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].size, 11);

    // Download in-memory & stream to disk
    let downloaded = client
        .download_file("media", "docs/hello world.txt")
        .await
        .unwrap();
    assert_eq!(&downloaded.data[..], b"Hello World");
    assert_eq!(downloaded.filename.as_deref(), Some("hello world.txt"));

    let local_dst = temp_dir.join("nested/downloaded.txt");
    let written = client
        .download_to_path("media", "docs/hello world.txt", &local_dst)
        .await
        .unwrap();
    assert_eq!(written, 11);
    assert_eq!(tokio::fs::read(&local_dst).await.unwrap(), b"Hello World");

    // Thumbnails (both primary and alt routes)
    let thumb_opts = ThumbnailOptions::new(200, 150);
    let thumb = client
        .get_thumbnail("media", "pics/logo.png", Some(&thumb_opts))
        .await
        .unwrap();
    assert_eq!(thumb.content_type, "image/png");
    assert_eq!(&thumb.data[..], &[0x89, b'P', b'N', b'G']);

    let thumb_alt = client
        .get_thumbnail_alt("media", "pics/logo.png", Some(&thumb_opts))
        .await
        .unwrap();
    assert_eq!(&thumb_alt.data[..], &[0x89, b'P', b'N', b'G']);

    // Direct URLs with query auth
    let direct_url = client
        .file_url("media", "docs/hello world.txt", true)
        .unwrap();
    assert!(direct_url.ends_with("/api/buckets/media/files/docs/hello%20world.txt?api_key=fs3_test_key_999"));

    let thumb_url = client
        .thumbnail_url("media", "pics/logo.png", Some(&thumb_opts), true)
        .unwrap();
    assert!(thumb_url.contains("width=200&height=150&api_key=fs3_test_key_999"));

    // Quota exceeded error classification
    let err = client
        .upload_bytes("media", "over_quota.bin", vec![0u8; 64], None)
        .await
        .unwrap_err();
    assert!(err.is_forbidden());
    assert!(err.is_quota_exceeded());

    // Delete file & delete bucket by name
    client
        .delete_file("media", "docs/hello world.txt")
        .await
        .unwrap();
    client.delete_bucket_by_name("media").await.unwrap();

    // Verify X-API-Key header, query strings, and bodies were sent properly
    {
        let reqs = recorded.lock().unwrap();
        assert!(
            reqs.iter()
                .all(|r| r.headers.get("x-api-key").map(String::as_str) == Some("fs3_test_key_999"))
        );
        assert!(reqs.iter().any(|r| r.query.contains("width=200") && r.query.contains("height=150")));
        assert!(reqs.iter().any(|r| r.body == b"Hello World"));
    }

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}
