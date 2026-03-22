//! Boolean query parser with stemming for code search.
//!
//! Supports Elasticsearch-style queries:
//! - Boolean: "error AND handling", "login OR auth", "database NOT sqlite"
//! - Phrases: `"user authentication"` (exact match)
//! - Plain terms: matched with optional stemming
//!
//! Default operator between terms is OR.

use std::path::Path;

use regex::Regex;

/// A parsed search query with boolean semantics.
#[derive(Debug)]
pub struct Query {
    /// All must match (AND).
    pub must: Vec<Matcher>,
    /// At least one must match (OR / default).
    pub should: Vec<Matcher>,
    /// None may match (NOT).
    pub must_not: Vec<Matcher>,
}

/// A compiled matcher for a single term or phrase.
#[derive(Debug)]
pub struct Matcher {
    pub regex: Regex,
    pub original: String,
}

impl Query {
    /// Does this query match a given line?
    pub fn matches_line(&self, line: &str) -> bool {
        // All must_not must NOT match
        if self.must_not.iter().any(|m| m.regex.is_match(line)) {
            return false;
        }

        // All must terms must match
        let must_ok = self.must.is_empty() || self.must.iter().all(|m| m.regex.is_match(line));

        // At least one should term must match (if any exist)
        let should_ok =
            self.should.is_empty() || self.should.iter().any(|m| m.regex.is_match(line));

        must_ok && should_ok
    }

    /// Does this query match anywhere in a file's content?
    /// For AND semantics: each must term appears somewhere in the file.
    /// For OR semantics: at least one should term appears somewhere.
    /// For NOT: no must_not term appears anywhere.
    pub fn matches_file(&self, content: &str) -> bool {
        if self
            .must_not
            .iter()
            .any(|m| content.lines().any(|l| m.regex.is_match(l)))
        {
            return false;
        }

        let must_ok = self
            .must
            .iter()
            .all(|m| content.lines().any(|l| m.regex.is_match(l)));

        let should_ok = self.should.is_empty()
            || self
                .should
                .iter()
                .any(|m| content.lines().any(|l| m.regex.is_match(l)));

        must_ok && should_ok
    }

