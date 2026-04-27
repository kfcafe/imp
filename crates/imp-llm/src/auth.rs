use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::truncate_chars_with_suffix;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

pub type ApiKey = String;
const KEYRING_SERVICE: &str = "imp";
const LEGACY_KEYRING_SERVICES: &[&str] = &["imp-cli", "impeccable", "mana"];

fn provider_lookup_candidates(provider: &str) -> Vec<String> {
    let mut candidates = vec![provider.to_string()];
    let lower = provider.to_lowercase();
    if lower != provider {
        candidates.push(lower);
    }
    if provider == "render" {
        candidates.push("Render".to_string());
    }
    dedupe_strings(candidates)
}

fn field_lookup_candidates(field: &str) -> Vec<String> {
    let mut candidates = vec![field.to_string()];
    if field == "secrets_key" {
        candidates.push("secret_key".to_string());
    }
    if field == "secret_key" {
        candidates.push("secrets_key".to_string());
    }
    dedupe_strings(candidates)
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    deduped
}

trait SecretBackend: Send + Sync {
    fn get(&self, provider: &str, field: &str) -> Result<Option<String>>;
    fn set(&self, provider: &str, field: &str, value: &str) -> Result<()>;
    fn delete(&self, provider: &str, field: &str) -> Result<()>;
}

struct KeyringBackend;

impl KeyringBackend {
    fn entry(service: &str, provider: &str, field: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(service, &format!("{provider}:{field}"))
            .map_err(|e| crate::error::Error::Auth(format!("Secure storage init failed: {e}")))
    }

    fn read_entry(service: &str, provider: &str, field: &str) -> Result<Option<String>> {
        let entry = Self::entry(service, provider, field)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(Self::map_error("read", provider, field, error)),
        }
    }

    fn lookup_secret(provider: &str, field: &str) -> Result<Option<String>> {
        let providers = provider_lookup_candidates(provider);
        let fields = field_lookup_candidates(field);
        for candidate_provider in &providers {
            for candidate_field in &fields {
                if let Some(value) =
                    Self::read_entry(KEYRING_SERVICE, candidate_provider, candidate_field)?
                {
                    return Ok(Some(value));
                }
            }
        }
        for service in LEGACY_KEYRING_SERVICES {
            for candidate_provider in &providers {
                for candidate_field in &fields {
                    if let Some(value) =
                        Self::read_entry(service, candidate_provider, candidate_field)?
                    {
                        return Ok(Some(value));
                    }
                }
            }
        }
        Ok(None)
    }

    fn map_error(
        action: &str,
        provider: &str,
        field: &str,
        error: keyring::Error,
    ) -> crate::error::Error {
        crate::error::Error::Auth(format!(
            "Secure storage {action} failed for {provider}.{field}: {error}"
        ))
    }
}

impl SecretBackend for KeyringBackend {
    fn get(&self, provider: &str, field: &str) -> Result<Option<String>> {
        Self::lookup_secret(provider, field)
    }

    fn set(&self, provider: &str, field: &str, value: &str) -> Result<()> {
        let entry = Self::entry(KEYRING_SERVICE, provider, field)?;
        entry
            .set_password(value)
            .map_err(|error| Self::map_error("write", provider, field, error))
    }

