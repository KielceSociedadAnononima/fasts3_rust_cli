use reqwest::Method;

use crate::client::FastS3Client;
use crate::encoding::encode_path_segment;
use crate::error::Result;
use crate::models::{CreateDiskRequest, Disk, StatusMessageResponse};

impl FastS3Client {
    /// Lists all registered storage volumes and their OS/FastS3 space telemetry (`GET /api/disks`, requires Admin).
    pub async fn list_disks(&self) -> Result<Vec<Disk>> {
        let req = self.request(Method::GET, "/api/disks")?;
        Self::send_json(req).await
    }

    /// Registers a new secondary storage volume (`POST /api/disks`, requires Admin).
    pub async fn add_disk(&self, payload: &CreateDiskRequest) -> Result<Disk> {
        let req = self.request(Method::POST, "/api/disks")?.json(payload);
        Self::send_json(req).await
    }

    /// Unregisters a non-default storage volume with no active buckets (`DELETE /api/disks/{id}`, requires Admin).
    pub async fn delete_disk(&self, disk_id: &str) -> Result<StatusMessageResponse> {
        let encoded_id = encode_path_segment(disk_id);
        let req = self.request(Method::DELETE, &format!("/api/disks/{encoded_id}"))?;
        Self::send_json(req).await
    }
}