    /// Find lines in content that match any positive term.
    /// Used after `matches_file` confirms the file is relevant.
    pub fn matching_lines(&self, content: &str) -> Vec<usize> {
        content
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                let any_positive = self.must.iter().any(|m| m.regex.is_match(line))
                    || self.should.iter().any(|m| m.regex.is_match(line));
                let no_negative = !self.must_not.iter().any(|m| m.regex.is_match(line));
                any_positive && no_negative
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Is this a simple single-term query? (optimization path)
    pub fn is_simple(&self) -> bool {
        self.must.is_empty() && self.must_not.is_empty() && self.should.len() == 1
    }

    /// Get the single regex for simple queries.
    pub fn simple_regex(&self) -> Option<&Regex> {
        if self.is_simple() {
            Some(&self.should[0].regex)
        } else {
            None
        }
    }
}

/// Parse a query string into a structured Query.
///
/// Syntax:
/// - `term` → OR term (default)
/// - `term1 AND term2` → both must match
/// - `term1 OR term2` → either must match
/// - `NOT term` → must not match
/// - `term1 AND NOT term2` → term1 required, term2 excluded
/// - `"exact phrase"` → literal phrase match
///
/// When `exact` is false, plain terms use stemming (suffix stripping + prefix match).
/// When `exact` is true, terms use word-boundary matching.
pub fn parse(input: &str, exact: bool, ignore_case: bool) -> std::result::Result<Query, String> {
    let tokens = tokenize(input);

    let mut must = Vec::new();
    let mut should = Vec::new();
    let mut must_not = Vec::new();
    let mut has_and = false;
    let mut next_not = false;

    // First pass: detect if AND is used (changes default grouping)
    for token in &tokens {
        if matches!(token, Token::And) {
            has_and = true;
            break;
        }
    }

    for token in tokens {
        match token {
            Token::And => {
                // AND is implicit in the grouping logic
            }
            Token::Or => {
                // Next term goes to should
            }
            Token::Not => {
                next_not = true;
            }
            Token::Term(term) => {
                let matcher = build_matcher(&term, false, exact, ignore_case)?;
                if next_not {
                    must_not.push(matcher);
                    next_not = false;
                } else if has_and {
                    must.push(matcher);
                } else {
                    should.push(matcher);
                }
            }
            Token::Phrase(phrase) => {
                let matcher = build_matcher(&phrase, true, true, ignore_case)?;
                if next_not {
                    must_not.push(matcher);
                    next_not = false;
                } else if has_and {
                    must.push(matcher);
                } else {
                    should.push(matcher);
                }
            }
        }
    }

    Ok(Query {
        must,
        should,
        must_not,
    })
}

fn build_matcher(
    term: &str,
    is_phrase: bool,
    exact: bool,
    ignore_case: bool,
) -> std::result::Result<Matcher, String> {
    let pattern = if is_phrase || exact {
        // Exact: escape and use word boundaries
        format!(r"\b{}\b", regex::escape(term))
    } else {
        // Stemmed: strip suffix, match as prefix
        let stemmed = stem(term);
        if stemmed.len() < term.len() {
            // Stem is shorter — match stem followed by optional word chars
            format!(r"(?i)\b{}\w*", regex::escape(&stemmed))
        } else {
            // No stemming applied — just case-insensitive match
            regex::escape(term)
        }
    };

    let regex = regex::RegexBuilder::new(&pattern)
        .case_insensitive(ignore_case || (!exact && !is_phrase))
        .build()
        .map_err(|e| format!("invalid pattern '{term}': {e}"))?;

    Ok(Matcher {
        regex,
        original: term.to_string(),
    })
}

// ── tokenizer ───────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum Token {
    Term(String),
    Phrase(String),
    And,
    Or,
    Not,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '"' {
            chars.next(); // consume opening quote
            let mut phrase = String::new();
            while let Some(&c) = chars.peek() {
                if c == '"' {
                    chars.next(); // consume closing quote
                    break;
                }
                phrase.push(c);
                chars.next();
            }
            if !phrase.is_empty() {
                tokens.push(Token::Phrase(phrase));
            }
            continue;
        }

        // Collect a word
        let mut word = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '"' {
                break;
            }
            word.push(c);
            chars.next();
        }

        match word.as_str() {
            "AND" => tokens.push(Token::And),
            "OR" => tokens.push(Token::Or),
            "NOT" => tokens.push(Token::Not),
            _ => tokens.push(Token::Term(word)),
        }
    }

    tokens
}

// ── stemming ────────────────────────────────────────────────────────

/// Simple English suffix stripping for code search.
///
/// Strips one layer of common suffixes to produce a stem that matches
/// related word forms. Conservative: requires at least 4 chars remaining
/// after stripping to avoid over-stemming.
///
/// Examples:
///   authenticate → authenticat (matches authentication, authenticated)
///   handling → handl (matches handler, handle, handled)
///   errors → error
///   connection → connect (matches connected, connecting)
pub fn stem(word: &str) -> String {
    let w = word.to_lowercase();

    // Ordered by suffix length (longest first) to strip the most specific suffix
    let suffixes = &[
        "ication", // authentication → authent
        "ation",   // authorization → authoriz
        "ition",   // definition → defin
        "ction",   // connection → conne  (try before "tion")
        "tion",    // completion → comple
        "sion",    // expression → expres
        "ment",    // management → manage
        "ness",    // readiness → readi
        "able",    // readable → read
        "ible",    // accessible → access
        "ally",    // automatically → automatic
        "ence",    // reference → refer
        "ance",    // performance → perform
        "ings",    // settings → sett
        "ated",    // authenticated → authentic
        "ized",    // authorized → author
        "ised",    // recognised → recogn
        "ting",    // connecting → connect
        "less",    // careless → care
        "ful",     // successful → success
        "ous",     // dangerous → danger
        "ive",     // recursive → recurs
        "ity",     // security → secur
        "ing",     // handling → handl
        "ted",     // connected → connec
        "ers",     // handlers → handl
        "ies",     // queries → quer
        "ied",     // applied → appl
        "ion",     // expression → express
        "ed",      // handled → handl
        "er",      // handler → handl
        "es",      // matches → match
        "ly",      // directly → direct
        "al",      // functional → function
        "or",      // executor → execut
        "ar",      // similar → simil
        "s",       // errors → error
    ];

    let min_stem = 4;

    for suffix in suffixes {
        if w.len() > suffix.len() + min_stem && w.ends_with(suffix) {
            return w[..w.len() - suffix.len()].to_string();
        }
    }

    w
}