    fn delete(&self, provider: &str, field: &str) -> Result<()> {
        let providers = provider_lookup_candidates(provider);
        let fields = field_lookup_candidates(field);
        let mut first_error = None;
        for service in
            std::iter::once(KEYRING_SERVICE).chain(LEGACY_KEYRING_SERVICES.iter().copied())
        {
            for candidate_provider in &providers {
                for candidate_field in &fields {
                    let entry = Self::entry(service, candidate_provider, candidate_field)?;
                    match entry.delete_credential() {
                        Ok(()) | Err(keyring::Error::NoEntry) => {}
                        Err(error) if first_error.is_none() => {
                            first_error = Some(Self::map_error(
                                "delete",
                                candidate_provider,
                                candidate_field,
                                error,
                            ));
                        }
                        Err(_) => {}
                    }
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

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
    SecretFields { fields: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretFieldStatus {
    Present,
    Missing,
    Error(String),
}

impl SecretFieldStatus {
    #[must_use]
    pub fn is_present(&self) -> bool {
        matches!(self, Self::Present)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretStatus {
    pub provider: String,
    pub fields: Vec<(String, SecretFieldStatus)>,
}

impl SecretStatus {
    #[must_use]
    pub fn is_usable(&self) -> bool {
        self.fields.iter().all(|(_, status)| status.is_present())
    }
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
    backend: Arc<dyn SecretBackend>,
}

impl AuthStore {
    pub fn new(path: PathBuf) -> Self {
        Self::new_with_backend(path, Arc::new(KeyringBackend))
    }

    fn new_with_backend(path: PathBuf, backend: Arc<dyn SecretBackend>) -> Self {
        Self {
            runtime_keys: HashMap::new(),
            stored: HashMap::new(),
            path,
            backend,
        }
    }

    /// Load stored credentials from disk.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        Self::load_with_backend(path, Arc::new(KeyringBackend))
    }

    fn load_with_backend(path: &std::path::Path, backend: Arc<dyn SecretBackend>) -> Result<Self> {
        let stored = if path.exists() {
            let data = std::fs::read_to_string(path)?;
            serde_json::from_str(&data).map_err(|error| {
                crate::error::Error::Auth(format!(
                    "Failed to parse auth metadata at {}: {error}",
                    path.display()
                ))
            })?
        } else {
            HashMap::new()
        };
        Ok(Self {
            runtime_keys: HashMap::new(),
            stored,
            path: path.to_path_buf(),
            backend,
        })
    }

    /// Set a runtime override (not persisted).
    /// Empty or whitespace-only values are treated as absent.
    pub fn set_runtime_key(&mut self, provider: &str, key: String) {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            self.runtime_keys.remove(provider);
            return;
        }
        self.runtime_keys
            .insert(provider.to_string(), trimmed.to_string());
    }

    /// Check whether credentials are usable for a provider without producing an error.
    /// Returns true if a runtime key, readable stored credential, or env var is available.
    pub fn has_credentials(&self, provider: &str) -> bool {
        self.resolve(provider).is_ok()
    }

    /// Resolution order: runtime override -> stored -> env var -> error.
    pub fn resolve(&self, provider: &str) -> Result<ApiKey> {
        if let Some(key) = self.runtime_keys.get(provider) {
            return Ok(key.clone());
        }

        if let Some(StoredCredential::OAuth(oauth)) = self.stored.get(provider) {
            return Ok(oauth.access_token.clone());
        }

        self.resolve_secret_field(provider, "api_key")
    }

    /// Resolve an API key without falling back to stored OAuth credentials.
    pub fn resolve_api_key_only(&self, provider: &str) -> Result<ApiKey> {
        self.resolve_secret_field(provider, "api_key")
    }

    /// Resolve a named secret field for any stored provider/service.
    pub fn resolve_secret_field(&self, provider: &str, field: &str) -> Result<String> {
        if field == "api_key" {
            if let Some(key) = self.runtime_keys.get(provider) {
                return Ok(key.clone());
            }
        }

        if let Some((stored_provider, credential)) = self.stored_credential(provider) {
            match credential {
                StoredCredential::ApiKey { key } if field == "api_key" => return Ok(key.clone()),
                StoredCredential::SecretFields { fields } => {
                    if fields.iter().any(|name| name == field) {
                        return self
                            .backend
                            .get(stored_provider, field)?
                            .ok_or_else(|| missing_secret_error(stored_provider, field));
                    }
                }
                StoredCredential::OAuth(_) => {}
                StoredCredential::ApiKey { .. } => {}
            }
        }

        if let Some(value) = resolve_env_secret(provider, field) {
            return Ok(value);
        }

        Err(missing_secret_error(provider, field))
    }

    /// Store multiple named secret fields securely in the OS keychain and persist only metadata.
    pub fn store_secret_fields(
        &mut self,
        provider: &str,
        fields: HashMap<String, String>,
    ) -> Result<()> {
        if fields.is_empty() {
            return Err(crate::error::Error::Auth(format!(
                "No secret fields provided for {provider}."
            )));
        }

        let mut field_names = Vec::with_capacity(fields.len());
        for (field, value) in &fields {
            let field = field.trim();
            if field.is_empty() {
                return Err(crate::error::Error::Auth(format!(
                    "Secret field names for {provider} cannot be empty."
                )));
            }
            if value.trim().is_empty() {
                return Err(crate::error::Error::Auth(format!(
                    "Secret value for {provider}.{field} cannot be empty."
                )));
            }
            self.backend.set(provider, field, value)?;
            field_names.push(field.to_string());
        }

        field_names.sort();
        field_names.dedup();
        self.stored.insert(
            provider.to_string(),
            StoredCredential::SecretFields {
                fields: field_names,
            },
        );
        self.save()
    }

    fn stored_credential(&self, provider: &str) -> Option<(&str, &StoredCredential)> {
        provider_lookup_candidates(provider)
            .into_iter()
            .find_map(|candidate| {
                self.stored
                    .get_key_value(&candidate)
                    .map(|(stored_provider, credential)| (stored_provider.as_str(), credential))
            })
    }

    /// Check whether stored secret metadata points at readable secure-storage values.
    pub fn secret_status(&self, provider: &str) -> Option<SecretStatus> {
        let (stored_provider, credential) = self.stored_credential(provider)?;
        let fields = match credential {
            StoredCredential::SecretFields { fields } => fields
                .iter()
                .map(|field| {
                    let status = match self.backend.get(stored_provider, field) {
                        Ok(Some(value)) if !value.trim().is_empty() => SecretFieldStatus::Present,
                        Ok(_) => SecretFieldStatus::Missing,
                        Err(error) => SecretFieldStatus::Error(error.to_string()),
                    };
                    (field.clone(), status)
                })
                .collect(),
            StoredCredential::ApiKey { key } => vec![(
                "api_key".to_string(),
                if key.trim().is_empty() {
                    SecretFieldStatus::Missing
                } else {
                    SecretFieldStatus::Present
                },
            )],
            StoredCredential::OAuth(oauth) => vec![(
                "access_token".to_string(),
                if oauth.access_token.trim().is_empty() {
                    SecretFieldStatus::Missing
                } else {
                    SecretFieldStatus::Present
                },
            )],
        };

        Some(SecretStatus {
            provider: stored_provider.to_string(),
            fields,
        })
    }

    /// Resolve all stored secret fields for a provider into a map.
    pub fn resolve_secret_fields(&self, provider: &str) -> Result<HashMap<String, String>> {
        match self.stored_credential(provider) {
            Some((stored_provider, StoredCredential::SecretFields { fields })) => fields
                .iter()
                .map(|field| {
                    self.resolve_secret_field(stored_provider, field)
                        .map(|value| (field.clone(), value))
                })
                .collect(),
            Some((_stored_provider, StoredCredential::ApiKey { key })) => {
                Ok(HashMap::from([("api_key".to_string(), key.clone())]))
            }
            Some((_stored_provider, StoredCredential::OAuth(oauth))) => Ok(HashMap::from([(
                "access_token".to_string(),
                oauth.access_token.clone(),
            )])),
            None => {
                if let Some(api_key) = resolve_env_secret(provider, "api_key") {
                    Ok(HashMap::from([("api_key".to_string(), api_key)]))
                } else {
                    Err(missing_secret_error(provider, "api_key"))
                }
            }
        }
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
        if let Some(StoredCredential::OAuth(oauth)) = self.stored.get(provider) {
            if oauth.is_expired() {
                let refresh_token = oauth.refresh_token.clone();
                let result = match provider {
                    "anthropic" => {
                        crate::oauth::anthropic::AnthropicOAuth::new()
                            .refresh_token(&refresh_token)
                            .await
                    }
                    "kimi-code" => {
                        crate::oauth::kimi_code::KimiCodeOAuth::new()
                            .refresh_token(&refresh_token)
                            .await
                    }
                    _ => {
                        return Err(crate::error::Error::Auth(format!(
                            "OAuth refresh not implemented for provider: {provider}"
                        )));
                    }
                };
                match result {
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
    pub async fn resolve_or_refresh<F, Fut>(
        &mut self,
        provider: &str,
        refresh_fn: F,
    ) -> Result<ApiKey>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<OAuthCredential>>,
    {
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
        if let Some(StoredCredential::SecretFields { fields }) = self.stored.remove(provider) {
            for field in fields {
                self.backend.delete(provider, &field)?;
            }
        }
        self.save()
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.stored)?;
        let temp_path = self.path.with_extension("json.tmp");
        std::fs::write(&temp_path, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&temp_path, perms);
        }
        std::fs::rename(&temp_path, &self.path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&self.path, perms);
        }
        Ok(())
    }
}

fn resolve_env_secret(provider: &str, field: &str) -> Option<String> {
    if field == "api_key" {
        let registry = crate::model::ProviderRegistry::with_builtins();
        if let Some(meta) = registry.find(provider) {
            for env_var in meta.env_vars {
                if let Ok(value) = std::env::var(env_var) {
                    if !value.trim().is_empty() {
                        return Some(value);
                    }
                }
            }
        }
    }

    let env_var = env_var_name(provider, field);
    std::env::var(&env_var)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn env_var_name(provider: &str, field: &str) -> String {
    let provider = provider.to_uppercase().replace('-', "_");
    let field = field.to_uppercase().replace('-', "_");
    format!("{provider}_{field}")
}

fn missing_secret_error(provider: &str, field: &str) -> crate::error::Error {
    crate::error::Error::Auth(format!(
        "No readable secret field '{field}' found for {provider}. Set {} or run `imp secrets {provider}` to save it again.",
        env_var_name(provider, field)
    ))
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
        "kimi-code" => Some(OAuthDisplayInfo {
            account_id: None,
            plan: Some("Kimi Code".into()),
            using_subscription: true,
        }),
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
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockSecretBackend {
        values: Mutex<HashMap<(String, String), String>>,
    }

    impl SecretBackend for MockSecretBackend {
        fn get(&self, provider: &str, field: &str) -> Result<Option<String>> {
            Ok(self
                .values
                .lock()
                .unwrap()
                .get(&(provider.to_string(), field.to_string()))
                .cloned())
        }

        fn set(&self, provider: &str, field: &str, value: &str) -> Result<()> {
            self.values
                .lock()
                .unwrap()
                .insert((provider.to_string(), field.to_string()), value.to_string());
            Ok(())
        }

        fn delete(&self, provider: &str, field: &str) -> Result<()> {
            self.values
                .lock()
                .unwrap()
                .remove(&(provider.to_string(), field.to_string()));
            Ok(())
        }
    }

    fn test_store(path: std::path::PathBuf) -> AuthStore {
        AuthStore::new_with_backend(path, Arc::new(MockSecretBackend::default()))
    }

    fn test_store_with_backend(
        path: std::path::PathBuf,
        backend: Arc<dyn SecretBackend>,
    ) -> AuthStore {
        AuthStore::new_with_backend(path, backend)
    }

    fn test_load_with_backend(
        path: &std::path::Path,
        backend: Arc<dyn SecretBackend>,
    ) -> AuthStore {
        AuthStore::load_with_backend(path, backend).unwrap()
    }

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
        let mut store = test_store(path);

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
    fn test_secure_secret_fields_store_and_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path.clone());
        let mut fields = HashMap::new();
        fields.insert("api_key".to_string(), "test-api".to_string());
        fields.insert("secret_key".to_string(), "test-secret".to_string());
        store.store_secret_fields("test-service", fields).unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(!data.contains("test-api"));
        assert!(!data.contains("test-secret"));
        assert_eq!(
            store
                .resolve_secret_field("test-service", "api_key")
                .unwrap(),
            "test-api"
        );
        assert_eq!(
            store
                .resolve_secret_field("test-service", "secret_key")
                .unwrap(),
            "test-secret"
        );
    }

    #[test]
    fn test_secure_secret_fields_persist_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let backend: Arc<dyn SecretBackend> = Arc::new(MockSecretBackend::default());
        let mut store = test_store_with_backend(path.clone(), Arc::clone(&backend));
        store
            .store_secret_fields(
                "test-service",
                HashMap::from([
                    ("api_key".to_string(), "test-api".to_string()),
                    ("secret_key".to_string(), "test-secret".to_string()),
                ]),
            )
            .unwrap();

        let loaded = test_load_with_backend(&path, backend);
        let resolved = loaded.resolve_secret_fields("test-service").unwrap();
        assert_eq!(
            resolved.get("api_key").map(String::as_str),
            Some("test-api")
        );
        assert_eq!(
            resolved.get("secret_key").map(String::as_str),
            Some("test-secret")
        );
    }

    #[test]
    fn test_secure_remove_deletes_secret_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let backend: Arc<dyn SecretBackend> = Arc::new(MockSecretBackend::default());
        let mut store = test_store_with_backend(path, Arc::clone(&backend));
        store
            .store_secret_fields(
                "test-service",
                HashMap::from([
                    ("api_key".to_string(), "test-api".to_string()),
                    ("secret_key".to_string(), "test-secret".to_string()),
                ]),
            )
            .unwrap();

        store.remove("test-service").unwrap();
        assert!(store
            .resolve_secret_field("test-service", "api_key")
            .is_err());
        assert!(backend.get("test-service", "api_key").unwrap().is_none());
    }

    #[test]
    fn test_oauth_detect_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

        let fresh = OAuthCredential {
            access_token: "fresh".into(),
            refresh_token: "rt".into(),
            expires_at: crate::now() + 3600,
        };
        store
            .store("anthropic", StoredCredential::OAuth(fresh))
            .unwrap();
        assert!(!store.is_oauth_expired("anthropic"));

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
        let mut store = test_store(path);

        let expired = OAuthCredential {
            access_token: "old-access".into(),
            refresh_token: "rt-for-refresh".into(),
            expires_at: 0,
        };
        store
            .store("anthropic", StoredCredential::OAuth(expired))
            .unwrap();

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
        let resolved = store.resolve("anthropic").unwrap();
        assert_eq!(resolved, "new-access");
    }

    #[tokio::test]
    async fn test_oauth_resolve_or_refresh_not_expired() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

        let fresh = OAuthCredential {
            access_token: "still-valid".into(),
            refresh_token: "rt".into(),
            expires_at: crate::now() + 3600,
        };
        store
            .store("anthropic", StoredCredential::OAuth(fresh))
            .unwrap();

        let key = store
            .resolve_or_refresh("anthropic", |_| async {
                panic!("refresh should not be called for non-expired token");
            })
            .await
            .unwrap();

        assert_eq!(key, "still-valid");
    }

    #[test]
    fn test_load_invalid_auth_metadata_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        std::fs::write(&path, "{not valid json").unwrap();

        let backend: Arc<dyn SecretBackend> = Arc::new(MockSecretBackend::default());
        let err = match AuthStore::load_with_backend(&path, backend) {
            Ok(_) => panic!("invalid auth metadata should error"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to parse auth metadata"));
        assert!(msg.contains("auth.json"));
    }

    #[test]
    fn test_save_writes_atomically_without_leaving_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path.clone());

        store
            .store(
                "openai",
                StoredCredential::ApiKey {
                    key: "sk-atomic".into(),
                },
            )
            .unwrap();

        assert!(path.exists());
        assert!(!path.with_extension("json.tmp").exists());
        let loaded = test_load_with_backend(&path, Arc::new(MockSecretBackend::default()));
        assert_eq!(loaded.resolve("openai").unwrap(), "sk-atomic");
    }

    #[test]
    fn test_oauth_store_persist_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let backend: Arc<dyn SecretBackend> = Arc::new(MockSecretBackend::default());

        {
            let mut store = test_store_with_backend(path.clone(), Arc::clone(&backend));
            let cred = OAuthCredential {
                access_token: "persisted-token".into(),
                refresh_token: "persisted-rt".into(),
                expires_at: crate::now() + 3600,
            };
            store
                .store("anthropic", StoredCredential::OAuth(cred))
                .unwrap();
        }

        let store = test_load_with_backend(&path, backend);
        let key = store.resolve("anthropic").unwrap();
        assert_eq!(key, "persisted-token");
    }

