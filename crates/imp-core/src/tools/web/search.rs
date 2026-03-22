//! Search provider implementations — Tavily, Exa, Linkup, Perplexity.
//!
//! Each provider hits its own HTTP API and maps results to a common
//! `SearchResponse` type. The web tool dispatches based on config.

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
    let api_key =
        std::env::var(provider.env_key_name()).map_err(|_| SearchError::MissingApiKey(provider))?;

    let response = match provider {
        SearchProvider::Tavily => tavily_search(client, &api_key, query, max_results).await,
        SearchProvider::Exa => exa_search(client, &api_key, query, max_results).await,
        SearchProvider::Linkup => linkup_search(client, &api_key, query, max_results).await,
        SearchProvider::Perplexity => perplexity_search(client, &api_key, query, max_results).await,
    }?;

    Ok(response)
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
                "{} not set. Add it to your environment or secrets.",
                provider.env_key_name()
            ),
            Self::Request(msg) => write!(f, "Request failed: {msg}"),
            Self::Api(msg) => write!(f, "API error: {msg}"),
            Self::Parse(msg) => write!(f, "Failed to parse response: {msg}"),
        }
    }
}