// ── language extension mapping ──────────────────────────────────────

/// Map a language name to file extensions for filtering.
pub fn language_extensions(language: &str) -> Option<&'static [&'static str]> {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => Some(&["rs"]),
        "typescript" | "ts" => Some(&["ts", "tsx"]),
        "javascript" | "js" => Some(&["js", "jsx", "mjs", "cjs"]),
        "python" | "py" => Some(&["py", "pyi"]),
        "go" | "golang" => Some(&["go"]),
        "java" => Some(&["java"]),
        "ruby" | "rb" => Some(&["rb"]),
        "c" => Some(&["c", "h"]),
        "cpp" | "c++" | "cxx" => Some(&["cpp", "cc", "cxx", "hpp", "hxx", "h"]),
        "swift" => Some(&["swift"]),
        "php" => Some(&["php"]),
        "elixir" | "ex" => Some(&["ex", "exs"]),
        "zig" => Some(&["zig"]),
        "lua" => Some(&["lua"]),
        "shell" | "bash" | "sh" => Some(&["sh", "bash", "zsh"]),
        "toml" => Some(&["toml"]),
        "yaml" | "yml" => Some(&["yaml", "yml"]),
        "json" => Some(&["json"]),
        "markdown" | "md" => Some(&["md", "markdown"]),
        _ => None,
    }
}

