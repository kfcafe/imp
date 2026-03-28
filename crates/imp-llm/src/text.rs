/// Return the longest prefix containing at most `max_chars` Unicode scalar values.
#[must_use]
pub fn prefix_chars(s: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }

    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Truncate to at most `max_chars` characters.
#[must_use]
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    prefix_chars(s, max_chars).to_string()
}

/// Truncate to at most `max_chars` characters and append `suffix` when truncated.
#[must_use]
pub fn truncate_chars_with_suffix(s: &str, max_chars: usize, suffix: &str) -> String {
    let prefix = prefix_chars(s, max_chars);
    if prefix.len() == s.len() {
        s.to_string()
    } else {
        format!("{prefix}{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_chars_respects_unicode_boundaries() {
        let s = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa—bbb";
        let prefix = prefix_chars(s, 87);
        assert_eq!(prefix, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa—");
    }

    #[test]
    fn truncate_chars_with_suffix_only_appends_when_needed() {
        assert_eq!(truncate_chars_with_suffix("hello", 10, "…"), "hello");
        assert_eq!(truncate_chars_with_suffix("hello world", 5, "…"), "hello…");
    }

    #[test]
    fn truncate_chars_handles_zero() {
        assert_eq!(prefix_chars("hello", 0), "");
        assert_eq!(truncate_chars("hello", 0), "");
    }
}
