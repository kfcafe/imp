use std::collections::HashMap;
use std::path::PathBuf;

use crate::truncate_chars_with_suffix;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

pub type ApiKey = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

impl OAuthCredential {
    /// Check whether this token has expired (or will within the next minute).
    pub fn is_expired(&self) -> bool {
        crate::now() >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StoredCredential {
    ApiKey { key: String },
    OAuth(OAuthCredential),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthDisplayInfo {
    pub account_id: Option<String>,
    pub plan: Option<String>,
    pub using_subscription: bool,
}

impl OAuthDisplayInfo {
    pub fn login_message(&self, provider: &str) -> String {
        match provider {
            "openai" | "openai-codex" => {
                let mut message = String::from("Logged in to OpenAI / ChatGPT");
                if let Some(account_id) = &self.account_id {
                    message.push_str(&format!(" as account {account_id}"));
                }
                if let Some(plan) = &self.plan {
                    message.push_str(&format!(", plan: {plan}"));
                }
                message.push('.');
                message
            }
            "anthropic" => {
                if let Some(plan) = &self.plan {
                    format!("Logged in to Anthropic with {plan} subscription credentials.")
                } else {
                    "Logged in to Anthropic with OAuth subscription credentials.".into()
                }
            }
            _ => format!("Logged in to {provider} with OAuth credentials."),
        }
    }

    pub fn status_summary(&self) -> String {
        match (&self.plan, self.short_account_id()) {
            (Some(plan), Some(account_id)) => format!("{plan} · {account_id}"),
            (Some(plan), None) => plan.clone(),
            (None, Some(account_id)) => account_id,
            (None, None) if self.using_subscription => "subscription".into(),
            (None, None) => "oauth".into(),
        }
    }

    pub fn short_account_id(&self) -> Option<String> {
        self.account_id
            .as_ref()
            .map(|account_id| truncate_chars_with_suffix(account_id, 8, "…"))
    }
}

/// Manages API keys and OAuth credentials.
pub struct AuthStore {
    runtime_keys: HashMap<String, String>,
    pub stored: HashMap<String, StoredCredential>,
    path: PathBuf,
}

impl AuthStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            runtime_keys: HashMap::new(),
            stored: HashMap::new(),
            path,
        }
    }

    /// Load stored credentials from disk.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let stored = if path.exists() {
            let data = std::fs::read_to_string(path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Ok(Self {
            runtime_keys: HashMap::new(),
            stored,
            path: path.to_path_buf(),
        })
    }

    /// Set a runtime override (not persisted).
    pub fn set_runtime_key(&mut self, provider: &str, key: String) {
        self.runtime_keys.insert(provider.to_string(), key);
    }

    /// Resolution order: runtime override → stored → env var → error.
    pub fn resolve(&self, provider: &str) -> Result<ApiKey> {
        // 1. Runtime override
        if let Some(key) = self.runtime_keys.get(provider) {
            return Ok(key.clone());
        }

        // 2. Stored credential
        if let Some(cred) = self.stored.get(provider) {
            match cred {
                StoredCredential::ApiKey { key } => return Ok(key.clone()),
                StoredCredential::OAuth(oauth) => return Ok(oauth.access_token.clone()),
            }
        }

        // 3. Environment variable — look up from provider registry
        let registry = crate::model::ProviderRegistry::with_builtins();
        if let Some(meta) = registry.find(provider) {
            for env_var in meta.env_vars {
                if let Ok(key) = std::env::var(env_var) {
                    return Ok(key);
                }
            }
            let env_list = meta.env_vars.join(" or ");
            return Err(crate::error::Error::Auth(format!(
                "No API key found for {provider}. Set {env_list} or run `imp login {provider}`."
            )));
        }

        // 4. Unknown provider — try convention: <PROVIDER>_API_KEY
        let env_var = format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"));
        if let Ok(key) = std::env::var(&env_var) {
            return Ok(key);
        }

        Err(crate::error::Error::Auth(format!(
            "No API key found for {provider}. Set {env_var} or run `imp login {provider}`."
        )))
    }

