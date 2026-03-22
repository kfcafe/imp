//! Web tool — search the web and read pages.
//!
//! Single tool with two actions:
//! - `search`: query a search API (Tavily, Exa, Linkup, or Perplexity)
//! - `read`: fetch a URL and extract readable content natively
//!
//! Search provider is config-driven (`[web] search_provider = "tavily"`).
//! Reading is native — reqwest + readability, no API key needed.

pub mod read;
pub mod search;
pub mod types;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::sync::OnceLock;
use std::time::Duration;

use super::{truncate_head, truncate_line, Tool, ToolContext, ToolOutput, TruncationResult};
use crate::error::Result;
use types::SearchProvider;

const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_LINE_CHARS: usize = 500;

/// Shared HTTP client for all web operations.
fn http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub struct WebTool;

#[async_trait]
impl Tool for WebTool {
    fn name(&self) -> &str {
        "web"
    }
    fn label(&self) -> &str {
        "Web"
    }
    fn description(&self) -> &str {
        "Search the web or read a page. action=search queries a search engine, action=read fetches and extracts page content."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["search", "read"],
                    "description": "search: web search. read: fetch and extract a page."
                },
                "query": {
                    "type": "string",
                    "description": "Search query (search action)"
                },
                "url": {
                    "type": "string",
                    "description": "URL to read (read action)"
                },
                "provider": {
                    "type": "string",
                    "enum": ["tavily", "exa", "linkup", "perplexity"],
                    "description": "Search provider override (default from config)"
                },
                "maxResults": {
                    "type": "number",
                    "description": "Max search results (default: 5, max: 20)"
                }
            },
            "required": ["action"]
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        match params["action"].as_str() {
            Some("search") => execute_search(params, &ctx).await,
            Some("read") => execute_read(params).await,
            Some(other) => Ok(ToolOutput::error(format!("Unknown web action: {other}"))),
            None => Ok(ToolOutput::error("Missing 'action' parameter")),
        }
    }
}

// ── search action ───────────────────────────────────────────────────