/// Patterns that identify test files.
pub fn is_test_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name_lower = name.to_lowercase();

    // Common test file patterns
    name_lower.ends_with("_test.go")
        || name_lower.ends_with("_test.rs")
        || name_lower.ends_with(".test.ts")
        || name_lower.ends_with(".test.tsx")
        || name_lower.ends_with(".test.js")
        || name_lower.ends_with(".test.jsx")
        || name_lower.ends_with(".spec.ts")
        || name_lower.ends_with(".spec.tsx")
        || name_lower.ends_with(".spec.js")
        || name_lower.ends_with(".spec.jsx")
        || name_lower.starts_with("test_")
        || name_lower == "conftest.py"
        || path.components().any(|c| {
            let s = c.as_os_str().to_str().unwrap_or("");
            s == "tests" || s == "test" || s == "__tests__" || s == "spec" || s == "specs"
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple() {
        let tokens = tokenize("hello world");
        assert_eq!(
            tokens,
            vec![Token::Term("hello".into()), Token::Term("world".into())]
        );
    }

    #[test]
    fn tokenize_boolean() {
        let tokens = tokenize("error AND handling");
        assert_eq!(
            tokens,
            vec![
                Token::Term("error".into()),
                Token::And,
                Token::Term("handling".into())
            ]
        );
    }

    #[test]
    fn tokenize_not() {
        let tokens = tokenize("database NOT sqlite");
        assert_eq!(
            tokens,
            vec![
                Token::Term("database".into()),
                Token::Not,
                Token::Term("sqlite".into())
            ]
        );
    }

    #[test]
    fn tokenize_phrase() {
        let tokens = tokenize("\"user authentication\" AND error");
        assert_eq!(
            tokens,
            vec![
                Token::Phrase("user authentication".into()),
                Token::And,
                Token::Term("error".into())
            ]
        );
    }

    #[test]
    fn tokenize_or() {
        let tokens = tokenize("login OR auth");
        assert_eq!(
            tokens,
            vec![
                Token::Term("login".into()),
                Token::Or,
                Token::Term("auth".into())
            ]
        );
    }

    #[test]
    fn parse_and_query() {
        let q = parse("error AND handling", false, false).unwrap();
        assert_eq!(q.must.len(), 2);
        assert_eq!(q.should.len(), 0);
        assert!(q.matches_line("error handling here"));
        assert!(!q.matches_line("just an error"));
    }

    #[test]
    fn parse_or_query() {
        let q = parse("login OR auth", false, false).unwrap();
        assert_eq!(q.should.len(), 2);
        assert!(q.matches_line("login page"));
        assert!(q.matches_line("auth token"));
        assert!(!q.matches_line("nothing here"));
    }

    #[test]
    fn parse_not_query() {
        let q = parse("database NOT sqlite", false, false).unwrap();
        assert!(q.matches_line("database connection"));
        assert!(!q.matches_line("database sqlite connection"));
    }

    #[test]
    fn parse_phrase_query() {
        let q = parse("\"user authentication\"", true, false).unwrap();
        assert!(q.matches_line("the user authentication system"));
        assert!(!q.matches_line("the user and authentication"));
    }

    #[test]
    fn parse_simple_term() {
        let q = parse("ToolOutput", false, false).unwrap();
        assert!(q.is_simple());
        assert!(q.matches_line("fn text() -> ToolOutput {"));
    }

    #[test]
    fn stem_common_words() {
        assert_eq!(stem("authentication"), "authent");
        assert_eq!(stem("handling"), "handl");
        assert_eq!(stem("errors"), "error");
        assert_eq!(stem("handler"), "handl");
        assert_eq!(stem("performance"), "perform");
        // "connected": strips "ed" → "connect" (min_stem=4, 9>2+4)
        assert_eq!(stem("connected"), "connect");
    }

    #[test]
    fn stem_short_words_unchanged() {
        // Words too short to stem safely
        assert_eq!(stem("run"), "run");
        assert_eq!(stem("go"), "go");
        assert_eq!(stem("get"), "get");
    }

    #[test]
    fn stemmed_search_matches_variants() {
        let q = parse("authenticate", false, false).unwrap();
        // Should match via stemming: "authenticat" prefix
        assert!(q.matches_line("authentication required"));
        assert!(q.matches_line("authenticated user"));
        assert!(q.matches_line("fn authenticate()"));
    }

    #[test]
    fn exact_search_no_stemming() {
        let q = parse("authenticate", true, false).unwrap();
        assert!(q.matches_line("fn authenticate() {"));
        // Exact mode: word boundary, so "authentication" should NOT match
        assert!(!q.matches_line("authentication required"));
    }

    #[test]
    fn file_level_and_matching() {
        let content = "fn handle_error() {\n    log(\"something\");\n}\n\nfn setup_logging() {\n    // configure\n}";
        let q = parse("error AND logging", false, false).unwrap();
        assert!(q.matches_file(content));

        let q2 = parse("error AND database", false, false).unwrap();
        assert!(!q2.matches_file(content));
    }

    #[test]
    fn test_file_detection() {
        assert!(is_test_file(Path::new("src/auth_test.go")));
        assert!(is_test_file(Path::new("src/auth.test.ts")));
        assert!(is_test_file(Path::new("src/auth.spec.js")));
        assert!(is_test_file(Path::new("test_auth.py")));
        assert!(is_test_file(Path::new("tests/integration.rs")));
        assert!(is_test_file(Path::new("__tests__/auth.tsx")));
        assert!(!is_test_file(Path::new("src/auth.rs")));
        assert!(!is_test_file(Path::new("src/main.ts")));
    }

    #[test]
    fn language_extension_mapping() {
        assert_eq!(language_extensions("rust"), Some(&["rs"][..]));
        assert_eq!(language_extensions("typescript"), Some(&["ts", "tsx"][..]));
        assert_eq!(language_extensions("python"), Some(&["py", "pyi"][..]));
        assert_eq!(language_extensions("unknown"), None);
    }
}
