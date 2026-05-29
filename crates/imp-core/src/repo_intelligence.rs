use crate::tools::scan;
use crate::tools::scan::types::ScanResult;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const CACHE_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoIndexSummary {
    pub symbols: usize,
    pub tests: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CachedRepoContextSummary {
    version: u32,
    fingerprint: RepoFingerprint,
    summary: RepoContextSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RepoFingerprint {
    root: PathBuf,
    signature: u64,
}

pub fn index_repo(root: &Path) -> crate::Result<RepoIndexSummary> {
    let files = repo_context_files(root)?;
    Ok(index_files(&files, root))
}

pub fn summarize_repo_context(root: &Path) -> crate::Result<Option<RepoContextSummary>> {
    let files = repo_context_files(root)?;
    if files.is_empty() {
        return Ok(None);
    }

    let fingerprint = repo_fingerprint(root, &files);
    if let Some(summary) = read_cached_summary(&fingerprint) {
        return Ok(Some(summary));
    }

    let index = index_files(&files, root);
    let summary = RepoContextSummary {
        root: root.to_path_buf(),
        primary_language: primary_language(&files),
        files: files.len(),
        symbols: index.symbols,
        tests: index.tests,
    };
    let _ = write_cached_summary(&fingerprint, &summary);
    Ok(Some(summary))
}

pub fn index_files(files: &[PathBuf], root: &Path) -> RepoIndexSummary {
    let result = scan::extract_files(files, root);
    summarize_scan_result(&result)
}

fn repo_context_files(root: &Path) -> crate::Result<Vec<PathBuf>> {
    if let Some(files) = git_source_files(root) {
        return Ok(files);
    }

    let mut files = scan::collect_source_files(root)?;
    files.retain(|file| !is_heavyweight_context_path(root, file));
    Ok(files)
}

fn git_source_files(root: &Path) -> Option<Vec<PathBuf>> {
    if !root.is_dir() {
        return None;
    }
    let output = Command::new("git")
        .arg("ls-files")
        .arg("-z")
        .arg("--cached")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let files = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|bytes| !bytes.is_empty())
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .map(|relative| root.join(relative))
        .filter(|path| scan::is_supported(path))
        .collect::<Vec<_>>();
    Some(files)
}

fn is_heavyweight_context_path(root: &Path, path: &Path) -> bool {
    scan_ignored_context_path(root, path)
}

fn scan_ignored_context_path(root: &Path, path: &Path) -> bool {
    let Some(relative) = path.strip_prefix(root).ok() else {
        return false;
    };
    let output = Command::new("git")
        .arg("check-ignore")
        .arg("--quiet")
        .arg(relative)
        .current_dir(root)
        .status();
    matches!(output, Ok(status) if status.success())
}

fn repo_fingerprint(root: &Path, files: &[PathBuf]) -> RepoFingerprint {
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = DefaultHasher::new();
    CACHE_VERSION.hash(&mut hasher);
    canonical_root.hash(&mut hasher);
    files.len().hash(&mut hasher);

    if let Some(git_signature) = git_signature(root) {
        git_signature.hash(&mut hasher);
    } else {
        let mut sorted_files = files.to_vec();
        sorted_files.sort();
        for file in sorted_files {
            let relative = file.strip_prefix(root).unwrap_or(&file);
            relative.hash(&mut hasher);
            if let Ok(meta) = fs::metadata(&file) {
                meta.len().hash(&mut hasher);
                if let Ok(modified) = meta.modified() {
                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        duration.as_secs().hash(&mut hasher);
                        duration.subsec_nanos().hash(&mut hasher);
                    }
                }
            }
        }
    }

    RepoFingerprint {
        root: canonical_root,
        signature: hasher.finish(),
    }
}

fn git_signature(root: &Path) -> Option<String> {
    if !root.is_dir() {
        return None;
    }
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !head.status.success() {
        return None;
    }
    let status = Command::new("git")
        .args(["status", "--porcelain=v1", "--untracked-files=normal"])
        .current_dir(root)
        .output()
        .ok()?;
    if !status.status.success() {
        return None;
    }

    let mut signature = String::new();
    signature.push_str(std::str::from_utf8(&head.stdout).ok()?.trim());
    signature.push('\n');
    signature.push_str(std::str::from_utf8(&status.stdout).ok()?);
    Some(signature)
}

fn cache_path(fingerprint: &RepoFingerprint) -> PathBuf {
    crate::storage::global_indexes_dir()
        .join("repo-intelligence")
        .join(format!("{:016x}.json", fingerprint.signature))
}

fn read_cached_summary(fingerprint: &RepoFingerprint) -> Option<RepoContextSummary> {
    let path = cache_path(fingerprint);
    let text = fs::read_to_string(path).ok()?;
    let cached: CachedRepoContextSummary = serde_json::from_str(&text).ok()?;
    if cached.version == CACHE_VERSION && cached.fingerprint == *fingerprint {
        Some(cached.summary)
    } else {
        None
    }
}

fn write_cached_summary(
    fingerprint: &RepoFingerprint,
    summary: &RepoContextSummary,
) -> io::Result<()> {
    let path = cache_path(fingerprint);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let cached = CachedRepoContextSummary {
        version: CACHE_VERSION,
        fingerprint: fingerprint.clone(),
        summary: summary.clone(),
    };
    let json = serde_json::to_vec(&cached).map_err(io::Error::other)?;
    fs::write(path, json)
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
