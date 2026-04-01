//! Context prefill assembly for mana dispatch.
//!
//! When `imp run <unit_id>` dispatches an agent, the unit description often
//! references files the agent will need. Instead of making the agent spend
//! turns reading those files, we assemble them into a cached prefix message
//! that precedes the task prompt.
//!
//! The assembled context gets `cache_control` breakpoints so every subsequent
//! turn in the agent's session gets `cache_read` on the file contents — no
//! re-transmission cost.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use imp_llm::message::{ContentBlock, Message, UserMessage};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How to extract content from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMode {
    /// Include the entire file (up to per-file budget).
    Full,
    /// Include only the last N lines.
    Tail(usize),
    /// Include a specific line range (1-indexed, inclusive).
    Range(usize, usize),
}

/// A file to include in the prefill context.
#[derive(Debug, Clone)]
pub struct FileSpec {
    pub path: PathBuf,
    pub mode: FileMode,
}

/// Configuration for context assembly.
#[derive(Debug, Clone)]
pub struct PrefillConfig {
    /// Max total estimated tokens for all assembled context. Default: 50_000.
    pub budget_tokens: usize,
    /// Max estimated tokens per individual file. Default: 10_000.
    pub per_file_tokens: usize,
}

impl Default for PrefillConfig {
    fn default() -> Self {
        Self {
            budget_tokens: 50_000,
            per_file_tokens: 10_000,
        }
    }
}

/// Result of context assembly.
#[derive(Debug)]
pub struct AssembledContext {
    /// Messages to inject before the first prompt.
    pub messages: Vec<Message>,
    /// Files that were successfully included.
    pub included_files: Vec<PathBuf>,
    /// Warnings (missing files, truncations, budget exceeded).
    pub warnings: Vec<String>,
    /// Estimated token count of assembled context.
    pub estimated_tokens: usize,
}

