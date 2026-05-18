use serde::{Deserialize, Serialize};

/// Which content format the server returned for a page read.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentFormat {
    Markdown,
    PlainText,
    Html,
}

impl ContentFormat {
    pub fn name(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::PlainText => "plain text",
            Self::Html => "html",
        }
    }
}

/// Which search provider to use.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchProvider {
    #[default]
    Tavily,
    Exa,
    Linkup,
    Perplexity,
    GitHub,
}

impl SearchProvider {
    pub fn env_key_name(self) -> &'static str {
        match self {
            Self::Tavily => "TAVILY_API_KEY",
            Self::Exa => "EXA_API_KEY",
            Self::Linkup => "LINKUP_API_KEY",
            Self::Perplexity => "PERPLEXITY_API_KEY",
            Self::GitHub => "GITHUB_TOKEN",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::Exa => "exa",
            Self::Linkup => "linkup",
            Self::Perplexity => "perplexity",
            Self::GitHub => "github",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response from a search provider.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    /// AI-generated answer summary (some providers support this).
    pub answer: Option<String>,
    pub provider: SearchProvider,
}

#[derive(Debug, Clone, Copy)]
pub enum ExtractionQuality {
    Good,
    Partial,
    Poor,
}

impl ExtractionQuality {
    pub fn name(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Partial => "partial",
            Self::Poor => "poor",
        }
    }
}

/// Extracted page content from a read operation.
#[derive(Debug, Clone)]
pub struct PageContent {
    pub title: Option<String>,
    pub text: String,
    /// Final URL after any redirects.
    pub url: String,
    /// Number of chars in the extracted text.
    pub content_length: usize,
    /// Original URL before redirects (same as `url` if no redirect occurred).
    pub requested_url: String,
    /// HTTP status code of the response.
    pub status_code: u16,
    /// Content-Type header value from the response.
    pub content_type: Option<String>,
    /// Which content format was actually received from the server.
    pub format_received: ContentFormat,
    /// Whether the response URL differs from the requested URL (redirect occurred).
    pub was_redirected: bool,
    /// Size in bytes of the raw response body before extraction.
    pub raw_body_bytes: usize,
    /// Informational warnings about potential page quality or extraction issues.
    pub diagnostics: Vec<String>,
    /// Heuristic extraction quality signal.
    pub quality: ExtractionQuality,
    /// Compact reasons for non-good quality.
    pub quality_reasons: Vec<String>,
}

/// Web tool configuration, typically from `[web]` in config.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebConfig {
    /// Default search provider.
    pub search_provider: Option<SearchProvider>,
}
