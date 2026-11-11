use reqwest::Method;

use crate::auth::Credentials;
use crate::client::FastS3Client;
use crate::error::Result;
use crate::models::{
    AuthVerifyResponse, BootstrapLoginRequest, BootstrapLoginResponse, BootstrapSetupRequest,
    BootstrapSetupResponse, BootstrapStatusResponse, BootstrapTokenResponse, ChangePasswordRequest,
    ChangePasswordResponse, UserLoginRequest, UserLoginResponse,
};

impl FastS3Client {
    /// Validates the currently configured credentials against `GET /api/auth/verify`.
    pub async fn verify_auth(&self) -> Result<AuthVerifyResponse> {
        let req = self.request(Method::GET, "/api/auth/verify")?;
        Self::send_json(req).await
    }

    /// Checks whether initial bootstrap setup has already been completed (`GET /api/auth/bootstrap-status`).
    pub async fn bootstrap_status(&self) -> Result<BootstrapStatusResponse> {
        let req = self.unauth_request(Method::GET, "/api/auth/bootstrap-status")?;
        Self::send_json(req).await
    }

    /// Validates a Bootstrap Token via `POST /api/auth/bootstrap-login` without mutating client credentials.
    pub async fn bootstrap_login(&self, token: &str) -> Result<BootstrapLoginResponse> {
        let payload = BootstrapLoginRequest {
            token: token.to_string(),
        };
        let req = self
            .unauth_request(Method::POST, "/api/auth/bootstrap-login")?
            .json(&payload);
        Self::send_json(req).await
    }

    /// Validates a Bootstrap Token via `POST /api/auth/bootstrap-login` and stores it as Bearer credentials on this client.
    pub async fn bootstrap_login_and_authenticate(
        &self,
        token: &str,
    ) -> Result<BootstrapLoginResponse> {
        let resp = self.bootstrap_login(token).await?;
        self.set_credentials(Credentials::bearer(resp.token.clone()));
        Ok(resp)
    }

    /// Configures the initial administrator account using the Bootstrap Token (`POST /api/auth/bootstrap-setup`).
    pub async fn bootstrap_setup(
        &self,
        payload: &BootstrapSetupRequest,
    ) -> Result<BootstrapSetupResponse> {
        let req = self
            .unauth_request(Method::POST, "/api/auth/bootstrap-setup")?
            .json(payload);
        Self::send_json(req).await
    }

    /// Runs `POST /api/auth/bootstrap-setup` and automatically updates this client with the new Basic Auth credentials.
    pub async fn bootstrap_setup_and_authenticate(
        &self,
        payload: &BootstrapSetupRequest,
    ) -> Result<BootstrapSetupResponse> {
        let resp = self.bootstrap_setup(payload).await?;
        if let Some(creds) = Credentials::from_basic_auth_header(&resp.basic_auth) {
            self.set_credentials(creds);
        } else {
            self.set_credentials(Credentials::basic(&payload.username, &payload.password));
        }
        Ok(resp)
    }

    /// Authenticates a user with `username` and `password` (`POST /api/auth/login`) without mutating client credentials.
    pub async fn login(&self, username: &str, password: &str) -> Result<UserLoginResponse> {
        let payload = UserLoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };
        let req = self
            .unauth_request(Method::POST, "/api/auth/login")?
            .json(&payload);
        Self::send_json(req).await
    }

    /// Authenticates via `POST /api/auth/login` and stores the resulting Basic Auth credentials on this client.
    pub async fn login_and_authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<UserLoginResponse> {
        let resp = self.login(username, password).await?;
        if let Some(creds) = Credentials::from_basic_auth_header(&resp.basic_auth) {
            self.set_credentials(creds);
        } else {
            self.set_credentials(Credentials::basic(username, password));
        }
        Ok(resp)
    }

    /// Changes the active user's password via `POST /api/auth/change-password`.
    pub async fn change_password(
        &self,
        current_password: Option<&str>,
        new_password: &str,
    ) -> Result<ChangePasswordResponse> {
        let payload = ChangePasswordRequest {
            current_password: current_password.map(ToOwned::to_owned),
            new_password: new_password.to_string(),
        };
        let req = self
            .request(Method::POST, "/api/auth/change-password")?
            .json(&payload);
        Self::send_json(req).await
    }

    /// Changes the active user's password and updates the client's stored Basic Auth credentials if applicable.
    pub async fn change_password_and_authenticate(
        &self,
        current_password: Option<&str>,
        new_password: &str,
    ) -> Result<ChangePasswordResponse> {
        let resp = self.change_password(current_password, new_password).await?;
        if let Some(creds) = Credentials::from_basic_auth_header(&resp.basic_auth) {
            self.set_credentials(creds);
        }
        Ok(resp)
    }

    /// Retrieves the server's Bootstrap Token (`GET /api/auth/bootstrap-token`, requires Admin).
    pub async fn get_bootstrap_token(&self) -> Result<Option<String>> {
        let req = self.request(Method::GET, "/api/auth/bootstrap-token")?;
        let resp: BootstrapTokenResponse = Self::send_json(req).await?;
        Ok(resp.bootstrap_token)
    }
}
