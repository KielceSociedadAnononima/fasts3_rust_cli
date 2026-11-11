use reqwest::Method;

use crate::client::FastS3Client;
use crate::encoding::encode_path_segment;
use crate::error::Result;
use crate::models::{
    ApiKeyInfo, CreateApiKeyRequest, CreateUserRequest, CreatedApiKey, Permission,
    SetPermissionRequest, StatusMessageResponse, UpdateUserRequest, UserResponse,
};

impl FastS3Client {
    // ========================================================================
    // API Keys Endpoints
    // ========================================================================

    /// Lists API keys (`GET /api/api-keys`).
    ///
    /// Administrators receive all keys across all users; regular users receive only their own keys.
    pub async fn list_api_keys(&self) -> Result<Vec<ApiKeyInfo>> {
        let req = self.request(Method::GET, "/api/api-keys")?;
        Self::send_json(req).await
    }

    /// Generates a new API key (`POST /api/api-keys`).
    ///
    /// `user_id` may be `None` to create a key for the currently authenticated user, or `Some(id)` when an admin creates a key on behalf of another user.
    pub async fn create_api_key(
        &self,
        name: &str,
        user_id: Option<i64>,
    ) -> Result<CreatedApiKey> {
        let payload = CreateApiKeyRequest {
            name: name.to_string(),
            user_id,
        };
        let req = self.request(Method::POST, "/api/api-keys")?.json(&payload);
        Self::send_json(req).await
    }

    /// Revokes an API key by ID (`DELETE /api/api-keys/{id}`).
    pub async fn delete_api_key(&self, id: i64) -> Result<StatusMessageResponse> {
        let req = self.request(Method::DELETE, &format!("/api/api-keys/{id}"))?;
        Self::send_json(req).await
    }

    // ========================================================================
    // Users Endpoints
    // ========================================================================

    /// Lists all users (`GET /api/users`, requires Admin).
    pub async fn list_users(&self) -> Result<Vec<UserResponse>> {
        let req = self.request(Method::GET, "/api/users")?;
        Self::send_json(req).await
    }

    /// Helper that looks up a user by exact username (case-sensitive) using `list_users`.
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<UserResponse>> {
        let users = self.list_users().await?;
        Ok(users.into_iter().find(|u| u.username == username))
    }

    /// Creates a new user (`POST /api/users`, requires Admin).
    pub async fn create_user(&self, payload: &CreateUserRequest) -> Result<UserResponse> {
        let req = self.request(Method::POST, "/api/users")?.json(payload);
        Self::send_json(req).await
    }

    /// Updates a user's password and/or admin status (`PUT /api/users/{id}`).
    pub async fn update_user(
        &self,
        id: i64,
        payload: &UpdateUserRequest,
    ) -> Result<UserResponse> {
        let req = self
            .request(Method::PUT, &format!("/api/users/{id}"))?
            .json(payload);
        Self::send_json(req).await
    }

    /// Deletes a user by numeric ID (`DELETE /api/users/{id}`, requires Admin).
    pub async fn delete_user(&self, id: i64) -> Result<StatusMessageResponse> {
        let req = self.request(Method::DELETE, &format!("/api/users/{id}"))?;
        Self::send_json(req).await
    }

    // ========================================================================
    // Permissions Endpoints
    // ========================================================================

    /// Lists all bucket permission assignments across all users (`GET /api/permissions`, requires Admin).
    pub async fn list_permissions(&self) -> Result<Vec<Permission>> {
        let req = self.request(Method::GET, "/api/permissions")?;
        Self::send_json(req).await
    }

    /// Grants or updates granular bucket permissions for a user (`POST /api/permissions`, requires Admin).
    pub async fn set_permission(&self, payload: &SetPermissionRequest) -> Result<Permission> {
        let req = self
            .request(Method::POST, "/api/permissions")?
            .json(payload);
        Self::send_json(req).await
    }

    /// Revokes a permission assignment by its numeric permission ID (`DELETE /api/permissions/{id}`, requires Admin).
    pub async fn delete_permission(&self, id: i64) -> Result<StatusMessageResponse> {
        let req = self.request(Method::DELETE, &format!("/api/permissions/{id}"))?;
        Self::send_json(req).await
    }

    /// Lists all permission entries configured on a specific bucket (`GET /api/buckets/{bucket}/permissions`).
    pub async fn get_bucket_permissions(&self, bucket: &str) -> Result<Vec<Permission>> {
        let encoded_bkt = encode_path_segment(bucket);
        let req = self.request(
            Method::GET,
            &format!("/api/buckets/{encoded_bkt}/permissions"),
        )?;
        Self::send_json(req).await
    }

    /// Revokes a user's permissions on a specific bucket (`DELETE /api/buckets/{bucket}/permissions/{user_id}`, requires Admin).
    pub async fn delete_bucket_permission(
        &self,
        bucket: &str,
        user_id: i64,
    ) -> Result<StatusMessageResponse> {
        let encoded_bkt = encode_path_segment(bucket);
        let req = self.request(
            Method::DELETE,
            &format!("/api/buckets/{encoded_bkt}/permissions/{user_id}"),
        )?;
        Self::send_json(req).await
    }
}
