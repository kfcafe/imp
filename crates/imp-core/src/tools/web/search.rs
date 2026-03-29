//! Search provider implementations — Tavily, Exa, Linkup, Perplexity.
//!
//! Each provider hits its own HTTP API and maps results to a common
//! `SearchResponse` type. Credentials can come from environment variables
//! or imp's persisted auth store (`~/.config/imp/auth.json`).

use std::path::Path;

use imp_llm::auth::AuthStore;
use reqwest::Client;
use serde_json::{json, Value};

use super::types::{SearchProvider, SearchResponse, SearchResult};

/// Execute a search against the given provider.
pub async fn search(
    client: &Client,
    provider: SearchProvider,
    query: &str,
    max_results: usize,
) -> Result<SearchResponse, SearchError> {
    let api_key = resolve_api_key(provider, std::env::var(provider.env_key_name()).ok(), None)?;

    let response = match provider {
        SearchProvider::Tavily => tavily_search(client, &api_key, query, max_results).await,
        SearchProvider::Exa => exa_search(client, &api_key, query, max_results).await,
        SearchProvider::Linkup => linkup_search(client, &api_key, query, max_results).await,
        SearchProvider::Perplexity => perplexity_search(client, &api_key, query, max_results).await,
    }?;

    Ok(response)
}

// ── credential resolution ──────────────────────────────────────────

fn resolve_api_key(
    provider: SearchProvider,
    env_value: Option<String>,
    auth_path: Option<&Path>,
) -> Result<String, SearchError> {
    if let Some(key) = env_value.filter(|value| !value.trim().is_empty()) {
        return Ok(key);
    }

    let auth_path = auth_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::config::Config::user_config_dir().join("auth.json"));
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));

    auth_store
        .resolve_api_key_only(provider.name())
        .map_err(|_| SearchError::MissingApiKey(provider))
}

// ── tavily ──────────────────────────────────────────────────────────

async fn tavily_search(
    client: &Client,
    api_key: &str,
    query: &str,
    max_results: usize,
) -> Result<SearchResponse, SearchError> {
    let body = json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "include_answer": true,
        "max_results": max_results.min(10),
    });

    let resp = client
        .post("https://api.tavily.com/search")
        .json(&body)
        .send()
        .await
        .map_err(|e| SearchError::Request(e.to_string()))?;

    let status = resp.status();
    let data: Value = resp
        .json()
        .await
        .map_err(|e| SearchError::Parse(e.to_string()))?;

    if !status.is_success() {
        return Err(SearchError::Api(format!(
            "Tavily {status}: {}",
            data.get("detail")
                .or(data.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        )));
    }

    let answer = data.get("answer").and_then(Value::as_str).map(String::from);
    let results = data
        .get("results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|r| SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["content"].as_str().map(String::from),
                    date: None,
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResponse {
        results,
        answer,
        provider: SearchProvider::Tavily,
    })
}

// ── exa ─────────────────────────────────────────────────────────────

