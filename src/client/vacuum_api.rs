use reqwest::Method;

use crate::client::FastS3Client;
use crate::error::Result;
use crate::models::VacuumStats;

impl FastS3Client {
    /// Retrieves statistics from the most recent AutoVacuum pass (`GET /api/vacuum/status`, requires Admin).
    pub async fn vacuum_status(&self) -> Result<VacuumStats> {
        let req = self.request(Method::GET, "/api/vacuum/status")?;
        Self::send_json(req).await
    }

    /// Immediately triggers a full AutoVacuum cleanup and bucket quota reconciliation pass (`POST /api/vacuum`, requires Admin).
    pub async fn trigger_vacuum(&self) -> Result<VacuumStats> {
        let req = self.request(Method::POST, "/api/vacuum")?;
        Self::send_json(req).await
    }
}
