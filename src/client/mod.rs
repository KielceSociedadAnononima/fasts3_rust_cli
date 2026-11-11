mod auth_api;
mod buckets_api;
mod disks_api;
mod files_api;
mod users_api;
mod vacuum_api;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use reqwest::{Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use url::Url;

use crate::auth::Credentials;
use crate::error::{ApiErrorPayload, FastS3Error, Result};

const DEFAULT_USER_AGENT: &str = concat!("fasts3-rust-client/", env!("CARGO_PKG_VERSION"));

/// Builder for configuring a [`FastS3Client`] instance.
#[derive(Debug, Clone)]
pub struct FastS3ClientBuilder {
    base_url: String,
    credentials: Credentials,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    user_agent: String,
    http_client: Option<reqwest::Client>,
}

impl FastS3ClientBuilder {
    /// Starts building a [`FastS3Client`] targeting `base_url` (e.g. `"http://localhost:8080"`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            credentials: Credentials::None,
            timeout: None,
            connect_timeout: Some(Duration::from_secs(10)),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            http_client: None,
        }
    }

    /// Configures initial authentication credentials.
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = credentials;
        self
    }

    /// Configures HTTP Basic Authentication (`Authorization: Basic ...`).
    pub fn basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.credentials = Credentials::basic(username, password);
        self
    }

    /// Configures Bearer Token authentication (`Authorization: Bearer ...`) for Bootstrap Token or API Key.
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.credentials = Credentials::bearer(token);
        self
    }

    /// Configures API Key authentication (`X-API-Key: ...`).
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.credentials = Credentials::api_key(key);
        self
    }

    /// Sets the overall request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the TCP connection timeout (defaults to 10s).
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Overrides the `User-Agent` header sent with every request.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Supplies a pre-configured `reqwest::Client`.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Validates configuration and builds the [`FastS3Client`].
    pub fn build(self) -> Result<FastS3Client> {
        let trimmed = self.base_url.trim().trim_end_matches('/').to_string();
        let parsed = Url::parse(&trimmed)?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(FastS3Error::InvalidArgument(format!(
                "Unsupported URL scheme '{}': expected http or https",
                parsed.scheme()
            )));
        }

        let http = match self.http_client {
            Some(c) => c,
            None => {
                let mut b = reqwest::Client::builder().user_agent(self.user_agent);
                if let Some(t) = self.timeout {
                    b = b.timeout(t);
                }
                if let Some(ct) = self.connect_timeout {
                    b = b.connect_timeout(ct);
                }
                b.build()?
            }
        };

        Ok(FastS3Client {
            base_url: trimmed,
            http,
            credentials: Arc::new(RwLock::new(self.credentials)),
        })
    }
}

/// Asynchronous, thread-safe (`Clone + Send + Sync`) client for the FastS3 REST API.
#[derive(Debug, Clone)]
pub struct FastS3Client {
    pub(crate) base_url: String,
    pub(crate) http: reqwest::Client,
    pub(crate) credentials: Arc<RwLock<Credentials>>,
}

impl FastS3Client {
    /// Creates an unauthenticated client pointing to `base_url`.
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        FastS3ClientBuilder::new(base_url).build()
    }

    /// Creates a [`FastS3ClientBuilder`] for advanced customization.
    pub fn builder(base_url: impl Into<String>) -> FastS3ClientBuilder {
        FastS3ClientBuilder::new(base_url)
    }

    /// Creates a client authenticated with HTTP Basic Auth (`username` & `password`).
    pub fn with_basic_auth(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self> {
        FastS3ClientBuilder::new(base_url)
            .basic_auth(username, password)
            .build()
    }

    /// Creates a client authenticated with a Bearer token (Bootstrap Token or API Key).
    pub fn with_bearer_token(
        base_url: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self> {
        FastS3ClientBuilder::new(base_url)
            .bearer_token(token)
            .build()
    }

    /// Creates a client authenticated with an `X-API-Key` header.
    pub fn with_api_key(base_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        FastS3ClientBuilder::new(base_url).api_key(api_key).build()
    }

    /// Returns the normalized base URL configured on this client.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns a snapshot of the active [`Credentials`].
    pub fn credentials(&self) -> Credentials {
        self.credentials
            .read()
            .expect("credentials RwLock poisoned")
            .clone()
    }

    /// Updates the active [`Credentials`] for all subsequent requests made by this client (and its clones).
    pub fn set_credentials(&self, credentials: Credentials) {
        let mut guard = self
            .credentials
            .write()
            .expect("credentials RwLock poisoned");
        *guard = credentials;
    }

    /// Clears active credentials, reverting the client to anonymous mode.
    pub fn clear_credentials(&self) {
        self.set_credentials(Credentials::None);
    }

    /// Builds a full endpoint `Url` preserving any base path prefix.
    pub(crate) fn endpoint_url(&self, path: &str) -> Result<Url> {
        let rel = path.trim_start_matches('/');
        let full = format!("{}/{}", self.base_url, rel);
        Ok(Url::parse(&full)?)
    }

    /// Starts an authenticated `RequestBuilder` for `method` and `path`.
    pub(crate) fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self.endpoint_url(path)?;
        let creds = self.credentials();
        Ok(creds.apply(self.http.request(method, url)))
    }

    /// Starts an unauthenticated `RequestBuilder` for endpoints that do not require active session headers.
    pub(crate) fn unauth_request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self.endpoint_url(path)?;
        Ok(self.http.request(method, url))
    }

    /// Checks HTTP status, returning the `Response` on 2xx or converting the body into [`FastS3Error::Api`].
    pub(crate) async fn ensure_success(resp: Response) -> Result<Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }

        let code = status.as_u16();
        let body_text = resp.text().await.unwrap_or_default();

        if let Ok(parsed) = serde_json::from_str::<ApiErrorPayload>(&body_text) {
            return Err(FastS3Error::Api {
                status: parsed.status.unwrap_or(code),
                message: parsed.error,
            });
        }

        let message = if body_text.trim().is_empty() {
            status
                .canonical_reason()
                .unwrap_or("Unknown HTTP error")
                .to_string()
        } else {
            body_text.trim().to_string()
        };

        Err(FastS3Error::Api {
            status: code,
            message,
        })
    }

    /// Sends a request and deserializes a JSON response body into `T`.
    pub(crate) async fn send_json<T: DeserializeOwned>(builder: RequestBuilder) -> Result<T> {
        let resp = builder.send().await?;
        let ok_resp = Self::ensure_success(resp).await?;
        let bytes = ok_resp.bytes().await?;
        Ok(serde_json::from_slice::<T>(&bytes)?)
    }
}
