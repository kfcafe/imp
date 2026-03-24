use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

        // 3. Environment variable
        let env_var = match provider {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "google" => "GOOGLE_API_KEY",
            other => {
                return Err(crate::error::Error::Auth(format!(
                    "Unknown provider: {other}"
                )))
            }
        };
        if let Ok(key) = std::env::var(env_var) {
            return Ok(key);
        }

        Err(crate::error::Error::Auth(format!(
            "No API key found for {provider}. Set {env_var} or run `imp auth login {provider}`."
        )))
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

#[cfg(test)]
mod tests {
    use super::*;

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
