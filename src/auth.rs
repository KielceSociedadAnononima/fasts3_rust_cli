use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use reqwest::RequestBuilder;

/// Authentication credentials supported by FastS3.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Credentials {
    /// Unauthenticated (anonymous) access — suitable for public buckets and login/bootstrap endpoints.
    #[default]
    None,

    /// HTTP Basic Authentication (`Authorization: Basic <base64(username:password)>`).
    Basic { username: String, password: String },

    /// Custom FastS3 user/password headers (`X-FastS3-User` and `X-FastS3-Pass`).
    CustomHeaders { username: String, password: String },

    /// HTTP Bearer Token (`Authorization: Bearer <token>`), valid for both Bootstrap Tokens and `fs3_...` API Keys.
    Bearer(String),

    /// User-scoped API Key header (`X-API-Key: <api_key>`).
    ApiKey(String),

    /// FastS3 token header (`X-FastS3-Token: <token>`).
    FastS3Token(String),
}

impl Credentials {
    /// Creates `Credentials::Basic` from username and password.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Creates `Credentials::CustomHeaders` (`X-FastS3-User` / `X-FastS3-Pass`).
    pub fn custom_headers(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::CustomHeaders {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Creates `Credentials::Bearer` for a Bootstrap Token or API Key.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }

    /// Creates `Credentials::ApiKey` (`X-API-Key` header).
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey(key.into())
    }

    /// Creates `Credentials::FastS3Token` (`X-FastS3-Token` header).
    pub fn fasts3_token(token: impl Into<String>) -> Self {
        Self::FastS3Token(token.into())
    }

    /// Parses a `"Basic <base64>"` header value returned by `/api/auth/login` or `/api/auth/bootstrap-setup`.
    pub fn from_basic_auth_header(header_value: &str) -> Option<Self> {
        let trimmed = header_value.trim();
        let encoded = trimmed
            .strip_prefix("Basic ")
            .or_else(|| trimmed.strip_prefix("basic "))
            .unwrap_or(trimmed)
            .trim();
        let decoded = BASE64_STANDARD.decode(encoded).ok()?;
        let creds_str = String::from_utf8(decoded).ok()?;
        let (username, password) = creds_str.split_once(':')?;
        Some(Self::Basic {
            username: username.to_string(),
            password: password.to_string(),
        })
    }

    /// Applies authentication headers to an outgoing `reqwest::RequestBuilder`.
    pub(crate) fn apply(&self, builder: RequestBuilder) -> RequestBuilder {
        match self {
            Credentials::None => builder,
            Credentials::Basic { username, password } => {
                let encoded = BASE64_STANDARD.encode(format!("{username}:{password}"));
                builder.header("Authorization", format!("Basic {encoded}"))
            }
            Credentials::CustomHeaders { username, password } => builder
                .header("X-FastS3-User", username)
                .header("X-FastS3-Pass", password),
            Credentials::Bearer(token) => {
                builder.header("Authorization", format!("Bearer {}", token.trim()))
            }
            Credentials::ApiKey(key) => builder.header("X-API-Key", key.trim()),
            Credentials::FastS3Token(token) => builder.header("X-FastS3-Token", token.trim()),
        }
    }

    /// Returns a query parameter pair `(key, raw_value)` suitable for FastS3's URL query authentication
    /// (`?token=...`, `?api_key=...`, or `?auth=<base64>`).
    pub(crate) fn to_query_param(&self) -> Option<(&'static str, String)> {
        match self {
            Credentials::None => None,
            Credentials::Basic { username, password }
            | Credentials::CustomHeaders { username, password } => {
                let encoded = BASE64_STANDARD.encode(format!("{username}:{password}"));
                Some(("auth", encoded))
            }
            Credentials::Bearer(token) | Credentials::FastS3Token(token) => {
                Some(("token", token.trim().to_string()))
            }
            Credentials::ApiKey(key) => Some(("api_key", key.trim().to_string())),
        }
    }
}
