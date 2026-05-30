pub fn format_error_for_display(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Unknown error.".to_string();
    }

    if let Some(message) = extract_json_message(trimmed) {
        let lower_message = message.to_ascii_lowercase();
        if is_context_full_error(&lower_message) {
            return normalize_context_full_message(&message);
        }
        return with_auth_hint(&message);
    }

    let lower = trimmed.to_ascii_lowercase();
    if is_context_full_error(&lower) {
        return normalize_context_full_message(trimmed);
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

fn is_context_full_error(lower: &str) -> bool {
    lower.contains("context full")
        || lower.contains("context too long")
        || lower.contains("exceeds the") && lower.contains("token window")
        || lower.contains("maximum context")
        || lower.contains("context_length_exceeded")
}

fn normalize_context_full_message(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if message.contains("Run /compact") || message.contains("run /compact") {
        message.to_string()
    } else if lower.contains("context full") {
        format!("{message} Run /compact or start a new chat to continue.")
    } else {
        format!("Context full: {message}. Run /compact or start a new chat to continue.")
    }
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
    fn normalizes_context_full_errors() {
        let raw = "Context too long: 210000 tokens exceeds 200000";
        assert_eq!(
            format_error_for_display(raw),
            "Context full: Context too long: 210000 tokens exceeds 200000. Run /compact or start a new chat to continue."
        );
    }

    #[test]
    fn context_full_message_does_not_duplicate_compact_hint() {
        let raw = "Context full: estimated 210000 tokens exceeds the 200000 token window for claude. Run /compact or start a new chat to continue.";
        assert_eq!(format_error_for_display(raw), raw);
    }

    #[test]
    fn normalizes_provider_context_length_json_errors() {
        let raw = "Provider error: HTTP 400 Bad Request: {\"error\":{\"type\":\"context_length_exceeded\",\"message\":\"maximum context length is 200000 tokens\"}}";
        assert_eq!(
            format_error_for_display(raw),
            "Context full: maximum context length is 200000 tokens. Run /compact or start a new chat to continue."
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