    #[test]
    fn test_oauth_remove_credential() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

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
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(store.resolve("anthropic").is_err());
    }

    #[test]
    fn test_resolve_order_runtime_over_stored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

        store
            .store(
                "anthropic",
                StoredCredential::ApiKey {
                    key: "stored-key".into(),
                },
            )
            .unwrap();

        store.set_runtime_key("anthropic", "runtime-key".into());
        let key = store.resolve("anthropic").unwrap();
        assert_eq!(key, "runtime-key");
    }

    #[test]
    fn test_set_runtime_key_ignores_empty_or_whitespace_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

        store.set_runtime_key("openai", "runtime-key".into());
        assert_eq!(store.resolve("openai").unwrap(), "runtime-key");

        store.set_runtime_key("openai", "   ".into());
        assert!(store.resolve("openai").is_err());
    }

    #[test]
    fn test_resolve_stored_api_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

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
    fn test_resolve_env_secret_uses_moonshot_env_vars() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let store = test_store(path);

        std::env::remove_var("KIMI_API_KEY");
        std::env::set_var("MOONSHOT_API_KEY", "moonshot-env-key");
        let key = store.resolve("moonshot").unwrap();
        assert_eq!(key, "moonshot-env-key");
        std::env::remove_var("MOONSHOT_API_KEY");

        std::env::set_var("KIMI_API_KEY", "kimi-env-key");
        let key = store.resolve("moonshot").unwrap();
        assert_eq!(key, "kimi-env-key");
        std::env::remove_var("KIMI_API_KEY");
    }

    #[test]
    fn test_resolve_api_key_only_ignores_oauth_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

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
        let mut store = test_store(path);

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
        let mut store = test_store(path);

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
            access_token: jwt_with_openai_auth("pro", "acct-12345678"),
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
        let mut store = test_store(path);

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
        std::env::remove_var("GOOGLE_API_KEY");
        let result = store.resolve("google");
        assert!(result.is_err());
    }

    #[test]
    fn provider_lookup_candidates_include_legacy_render_casing() {
        assert_eq!(
            provider_lookup_candidates("render"),
            vec!["render".to_string(), "Render".to_string()]
        );
        assert_eq!(
            provider_lookup_candidates("Render"),
            vec!["Render".to_string(), "render".to_string()]
        );
    }

    #[test]
    fn field_lookup_candidates_support_porkbun_secret_key_typo() {
        assert_eq!(
            field_lookup_candidates("secrets_key"),
            vec!["secrets_key".to_string(), "secret_key".to_string()]
        );
    }

    #[test]
    fn resolve_secret_fields_uses_provider_alias_candidates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);

        store
            .store_secret_fields(
                "Render",
                HashMap::from([("api_key".to_string(), "render-secret".to_string())]),
            )
            .unwrap();

        let fields = store.resolve_secret_fields("render").unwrap();
        assert_eq!(fields.get("api_key").map(String::as_str), Some("render-secret"));
    }

    #[test]
    fn test_secret_status_reports_missing_keychain_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut store = test_store(path);
        store.stored.insert(
            "google".into(),
            StoredCredential::SecretFields {
                fields: vec!["api_key".into()],
            },
        );

        let status = store.secret_status("google").unwrap();
        assert_eq!(status.provider, "google");
        assert_eq!(
            status.fields,
            vec![("api_key".to_string(), SecretFieldStatus::Missing)]
        );
        assert!(!status.is_usable());
        assert!(!store.has_credentials("google"));
    }

    #[test]
    fn test_has_credentials_detects_kimi_env_var() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let store = test_store(path);

        std::env::remove_var("MOONSHOT_API_KEY");
        std::env::set_var("KIMI_API_KEY", "kimi-env-key");
        assert!(store.has_credentials("moonshot"));
        std::env::remove_var("KIMI_API_KEY");
    }

    #[test]
    fn test_unknown_provider_returns_auth_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let store = test_store(path);
        let result = store.resolve("unknown_provider");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::Error::Auth(_)));
    }
}