async fn execute_search(params: serde_json::Value, _ctx: &ToolContext) -> Result<ToolOutput> {
    let query = match params["query"].as_str() {
        Some(q) if !q.is_empty() => q,
        _ => return Ok(ToolOutput::error("Missing 'query' parameter")),
    };

    let max_results = params["maxResults"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or(5)
        .min(20);

    let provider = resolve_provider(&params);

    let response = match search::search(http_client(), provider, query, max_results).await {
        Ok(resp) => resp,
        Err(e) => return Ok(ToolOutput::error(e.to_string())),
    };

    Ok(ToolOutput::text(truncate_output(format_search_response(
        &response, query,
    ))))
}

fn resolve_provider(params: &serde_json::Value) -> SearchProvider {
    // Explicit param override
    if let Some(name) = params["provider"].as_str() {
        match name {
            "tavily" => return SearchProvider::Tavily,
            "exa" => return SearchProvider::Exa,
            "linkup" => return SearchProvider::Linkup,
            "perplexity" => return SearchProvider::Perplexity,
            _ => {}
        }
    }

    // Env-driven default: IMP_WEB_PROVIDER=exa (or config later)
    if let Ok(env_provider) = std::env::var("IMP_WEB_PROVIDER") {
        match env_provider.to_lowercase().as_str() {
            "tavily" => return SearchProvider::Tavily,
            "exa" => return SearchProvider::Exa,
            "linkup" => return SearchProvider::Linkup,
            "perplexity" => return SearchProvider::Perplexity,
            _ => {}
        }
    }

    // Auto-detect: pick whichever provider has an API key set
    for provider in [
        SearchProvider::Tavily,
        SearchProvider::Exa,
        SearchProvider::Linkup,
        SearchProvider::Perplexity,
    ] {
        if std::env::var(provider.env_key_name()).is_ok() {
            return provider;
        }
    }

    SearchProvider::default()
}

fn format_search_response(response: &types::SearchResponse, query: &str) -> String {
    let mut output = format!("Query: \"{}\" ({})\n", query, response.provider.name());

    if let Some(answer) = &response.answer {
        output.push_str(&format!("\n## Summary\n{answer}\n"));
    }

    if response.results.is_empty() {
        output.push_str("\nNo results found.\n");
        return output;
    }

    output.push_str(&format!(
        "\n## Results ({} found)\n",
        response.results.len()
    ));

    for result in &response.results {
        output.push_str(&format!("\n### {}\n", result.title));
        output.push_str(&format!("URL: {}\n", result.url));
        if let Some(date) = &result.date {
            output.push_str(&format!("Date: {date}\n"));
        }
        if let Some(snippet) = &result.snippet {
            output.push_str(&format!("{snippet}\n"));
        }
    }

    output
}

// ── read action ─────────────────────────────────────────────────────

async fn execute_read(params: serde_json::Value) -> Result<ToolOutput> {
    let url = match params["url"].as_str() {
        Some(u) if !u.is_empty() => u,
        _ => return Ok(ToolOutput::error("Missing 'url' parameter")),
    };

    let page = match read::fetch_and_extract(http_client(), url).await {
        Ok(page) => page,
        Err(e) => return Ok(ToolOutput::error(e.to_string())),
    };

    let title = page.title.as_deref().unwrap_or(url);
    let mut output = format!(
        "# {title}\nURL: {}\nLength: {} chars\n\n---\n\n",
        page.url, page.content_length
    );

    // Wrap content in delimiters to reduce prompt injection risk
    output.push_str("<web_content>\n");
    output.push_str(&page.text);
    output.push_str("\n</web_content>");

    Ok(ToolOutput::text(truncate_output(output)))
}

// ── output truncation ───────────────────────────────────────────────

fn truncate_output(text: String) -> String {
    if text.is_empty() {
        return text;
    }

    let truncated_lines = text
        .lines()
        .map(|line| truncate_line(line, MAX_LINE_CHARS))
        .collect::<Vec<_>>()
        .join("\n");

    let TruncationResult {
        content,
        truncated,
        output_lines,
        total_lines,
        temp_file,
        ..
    } = truncate_head(&truncated_lines, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);

    if !truncated {
        return content;
    }

    let mut result = content;
    result.push_str(&format!(
        "\n[Output truncated: showing first {output_lines} of {total_lines} lines{}]",
        temp_file
            .as_ref()
            .map(|p| format!(". Full output saved to {}", p.display()))
            .unwrap_or_default()
    ));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_search_with_answer() {
        let response = types::SearchResponse {
            results: vec![types::SearchResult {
                title: "Rust Lang".into(),
                url: "https://rust-lang.org".into(),
                snippet: Some("A systems programming language".into()),
                date: None,
            }],
            answer: Some("Rust is a systems programming language.".into()),
            provider: SearchProvider::Tavily,
        };

        let output = format_search_response(&response, "what is rust");
        assert!(output.contains("## Summary"));
        assert!(output.contains("Rust is a systems programming language"));
        assert!(output.contains("### Rust Lang"));
        assert!(output.contains("(tavily)"));
    }

    #[test]
    fn format_search_no_results() {
        let response = types::SearchResponse {
            results: vec![],
            answer: None,
            provider: SearchProvider::Exa,
        };

        let output = format_search_response(&response, "obscure query");
        assert!(output.contains("No results found"));
        assert!(output.contains("(exa)"));
    }

    #[test]
    fn truncate_output_respects_limits() {
        // Build text with enough lines to trigger line-based truncation
        let long_text = (0..5000)
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = truncate_output(long_text);
        assert!(result.len() <= MAX_OUTPUT_BYTES + 500); // slack for truncation message
        assert!(result.contains("[Output truncated"));
    }
}
