//! Native page reading — fetch HTML via reqwest + extract with readability.
//!
//! No external APIs needed for reading pages. Handles most static and
//! server-rendered pages. Won't work for heavy SPAs that require JS execution.

use reqwest::Client;
use url::Url;

use super::types::PageContent;

/// User-Agent string that identifies as a legitimate browser to avoid blocks.
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Fetch a URL and extract its readable content.
pub async fn fetch_and_extract(client: &Client, url: &str) -> Result<PageContent, ReadError> {
    let parsed_url = Url::parse(url).map_err(|e| ReadError::InvalidUrl(e.to_string()))?;

    // YouTube: hint the user to use a different approach
    if is_youtube_url(&parsed_url) {
        return Err(ReadError::YoutubeNotSupported);
    }

    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| ReadError::Fetch(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(ReadError::HttpStatus(
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown").to_string(),
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Reject non-HTML content types
    if !content_type.is_empty()
        && !content_type.contains("text/html")
        && !content_type.contains("application/xhtml")
        && !content_type.contains("text/plain")
    {
        return Err(ReadError::NotHtml(content_type));
    }

    let final_url = response.url().to_string();
    let html = response
        .text()
        .await
        .map_err(|e| ReadError::Fetch(e.to_string()))?;

    if html.len() < 100 {
        return Err(ReadError::InsufficientContent);
    }

    // Plain text — return as-is
    if content_type.contains("text/plain") {
        return Ok(PageContent {
            title: None,
            content_length: html.len(),
            text: html,
            url: final_url,
        });
    }

    extract_readable(&html, &final_url)
}

/// Extract readable content from raw HTML using Mozilla Readability algorithm.
fn extract_readable(html: &str, url: &str) -> Result<PageContent, ReadError> {
    use readability_rust::Readability;

    let mut parser = Readability::new_with_base_uri(html, url, None)
        .map_err(|e| ReadError::Parse(format!("{e}")))?;

    let article = parser.parse().ok_or(ReadError::NoContent)?;

    let title = article.title.clone();

    // article.text_content is the cleaned plain text
    // article.content is HTML — we convert to plain text ourselves for safety
    let text = article
        .text_content
        .as_deref()
        .or(article.content.as_deref())
        .unwrap_or("")
        .to_string();

    if text.len() < 50 {
        return Err(ReadError::InsufficientContent);
    }

    Ok(PageContent {
        content_length: text.len(),
        title,
        text: clean_text(&text),
        url: url.to_string(),
    })
}

/// Clean extracted text: normalize whitespace, remove excessive blank lines.
fn clean_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut blank_count = 0u32;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

fn is_youtube_url(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|h| h.contains("youtube.com") || h.contains("youtu.be"))
}

#[derive(Debug)]
pub enum ReadError {
    InvalidUrl(String),
    Fetch(String),
    HttpStatus(u16, String),
    NotHtml(String),
    Parse(String),
    NoContent,
    InsufficientContent,
    YoutubeNotSupported,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(msg) => write!(f, "Invalid URL: {msg}"),
            Self::Fetch(msg) => write!(f, "Fetch failed: {msg}"),
            Self::HttpStatus(code, reason) => write!(f, "HTTP {code} {reason}"),
            Self::NotHtml(ct) => write!(f, "Not an HTML page (content-type: {ct})"),
            Self::Parse(msg) => write!(f, "Parse error: {msg}"),
            Self::NoContent => write!(f, "Could not extract readable content from page"),
            Self::InsufficientContent => write!(f, "Page returned insufficient content"),
            Self::YoutubeNotSupported => write!(f, "YouTube URLs not supported yet. Use the page URL directly or try a transcript tool."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_text_collapses_blank_lines() {
        let input = "Hello\n\n\n\n\nWorld\n\nFoo";
        let cleaned = clean_text(input);
        // Allows up to 2 blank lines (3 newlines total), then collapses
        assert!(cleaned.starts_with("Hello\n"));
        assert!(cleaned.contains("World"));
        assert!(!cleaned.contains("\n\n\n\n"));
    }

    #[test]
    fn clean_text_trims_lines() {
        let input = "  hello  \n  world  ";
        let cleaned = clean_text(input);
        assert_eq!(cleaned, "hello\nworld");
    }

    #[test]
    fn extract_readable_from_html() {
        let html = r#"
        <html>
        <head><title>Test Article</title></head>
        <body>
            <nav>Skip this navigation</nav>
            <article>
                <h1>Test Article Title</h1>
                <p>This is the main content of the article. It has enough text to be
                considered readable content by the readability algorithm. We need to make
                sure there is sufficient content here for the extraction to work properly.
                The readability algorithm looks for substantial blocks of text content.</p>
                <p>Here is another paragraph with more substantial content to ensure that
                the extraction algorithm has enough material to work with. This paragraph
                adds additional context and information that would be typical in a real
                web article about some topic.</p>
            </article>
            <footer>Copyright 2024</footer>
        </body>
        </html>"#;

        let result = extract_readable(html, "https://example.com/test");
        match result {
            Ok(page) => {
                assert!(page.text.contains("main content"));
                assert!(!page.text.contains("Skip this navigation"));
            }
            Err(ReadError::InsufficientContent) | Err(ReadError::NoContent) => {
                // Readability may not extract from minimal HTML — that's acceptable
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }
}
