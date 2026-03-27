use std::collections::HashMap;
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::auth::OAuthCredential;
use crate::error::{Error, Result};

use super::pkce::PkceChallenge;

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CALLBACK_HOST_V4: &str = "127.0.0.1";
const CALLBACK_HOST_V6: &str = "::1";
const CALLBACK_PORT: u16 = 1455;
const CALLBACK_PATH: &str = "/auth/callback";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPES: &str = "openid profile email offline_access";

const SUCCESS_HTML: &str = "\
<html><body style=\"font-family:system-ui;text-align:center;padding:60px\">\
<h1>&#10003; Logged in</h1>\
<p>You can close this window and return to imp.</p>\
</body></html>";

const ERROR_HTML: &str = "\
<html><body style=\"font-family:system-ui;text-align:center;padding:60px\">\
<h1>Error</h1><p>OAuth state mismatch. Please try again.</p>\
</body></html>";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
}

fn default_expires_in() -> u64 {
    3600
}

/// ChatGPT/OpenAI OAuth handler using the normal local Codex sign-in path.
pub struct ChatGptOAuth {
    client_id: String,
    token_url: String,
}

impl Default for ChatGptOAuth {
    fn default() -> Self {
        Self {
            client_id: CLIENT_ID.to_string(),
            token_url: TOKEN_URL.to_string(),
        }
    }
}

impl ChatGptOAuth {
    /// Create a handler with the production OpenAI OAuth endpoints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a handler with a custom token URL (for tests).
    pub fn with_token_url(token_url: String) -> Self {
        Self {
            client_id: CLIENT_ID.to_string(),
            token_url,
        }
    }

    /// Build the authorization URL for the browser-based local callback flow.
    pub fn build_authorize_url(&self, pkce: &PkceChallenge, state: &str) -> String {
        let mut url = Url::parse(AUTHORIZE_URL).expect("valid authorize URL constant");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("scope", SCOPES)
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("id_token_add_organizations", "true")
            .append_pair("codex_cli_simplified_flow", "true")
            .append_pair("state", state)
            .append_pair("originator", "imp");
        url.to_string()
    }

    /// Exchange an authorization code for access + refresh tokens.
    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuthCredential> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", self.client_id.as_str()),
                ("code_verifier", code_verifier),
            ])
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
        Ok(to_oauth_credential(token, None))
    }

    /// Refresh an expired ChatGPT OAuth token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthCredential> {
        let client = reqwest::Client::new();
        let response = client
            .post(&self.token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", self.client_id.as_str()),
                ("refresh_token", refresh_token),
            ])
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
        Ok(to_oauth_credential(token, Some(refresh_token)))
    }

    /// Full login flow: local callback server + browser sign-in.
    pub async fn login<F, G, Fut>(
        &self,
        open_url: F,
        manual_code_input: G,
    ) -> Result<OAuthCredential>
    where
        F: FnOnce(&str),
        G: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        let pkce = PkceChallenge::generate();
        let state = generate_state();
        let auth_url = self.build_authorize_url(&pkce, &state);

        let server = CallbackServer::bind(CALLBACK_PORT).await?;
        open_url(&auth_url);

        let timeout = Duration::from_secs(300);
        match server.wait_for_code(&state, timeout).await {
            Ok(code) => {
                self.exchange_code(&code, &pkce.verifier, REDIRECT_URI)
                    .await
            }
            Err(_) => {
                let input = manual_code_input()
                    .await
                    .ok_or_else(|| Error::Auth("Login cancelled".into()))?;
                let code = parse_manual_code(&input)?;
                self.exchange_code(&code, &pkce.verifier, REDIRECT_URI)
                    .await
            }
        }
    }

    /// Manual login flow that does not wait on a localhost callback.
    pub async fn login_manual<F, G, Fut>(
        &self,
        open_url: F,
        manual_code_input: G,
    ) -> Result<OAuthCredential>
    where
        F: FnOnce(&str),
        G: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        let pkce = PkceChallenge::generate();
        let state = generate_state();
        let auth_url = self.build_authorize_url(&pkce, &state);
        open_url(&auth_url);

        let input = manual_code_input()
            .await
            .ok_or_else(|| Error::Auth("Login cancelled".into()))?;
        let code = parse_manual_code(&input)?;

        self.exchange_code(&code, &pkce.verifier, REDIRECT_URI)
            .await
    }

    /// Login flow that races the local callback against a manual pasted URL/code.
    ///
    /// This is useful when browsers cannot reach `localhost` callbacks reliably.
    pub async fn login_with_manual_fallback<F, G, Fut>(
        &self,
        open_url: F,
        manual_code_input: G,
    ) -> Result<OAuthCredential>
    where
        F: FnOnce(&str),
        G: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        let pkce = PkceChallenge::generate();
        let state = generate_state();
        let auth_url = self.build_authorize_url(&pkce, &state);
        let server = CallbackServer::bind(CALLBACK_PORT).await?;
        open_url(&auth_url);

        let timeout = Duration::from_secs(300);
        let wait_for_code = server.wait_for_code(&state, timeout);
        tokio::pin!(wait_for_code);

        let manual_input = manual_code_input();
        tokio::pin!(manual_input);

        let code = tokio::select! {
            result = &mut wait_for_code => result?,
            input = &mut manual_input => {
                let input = input.ok_or_else(|| Error::Auth("Login cancelled".into()))?;
                parse_manual_code(&input)?
            }
        };

        self.exchange_code(&code, &pkce.verifier, REDIRECT_URI)
            .await
    }
}

fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn to_oauth_credential(
    token: TokenResponse,
    fallback_refresh_token: Option<&str>,
) -> OAuthCredential {
    let expires_at = crate::now() + token.expires_in.saturating_sub(300);

    OAuthCredential {
        access_token: token.access_token,
        refresh_token: token
            .refresh_token
            .or_else(|| fallback_refresh_token.map(str::to_string))
            .unwrap_or_default(),
        expires_at,
    }
}

/// Parse an authorization code from user input.
pub fn parse_manual_code(input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Error::Auth(
            "Empty input — expected an authorization code or redirect URL".into(),
        ));
    }

    if let Ok(url) = Url::parse(input) {
        if let Some(code) = url
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.to_string())
        {
            if !code.is_empty() {
                return Ok(code);
            }
        }
    }

    if !input.contains(char::is_whitespace) {
        return Ok(input.to_string());
    }

    Err(Error::Auth(
        "Could not parse authorization code from input".into(),
    ))
}

pub struct CallbackServer {
    listener_v4: TcpListener,
    listener_v6: Option<TcpListener>,
    pub port: u16,
}

impl CallbackServer {
    pub async fn bind(port: u16) -> Result<Self> {
        let listener_v4 = TcpListener::bind(format!("{CALLBACK_HOST_V4}:{port}")).await?;
        let port = listener_v4.local_addr()?.port();
        let listener_v6 = TcpListener::bind(format!("[{CALLBACK_HOST_V6}]:{port}"))
            .await
            .ok();
        Ok(Self {
            listener_v4,
            listener_v6,
            port,
        })
    }

    pub async fn wait_for_code(self, expected_state: &str, timeout: Duration) -> Result<String> {
        let accept = async {
            if let Some(listener_v6) = self.listener_v6 {
                tokio::select! {
                    result = self.listener_v4.accept() => result,
                    result = listener_v6.accept() => result,
                }
            } else {
                self.listener_v4.accept().await
            }
        };

        let (mut stream, _) = tokio::time::timeout(timeout, accept)
            .await
            .map_err(|_| Error::Auth("Timeout waiting for OAuth callback".into()))?
            .map_err(|e| Error::Auth(format!("Failed to accept callback connection: {e}")))?;

        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await?;
        let request = String::from_utf8_lossy(&buf[..n]);

        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .ok_or_else(|| Error::Auth("Invalid HTTP request in callback".into()))?;

        if !path.starts_with(CALLBACK_PATH) {
            let _ = send_response(&mut stream, 404, "Not Found").await;
            return Err(Error::Auth(format!("Unexpected path: {path}")));
        }

        let url = Url::parse(&format!("http://localhost{path}"))
            .map_err(|e| Error::Auth(format!("Failed to parse callback URL: {e}")))?;

        let params: HashMap<String, String> = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let code = params
            .get("code")
            .filter(|code| !code.is_empty())
            .ok_or_else(|| Error::Auth("No authorization code in callback".into()))?;

        let state = params
            .get("state")
            .ok_or_else(|| Error::Auth("No state parameter in callback".into()))?;

        if state != expected_state {
            let _ = send_response(&mut stream, 400, ERROR_HTML).await;
            return Err(Error::Auth("State mismatch in OAuth callback".into()));
        }

        send_response(&mut stream, 200, SUCCESS_HTML).await?;
        Ok(code.clone())
    }
}

