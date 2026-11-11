use bytes::Bytes;
use serde::{Deserialize, Serialize};

// ============================================================================
// Common Responses
// ============================================================================

/// Generic `{ "status": "...", "message": "..." }` confirmation payload returned by mutating endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusMessageResponse {
    pub status: String,
    pub message: String,
}

// ============================================================================
// Authentication & Bootstrap Models
// ============================================================================

/// Response from `GET /api/auth/verify`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthVerifyResponse {
    pub status: String,
    pub is_admin: bool,
    pub username: String,
    #[serde(default)]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub is_bootstrap: Option<bool>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

/// Response from `GET /api/auth/bootstrap-status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapStatusResponse {
    pub is_configured: bool,
    pub message: String,
}

/// Request payload for `POST /api/auth/bootstrap-login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapLoginRequest {
    pub token: String,
}

/// Response from `POST /api/auth/bootstrap-login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapLoginResponse {
    pub status: String,
    pub is_admin: bool,
    pub username: String,
    pub token: String,
}

/// Request payload for `POST /api/auth/bootstrap-setup`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapSetupRequest {
    pub token: String,
    pub username: String,
    pub password: String,
}

/// Response from `POST /api/auth/bootstrap-setup`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapSetupResponse {
    pub status: String,
    pub message: String,
    pub username: String,
    pub user_id: i64,
    pub is_admin: bool,
    pub basic_auth: String,
}

/// Request payload for `POST /api/auth/login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserLoginRequest {
    pub username: String,
    pub password: String,
}

/// Response from `POST /api/auth/login`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserLoginResponse {
    pub status: String,
    pub is_admin: bool,
    pub username: String,
    #[serde(default)]
    pub user_id: Option<i64>,
    pub basic_auth: String,
}

/// Request payload for `POST /api/auth/change-password`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_password: Option<String>,
    pub new_password: String,
}

/// Response from `POST /api/auth/change-password`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangePasswordResponse {
    pub status: String,
    pub message: String,
    pub username: String,
    pub basic_auth: String,
}

/// Response from `GET /api/auth/bootstrap-token`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapTokenResponse {
    pub bootstrap_token: Option<String>,
}

// ============================================================================
// API Keys Models
// ============================================================================

/// Metadata of a persisted user API Key returned by `GET /api/api-keys`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub is_admin: bool,
    pub name: String,
    pub key_prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Newly created API Key returned once by `POST /api/api-keys`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatedApiKey {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub is_admin: bool,
    pub name: String,
    pub key_prefix: String,
    pub api_key: String,
    pub created_at: String,
}

/// Request payload for `POST /api/api-keys`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
}

// ============================================================================
// Users & Permissions Models
// ============================================================================

/// User record returned by FastS3 user management endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub created_at: String,
}

/// Request payload for `POST /api/users`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_admin: Option<bool>,
}

impl CreateUserRequest {
    pub fn new(username: impl Into<String>, password: impl Into<String>, is_admin: bool) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            is_admin: Some(is_admin),
        }
    }
}

/// Request payload for `PUT /api/users/{id}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_admin: Option<bool>,
}

/// Granular per-bucket user permission entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permission {
    pub id: i64,
    pub user_id: i64,
    #[serde(default)]
    pub username: Option<String>,
    pub bucket_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
}

/// Request payload for `POST /api/permissions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPermissionRequest {
    pub user_id: i64,
    pub bucket_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
}

// ============================================================================
// Multi-Disk Storage Models
// ============================================================================

/// Registered storage volume returned by `GET /api/disks` and `POST /api/disks`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disk {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_default: bool,
    pub created_at: String,
    #[serde(default)]
    pub total_space: Option<u64>,
    #[serde(default)]
    pub available_space: Option<u64>,
    #[serde(default)]
    pub used_space: Option<u64>,
    #[serde(default)]
    pub fasts3_used_space: Option<u64>,
}

/// Request payload for `POST /api/disks`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDiskRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub path: String,
}

impl CreateDiskRequest {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            path: path.into(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

// ============================================================================
// Bucket Models
// ============================================================================

/// Bucket metadata returned by `GET /api/buckets`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bucket {
    #[serde(default)]
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default)]
    pub disk_path: Option<String>,
    #[serde(default)]
    pub disk_id: Option<String>,
    #[serde(default)]
    pub quota: u64,
    #[serde(default)]
    pub used: u64,
}

