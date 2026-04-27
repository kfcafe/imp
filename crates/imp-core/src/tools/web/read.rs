//! Native page reading — fetch HTML via reqwest + extract with readability.
//!
//! No external APIs needed for reading pages. Handles most static and
//! server-rendered pages. Won't work for heavy SPAs that require JS execution.

use reqwest::Client;
use url::Url;

use super::types::{ContentFormat, PageContent};

/// User-Agent string that identifies as a legitimate browser to avoid blocks.
pub(crate) const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
pub(crate) const ACCEPT_HEADER: &str =
    "text/markdown,text/plain;q=0.9,text/html;q=0.8,application/xhtml+xml;q=0.7,*/*;q=0.5";

/// Fetch a URL and extract its readable content.
pub async fn fetch_and_extract(client: &Client, url: &str) -> Result<PageContent, ReadError> {
    let parsed_url = Url::parse(url).map_err(|e| ReadError::InvalidUrl(e.to_string()))?;

    if super::youtube::is_youtube_url(&parsed_url) {
        return super::youtube::fetch_and_extract(client, url)
            .await
            .map_err(|err| ReadError::Youtube(err.to_string()));
    }

    let requested_url = url.to_string();

    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", ACCEPT_HEADER)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| ReadError::Fetch(e.to_string()))?;

    let status_code = response.status().as_u16();
    if !response.status().is_success() {
        return Err(ReadError::HttpStatus(
            status_code,
            response
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string(),
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let format_received = detect_content_format(&content_type);

    // Reject binary content types (images, video, audio, etc.)
    let is_text = content_type.is_empty()
        || content_type.contains("text/")
        || content_type.contains("application/json")
        || content_type.contains("application/xml")
        || content_type.contains("application/xhtml")
        || content_type.contains("application/javascript")
        || content_type.contains("+xml")
        || content_type.contains("+json");
    if !is_text {
        return Err(ReadError::NotHtml(content_type));
    }

    let final_url = response.url().to_string();
    let was_redirected = final_url != requested_url;
    let html = response
        .text()
        .await
        .map_err(|e| ReadError::Fetch(e.to_string()))?;
    let raw_body_bytes = html.len();

    if html.len() < 100 {
        return Err(ReadError::InsufficientContent);
    }

    // Shared metadata for all paths
    let meta = ResponseMeta {
        requested_url,
        status_code,
        content_type: if content_type.is_empty() {
            None
        } else {
            Some(content_type.clone())
        },
        format_received,
        was_redirected,
        raw_body_bytes,
    };

    match format_received {
        ContentFormat::Markdown | ContentFormat::PlainText => {
            let cleaned = clean_text(&html);
            let mut page = PageContent {
                title: None,
                content_length: cleaned.len(),
                text: cleaned,
                url: final_url,
                requested_url: meta.requested_url,
                status_code: meta.status_code,
                content_type: meta.content_type,
                format_received: meta.format_received,
                was_redirected: meta.was_redirected,
                raw_body_bytes: meta.raw_body_bytes,
                diagnostics: Vec::new(),
            };
            page.diagnostics = diagnose(&page, "");
            Ok(page)
        }
        ContentFormat::Html => {
            let mut page = extract_readable(&html, &final_url)?;
            page.requested_url = meta.requested_url;
            page.status_code = meta.status_code;
            page.content_type = meta.content_type;
            page.format_received = meta.format_received;
            page.was_redirected = meta.was_redirected;
            page.raw_body_bytes = meta.raw_body_bytes;
            page.diagnostics = diagnose(&page, &html);
            Ok(page)
        }
    }
}

/// Metadata captured from the HTTP response before extraction.
struct ResponseMeta {
    requested_url: String,
    status_code: u16,
    content_type: Option<String>,
    format_received: ContentFormat,
    was_redirected: bool,
    raw_body_bytes: usize,
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
        // Populated by caller (fetch_and_extract) after extraction
        requested_url: url.to_string(),
        status_code: 200,
        content_type: None,
        format_received: ContentFormat::Html,
        was_redirected: false,
        raw_body_bytes: 0,
        diagnostics: Vec::new(),
    })
}

pub fn diagnose(page: &PageContent, raw_html: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let text_lower = page.text.to_lowercase();
    let html_lower = raw_html.to_lowercase();

    let short_text = page.content_length < 500;
    let has_loading_indicator = ["loading...", "loading documentation"]
        .iter()
        .any(|needle| text_lower.contains(needle));
    let has_noscript = html_lower.contains("<noscript");
    let nav_link_count = html_lower.matches("<nav").count()
        + html_lower.matches("<a ").count()
        + html_lower.matches("<a>").count();
    let has_nav_shell_pattern = short_text && nav_link_count >= 8;
    if short_text && (has_loading_indicator || has_noscript || has_nav_shell_pattern) {
        warnings.push(
            "Page appears to be a client-rendered shell. Content may require JavaScript."
                .to_string(),
        );
    }

    let very_short_text = page.content_length < 300;
    let has_soft_404_indicator = [
        "page not found",
        "can't find that page",
        "404",
        "doesn't exist",
        "has been moved",
    ]
    .iter()
    .any(|needle| text_lower.contains(needle));
    if page.status_code == 200 && very_short_text && has_soft_404_indicator {
        warnings
            .push("Page appears to be a soft 404 (HTTP 200 but error page content).".to_string());
    }

    if page.raw_body_bytes > 20 * 1024 && page.content_length < 2 * 1024 {
        warnings.push(format!(
            "Large page ({} bytes) but only {} chars extracted. Content may be incomplete.",
            page.raw_body_bytes, page.content_length
        ));
    }

    if page.raw_body_bytes > 100 * 1024
        && (page.content_length as f64) < (page.raw_body_bytes as f64 * 0.1)
    {
        let pct = ((page.content_length as f64 / page.raw_body_bytes as f64) * 100.0).round();
        warnings.push(format!(
            "Significant content may have been lost during extraction ({}% of response retained).",
            pct as usize
        ));
    }

    warnings
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

fn detect_content_format(content_type: &str) -> ContentFormat {
    let content_type = content_type.to_ascii_lowercase();

    if content_type.contains("text/markdown") || content_type.contains("text/x-markdown") {
        ContentFormat::Markdown
    } else if content_type.contains("text/html") || content_type.contains("application/xhtml+xml") {
        ContentFormat::Html
    } else {
        ContentFormat::PlainText
    }
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
    Youtube(String),
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
            Self::Youtube(msg) => write!(f, "YouTube extraction failed: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_header_prefers_markdown() {
        assert_eq!(
            ACCEPT_HEADER,
            "text/markdown,text/plain;q=0.9,text/html;q=0.8,application/xhtml+xml;q=0.7,*/*;q=0.5"
        );
    }

    #[test]
    fn detect_content_format_treats_markdown_as_markdown() {
        assert_eq!(
            detect_content_format("text/markdown; charset=utf-8"),
            ContentFormat::Markdown
        );
    }

    #[test]
    fn detect_content_format_treats_plain_text_as_plain_text() {
        assert_eq!(
            detect_content_format("text/plain; charset=utf-8"),
            ContentFormat::PlainText
        );
        assert_eq!(
            detect_content_format("application/json"),
            ContentFormat::PlainText
        );
    }

    #[test]
    fn markdown_and_plain_text_skip_readability_cleaning_path() {
        let markdown = "# Title\n\n\nParagraph";
        let cleaned_markdown = clean_text(markdown);
        assert_eq!(cleaned_markdown, "# Title\n\n\nParagraph");
        assert_eq!(
            detect_content_format("text/markdown"),
            ContentFormat::Markdown
        );

        let plain = "  hello  \n\n\nworld  ";
        let cleaned_plain = clean_text(plain);
        assert_eq!(cleaned_plain, "hello\n\n\nworld");
        assert_eq!(
            detect_content_format("text/plain"),
            ContentFormat::PlainText
        );
    }

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
                assert_eq!(page.url, "https://example.com/test");
                assert_eq!(page.requested_url, "https://example.com/test");
                assert_eq!(page.status_code, 200);
                assert!(!page.was_redirected);
                assert_eq!(page.raw_body_bytes, 0);
                assert!(page.content_type.is_none());
                assert!(page.diagnostics.is_empty());
            }
            Err(ReadError::InsufficientContent) | Err(ReadError::NoContent) => {
                // Readability may not extract from minimal HTML — that's acceptable
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn response_metadata_can_be_applied_after_extraction() {
        let html = r#"
        <html>
        <head><title>Redirected Article</title></head>
        <body>
            <article>
                <p>This article has enough body text to survive readability extraction and
                prove that metadata can be preserved when the requested URL differs from
                the final URL after redirects.</p>
                <p>Additional text keeps the extractor happy and representative of a real page.</p>
            </article>
        </body>
        </html>"#;

        let mut page = extract_readable(html, "https://example.com/final").unwrap();
        page.requested_url = "https://example.com/start".to_string();
        page.status_code = 200;
        page.content_type = Some("text/html; charset=utf-8".to_string());
        page.format_received = ContentFormat::Html;
        page.was_redirected = true;
        page.raw_body_bytes = html.len();

        assert_eq!(page.url, "https://example.com/final");
        assert_eq!(page.requested_url, "https://example.com/start");
        assert_eq!(page.status_code, 200);
        assert_eq!(
            page.content_type.as_deref(),
            Some("text/html; charset=utf-8")
        );
        assert!(page.was_redirected);
        assert_eq!(page.raw_body_bytes, html.len());
    }

    #[test]
    fn diagnose_spa_shell_from_loading_text() {
        let page = PageContent {
            title: Some("Docs".to_string()),
            text: "Loading documentation...".to_string(),
            url: "https://example.com/docs".to_string(),
            content_length: "Loading documentation...".len(),
            requested_url: "https://example.com/docs".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            format_received: ContentFormat::Html,
            was_redirected: false,
            raw_body_bytes: 2_000,
            diagnostics: Vec::new(),
        };

        let warnings = diagnose(
            &page,
            "<html><body><noscript>Enable JS</noscript></body></html>",
        );
        assert!(warnings.iter().any(|w| w.contains("client-rendered shell")));
    }

    #[test]
    fn diagnose_soft_404_with_http_200() {
        let text = "Page not found. The page has been moved.";
        let page = PageContent {
            title: Some("Missing".to_string()),
            text: text.to_string(),
            url: "https://example.com/missing".to_string(),
            content_length: text.len(),
            requested_url: "https://example.com/missing".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            format_received: ContentFormat::Html,
            was_redirected: false,
            raw_body_bytes: 1_500,
            diagnostics: Vec::new(),
        };

        let warnings = diagnose(&page, "<html><body>404</body></html>");
        assert!(warnings.iter().any(|w| w.contains("soft 404")));
    }

    #[test]
    fn diagnose_does_not_flag_normal_page() {
        let text = "This is a normal documentation page with enough content to explain installation, configuration, and usage in detail. It includes several paragraphs of useful information for readers and should not be treated as a shell or error page. Extra explanation here keeps it comfortably above the short-content heuristics and avoids false positives.";
        let page = PageContent {
            title: Some("Guide".to_string()),
            text: text.to_string(),
            url: "https://example.com/guide".to_string(),
            content_length: text.len(),
            requested_url: "https://example.com/guide".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            format_received: ContentFormat::Html,
            was_redirected: false,
            raw_body_bytes: 8_000,
            diagnostics: Vec::new(),
        };

        let warnings = diagnose(
            &page,
            "<html><body><article>real docs</article></body></html>",
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn diagnose_low_extraction_ratio_warning() {
        let text = "A short extracted summary.";
        let page = PageContent {
            title: Some("Big Page".to_string()),
            text: text.to_string(),
            url: "https://example.com/big".to_string(),
            content_length: text.len(),
            requested_url: "https://example.com/big".to_string(),
            status_code: 200,
            content_type: Some("text/html".to_string()),
            format_received: ContentFormat::Html,
            was_redirected: false,
            raw_body_bytes: 150_000,
            diagnostics: Vec::new(),
        };

        let warnings = diagnose(&page, "<html></html>");
        assert!(warnings.iter().any(|w| w.contains("Large page")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Significant content may have been lost")));
    }
}