async fn exa_search(
    client: &Client,
    api_key: &str,
    query: &str,
    max_results: usize,
) -> Result<SearchResponse, SearchError> {
    let body = json!({
        "query": query,
        "numResults": max_results.min(20),
        "type": "auto",
    });

    let resp = client
        .post("https://api.exa.ai/search")
        .header("x-api-key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| SearchError::Request(e.to_string()))?;

    let status = resp.status();
    let data: Value = resp
        .json()
        .await
        .map_err(|e| SearchError::Parse(e.to_string()))?;

    if !status.is_success() {
        return Err(SearchError::Api(format!(
            "Exa {status}: {}",
            data.get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        )));
    }

    let results = data
        .get("results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|r| SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["text"].as_str().map(|t| truncate(t, 500)),
                    date: r["publishedDate"].as_str().map(String::from),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResponse {
        results,
        answer: None,
        provider: SearchProvider::Exa,
    })
}

// ── linkup ──────────────────────────────────────────────────────────

async fn linkup_search(
    client: &Client,
    api_key: &str,
    query: &str,
    max_results: usize,
) -> Result<SearchResponse, SearchError> {
    let body = json!({
        "q": query,
        "depth": "standard",
        "outputType": "sourcedAnswer",
        "includeSources": true,
        "maxResults": max_results.min(10),
    });

    let resp = client
        .post("https://api.linkup.so/v1/search")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| SearchError::Request(e.to_string()))?;

    let status = resp.status();
    let data: Value = resp
        .json()
        .await
        .map_err(|e| SearchError::Parse(e.to_string()))?;

    if !status.is_success() {
        return Err(SearchError::Api(format!(
            "Linkup {status}: {}",
            data.get("error")
                .or(data.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        )));
    }

    let answer = data.get("answer").and_then(Value::as_str).map(String::from);
    let results = data
        .get("sources")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|r| SearchResult {
                    title: r["name"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["snippet"].as_str().map(String::from),
                    date: None,
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResponse {
        results,
        answer,
        provider: SearchProvider::Linkup,
    })
}

// ── perplexity ──────────────────────────────────────────────────────

async fn perplexity_search(
    client: &Client,
    api_key: &str,
    query: &str,
    max_results: usize,
) -> Result<SearchResponse, SearchError> {
    let body = json!({
        "query": query,
        "max_results": max_results.min(20),
    });

    let resp = client
        .post("https://api.perplexity.ai/search")
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| SearchError::Request(e.to_string()))?;

    let status = resp.status();
    let data: Value = resp
        .json()
        .await
        .map_err(|e| SearchError::Parse(e.to_string()))?;

    if !status.is_success() {
        return Err(SearchError::Api(format!(
            "Perplexity {status}: {}",
            data.get("error")
                .or(data.get("detail"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        )));
    }

    let results = data
        .get("results")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|r| SearchResult {
                    title: r["title"].as_str().unwrap_or("").to_string(),
                    url: r["url"].as_str().unwrap_or("").to_string(),
                    snippet: r["snippet"].as_str().map(String::from),
                    date: r["date"].as_str().map(String::from),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResponse {
        results,
        answer: None,
        provider: SearchProvider::Perplexity,
    })
}

// ── helpers ─────────────────────────────────────────────────────────

fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}

#[derive(Debug)]
pub enum SearchError {
    MissingApiKey(SearchProvider),
    Request(String),
    Api(String),
    Parse(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingApiKey(provider) => write!(
                f,
                "{} not set. Run `imp login {}` or set {} in your environment.",
                provider.env_key_name(),
                provider.name(),
                provider.env_key_name()
            ),
            Self::Request(msg) => write!(f, "Request failed: {msg}"),
            Self::Api(msg) => write!(f, "API error: {msg}"),
            Self::Parse(msg) => write!(f, "Failed to parse response: {msg}"),
        }
    }
}

impl std::error::Error for SearchError {}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_llm::auth::StoredCredential;
    use tempfile::tempdir;

    #[test]
    fn resolve_api_key_uses_explicit_env_value() {
        let key = resolve_api_key(
            SearchProvider::Exa,
            Some("exa-env-key".to_string()),
            None,
        )
        .unwrap();

        assert_eq!(key, "exa-env-key");
    }

    #[test]
    fn resolve_api_key_reads_imp_auth_store() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        let mut auth_store = AuthStore::new(auth_path.clone());
        auth_store
            .store(
                "tavily",
                StoredCredential::ApiKey {
                    key: "tvly-saved-key".into(),
                },
            )
            .unwrap();

        let key = resolve_api_key(SearchProvider::Tavily, None, Some(&auth_path)).unwrap();
        assert_eq!(key, "tvly-saved-key");
    }

    #[test]
    fn resolve_api_key_missing_reports_provider() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        let err = resolve_api_key(SearchProvider::Exa, None, Some(&auth_path)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("EXA_API_KEY"));
        assert!(msg.contains("imp login exa"));
    }
}
