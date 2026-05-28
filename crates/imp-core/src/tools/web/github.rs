use imp_llm::auth::AuthStore;
use reqwest::Client;
use serde_json::{json, Value};
use url::Url;

use super::types::{SearchProvider, SearchResponse, SearchResult};

const API: &str = "https://api.github.com";
const RAW: &str = "https://raw.githubusercontent.com";

pub async fn search(
    client: &Client,
    query: &str,
    max_results: usize,
    github_params: Option<&Value>,
) -> Result<SearchResponse, GitHubError> {
    let params = github_params.unwrap_or(&Value::Null);
    match search_type(params) {
        "repositories" => search_repositories(client, query, max_results, params).await,
        "issues" => search_issues(client, query, max_results, params, "issue").await,
        "pull_requests" => search_issues(client, query, max_results, params, "pr").await,
        "code" => search_code(client, query, max_results, params).await,
        "releases" => Err(GitHubError::UnsupportedType(
            "releases search is not supported by GitHub Search API yet; use web.read on a repository releases URL"
                .to_string(),
        )),
        other => Err(GitHubError::UnsupportedType(format!(
            "GitHub search type '{other}' is not implemented"
        ))),
    }
}

pub fn is_github_url(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed.host_str().map(|host| {
                host.trim_end_matches('.')
                    .eq_ignore_ascii_case("github.com")
            })
        })
        .unwrap_or(false)
}

pub async fn read_url(client: &Client, url: &str) -> Result<GitHubRead, GitHubError> {
    let parsed = Url::parse(url).map_err(|e| GitHubError::InvalidUrl(e.to_string()))?;
    let host = parsed
        .host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host != "github.com" {
        return Err(GitHubError::InvalidUrl("not a github.com URL".to_string()));
    }
    let parts = parsed
        .path_segments()
        .map(|s| s.collect::<Vec<_>>())
        .unwrap_or_default();
    if parts.len() < 2 {
        return Err(GitHubError::InvalidUrl(
            "expected github.com/{owner}/{repo}".to_string(),
        ));
    }
    let owner = parts[0];
    let repo = parts[1];

    match parts.get(2).copied() {
        None => read_repository(client, owner, repo).await,
        Some("blob") if parts.len() >= 5 => {
            read_blob(client, owner, repo, parts[3], &parts[4..]).await
        }
        Some("tree") => read_repository(client, owner, repo).await,
        Some("issues") if parts.len() >= 4 => {
            read_issue(client, owner, repo, parts[3], false).await
        }
        Some("pull") if parts.len() >= 4 => read_issue(client, owner, repo, parts[3], true).await,
        Some("releases") => read_releases(client, owner, repo).await,
        _ => read_repository(client, owner, repo).await,
    }
}

async fn search_repositories(
    client: &Client,
    query: &str,
    max_results: usize,
    params: &Value,
) -> Result<SearchResponse, GitHubError> {
    let search_query = build_query(query, params, None)?;
    let data = github_get(
        client,
        &format!("{API}/search/repositories"),
        &[
            ("q", search_query.as_str()),
            ("sort", "stars"),
            ("order", "desc"),
            ("per_page", &max_results.min(20).to_string()),
        ],
    )
    .await?;
    Ok(SearchResponse {
        results: items(&data)
            .iter()
            .map(repository_result_from_json)
            .collect(),
        answer: None,
        provider: SearchProvider::GitHub,
    })
}

async fn search_issues(
    client: &Client,
    query: &str,
    max_results: usize,
    params: &Value,
    kind: &str,
) -> Result<SearchResponse, GitHubError> {
    let search_query = build_query(query, params, Some(kind))?;
    let data = github_get(
        client,
        &format!("{API}/search/issues"),
        &[
            ("q", search_query.as_str()),
            ("sort", "updated"),
            ("order", "desc"),
            ("per_page", &max_results.min(20).to_string()),
        ],
    )
    .await?;
    Ok(SearchResponse {
        results: items(&data)
            .iter()
            .map(|item| issue_result_from_json(item, kind))
            .collect(),
        answer: None,
        provider: SearchProvider::GitHub,
    })
}