async fn send_response(stream: &mut tokio::net::TcpStream, status: u16, body: &str) -> Result<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\n\
         Content-Type: text/html\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener as TokioListener;

    #[test]
    fn build_authorize_url_includes_codex_params() {
        let oauth = ChatGptOAuth::new();
        let pkce = PkceChallenge::generate();
        let state = "test-state";
        let url = Url::parse(&oauth.build_authorize_url(&pkce, state)).unwrap();

        assert_eq!(url.host_str(), Some("auth.openai.com"));
        assert_eq!(url.path(), "/oauth/authorize");

        let params: HashMap<String, String> = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        assert_eq!(params.get("client_id").unwrap(), CLIENT_ID);
        assert_eq!(params.get("response_type").unwrap(), "code");
        assert_eq!(params.get("redirect_uri").unwrap(), REDIRECT_URI);
        assert_eq!(params.get("scope").unwrap(), SCOPES);
        assert_eq!(params.get("code_challenge").unwrap(), &pkce.challenge);
        assert_eq!(params.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(params.get("state").unwrap(), state);
        assert_eq!(params.get("id_token_add_organizations").unwrap(), "true");
        assert_eq!(params.get("codex_cli_simplified_flow").unwrap(), "true");
        assert_eq!(params.get("originator").unwrap(), "imp");
    }

    #[test]
    fn parse_manual_code_accepts_raw_code() {
        assert_eq!(parse_manual_code("abc123").unwrap(), "abc123");
    }

    #[test]
    fn parse_manual_code_accepts_callback_url() {
        let input = "http://localhost:1455/auth/callback?code=my-code&state=test-state";
        assert_eq!(parse_manual_code(input).unwrap(), "my-code");
    }

    #[test]
    fn parse_manual_code_rejects_whitespace() {
        assert!(parse_manual_code("has spaces").is_err());
    }

    #[tokio::test]
    async fn callback_server_receives_code() {
        let server = CallbackServer::bind(0).await.unwrap();
        let port = server.port;
        let expected_state = "state-123";

        let task = tokio::spawn(async move {
            server
                .wait_for_code(expected_state, Duration::from_secs(5))
                .await
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        let request = format!(
            "GET /auth/callback?code=auth-code-123&state={expected_state} HTTP/1.1\r\n\
             Host: localhost:{port}\r\n\
             \r\n"
        );
        client.write_all(request.as_bytes()).await.unwrap();

        let mut response = vec![0u8; 4096];
        let n = client.read(&mut response).await.unwrap();
        let response = String::from_utf8_lossy(&response[..n]);
        assert!(response.contains("200 OK"));
        assert!(response.contains("Logged in"));

        let code = task.await.unwrap().unwrap();
        assert_eq!(code, "auth-code-123");
    }

    async fn start_mock_token_server() -> (TokioListener, u16) {
        let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    async fn serve_token_response(listener: TokioListener, status: u16, body: String) {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let _ = stream.read(&mut buf).await.unwrap();

        let status_text = if status == 200 { "OK" } else { "Bad Request" };
        let response = format!(
            "HTTP/1.1 {status} {status_text}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
    }

    #[tokio::test]
    async fn exchange_code_returns_tokens() {
        let body = serde_json::json!({
            "access_token": "openai-access-token",
            "refresh_token": "openai-refresh-token",
            "expires_in": 3600
        })
        .to_string();
        let (listener, port) = start_mock_token_server().await;
        tokio::spawn(serve_token_response(listener, 200, body));

        let oauth = ChatGptOAuth::with_token_url(format!("http://127.0.0.1:{port}/oauth/token"));
        let cred = oauth
            .exchange_code("auth-code-123", "verifier-xyz", REDIRECT_URI)
            .await
            .unwrap();

        assert_eq!(cred.access_token, "openai-access-token");
        assert_eq!(cred.refresh_token, "openai-refresh-token");
    }

    #[tokio::test]
    async fn login_manual_accepts_pasted_callback_url() {
        let body = serde_json::json!({
            "access_token": "openai-access-token",
            "refresh_token": "openai-refresh-token",
            "expires_in": 3600
        })
        .to_string();
        let (listener, port) = start_mock_token_server().await;
        tokio::spawn(serve_token_response(listener, 200, body));

        let oauth = ChatGptOAuth::with_token_url(format!("http://127.0.0.1:{port}/oauth/token"));
        let printed_url = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let printed_url_clone = printed_url.clone();
        let credential = oauth
            .login_manual(
                move |url| {
                    *printed_url_clone.lock().unwrap() = url.to_string();
                },
                || async {
                    Some("http://localhost:1455/auth/callback?code=my-code&state=test-state".into())
                },
            )
            .await
            .unwrap();

        assert!(printed_url
            .lock()
            .unwrap()
            .contains("auth.openai.com/oauth/authorize"));
        assert_eq!(credential.access_token, "openai-access-token");
        assert_eq!(credential.refresh_token, "openai-refresh-token");
    }

    #[tokio::test]
    async fn refresh_token_keeps_old_refresh_when_none_returned() {
        let body = serde_json::json!({
            "access_token": "openai-refreshed",
            "expires_in": 3600
        })
        .to_string();
        let (listener, port) = start_mock_token_server().await;
        tokio::spawn(serve_token_response(listener, 200, body));

        let oauth = ChatGptOAuth::with_token_url(format!("http://127.0.0.1:{port}/oauth/token"));
        let tokens = oauth.refresh_token("keep-me").await.unwrap();

        assert_eq!(tokens.access_token, "openai-refreshed");
        assert_eq!(tokens.refresh_token, "keep-me");
    }
}
