pub fn format_error_for_display(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Unknown error.".to_string();
    }

    if let Some(message) = extract_json_message(trimmed) {
        return with_auth_hint(&message);
    }

    if looks_like_html_error(trimmed) {
        let status = extract_http_status(trimmed);
        let title = extract_html_title(trimmed);

        return match (status, title) {
            (Some(status), Some(title)) => format!(
                "Provider returned an HTML error page ({status}: {title}). This usually means an auth, gateway, proxy, or rate-limit issue."
            ),
            (Some(status), None) => format!(
                "Provider returned an HTML error page ({status}). This usually means an auth, gateway, proxy, or rate-limit issue."
            ),
            (None, Some(title)) => format!(
                "Provider returned an HTML error page ({title}). This usually means an auth, gateway, proxy, or rate-limit issue."
            ),
            (None, None) => "Provider returned an HTML error page. This usually means an auth, gateway, proxy, or rate-limit issue.".to_string(),
        };
    }

    trimmed.to_string()
}

fn extract_json_message(raw: &str) -> Option<String> {
    let json_start = raw.find('{')?;
    let parsed = serde_json::from_str::<serde_json::Value>(&raw[json_start..]).ok()?;

    parsed
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            parsed
                .get("message")
                .and_then(|message| message.as_str())
                .map(ToOwned::to_owned)
        })
}

fn with_auth_hint(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    let needs_login_hint = lower.contains("expired")
        || lower.contains("oauth")
        || (lower.contains("token")
            && (lower.contains("expired")
                || lower.contains("invalid")
                || lower.contains("refresh")));

    if needs_login_hint {
        format!("{message} (use /login to refresh)")
    } else {
        message.to_string()
    }
}

fn looks_like_html_error(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("<!doctype html")
        || lower.contains("<html")
        || lower.contains("<head")
        || lower.contains("<body")
        || lower.contains("<title")
}

fn extract_http_status(raw: &str) -> Option<String> {
    let start = raw.find("HTTP ")?;
    let rest = &raw[start..];
    let end = rest.find([':', '\n', '<']).unwrap_or(rest.len());
    let status = rest[..end].trim();
    (!status.is_empty()).then(|| status.to_string())
}

fn extract_html_title(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let title_start = lower.find("<title")?;
    let open_end = lower[title_start..].find('>')? + title_start + 1;
    let close_start = lower[open_end..].find("</title>")? + open_end;
    let title = raw[open_end..close_start].trim();
    (!title.is_empty()).then(|| title.to_string())
}

#[cfg(test)]
mod tests {
    use super::format_error_for_display;

    #[test]
    fn extracts_nested_json_error_message() {
        let raw = "Provider error: HTTP 401 Unauthorized: {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"OAuth token has expired\"}}";
        assert_eq!(
            format_error_for_display(raw),
            "OAuth token has expired (use /login to refresh)"
        );
    }

    #[test]
    fn extracts_simple_json_message() {
        let raw = "Provider error: HTTP 429 Too Many Requests: {\"message\":\"Rate limited\"}";
        assert_eq!(format_error_for_display(raw), "Rate limited");
    }

    #[test]
    fn collapses_html_error_pages_to_summary() {
        let raw = "Provider error: HTTP 403 Forbidden: <!DOCTYPE html><html><head><title>Attention Required! | Cloudflare</title></head><body>blocked</body></html>";
        assert_eq!(
            format_error_for_display(raw),
            "Provider returned an HTML error page (HTTP 403 Forbidden: Attention Required! | Cloudflare). This usually means an auth, gateway, proxy, or rate-limit issue."
        );
    }

    #[test]
    fn leaves_plain_text_errors_alone() {
        let raw = "Provider error: connection reset by peer";
        assert_eq!(format_error_for_display(raw), raw);
    }

    #[test]
    fn replaces_empty_errors_with_unknown_error() {
        assert_eq!(format_error_for_display("   \n"), "Unknown error.");
    }
}
