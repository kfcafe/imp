//! Web tool — search the web and read pages.
//!
//! Single tool with two actions:
//! - `search`: query a search API (Tavily, Exa, Linkup, or Perplexity), or GitHub when `sources` includes `github`
//! - `read`: fetch a URL and extract readable content natively
//!
//! Search provider is config-driven (`[web] search_provider = "tavily"`).
//! Reading is native — reqwest + readability, no API key needed.

pub mod read;
pub mod search;
pub mod types;
pub mod youtube;

mod github;

use async_trait::async_trait;
use imp_llm::ContentBlock;
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
        "Search the web or read a page. YouTube URLs are read through native HTTP metadata/transcript extraction."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["search", "read"] },
                "query": { "type": "string" },
                "url": { "type": "string" },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 20 },
                "sources": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["web", "github"] },
                    "description": "Optional search source. Use ['github'] for read-only GitHub repository search."
                },
                "github": {
                    "type": "object",
                    "properties": {
                        "type": { "type": "string", "enum": ["repositories", "issues", "pull_requests", "code", "releases"] },
                        "owner": { "type": "string" },
                        "repo": { "type": "string" },
                        "org": { "type": "string" },
                        "language": { "type": "string" },
                        "topic": { "type": "string" },
                        "min_stars": { "type": "integer", "minimum": 0 },
                        "updated_since": { "type": "string", "description": "ISO date such as 2025-01-01" }
                    },
                    "additionalProperties": false
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

async fn execute_search(params: serde_json::Value, ctx: &ToolContext) -> Result<ToolOutput> {
    let query = match params["query"].as_str() {
        Some(q) if !q.is_empty() => q,
        _ => return Ok(ToolOutput::error("web search requires query")),
    };

    let max_results = max_results_from_params(&params);

    if should_search_github(&params) {
        let response =
            match github::search(http_client(), query, max_results, params.get("github")).await {
                Ok(resp) => resp,
                Err(e) => return Ok(ToolOutput::error(e.to_string())),
            };

        return Ok(ToolOutput {
            content: vec![ContentBlock::Text {
                text: truncate_output(format_search_response(&response, query)),
            }],
            details: json!({
                "action": "search",
                "source": "github",
                "provider": response.provider.name(),
                "query": query,
                "max_results": max_results,
                "results_count": response.results.len(),
                "has_answer": response.answer.is_some(),
                "results": response.results,
            }),
            is_error: false,
        });
    }

    let provider = resolve_provider(&params, ctx);

    let response = match search::search(http_client(), provider, query, max_results).await {
        Ok(resp) => resp,
        Err(e) => return Ok(ToolOutput::error(e.to_string())),
    };

    Ok(ToolOutput {
        content: vec![ContentBlock::Text {
            text: truncate_output(format_search_response(&response, query)),
        }],
        details: json!({
            "action": "search",
            "provider": response.provider.name(),
            "query": query,
            "max_results": max_results,
            "results_count": response.results.len(),
            "has_answer": response.answer.is_some(),
            "results": response.results,
        }),
        is_error: false,
    })
}

fn max_results_from_params(params: &serde_json::Value) -> usize {
    params
        .get("max_results")
        .or_else(|| params.get("maxResults"))
        .and_then(|value| value.as_u64())
        .map(|n| n as usize)
        .unwrap_or(5)
        .clamp(1, 20)
}

fn should_search_github(params: &serde_json::Value) -> bool {
    params
        .get("sources")
        .and_then(|value| value.as_array())
        .is_some_and(|sources| {
            sources.iter().any(|source| {
                source
                    .as_str()
                    .is_some_and(|s| s.eq_ignore_ascii_case("github"))
            })
        })
}

fn resolve_provider(_params: &serde_json::Value, ctx: &ToolContext) -> SearchProvider {
    // Env-driven default: IMP_WEB_PROVIDER=exa
    if let Ok(env_provider) = std::env::var("IMP_WEB_PROVIDER") {
        match env_provider.to_lowercase().as_str() {
            "tavily" => return SearchProvider::Tavily,
            "exa" => return SearchProvider::Exa,
            "linkup" => return SearchProvider::Linkup,
            "perplexity" => return SearchProvider::Perplexity,
            _ => {}
        }
    }

    // Config-driven default: [web] search_provider = "exa"
    let config_dir = crate::config::Config::user_config_dir();
    if let Ok(config) = crate::config::Config::resolve(&config_dir, Some(&ctx.cwd)) {
        if let Some(provider) = config.web.search_provider {
            return provider;
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
        _ => return Ok(ToolOutput::error("web read requires url")),
    };

    if github::is_github_url(url) {
        let gh = match github::read_url(http_client(), url).await {
            Ok(read) => read,
            Err(e) => return Ok(ToolOutput::error(e.to_string())),
        };
        let mut output = format!(
            "# {}\nURL: {}\nSource: GitHub ({})\n\n---\n\n",
            gh.title, gh.url, gh.kind
        );
        output.push_str("<web_content>\n");
        output.push_str(&gh.text);
        output.push_str("\n</web_content>");
        return Ok(ToolOutput {
            content: vec![ContentBlock::Text {
                text: truncate_output(output),
            }],
            details: json!({
                "action": "read",
                "source": "github",
                "kind": gh.kind,
                "title": gh.title,
                "url": gh.url,
                "content_length": gh.text.len(),
                "github": gh.details,
            }),
            is_error: false,
        });
    }

    let page = match read::fetch_and_extract(http_client(), url).await {
        Ok(page) => page,
        Err(e) => return Ok(ToolOutput::error(e.to_string())),
    };

    let title = page.title.as_deref().unwrap_or(url);
    let mut output = format!("# {title}\nURL: {}\n", page.url);

    if page.was_redirected {
        output.push_str(&format!("Requested: {}\n", page.requested_url));
    }

    output.push_str(&format!("Status: {}\n", page.status_code));
    output.push_str(&format!(
        "Content-Type: {}\n",
        page.content_type.as_deref().unwrap_or("unknown")
    ));
    output.push_str(&format!(
        "Format: {} (requested markdown, received {})\n",
        page.format_received.name(),
        page.format_received.name()
    ));
    output.push_str(&format!(
        "Response size: {} bytes → {} chars extracted\n",
        page.raw_body_bytes, page.content_length
    ));

    if !page.diagnostics.is_empty() {
        output.push_str("\n⚠ Diagnostics:\n");
        for warning in &page.diagnostics {
            output.push_str(&format!("- {warning}\n"));
        }
    }

    output.push_str("\n---\n\n");

    // Wrap content in delimiters to reduce prompt injection risk
    output.push_str("<web_content>\n");
    output.push_str(&page.text);
    output.push_str("\n</web_content>");

    Ok(ToolOutput {
        content: vec![ContentBlock::Text {
            text: truncate_output(output),
        }],
        details: json!({
            "action": "read",
            "requested_url": page.requested_url,
            "final_url": page.url,
            "status_code": page.status_code,
            "content_type": page.content_type,
            "format_received": page.format_received.name(),
            "was_redirected": page.was_redirected,
            "raw_body_bytes": page.raw_body_bytes,
            "content_length": page.content_length,
            "quality": page.quality.name(),
            "quality_reasons": page.quality_reasons,
            "diagnostics": page.diagnostics,
        }),
        is_error: false,
    })
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
    fn schema_hides_provider_and_uses_max_results() {
        let schema = WebTool.parameters();
        let properties = schema["properties"].as_object().unwrap();
        assert!(properties.contains_key("max_results"));
        assert!(!properties.contains_key("maxResults"));
        assert!(!properties.contains_key("provider"));
    }

    #[test]
    fn resolve_provider_prefers_env_over_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".imp")).unwrap();
        std::fs::write(
            dir.path().join(".imp").join("config.toml"),
            "[web]\nsearch_provider = \"exa\"\n",
        )
        .unwrap();

        let old = std::env::var("IMP_WEB_PROVIDER").ok();
        std::env::set_var("IMP_WEB_PROVIDER", "tavily");

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: std::sync::Arc::new(crate::ui::NullInterface),
            file_cache: std::sync::Arc::new(crate::tools::FileCache::new()),
            checkpoint_state: std::sync::Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: std::sync::Arc::new(std::sync::Mutex::new(
                crate::tools::FileTracker::new(),
            )),
            anchor_store: std::sync::Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            run_policy: Default::default(),
            config: std::sync::Arc::new(crate::config::Config::default()),
            supporting_provenance: Vec::new(),
        };

        let provider = resolve_provider(&serde_json::json!({}), &ctx);
        assert_eq!(provider, SearchProvider::Tavily);

        match old {
            Some(value) => std::env::set_var("IMP_WEB_PROVIDER", value),
            None => std::env::remove_var("IMP_WEB_PROVIDER"),
        }
    }

    #[test]
    fn max_results_accepts_legacy_camel_case() {
        let modern = serde_json::json!({"max_results": 7});
        let legacy = serde_json::json!({"maxResults": 8});
        let clamped = serde_json::json!({"max_results": 99});

        assert_eq!(max_results_from_params(&modern), 7);
        assert_eq!(max_results_from_params(&legacy), 8);
        assert_eq!(max_results_from_params(&clamped), 20);
    }

    #[test]
    fn format_search_with_answer() {
        let response = types::SearchResponse {
            results: vec![types::SearchResult {
                title: "Rust Lang".into(),
                url: "https://rust-lang.org".into(),
                snippet: Some("A systems programming language".into()),
                date: None,
                source_type: None,
                kind: None,
                metadata: None,
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
