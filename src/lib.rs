//! # FastS3 Rust Client (`fasts3_rust_cli`)
//!
//! Complete asynchronous Rust client library (SDK) for consuming every endpoint of the
//! **FastS3** high-performance object storage server.
//!
//! ## Features
//! - **All Authentication Modes**: Basic Auth, Bearer Token (Bootstrap Token & API Keys), `X-API-Key`, `X-FastS3-Token`, `X-FastS3-User`/`X-FastS3-Pass`, and Anonymous access.
//! - **Dynamic Session Management**: Thread-safe credential switching (`Clone + Send + Sync`) and `*_and_authenticate` helpers.
//! - **Buckets & Multi-Disk**: Full CRUD for buckets, quotas, visibility, and physical storage volumes with space telemetry.
//! - **Files & Streaming**: Raw binary uploads (`PUT`), zero-buffering async stream & local file uploads/downloads, Base64 JSON uploads (`POST`), detailed listings, and on-the-fly image thumbnails.
//! - **RBAC, Users & API Keys**: Full user lifecycle, granular per-bucket permissions (`read`/`write`/`delete`), and scoped API key management.
//! - **AutoVacuum**: Inspect status and trigger immediate storage cleanup & quota reconciliation.
//!
//! ## Quick Example
//! ```no_run
//! use fasts3_rust_cli::{CreateBucketRequest, FastS3Client};
//!
//! #[tokio::main]
//! async fn main() -> fasts3_rust_cli::Result<()> {
//!     let client = FastS3Client::with_basic_auth("http://localhost:8080", "admin", "secret")?;
//!
//!     client
//!         .create_bucket(&CreateBucketRequest::new("docs").quota(100 * 1024 * 1024))
//!         .await?;
//!
//!     client
//!         .upload_bytes("docs", "guides/hello.txt", "Hola FastS3!", Some("text/plain"))
//!         .await?;
//!
//!     let bytes = client.download_bytes("docs", "guides/hello.txt").await?;
//!     assert_eq!(bytes, b"Hola FastS3!");
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod client;
pub(crate) mod encoding;
pub mod error;
pub mod models;

pub use auth::Credentials;
pub use client::{FastS3Client, FastS3ClientBuilder};
pub use error::{FastS3Error, Result};
pub use models::*;