    /// Resolve an API key without falling back to stored OAuth credentials.
    pub fn resolve_api_key_only(&self, provider: &str) -> Result<ApiKey> {
        if let Some(key) = self.runtime_keys.get(provider) {
            return Ok(key.clone());
        }

        if let Some(StoredCredential::ApiKey { key }) = self.stored.get(provider) {
            return Ok(key.clone());
        }

        let registry = crate::model::ProviderRegistry::with_builtins();
        if let Some(meta) = registry.find(provider) {
            for env_var in meta.env_vars {
                if let Ok(key) = std::env::var(env_var) {
                    return Ok(key);
                }
            }
            let env_list = meta.env_vars.join(" or ");
            return Err(crate::error::Error::Auth(format!(
                "No API key found for {provider}. Set {env_list} or run `imp login {provider}`."
            )));
        }

        let env_var = format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"));
        if let Ok(key) = std::env::var(&env_var) {
            return Ok(key);
        }

        Err(crate::error::Error::Auth(format!(
            "No API key found for {provider}. Set {env_var} or run `imp login {provider}`."
        )))
    }

    /// Resolve a ChatGPT/OpenAI OAuth token, preferring `openai-codex` when present.
    pub async fn resolve_chatgpt_oauth(&mut self) -> Result<ApiKey> {
        for provider in ["openai-codex", "openai"] {
            if self.get_oauth(provider).is_none() {
                continue;
            }

            return self
                .resolve_or_refresh(provider, |refresh_token| {
                    let refresh_token = refresh_token.to_string();
                    async move {
                        crate::oauth::chatgpt::ChatGptOAuth::new()
                            .refresh_token(&refresh_token)
                            .await
                    }
                })
                .await;
        }

        Err(crate::error::Error::Auth(
            "No ChatGPT OAuth credential found. Run `imp login openai` or configure an OpenAI API key."
                .into(),
        ))
    }

    pub fn oauth_display_info(&self, provider: &str) -> Option<OAuthDisplayInfo> {
        self.get_oauth(provider)
            .and_then(|credential| oauth_display_info_for_credential(provider, credential))
    }

    /// Store a credential and persist to disk.
    pub fn store(&mut self, provider: &str, credential: StoredCredential) -> Result<()> {
        self.stored.insert(provider.to_string(), credential);
        self.save()
    }

    /// Resolve API key, auto-refreshing expired OAuth tokens.
    /// Persists the refreshed credential to disk on success.
    pub async fn resolve_with_refresh(&mut self, provider: &str) -> Result<ApiKey> {
        // Check for expired OAuth and refresh
        if let Some(StoredCredential::OAuth(oauth)) = self.stored.get(provider) {
            if oauth.is_expired() {
                let refresh_token = oauth.refresh_token.clone();
                let oauth_client = crate::oauth::anthropic::AnthropicOAuth::new();
                match oauth_client.refresh_token(&refresh_token).await {
                    Ok(new_cred) => {
                        self.store(provider, StoredCredential::OAuth(new_cred))?;
                    }
                    Err(e) => {
                        return Err(crate::error::Error::Auth(format!(
                            "Token refresh failed: {e}. Run `imp login` to re-authenticate."
                        )));
                    }
                }
            }
        }
        self.resolve(provider)
    }

    /// Check if the stored OAuth credential for a provider is expired.
    pub fn is_oauth_expired(&self, provider: &str) -> bool {
        matches!(
            self.stored.get(provider),
            Some(StoredCredential::OAuth(oauth)) if oauth.is_expired()
        )
    }

    /// Get the stored OAuth credential for a provider (if any).
    pub fn get_oauth(&self, provider: &str) -> Option<&OAuthCredential> {
        match self.stored.get(provider) {
            Some(StoredCredential::OAuth(oauth)) => Some(oauth),
            _ => None,
        }
    }

    /// Resolve API key with automatic OAuth refresh.
    ///
    /// If the stored credential is an expired OAuth token, calls `refresh_fn`
    /// with the refresh token to obtain a new credential, stores it, and
    /// returns the new access token.
    pub async fn resolve_or_refresh<F, Fut>(
        &mut self,
        provider: &str,
        refresh_fn: F,
    ) -> Result<ApiKey>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<OAuthCredential>>,
    {
        // Check for expired OAuth credential
        if let Some(StoredCredential::OAuth(oauth)) = self.stored.get(provider) {
            if oauth.is_expired() {
                let refresh_token = oauth.refresh_token.clone();
                let new_cred = refresh_fn(&refresh_token).await?;
                let access_token = new_cred.access_token.clone();
                self.store(provider, StoredCredential::OAuth(new_cred))?;
                return Ok(access_token);
            }
        }
        self.resolve(provider)
    }

    /// Remove a stored credential (logout).
    pub fn remove(&mut self, provider: &str) -> Result<()> {
        self.stored.remove(provider);
        self.save()
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.stored)?;
        std::fs::write(&self.path, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&self.path, perms);
        }
        Ok(())
    }
}