async fn search_code(
    client: &Client,
    query: &str,
    max_results: usize,
    params: &Value,
) -> Result<SearchResponse, GitHubError> {
    let search_query = build_query(query, params, None)?;
    let data = github_get(
        client,
        &format!("{API}/search/code"),
        &[
            ("q", search_query.as_str()),
            ("per_page", &max_results.min(20).to_string()),
        ],
    )
    .await?;
    Ok(SearchResponse {
        results: items(&data).iter().map(code_result_from_json).collect(),
        answer: None,
        provider: SearchProvider::GitHub,
    })
}

async fn read_repository(
    client: &Client,
    owner: &str,
    repo: &str,
) -> Result<GitHubRead, GitHubError> {
    let repo_json = github_get(client, &format!("{API}/repos/{owner}/{repo}"), &[]).await?;
    let readme = github_get(client, &format!("{API}/repos/{owner}/{repo}/readme"), &[])
        .await
        .ok()
        .and_then(|v| v["download_url"].as_str().map(str::to_string));
    let readme_text = if let Some(url) = readme {
        fetch_text(client, &url).await.ok()
    } else {
        None
    };
    let title = repo_json["full_name"]
        .as_str()
        .unwrap_or("GitHub repository")
        .to_string();
    let mut text = format!("# {title}\n\n{}\n\nStars: {}\nForks: {}\nLanguage: {}\nLicense: {}\nArchived: {}\nPushed at: {}\nDefault branch: {}\n",
        repo_json["description"].as_str().unwrap_or(""),
        repo_json["stargazers_count"].as_u64().unwrap_or(0),
        repo_json["forks_count"].as_u64().unwrap_or(0),
        repo_json["language"].as_str().unwrap_or("unknown"),
        repo_json["license"]["spdx_id"].as_str().unwrap_or("unknown"),
        repo_json["archived"].as_bool().unwrap_or(false),
        repo_json["pushed_at"].as_str().unwrap_or("unknown"),
        repo_json["default_branch"].as_str().unwrap_or("unknown"));
    if let Some(readme) = readme_text {
        text.push_str("\n--- README ---\n");
        text.push_str(&readme);
    }
    Ok(GitHubRead {
        kind: "repository".into(),
        title,
        url: repo_json["html_url"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        text,
        details: repo_json,
    })
}

async fn read_blob(
    client: &Client,
    owner: &str,
    repo: &str,
    reference: &str,
    path: &[&str],
) -> Result<GitHubRead, GitHubError> {
    let path = path.join("/");
    let url = format!("{RAW}/{owner}/{repo}/{reference}/{path}");
    let text = fetch_text(client, &url).await?;
    Ok(GitHubRead {
        kind: "file".into(),
        title: format!("{owner}/{repo}/{path}"),
        url,
        text,
        details: json!({"owner": owner, "repo": repo, "ref": reference, "path": path}),
    })
}

async fn read_issue(
    client: &Client,
    owner: &str,
    repo: &str,
    number: &str,
    pull: bool,
) -> Result<GitHubRead, GitHubError> {
    let issue = github_get(
        client,
        &format!("{API}/repos/{owner}/{repo}/issues/{number}"),
        &[],
    )
    .await?;
    let comments = github_get(
        client,
        &format!("{API}/repos/{owner}/{repo}/issues/{number}/comments"),
        &[("per_page", "20")],
    )
    .await
    .unwrap_or_else(|_| json!([]));
    let title = issue["title"]
        .as_str()
        .unwrap_or("GitHub issue")
        .to_string();
    let mut text = format!(
        "# {title}\n\nState: {}\nAuthor: {}\nCreated: {}\nUpdated: {}\n\n{}\n",
        issue["state"].as_str().unwrap_or("unknown"),
        issue["user"]["login"].as_str().unwrap_or("unknown"),
        issue["created_at"].as_str().unwrap_or("unknown"),
        issue["updated_at"].as_str().unwrap_or("unknown"),
        issue["body"].as_str().unwrap_or("")
    );
    if let Some(comments) = comments.as_array() {
        for c in comments {
            text.push_str(&format!(
                "\n--- Comment by {} at {} ---\n{}\n",
                c["user"]["login"].as_str().unwrap_or("unknown"),
                c["created_at"].as_str().unwrap_or("unknown"),
                c["body"].as_str().unwrap_or("")
            ));
        }
    }
    Ok(GitHubRead {
        kind: if pull { "pull_request" } else { "issue" }.into(),
        title,
        url: issue["html_url"].as_str().unwrap_or_default().to_string(),
        text,
        details: issue,
    })
}

async fn read_releases(
    client: &Client,
    owner: &str,
    repo: &str,
) -> Result<GitHubRead, GitHubError> {
    let releases = github_get(
        client,
        &format!("{API}/repos/{owner}/{repo}/releases"),
        &[("per_page", "10")],
    )
    .await?;
    let mut text = format!("# {owner}/{repo} releases\n");
    if let Some(items) = releases.as_array() {
        for r in items {
            text.push_str(&format!(
                "\n## {} ({})\nPublished: {}\nPrerelease: {}\n\n{}\n",
                r["name"]
                    .as_str()
                    .or_else(|| r["tag_name"].as_str())
                    .unwrap_or("release"),
                r["tag_name"].as_str().unwrap_or("unknown"),
                r["published_at"].as_str().unwrap_or("unknown"),
                r["prerelease"].as_bool().unwrap_or(false),
                r["body"].as_str().unwrap_or("")
            ));
        }
    }
    Ok(GitHubRead {
        kind: "releases".into(),
        title: format!("{owner}/{repo} releases"),
        url: format!("https://github.com/{owner}/{repo}/releases"),
        text,
        details: releases,
    })
}

async fn github_get(
    client: &Client,
    url: &str,
    query: &[(&str, &str)],
) -> Result<Value, GitHubError> {
    let mut req = client.get(url).headers(github_headers()).query(query);
    if let Some(token) = resolve_github_token() {
        req = req.bearer_auth(token);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| GitHubError::Request(e.to_string()))?;
    let status = resp.status();
    let data: Value = resp
        .json()
        .await
        .map_err(|e| GitHubError::Parse(e.to_string()))?;
    if !status.is_success() {
        return Err(GitHubError::Api(format!(
            "GitHub {status}: {}",
            redacted_github_error_message(&data)
        )));
    }
    Ok(data)
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, GitHubError> {
    let mut req = client.get(url).headers(github_headers());
    if let Some(token) = resolve_github_token() {
        req = req.bearer_auth(token);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| GitHubError::Request(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(GitHubError::Api(format!(
            "GitHub {status}: failed to fetch content"
        )));
    }
    resp.text()
        .await
        .map_err(|e| GitHubError::Parse(e.to_string()))
}

fn github_headers() -> reqwest::header::HeaderMap {
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("imp-web-github"));
    headers
}

fn search_type(params: &Value) -> &str {
    params
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("repositories")
}
fn items(data: &Value) -> Vec<Value> {
    data.get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn build_query(query: &str, params: &Value, kind: Option<&str>) -> Result<String, GitHubError> {
    let mut parts = vec![query.trim().to_string()];
    if let Some(kind) = kind {
        parts.push(format!("type:{kind}"));
    }
    if let (Some(owner), Some(repo)) = (
        params.get("owner").and_then(Value::as_str),
        params.get("repo").and_then(Value::as_str),
    ) {
        parts.push(format!("repo:{owner}/{repo}"));
    }
    push_qualifier(
        &mut parts,
        "user",
        params.get("owner").filter(|_| params.get("repo").is_none()),
    );
    push_qualifier(&mut parts, "org", params.get("org"));
    push_qualifier(&mut parts, "language", params.get("language"));
    push_qualifier(&mut parts, "topic", params.get("topic"));
    push_qualifier(&mut parts, "pushed", params.get("updated_since"));
    push_qualifier(&mut parts, "state", params.get("state"));
    if let Some(min_stars) = params
        .get("min_stars")
        .or_else(|| params.get("minStars"))
        .and_then(Value::as_u64)
    {
        parts.push(format!("stars:>={min_stars}"));
    }
    Ok(parts.join(" "))
}

fn push_qualifier(parts: &mut Vec<String>, name: &str, value: Option<&Value>) {
    if let Some(value) = value
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        parts.push(format!("{name}:{}", value.trim()));
    }
}

fn repository_result_from_json(repo: &Value) -> SearchResult {
    let full_name = repo["full_name"].as_str().unwrap_or("unknown/unknown");
    SearchResult {
        title: full_name.to_string(),
        url: repo["html_url"].as_str().unwrap_or_default().to_string(),
        snippet: repo["description"].as_str().map(String::from),
        date: repo["updated_at"].as_str().map(String::from),
        source_type: Some("github".into()),
        kind: Some("repository".into()),
        metadata: Some(
            json!({"owner": repo["owner"]["login"].as_str(), "repo": repo["name"].as_str(), "full_name": full_name, "language": repo["language"].as_str(), "stars": repo["stargazers_count"].as_u64(), "forks": repo["forks_count"].as_u64(), "license": repo["license"]["spdx_id"].as_str(), "archived": repo["archived"].as_bool(), "fork": repo["fork"].as_bool(), "default_branch": repo["default_branch"].as_str(), "pushed_at": repo["pushed_at"].as_str(), "updated_at": repo["updated_at"].as_str(), "topics": repo["topics"].as_array().cloned().unwrap_or_default()}),
        ),
    }
}

fn issue_result_from_json(issue: &Value, kind: &str) -> SearchResult {
    SearchResult {
        title: issue["title"]
            .as_str()
            .unwrap_or("GitHub issue")
            .to_string(),
        url: issue["html_url"].as_str().unwrap_or_default().to_string(),
        snippet: issue["body"]
            .as_str()
            .map(|s| s.chars().take(500).collect()),
        date: issue["updated_at"].as_str().map(String::from),
        source_type: Some("github".into()),
        kind: Some(kind.into()),
        metadata: Some(
            json!({"state": issue["state"].as_str(), "comments": issue["comments"].as_u64(), "author": issue["user"]["login"].as_str(), "created_at": issue["created_at"].as_str(), "updated_at": issue["updated_at"].as_str(), "repository_url": issue["repository_url"].as_str()}),
        ),
    }
}

fn code_result_from_json(code: &Value) -> SearchResult {
    SearchResult {
        title: code["name"]
            .as_str()
            .unwrap_or("GitHub code result")
            .to_string(),
        url: code["html_url"].as_str().unwrap_or_default().to_string(),
        snippet: code["path"].as_str().map(String::from),
        date: None,
        source_type: Some("github".into()),
        kind: Some("code".into()),
        metadata: Some(
            json!({"path": code["path"].as_str(), "repo": code["repository"]["full_name"].as_str(), "owner": code["repository"]["owner"]["login"].as_str()}),
        ),
    }
}

fn redacted_github_error_message(data: &Value) -> String {
    match data.get("message").and_then(Value::as_str) {
        Some(message) => imp_llm::auth::redact_provider_error_body(message),
        None => imp_llm::auth::redact_provider_error_body(&data.to_string()),
    }
}

fn resolve_github_token() -> Option<String> {
    std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.trim().is_empty())
        .or_else(|| {
            let auth_path = crate::storage::existing_global_auth_path()
                .unwrap_or_else(crate::storage::global_auth_path);
            let auth_store = AuthStore::load(&auth_path).ok()?;
            auth_store.resolve_api_key_only("github").ok()
        })
}

pub struct GitHubRead {
    pub kind: String,
    pub title: String,
    pub url: String,
    pub text: String,
    pub details: Value,
}

#[derive(Debug)]
pub enum GitHubError {
    UnsupportedType(String),
    InvalidUrl(String),
    Request(String),
    Api(String),
    Parse(String),
}

impl std::fmt::Display for GitHubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedType(msg) => write!(f, "{msg}"),
            Self::InvalidUrl(msg) => write!(f, "Invalid GitHub URL: {msg}"),
            Self::Request(msg) => write!(f, "GitHub request failed: {msg}"),
            Self::Api(msg) => write!(f, "GitHub API error: {msg}"),
            Self::Parse(msg) => write!(f, "Failed to parse GitHub response: {msg}"),
        }
    }
}
impl std::error::Error for GitHubError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_github_error_message_removes_token_like_fields() {
        let data = json!({
            "message": "bad credentials access_token=github-secret"
        });
        let message = redacted_github_error_message(&data);
        assert!(!message.contains("github-secret"));
        assert!(message.contains("[REDACTED]"));
    }

    #[test]
    fn detects_only_github_urls() {
        assert!(is_github_url("https://github.com/owner/repo"));
        assert!(is_github_url("https://github.com./owner/repo"));
        assert!(!is_github_url(
            "https://raw.githubusercontent.com/owner/repo/main/README.md"
        ));
        assert!(!is_github_url("https://example.com/owner/repo"));
        assert!(!is_github_url("not a url"));
    }

    #[test]
    fn search_type_defaults_to_repositories() {
        assert_eq!(search_type(&Value::Null), "repositories");
        assert_eq!(search_type(&json!({"type":"code"})), "code");
    }

    #[test]
    fn build_repository_query_adds_supported_qualifiers() {
        let query = build_query(
            "rust html parser",
            &json!({
                "type": "repositories",
                "language": "Rust",
                "topic": "html",
                "min_stars": 100,
                "updated_since": "2025-01-01"
            }),
            None,
        )
        .unwrap();
        assert_eq!(
            query,
            "rust html parser language:Rust topic:html pushed:2025-01-01 stars:>=100"
        );
    }

    #[test]
    fn build_issue_query_adds_type_and_repo_without_user_qualifier() {
        let query = build_query(
            "memory leak",
            &json!({"owner":"owner", "repo":"repo", "state":"open"}),
            Some("issue"),
        )
        .unwrap();
        assert_eq!(query, "memory leak type:issue repo:owner/repo state:open");
    }

    #[test]
    fn build_owner_query_adds_user_qualifier_when_repo_absent() {
        let query = build_query("crawler", &json!({"owner":"microsoft"}), None).unwrap();
        assert_eq!(query, "crawler user:microsoft");
    }

    #[test]
    fn repository_result_preserves_github_metadata() {
        let result = repository_result_from_json(&repository_fixture());
        assert_eq!(result.title, "owner/repo");
        assert_eq!(result.source_type.as_deref(), Some("github"));
        assert_eq!(result.kind.as_deref(), Some("repository"));
        let metadata = result.metadata.as_ref().unwrap();
        assert_eq!(metadata["stars"], 42);
        assert_eq!(metadata["license"], "MIT");
        assert_eq!(metadata["topics"][0], "agent");
    }

    #[test]
    fn issue_result_preserves_state_and_author_metadata() {
        let result = issue_result_from_json(
            &json!({
                "title": "Bug: memory leak",
                "html_url": "https://github.com/owner/repo/issues/12",
                "body": "A".repeat(600),
                "state": "open",
                "comments": 3,
                "user": {"login": "asher"},
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-02T00:00:00Z",
                "repository_url": "https://api.github.com/repos/owner/repo"
            }),
            "issue",
        );

        assert_eq!(result.kind.as_deref(), Some("issue"));
        assert_eq!(result.title, "Bug: memory leak");
        assert_eq!(result.snippet.as_ref().unwrap().chars().count(), 500);
        let metadata = result.metadata.as_ref().unwrap();
        assert_eq!(metadata["state"], "open");
        assert_eq!(metadata["author"], "asher");
    }

    #[test]
    fn code_result_preserves_path_and_repo_metadata() {
        let result = code_result_from_json(&json!({
            "name": "lib.rs",
            "html_url": "https://github.com/owner/repo/blob/main/src/lib.rs",
            "path": "src/lib.rs",
            "repository": {
                "full_name": "owner/repo",
                "owner": {"login": "owner"}
            }
        }));

        assert_eq!(result.kind.as_deref(), Some("code"));
        assert_eq!(result.snippet.as_deref(), Some("src/lib.rs"));
        let metadata = result.metadata.as_ref().unwrap();
        assert_eq!(metadata["repo"], "owner/repo");
        assert_eq!(metadata["path"], "src/lib.rs");
    }

    #[test]
    fn unsupported_releases_search_reports_read_guidance() {
        let err = futures::executor::block_on(search(
            &Client::new(),
            "latest",
            5,
            Some(&json!({"type":"releases"})),
        ))
        .unwrap_err();
        assert!(err.to_string().contains("use web.read"));
    }

    fn repository_fixture() -> Value {
        json!({
            "full_name":"owner/repo",
            "html_url":"https://github.com/owner/repo",
            "description":"Example repo",
            "updated_at":"2026-01-01T00:00:00Z",
            "pushed_at":"2026-01-02T00:00:00Z",
            "owner":{"login":"owner"},
            "name":"repo",
            "language":"Rust",
            "stargazers_count":42,
            "forks_count":7,
            "license":{"spdx_id":"MIT"},
            "archived":false,
            "fork":false,
            "default_branch":"main",
            "topics":["agent"]
        })
    }
}
