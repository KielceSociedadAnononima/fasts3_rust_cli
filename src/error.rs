use serde::Deserialize;
use thiserror::Error;

/// Result alias for all FastS3 client operations.
pub type Result<T> = std::result::Result<T, FastS3Error>;

/// Standard JSON error payload returned by the FastS3 server.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ApiErrorPayload {
    pub error: String,
    #[serde(default)]
    pub status: Option<u16>,
}

/// Comprehensive error type for the FastS3 Rust client library.
#[derive(Debug, Error)]
pub enum FastS3Error {
    /// Server returned a non-2xx HTTP response.
    #[error("FastS3 API error (HTTP {status}): {message}")]
    Api { status: u16, message: String },

    /// Underlying HTTP transport error.
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid base URL or endpoint construction failure.
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// Local filesystem or stream I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid argument passed by the caller before sending the request.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

impl FastS3Error {
    /// Returns the HTTP status code if this error originated from an HTTP/API response.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            FastS3Error::Api { status, .. } => Some(*status),
            FastS3Error::Http(err) => err.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// Returns `true` if the server responded with `400 Bad Request`.
    pub fn is_bad_request(&self) -> bool {
        self.status_code() == Some(400)
    }

    /// Returns `true` if the server responded with `401 Unauthorized`.
    pub fn is_unauthorized(&self) -> bool {
        self.status_code() == Some(401)
    }

    /// Returns `true` if the server responded with `403 Forbidden`.
    pub fn is_forbidden(&self) -> bool {
        self.status_code() == Some(403)
    }

    /// Returns `true` if the server responded with `404 Not Found`.
    pub fn is_not_found(&self) -> bool {
        self.status_code() == Some(404)
    }

    /// Returns `true` if the server responded with `409 Conflict`.
    pub fn is_conflict(&self) -> bool {
        self.status_code() == Some(409)
    }

    /// Returns `true` if the error indicates bucket storage quota exceeded (`403` with quota message).
    pub fn is_quota_exceeded(&self) -> bool {
        match self {
            FastS3Error::Api { status, message } => {
                *status == 403 && message.to_ascii_lowercase().contains("quota")
            }
            _ => false,
        }
    }
}