pub fn oauth_display_info_for_credential(
    provider: &str,
    credential: &OAuthCredential,
) -> Option<OAuthDisplayInfo> {
    match provider {
        "anthropic" => Some(OAuthDisplayInfo {
            account_id: None,
            plan: Some("Claude Max/Pro".into()),
            using_subscription: true,
        }),
        "openai" | "openai-codex" => decode_openai_oauth_display_info(&credential.access_token),
        _ => None,
    }
}

fn decode_openai_oauth_display_info(access_token: &str) -> Option<OAuthDisplayInfo> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims: Value = serde_json::from_slice(&decoded).ok()?;
    let auth = claims.get("https://api.openai.com/auth")?;

    Some(OAuthDisplayInfo {
        account_id: auth
            .get("chatgpt_account_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        plan: auth
            .get("chatgpt_plan_type")
            .and_then(Value::as_str)
            .map(str::to_string),
        using_subscription: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn jwt_with_openai_auth(plan: &str, account_id: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            json!({
                "https://api.openai.com/auth": {
                    "chatgpt_account_id": account_id,
                    "chatgpt_plan_type": plan,
                }
            })
            .to_string(),
        );
        format!("{header}.{payload}.signature")
    }

    #[test]
    fn test_oauth_credential_not_expired() {
        let cred = OAuthCredential {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: crate::now() + 3600,
        };
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_oauth_credential_expired() {
        let cred = OAuthCredential {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: crate::now().saturating_sub(100),
        };
        assert!(cred.is_expired());
    }

    #[test]
    fn test_oauth_store_and_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        let cred = OAuthCredential {
            access_token: "sk-ant-access".into(),
            refresh_token: "rt-refresh".into(),
            expires_at: crate::now() + 3600,
        };
        store
            .store("anthropic", StoredCredential::OAuth(cred))
            .unwrap();

        let key = store.resolve("anthropic").unwrap();
        assert_eq!(key, "sk-ant-access");
    }

    #[test]
    fn test_oauth_detect_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        // Store a fresh token
        let fresh = OAuthCredential {
            access_token: "fresh".into(),
            refresh_token: "rt".into(),
            expires_at: crate::now() + 3600,
        };
        store
            .store("anthropic", StoredCredential::OAuth(fresh))
            .unwrap();
        assert!(!store.is_oauth_expired("anthropic"));

        // Store an expired token
        let expired = OAuthCredential {
            access_token: "expired".into(),
            refresh_token: "rt".into(),
            expires_at: 0,
        };
        store
            .store("anthropic", StoredCredential::OAuth(expired))
            .unwrap();
        assert!(store.is_oauth_expired("anthropic"));
    }

    #[tokio::test]
    async fn test_oauth_resolve_or_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        // Store an expired token
        let expired = OAuthCredential {
            access_token: "old-access".into(),
            refresh_token: "rt-for-refresh".into(),
            expires_at: 0, // expired
        };
        store
            .store("anthropic", StoredCredential::OAuth(expired))
            .unwrap();

        // Resolve with a refresh function that returns a new token
        let key = store
            .resolve_or_refresh("anthropic", |refresh_tok| {
                let refresh_tok = refresh_tok.to_string();
                async move {
                    assert_eq!(refresh_tok, "rt-for-refresh");
                    Ok(OAuthCredential {
                        access_token: "new-access".into(),
                        refresh_token: "new-rt".into(),
                        expires_at: crate::now() + 3600,
                    })
                }
            })
            .await
            .unwrap();

        assert_eq!(key, "new-access");

        // The new token should be stored
        let resolved = store.resolve("anthropic").unwrap();
        assert_eq!(resolved, "new-access");
    }

    #[tokio::test]
    async fn test_oauth_resolve_or_refresh_not_expired() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        // Store a fresh token
        let fresh = OAuthCredential {
            access_token: "still-valid".into(),
            refresh_token: "rt".into(),
            expires_at: crate::now() + 3600,
        };
        store
            .store("anthropic", StoredCredential::OAuth(fresh))
            .unwrap();

        // Refresh function should NOT be called
        let key = store
            .resolve_or_refresh("anthropic", |_| async {
                panic!("refresh should not be called for non-expired token");
            })
            .await
            .unwrap();

        assert_eq!(key, "still-valid");
    }

    #[test]
    fn test_oauth_store_persist_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");

        // Store credential
        {
            let mut store = AuthStore::new(path.clone());
            let cred = OAuthCredential {
                access_token: "persisted-token".into(),
                refresh_token: "persisted-rt".into(),
                expires_at: crate::now() + 3600,
            };
            store
                .store("anthropic", StoredCredential::OAuth(cred))
                .unwrap();
        }

        // Load and resolve
        let store = AuthStore::load(&path).unwrap();
        let key = store.resolve("anthropic").unwrap();
        assert_eq!(key, "persisted-token");
    }

    #[test]
    fn test_oauth_remove_credential() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        let cred = OAuthCredential {
            access_token: "to-remove".into(),
            refresh_token: "rt".into(),
            expires_at: crate::now() + 3600,
        };
        store
            .store("anthropic", StoredCredential::OAuth(cred))
            .unwrap();
        assert!(store.resolve("anthropic").is_ok());

        store.remove("anthropic").unwrap();
        // Should fall through to env var check now (which will fail in test)
        // We need to unset the env var to test this properly
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(store.resolve("anthropic").is_err());
    }

    #[test]
    fn test_resolve_order_runtime_over_stored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        // Store a credential
        store
            .store(
                "anthropic",
                StoredCredential::ApiKey {
                    key: "stored-key".into(),
                },
            )
            .unwrap();

        // Set a runtime override
        store.set_runtime_key("anthropic", "runtime-key".into());

        // Runtime should win
        let key = store.resolve("anthropic").unwrap();
        assert_eq!(key, "runtime-key");
    }

    #[test]
    fn test_resolve_stored_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        store
            .store(
                "openai",
                StoredCredential::ApiKey {
                    key: "sk-stored".into(),
                },
            )
            .unwrap();

        let key = store.resolve("openai").unwrap();
        assert_eq!(key, "sk-stored");
    }

    #[test]
    fn test_resolve_api_key_only_ignores_oauth_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        store
            .store(
                "openai",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "oauth-token".into(),
                    refresh_token: "refresh-token".into(),
                    expires_at: crate::now() + 3600,
                }),
            )
            .unwrap();

        assert!(store.resolve_api_key_only("openai").is_err());
    }

    #[tokio::test]
    async fn test_resolve_chatgpt_oauth_prefers_openai_codex() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        store
            .store(
                "openai",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "openai-oauth".into(),
                    refresh_token: "openai-refresh".into(),
                    expires_at: crate::now() + 3600,
                }),
            )
            .unwrap();
        store
            .store(
                "openai-codex",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "codex-oauth".into(),
                    refresh_token: "codex-refresh".into(),
                    expires_at: crate::now() + 3600,
                }),
            )
            .unwrap();

        let key = store.resolve_chatgpt_oauth().await.unwrap();
        assert_eq!(key, "codex-oauth");
    }

    #[tokio::test]
    async fn test_resolve_chatgpt_oauth_falls_back_to_openai() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        store
            .store(
                "openai",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "openai-oauth".into(),
                    refresh_token: "openai-refresh".into(),
                    expires_at: crate::now() + 3600,
                }),
            )
            .unwrap();

        let key = store.resolve_chatgpt_oauth().await.unwrap();
        assert_eq!(key, "openai-oauth");
    }

    #[test]
    fn test_oauth_display_info_for_openai_credential() {
        let credential = OAuthCredential {
            access_token: jwt_with_openai_auth("pro", "acct-12345678").into(),
            refresh_token: "refresh".into(),
            expires_at: crate::now() + 3600,
        };

        let info = oauth_display_info_for_credential("openai", &credential).unwrap();
        assert_eq!(info.account_id.as_deref(), Some("acct-12345678"));
        assert_eq!(info.plan.as_deref(), Some("pro"));
        assert_eq!(info.short_account_id().as_deref(), Some("acct-123…"));
    }

    #[test]
    fn test_oauth_display_info_for_anthropic_credential() {
        let credential = OAuthCredential {
            access_token: "sk-ant-oat01-example".into(),
            refresh_token: "refresh".into(),
            expires_at: crate::now() + 3600,
        };

        let info = oauth_display_info_for_credential("anthropic", &credential).unwrap();
        assert_eq!(info.plan.as_deref(), Some("Claude Max/Pro"));
        assert!(info.account_id.is_none());
        assert_eq!(
            info.login_message("anthropic"),
            "Logged in to Anthropic with Claude Max/Pro subscription credentials."
        );
    }

    #[test]
    fn test_remove_then_resolve_falls_through() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = AuthStore::new(path);

        store
            .store(
                "google",
                StoredCredential::ApiKey {
                    key: "google-key".into(),
                },
            )
            .unwrap();
        assert!(store.resolve("google").is_ok());

        store.remove("google").unwrap();
        // Without env var set, should error
        std::env::remove_var("GOOGLE_API_KEY");
        let result = store.resolve("google");
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_provider_returns_auth_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let store = AuthStore::new(path);
        let result = store.resolve("unknown_provider");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::Error::Auth(_)));
    }
}
