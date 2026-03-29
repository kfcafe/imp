use serde::{Deserialize, Serialize};

/// Which search provider to use.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchProvider {
    #[default]
    Tavily,
    Exa,
    Linkup,
    Perplexity,
}

impl SearchProvider {
    pub fn env_key_name(self) -> &'static str {
        match self {
            Self::Tavily => "TAVILY_API_KEY",
            Self::Exa => "EXA_API_KEY",
            Self::Linkup => "LINKUP_API_KEY",
            Self::Perplexity => "PERPLEXITY_API_KEY",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::Exa => "exa",
            Self::Linkup => "linkup",
            Self::Perplexity => "perplexity",
        }
    }
}

/// A single search result.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
    pub date: Option<String>,
}

/// Response from a search provider.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    /// AI-generated answer summary (some providers support this).
    pub answer: Option<String>,
    pub provider: SearchProvider,
}

/// Extracted page content from a read operation.
#[derive(Debug, Clone)]
pub struct PageContent {
    pub title: Option<String>,
    pub text: String,
    pub url: String,
    pub content_length: usize,
}

/// Web tool configuration, typically from `[web]` in config.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebConfig {
    /// Default search provider.
    pub search_provider: Option<SearchProvider>,
}
