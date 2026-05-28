use crate::tools::scan;
use crate::tools::scan::types::ScanResult;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoContextSummary {
    pub root: PathBuf,
    pub primary_language: Option<String>,
    pub files: usize,
    pub symbols: usize,
    pub tests: usize,
}

impl RepoContextSummary {
    pub fn render_prompt_layer(&self) -> String {
        let language = self.primary_language.as_deref().unwrap_or("unknown");
        let mut lines = vec![
            "# Repo Intelligence".to_string(),
            String::new(),
            format!("- root: {}", self.root.display()),
            format!("- primary language: {language}"),
            format!("- indexed files: {}", self.files),
            format!("- symbols: {}", self.symbols),
        ];
        if self.tests > 0 {
            lines.push(format!("- tests: {}", self.tests));
        }
        lines.push("- Use `scan` for symbol-aware lookup and related code discovery before broad text search.".to_string());
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoIndexSummary {
    pub symbols: usize,
    pub tests: usize,
}

pub fn index_repo(root: &Path) -> crate::Result<RepoIndexSummary> {
    let files = scan::collect_source_files(root)?;
    Ok(index_files(&files, root))
}

pub fn summarize_repo_context(root: &Path) -> crate::Result<Option<RepoContextSummary>> {
    let files = scan::collect_source_files(root)?;
    if files.is_empty() {
        return Ok(None);
    }
    let index = index_files(&files, root);
    Ok(Some(RepoContextSummary {
        root: root.to_path_buf(),
        primary_language: primary_language(&files),
        files: files.len(),
        symbols: index.symbols,
        tests: index.tests,
    }))
}

pub fn index_files(files: &[PathBuf], root: &Path) -> RepoIndexSummary {
    let result = scan::extract_files(files, root);
    summarize_scan_result(&result)
}

fn primary_language(files: &[PathBuf]) -> Option<String> {
    let mut counts = std::collections::HashMap::<&str, usize>::new();
    for file in files {
        let Some(ext) = file.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        let language = match ext {
            "rs" => "Rust",
            "ts" | "tsx" | "mts" | "cts" => "TypeScript",
            "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
            "py" | "pyw" => "Python",
            "go" => "Go",
            "java" => "Java",
            "kt" | "kts" => "Kotlin",
            "lua" => "Lua",
            "zig" => "Zig",
            "ex" | "exs" => "Elixir",
            "sh" | "bash" | "zsh" | "fish" => "Shell",
            "html" | "htm" => "HTML",
            "css" | "scss" | "sass" | "less" => "CSS",
            _ => continue,
        };
        *counts.entry(language).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(language, _)| language.to_string())
}

fn summarize_scan_result(result: &ScanResult) -> RepoIndexSummary {
    RepoIndexSummary {
        symbols: result.types.len() + result.functions.len(),
        tests: result
            .functions
            .values()
            .filter(|function| function.is_test)
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn indexes_rust_symbols_and_tests() {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("lib.rs");
        fs::write(
            &src,
            r#"
pub struct Widget;

pub fn build_widget() -> Widget { Widget }

#[test]
fn builds_widget() {}
"#,
        )
        .expect("write source");

        let summary = index_files(&[src], temp.path());
        assert!(summary.symbols >= 3, "summary = {summary:?}");
        assert_eq!(summary.tests, 1);
    }

    #[test]
    fn renders_prompt_layer() {
        let summary = RepoContextSummary {
            root: PathBuf::from("/repo"),
            primary_language: Some("Rust".to_string()),
            files: 12,
            symbols: 34,
            tests: 5,
        };

        let rendered = summary.render_prompt_layer();
        assert!(rendered.contains("# Repo Intelligence"));
        assert!(rendered.contains("- primary language: Rust"));
        assert!(rendered.contains("- symbols: 34"));
        assert!(rendered.contains("- tests: 5"));
    }
}
