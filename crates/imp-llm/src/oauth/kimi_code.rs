use std::collections::HashMap;
use std::time::Duration;

use rand::RngCore;
use serde::Deserialize;

use crate::auth::OAuthCredential;
use crate::error::{Error, Result};

const CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
const DEVICE_AUTH_URL: &str = "https://auth.kimi.com/api/oauth/device_authorization";
const TOKEN_URL: &str = "https://auth.kimi.com/api/oauth/token";

/// Kimi Code OAuth handler using OAuth 2.0 device flow.
pub struct KimiCodeOAuth {
    client_id: String,
    token_url: String,
    device_auth_url: String,
}

impl Default for KimiCodeOAuth {
    fn default() -> Self {
        Self {
            client_id: CLIENT_ID.to_string(),
            token_url: TOKEN_URL.to_string(),
            device_auth_url: DEVICE_AUTH_URL.to_string(),
        }
    }
}

impl KimiCodeOAuth {
    /// Create with production Kimi Code OAuth endpoints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom endpoints (for testing with a mock server).
    pub fn with_endpoints(device_auth_url: String, token_url: String) -> Self {
        Self {
            client_id: CLIENT_ID.to_string(),
            token_url,
            device_auth_url,
        }
    }

    /// Request a device authorization.
    pub async fn request_device_authorization(&self) -> Result<DeviceAuthorization> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.device_auth_url)
            .form(&[("client_id", self.client_id.as_str())])
            .headers(common_headers())
            .send()
            .await?;

        let status = response.status();
        let data: serde_json::Value = response.json().await?;

        if !status.is_success() {
            return Err(Error::Auth(format!(
                "Device authorization failed ({status}): {data}"
            )));
        }

        Ok(DeviceAuthorization {
            user_code: data["user_code"].as_str().unwrap_or("").to_string(),
            device_code: data["device_code"].as_str().unwrap_or("").to_string(),
            verification_uri: data["verification_uri"].as_str().unwrap_or("").to_string(),
            verification_uri_complete: data["verification_uri_complete"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            expires_in: data["expires_in"].as_u64(),
            interval: data["interval"].as_u64().unwrap_or(5).max(1),
        })
    }

    /// Poll the token endpoint for a device code.
    pub async fn request_device_token(
        &self,
        device_code: &str,
    ) -> Result<(u16, HashMap<String, serde_json::Value>)> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.token_url)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .headers(common_headers())
            .send()
            .await?;

        let status = response.status();
        let data: HashMap<String, serde_json::Value> = response.json().await?;
        Ok((status.as_u16(), data))
    }

    /// Exchange an authorization code for access + refresh tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthCredential> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", self.client_id.as_str()),
                ("code", code),
            ])
            .headers(common_headers())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Auth(format!(
                "Token exchange failed ({status}): {body}"
            )));
        }

        let token: TokenResponse = response.json().await?;
        Ok(to_oauth_credential(token))
    }

    /// Refresh an expired OAuth token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthCredential> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", self.client_id.as_str()),
                ("refresh_token", refresh_token),
            ])
            .headers(common_headers())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Auth(format!(
                "Token refresh failed ({status}): {body}"
            )));
        }

        let token: TokenResponse = response.json().await?;
        Ok(to_oauth_credential(token))
    }

    /// Full device-flow login: get device code, open browser, poll for token.
    ///
    /// `open_url` is called with the verification URL to open in the browser.
    /// `print_message` is called with status messages for the user.
    pub async fn login<F, G>(&self, open_url: F, mut print_message: G) -> Result<OAuthCredential>
    where
        F: FnOnce(&str),
        G: FnMut(&str),
    {
        let auth = self.request_device_authorization().await?;

        print_message("Please visit the following URL to finish authorization:");
        print_message(&format!(
            "Verification URL: {}",
            auth.verification_uri_complete
        ));
        open_url(&auth.verification_uri_complete);

        let interval = Duration::from_secs(auth.interval);
        let max_duration = auth
            .expires_in
            .map(|s| Duration::from_secs(s))
            .unwrap_or_else(|| Duration::from_secs(600));
        let start = std::time::Instant::now();
        let mut printed_wait = false;

        while start.elapsed() < max_duration {
            let (status, data) = self.request_device_token(&auth.device_code).await?;

            if status == 200 && data.get("access_token").is_some() {
                let token: TokenResponse = serde_json::from_value(
                    serde_json::to_value(&data).map_err(|e| Error::Auth(e.to_string()))?,
                )?;
                return Ok(to_oauth_credential(token));
            }

            if let Some(error) = data.get("error").and_then(|v| v.as_str()) {
                if error == "expired_token" {
                    return Err(Error::Auth(
                        "Device authorization expired. Please try again.".into(),
                    ));
                }
                if error == "authorization_pending" {
                    if !printed_wait {
                        print_message("Waiting for user authorization...");
                        printed_wait = true;
                    }
                } else {
                    let desc = data
                        .get("error_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(error);
                    return Err(Error::Auth(format!("OAuth error: {desc}")));
                }
            }

            tokio::time::sleep(interval).await;
        }

        Err(Error::Auth(
            "Device authorization timed out. Please try again.".into(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct DeviceAuthorization {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: Option<u64>,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: f64,
    #[allow(dead_code)]
    #[serde(default)]
    scope: String,
    #[allow(dead_code)]
    #[serde(default)]
    token_type: String,
}

fn to_oauth_credential(token: TokenResponse) -> OAuthCredential {
    let expires_in = token.expires_in as u64;
    let expires_at = crate::now() + expires_in.saturating_sub(300);
    OAuthCredential {
        access_token: token.access_token,
        refresh_token: token.refresh_token.unwrap_or_default(),
        expires_at,
    }
}

/// Build the standard Kimi Code request headers.
///
/// These headers match what kimi-cli sends so that the API accepts
/// requests from imp as a recognized coding agent.
pub fn common_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("KimiCLI/1.39.0"),
    );
    headers.insert(
        "X-Msh-Platform",
        reqwest::header::HeaderValue::from_static("kimi_cli"),
    );
    headers.insert(
        "X-Msh-Version",
        reqwest::header::HeaderValue::from_static("1.39.0"),
    );
    headers.insert(
        "X-Msh-Device-Name",
        reqwest::header::HeaderValue::from_str(&hostname())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("unknown")),
    );
    headers.insert(
        "X-Msh-Device-Model",
        reqwest::header::HeaderValue::from_str(&device_model())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("unknown")),
    );
    headers.insert(
        "X-Msh-Os-Version",
        reqwest::header::HeaderValue::from_str(&os_version())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("unknown")),
    );
    headers.insert(
        "X-Msh-Device-Id",
        reqwest::header::HeaderValue::from_str(&device_id())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("unknown")),
    );
    headers
}