/// Request payload for `POST /api/buckets`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBucketRequest {
    pub name: String,
    pub is_public: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_path: Option<String>,
    pub quota: u64,
}

impl CreateBucketRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_public: false,
            disk_id: None,
            disk_path: None,
            quota: 0,
        }
    }

    pub fn public(mut self, is_public: bool) -> Self {
        self.is_public = is_public;
        self
    }

    pub fn disk_id(mut self, disk_id: impl Into<String>) -> Self {
        self.disk_id = Some(disk_id.into());
        self
    }

    pub fn disk_path(mut self, disk_path: impl Into<String>) -> Self {
        self.disk_path = Some(disk_path.into());
        self
    }

    pub fn quota(mut self, quota_bytes: u64) -> Self {
        self.quota = quota_bytes;
        self
    }
}

/// Request payload for `PUT /api/buckets/{id}`.
///
/// Note: FastS3 expects the full `Bucket` JSON representation (including `name`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateBucketRequest {
    pub name: String,
    pub is_public: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_path: Option<String>,
    pub quota: u64,
}

impl From<&Bucket> for UpdateBucketRequest {
    fn from(b: &Bucket) -> Self {
        Self {
            name: b.name.clone(),
            is_public: b.is_public,
            disk_id: b.disk_id.clone(),
            disk_path: b.disk_path.clone(),
            quota: b.quota,
        }
    }
}

// ============================================================================
// Files & Thumbnails Models
// ============================================================================

/// Optional filter parameters for listing files (`GET /api/buckets/{bucket}/files` and `/details`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileListFilter {
    /// Case-insensitive substring match on the filename.
    pub name: Option<String>,
    /// Prefix/folder path filter inside the bucket (e.g. `"invoices/2026"`).
    pub subpath: Option<String>,
}

impl FileListFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_subpath(mut self, subpath: impl Into<String>) -> Self {
        self.subpath = Some(subpath.into());
        self
    }

    pub(crate) fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(ref n) = self.name {
            pairs.push(("name", n.clone()));
        }
        if let Some(ref s) = self.subpath {
            pairs.push(("subpath", s.clone()));
        }
        pairs
    }
}

/// Rich file metadata returned by `GET /api/buckets/{bucket}/files/details`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDetail {
    pub key: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub extension: String,
    pub is_image: bool,
}

/// Request payload for `POST /api/buckets/{bucket}/files/upload` (JSON + Base64 upload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonUploadRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub key: String,
    /// Base64-encoded binary content.
    pub data: String,
}

/// Resize dimensions for thumbnail endpoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThumbnailOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ThumbnailOptions {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    pub(crate) fn to_query_pairs(self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(w) = self.width {
            pairs.push(("width", w.to_string()));
        }
        if let Some(h) = self.height {
            pairs.push(("height", h.to_string()));
        }
        pairs
    }
}

/// Options for `GET /api/buckets/{bucket}/files/{key}` (supports on-the-fly thumbnail mode).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DownloadOptions {
    pub thumbnail: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl DownloadOptions {
    pub(crate) fn to_query_pairs(self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if self.thumbnail {
            pairs.push(("thumbnail", "true".to_string()));
        }
        if let Some(w) = self.width {
            pairs.push(("width", w.to_string()));
        }
        if let Some(h) = self.height {
            pairs.push(("height", h.to_string()));
        }
        pairs
    }
}

/// Downloaded object payload and HTTP metadata returned by `download_file`.
#[derive(Debug, Clone)]
pub struct DownloadedFile {
    pub data: Bytes,
    pub content_type: String,
    pub content_length: Option<u64>,
    pub filename: Option<String>,
}

/// Generated thumbnail binary payload and MIME type.
#[derive(Debug, Clone)]
pub struct ThumbnailResponse {
    pub data: Bytes,
    pub content_type: String,
}

// ============================================================================
// AutoVacuum Models
// ============================================================================

/// Report returned by `GET /api/vacuum/status` and `POST /api/vacuum`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VacuumStats {
    pub files_deleted: usize,
    pub bytes_reclaimed: u64,
    pub buckets_reconciled: usize,
    pub last_run_timestamp: u64,
    pub duration_ms: u128,
}