impl AssembledContext {
    /// An empty context (no files, no messages).
    pub fn empty() -> Self {
        Self {
            messages: Vec::new(),
            included_files: Vec::new(),
            warnings: Vec::new(),
            estimated_tokens: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Token estimation
// ---------------------------------------------------------------------------

/// Rough token estimate: 1 token ≈ 4 characters.
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Character budget from a token budget.
fn chars_from_tokens(tokens: usize) -> usize {
    tokens * 4
}

// ---------------------------------------------------------------------------
// File reading with mode application
// ---------------------------------------------------------------------------

/// Read a file and apply the extraction mode.
fn read_file_with_mode(path: &Path, mode: &FileMode) -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(match mode {
        FileMode::Full => content,
        FileMode::Tail(n) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(*n);
            lines[start..].join("\n")
        }
        FileMode::Range(start, end) => {
            let lines: Vec<&str> = content.lines().collect();
            let s = start.saturating_sub(1); // 1-indexed → 0-indexed
            let e = (*end).min(lines.len());
            if s >= lines.len() {
                String::new()
            } else {
                lines[s..e].join("\n")
            }
        }
    })
}

/// Truncate content to fit within a character budget, appending a note.
fn truncate_to_budget(content: &str, max_chars: usize) -> (String, bool) {
    if content.len() <= max_chars {
        return (content.to_string(), false);
    }
    let total_lines = content.lines().count();
    // Find a line boundary near the budget
    let mut end = 0;
    for (i, _) in content.char_indices() {
        if i > max_chars {
            break;
        }
        end = i;
    }
    // Back up to the last newline
    if let Some(nl) = content[..end].rfind('\n') {
        end = nl;
    }
    let truncated_lines = content[..end].lines().count();
    let mut result = content[..end].to_string();
    result.push_str(&format!(
        "\n[... truncated: showing {truncated_lines} of {total_lines} lines]"
    ));
    (result, true)
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Assemble context from file specs, reading from disk and respecting budgets.
///
/// Produces a single `Message::User` containing all file contents in an XML
/// structure. Returns an empty context if no files were successfully read.
pub fn assemble_context(
    specs: &[FileSpec],
    cwd: &Path,
    config: &PrefillConfig,
) -> AssembledContext {
    if specs.is_empty() {
        return AssembledContext::empty();
    }

    let mut included_files = Vec::new();
    let mut warnings = Vec::new();
    let mut file_sections = Vec::new();
    let mut total_chars: usize = 0;
    let char_budget = chars_from_tokens(config.budget_tokens);
    let per_file_char_budget = chars_from_tokens(config.per_file_tokens);

    // Overhead for XML wrapper: <context>\n...\n</context>
    let wrapper_overhead = "<context>\n</context>".len();
    total_chars += wrapper_overhead;

    for spec in specs {
        let resolved = if spec.path.is_absolute() {
            spec.path.clone()
        } else {
            cwd.join(&spec.path)
        };

        // Read the file
        let content = match read_file_with_mode(&resolved, &spec.mode) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(format!("{}: {e}", spec.path.display()));
                continue;
            }
        };

        if content.is_empty() {
            continue;
        }

        // Build the section XML
        let mode_note = match &spec.mode {
            FileMode::Full => String::new(),
            FileMode::Tail(n) => format!(r#" note="last {n} lines""#),
            FileMode::Range(s, e) => format!(r#" note="lines {s}-{e}""#),
        };
        let header = format!(r#"<file path="{}"{}>"#, spec.path.display(), mode_note);
        let footer = "</file>";
        let section_overhead = header.len() + footer.len() + 2; // newlines

        // Check per-file budget
        let (file_content, was_truncated) = truncate_to_budget(
            &content,
            per_file_char_budget.saturating_sub(section_overhead),
        );
        if was_truncated {
            warnings.push(format!(
                "{}: truncated to ~{} tokens (per-file budget)",
                spec.path.display(),
                config.per_file_tokens,
            ));
        }

        let section = format!("{header}\n{file_content}\n{footer}");
        let section_chars = section.len();

        // Check total budget
        if total_chars + section_chars > char_budget {
            warnings.push(format!(
                "{}: skipped (total budget of ~{} tokens exceeded)",
                spec.path.display(),
                config.budget_tokens,
            ));
            // Skip remaining files too
            for remaining in specs.iter().skip(included_files.len() + warnings.len()) {
                // Only warn for specs we haven't processed yet
                if !included_files.contains(&remaining.path) {
                    warnings.push(format!(
                        "{}: skipped (total budget exceeded)",
                        remaining.path.display(),
                    ));
                }
            }
            break;
        }

        total_chars += section_chars;
        file_sections.push(section);
        included_files.push(spec.path.clone());
    }

    if file_sections.is_empty() {
        return AssembledContext {
            messages: Vec::new(),
            included_files,
            warnings,
            estimated_tokens: 0,
        };
    }

    let xml = format!("<context>\n{}\n</context>", file_sections.join("\n"));
    let estimated_tokens = estimate_tokens(&xml);

    let message = Message::User(UserMessage {
        content: vec![ContentBlock::Text { text: xml }],
        timestamp: imp_llm::now(),
    });

    AssembledContext {
        messages: vec![message],
        included_files,
        warnings,
        estimated_tokens,
    }
}

// ---------------------------------------------------------------------------
// File path detection
// ---------------------------------------------------------------------------

/// Auto-detect file paths from a unit description string.
///
/// Scans for patterns that look like source file paths (e.g., `src/foo.rs`,
/// `crates/bar/baz.ts`). Supports optional mode suffixes:
/// - `path.rs:tail:50` → `Tail(50)`
/// - `path.rs:10-50` → `Range(10, 50)`
///
/// Deduplicates by path (first occurrence wins).
pub fn detect_file_paths(text: &str) -> Vec<FileSpec> {
    // Match sequences that look like file paths with known extensions.
    // The negative lookbehind-like logic is handled by checking the char
    // before the match.
    let extensions = [
        "rs", "ts", "tsx", "py", "go", "js", "jsx", "toml", "yaml", "yml", "json", "md", "sh",
        "sql", "zig", "c", "cpp", "h",
    ];
    let ext_pattern = extensions.join("|");
    let pattern = format!(
        r#"(?:^|[\s(`"'(])((?:[a-zA-Z_./])[a-zA-Z0-9_./-]*\.(?:{ext_pattern}))(?::([^\s)}}"'`]*))?"#,
    );
    let re = regex::Regex::new(&pattern).expect("valid regex");

    let mut seen = HashSet::new();
    let mut specs = Vec::new();

    for cap in re.captures_iter(text) {
        let path_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if path_str.is_empty() {
            continue;
        }

        let path = PathBuf::from(path_str);
        if seen.contains(&path) {
            continue;
        }
        seen.insert(path.clone());

        let mode = cap
            .get(2)
            .map(|m| parse_mode_suffix(m.as_str()))
            .unwrap_or(FileMode::Full);

        specs.push(FileSpec { path, mode });
    }

    specs
}

/// Parse a mode suffix string into a FileMode.
fn parse_mode_suffix(suffix: &str) -> FileMode {
    // tail:N
    if let Some(n_str) = suffix.strip_prefix("tail:") {
        if let Ok(n) = n_str.parse::<usize>() {
            return FileMode::Tail(n);
        }
    }
    // N-M (line range)
    if let Some(dash_pos) = suffix.find('-') {
        let start_str = &suffix[..dash_pos];
        let end_str = &suffix[dash_pos + 1..];
        if let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>()) {
            return FileMode::Range(start, end);
        }
    }
    FileMode::Full
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir_with_files(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
        dir
    }

    // -- Assembly tests --

    #[test]
    fn test_context_prefill_assembles_single_file() {
        let dir =
            temp_dir_with_files(&[("src/main.rs", "fn main() {\n    println!(\"hello\");\n}")]);
        let specs = vec![FileSpec {
            path: PathBuf::from("src/main.rs"),
            mode: FileMode::Full,
        }];
        let ctx = assemble_context(&specs, dir.path(), &PrefillConfig::default());
        assert_eq!(ctx.included_files.len(), 1);
        assert!(ctx.warnings.is_empty());
        assert!(!ctx.messages.is_empty());

        let text = message_text(&ctx.messages[0]);
        assert!(text.contains("<context>"));
        assert!(text.contains(r#"<file path="src/main.rs">"#));
        assert!(text.contains("fn main()"));
        assert!(text.contains("</file>"));
        assert!(text.contains("</context>"));
    }

    #[test]
    fn test_context_prefill_multiple_files() {
        let dir = temp_dir_with_files(&[("src/a.rs", "struct A;"), ("src/b.rs", "struct B;")]);
        let specs = vec![
            FileSpec {
                path: PathBuf::from("src/a.rs"),
                mode: FileMode::Full,
            },
            FileSpec {
                path: PathBuf::from("src/b.rs"),
                mode: FileMode::Full,
            },
        ];
        let ctx = assemble_context(&specs, dir.path(), &PrefillConfig::default());
        assert_eq!(ctx.included_files.len(), 2);
        let text = message_text(&ctx.messages[0]);
        assert!(text.contains("struct A"));
        assert!(text.contains("struct B"));
    }

    #[test]
    fn test_context_prefill_missing_file_warning() {
        let dir = temp_dir_with_files(&[("src/exists.rs", "exists")]);
        let specs = vec![
            FileSpec {
                path: PathBuf::from("src/missing.rs"),
                mode: FileMode::Full,
            },
            FileSpec {
                path: PathBuf::from("src/exists.rs"),
                mode: FileMode::Full,
            },
        ];
        let ctx = assemble_context(&specs, dir.path(), &PrefillConfig::default());
        assert_eq!(ctx.included_files.len(), 1);
        assert_eq!(ctx.included_files[0], PathBuf::from("src/exists.rs"));
        assert!(ctx.warnings.iter().any(|w| w.contains("missing.rs")));
    }

    #[test]
    fn test_context_prefill_per_file_budget() {
        // Create a file that's larger than 100 tokens (~400 chars)
        let big_content: String = (0..200)
            .map(|i| format!("line {i}: some content here\n"))
            .collect();
        let dir = temp_dir_with_files(&[("big.rs", &big_content)]);
        let specs = vec![FileSpec {
            path: PathBuf::from("big.rs"),
            mode: FileMode::Full,
        }];
        let config = PrefillConfig {
            budget_tokens: 100_000,
            per_file_tokens: 100, // ~400 chars — file will be truncated
        };
        let ctx = assemble_context(&specs, dir.path(), &config);
        assert_eq!(ctx.included_files.len(), 1);
        assert!(ctx.warnings.iter().any(|w| w.contains("truncated")));
        let text = message_text(&ctx.messages[0]);
        assert!(text.contains("[... truncated:"));
    }

    #[test]
    fn test_context_prefill_total_budget() {
        // Each file is ~4000 chars (~1000 tokens). Set budget to fit only one.
        let content_a: String = (0..200)
            .map(|i| format!("line_a_{i}: some padding content here\n"))
            .collect();
        let content_b: String = (0..200)
            .map(|i| format!("line_b_{i}: some padding content here\n"))
            .collect();
        let dir = temp_dir_with_files(&[("a.rs", &content_a), ("b.rs", &content_b)]);
        let specs = vec![
            FileSpec {
                path: PathBuf::from("a.rs"),
                mode: FileMode::Full,
            },
            FileSpec {
                path: PathBuf::from("b.rs"),
                mode: FileMode::Full,
            },
        ];
        let config = PrefillConfig {
            budget_tokens: 2500, // ~10000 chars — first file + XML wrapper fits, second doesn't
            per_file_tokens: 50_000,
        };
        let ctx = assemble_context(&specs, dir.path(), &config);
        // First file should be included, second skipped
        assert_eq!(
            ctx.included_files.len(),
            1,
            "included: {:?}, warnings: {:?}",
            ctx.included_files,
            ctx.warnings
        );
        assert!(ctx
            .warnings
            .iter()
            .any(|w| w.contains("b.rs") && w.contains("budget")));
    }

    #[test]
    fn test_context_prefill_tail_mode() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n";
        let dir = temp_dir_with_files(&[("f.rs", content)]);
        let specs = vec![FileSpec {
            path: PathBuf::from("f.rs"),
            mode: FileMode::Tail(3),
        }];
        let ctx = assemble_context(&specs, dir.path(), &PrefillConfig::default());
        let text = message_text(&ctx.messages[0]);
        assert!(!text.contains("line 1"));
        assert!(!text.contains("line 2"));
        assert!(text.contains("line 3"));
        assert!(text.contains("line 4"));
        assert!(text.contains("line 5"));
    }

    #[test]
    fn test_context_prefill_range_mode() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n";
        let dir = temp_dir_with_files(&[("f.rs", content)]);
        let specs = vec![FileSpec {
            path: PathBuf::from("f.rs"),
            mode: FileMode::Range(2, 4),
        }];
        let ctx = assemble_context(&specs, dir.path(), &PrefillConfig::default());
        let text = message_text(&ctx.messages[0]);
        assert!(!text.contains("line 1"));
        assert!(text.contains("line 2"));
        assert!(text.contains("line 3"));
        assert!(text.contains("line 4"));
        assert!(!text.contains("line 5"));
    }

    #[test]
    fn test_context_prefill_empty_specs() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = assemble_context(&[], dir.path(), &PrefillConfig::default());
        assert!(ctx.messages.is_empty());
        assert!(ctx.included_files.is_empty());
        assert_eq!(ctx.estimated_tokens, 0);
    }

    // -- Detection tests --

    #[test]
    fn test_context_prefill_detect_paths() {
        let text = "Modify src/auth.rs and read crates/imp-llm/src/provider.rs for context.";
        let specs = detect_file_paths(text);
        let paths: Vec<_> = specs.iter().map(|s| s.path.to_str().unwrap()).collect();
        assert!(paths.contains(&"src/auth.rs"));
        assert!(paths.contains(&"crates/imp-llm/src/provider.rs"));
    }

    #[test]
    fn test_context_prefill_detect_deduplicates() {
        let text = "Read src/foo.rs first, then modify src/foo.rs to add the function.";
        let specs = detect_file_paths(text);
        let foo_count = specs
            .iter()
            .filter(|s| s.path == PathBuf::from("src/foo.rs"))
            .count();
        assert_eq!(foo_count, 1);
    }

    #[test]
    fn test_context_prefill_detect_ignores_non_paths() {
        let text = "Handle errors gracefully. The users table needs updating.";
        let specs = detect_file_paths(text);
        // "errors" and "users" shouldn't match — they don't have path-like structure + extension
        assert!(specs.is_empty(), "got: {:?}", specs);
    }

    #[test]
    fn test_context_prefill_detect_tail_suffix() {
        let text = "Check patterns in tests/auth_test.rs:tail:50 for reference.";
        let specs = detect_file_paths(text);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].path, PathBuf::from("tests/auth_test.rs"));
        assert_eq!(specs[0].mode, FileMode::Tail(50));
    }

    #[test]
    fn test_context_prefill_detect_range_suffix() {
        let text = "See src/lib.rs:10-50 for the relevant types.";
        let specs = detect_file_paths(text);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].path, PathBuf::from("src/lib.rs"));
        assert_eq!(specs[0].mode, FileMode::Range(10, 50));
    }

    // -- Helpers --

    fn message_text(msg: &Message) -> String {
        match msg {
            Message::User(u) => u
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }
}