fn hostname() -> String {
    #[cfg(unix)]
    {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    }
    #[cfg(not(unix))]
    {
        "unknown".to_string()
    }
}

fn device_model() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("{} {}", os, arch)
}

fn os_version() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| std::env::consts::OS.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::consts::OS.to_string()
    }
}

fn device_id() -> String {
    // If the user already has kimi-cli installed, reuse its device id so
    // that imported tokens (whose JWT contains that device_id) match the
    // headers we send to the API.
    if let Some(ref p) = std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".kimi").join("device_id"))
    {
        if let Ok(id) = std::fs::read_to_string(p) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    // Fall back to imp's own device id.
    if let Some(ref p) =
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".imp").join("device_id"))
    {
        if let Ok(id) = std::fs::read_to_string(p) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let id = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();

    // Persist the newly generated id in imp's own directory.
    if let Some(ref p) =
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".imp").join("device_id"))
    {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, &id);
    }

    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener as TokioListener;

    async fn start_mock_listener() -> (TokioListener, u16) {
        let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    async fn serve_once(listener: TokioListener, status: u16, body: String) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let _ = stream.read(&mut buf).await.unwrap();
        let status_text = if status == 200 { "OK" } else { "Error" };
        let response = format!(
            "HTTP/1.1 {status} {status_text}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n\
             {body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_request_device_authorization() {
        let body = serde_json::json!({
            "user_code": "ABCD-EFGH",
            "device_code": "dev-123",
            "verification_uri": "https://auth.kimi.com/verify",
            "verification_uri_complete": "https://auth.kimi.com/verify?code=ABCD-EFGH",
            "expires_in": 600,
            "interval": 5
        })
        .to_string();

        let (listener, port) = start_mock_listener().await;
        tokio::spawn(serve_once(listener, 200, body));

        let oauth = KimiCodeOAuth::with_endpoints(
            format!("http://127.0.0.1:{port}/device"),
            format!("http://127.0.0.1:{port}/token"),
        );
        let auth = oauth.request_device_authorization().await.unwrap();
        assert_eq!(auth.user_code, "ABCD-EFGH");
        assert_eq!(auth.device_code, "dev-123");
        assert_eq!(auth.interval, 5);
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let body = serde_json::json!({
            "access_token": "new-access-token",
            "refresh_token": "new-refresh-token",
            "expires_in": 3600,
            "scope": "kimi-code",
            "token_type": "Bearer"
        })
        .to_string();

        let (listener, port) = start_mock_listener().await;
        tokio::spawn(serve_once(listener, 200, body));

        let oauth = KimiCodeOAuth::with_endpoints(
            format!("http://127.0.0.1:{port}/device"),
            format!("http://127.0.0.1:{port}/token"),
        );
        let cred = oauth.refresh_token("old-refresh").await.unwrap();
        assert_eq!(cred.access_token, "new-access-token");
        assert_eq!(cred.refresh_token, "new-refresh-token");
    }

    #[tokio::test]
    async fn test_token_response_with_float_expires_in() {
        let body = serde_json::json!({
            "access_token": "test-token",
            "refresh_token": "test-refresh",
            "expires_in": 900.0,
            "scope": "kimi-code",
            "token_type": "Bearer"
        })
        .to_string();

        let (listener, port) = start_mock_listener().await;
        tokio::spawn(serve_once(listener, 200, body));

        let oauth = KimiCodeOAuth::with_endpoints(
            format!("http://127.0.0.1:{port}/device"),
            format!("http://127.0.0.1:{port}/token"),
        );
        let cred = oauth.refresh_token("old-refresh").await.unwrap();
        assert_eq!(cred.access_token, "test-token");
        assert_eq!(cred.refresh_token, "test-refresh");
        // expires_at should be roughly now + 900 - 300 = now + 600
        let expected_min = crate::now() + 500;
        let expected_max = crate::now() + 700;
        assert!(
            cred.expires_at >= expected_min && cred.expires_at <= expected_max,
            "expires_at {} not in range [{}, {}]",
            cred.expires_at,
            expected_min,
            expected_max
        );
    }
}
