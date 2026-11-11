use reqwest::Method;

use crate::client::FastS3Client;
use crate::encoding::encode_path_segment;
use crate::error::{FastS3Error, Result};
use crate::models::{Bucket, CreateBucketRequest, StatusMessageResponse, UpdateBucketRequest};

impl FastS3Client {
    /// Lists buckets visible to the current credentials (`GET /api/buckets`).
    ///
    /// - Admins receive all buckets.
    /// - Authenticated users receive public buckets plus buckets where they hold permissions.
    /// - Anonymous callers receive public buckets only.
    pub async fn list_buckets(&self) -> Result<Vec<Bucket>> {
        let req = self.request(Method::GET, "/api/buckets")?;
        Self::send_json(req).await
    }

    /// Looks up a bucket by name (case-insensitive) via `list_buckets`.
    pub async fn get_bucket_by_name(&self, name: &str) -> Result<Option<Bucket>> {
        let buckets = self.list_buckets().await?;
        Ok(buckets
            .into_iter()
            .find(|b| b.name.eq_ignore_ascii_case(name)))
    }

    /// Creates a new bucket (`POST /api/buckets`, requires Admin).
    pub async fn create_bucket(&self, payload: &CreateBucketRequest) -> Result<StatusMessageResponse> {
        let req = self.request(Method::POST, "/api/buckets")?.json(payload);
        Self::send_json(req).await
    }

    /// Updates an existing bucket's visibility, quota, or disk assignment (`PUT /api/buckets/{id}`, requires Admin).
    pub async fn update_bucket(
        &self,
        id: u64,
        payload: &UpdateBucketRequest,
    ) -> Result<StatusMessageResponse> {
        let req = self
            .request(Method::PUT, &format!("/api/buckets/{id}"))?
            .json(payload);
        Self::send_json(req).await
    }

    /// Deletes a bucket and its files by numeric `id` and `name` (`DELETE /api/buckets/{id}/{name}`, requires Admin).
    pub async fn delete_bucket(&self, id: u64, name: &str) -> Result<StatusMessageResponse> {
        let encoded_name = encode_path_segment(name);
        let req = self.request(
            Method::DELETE,
            &format!("/api/buckets/{id}/{encoded_name}"),
        )?;
        Self::send_json(req).await
    }

    /// Convenience helper that resolves the bucket's numeric `id` by `name` and deletes it.
    pub async fn delete_bucket_by_name(&self, name: &str) -> Result<StatusMessageResponse> {
        let bucket = self
            .get_bucket_by_name(name)
            .await?
            .ok_or_else(|| FastS3Error::Api {
                status: 404,
                message: format!("Bucket '{name}' not found"),
            })?;
        self.delete_bucket(bucket.id, &bucket.name).await
    }
}
