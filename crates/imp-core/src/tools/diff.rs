use std::path::Path;

use async_trait::async_trait;
use serde_json::json;
use similar::TextDiff;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

// ── unified diff tool ───────────────────────────────────────────────

pub struct DiffTool;

#[async_trait]
impl Tool for DiffTool {
    fn name(&self) -> &str {
        "diff"
    }
    fn label(&self) -> &str {
        "Diff"
    }
    fn description(&self) -> &str {
        "Show or apply unified diffs. action=show previews changes, action=apply patches a file."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["show", "apply"], "description": "show: preview diff. apply: patch file." },
                "file": { "type": "string", "description": "File path" },
                "newContent": { "type": "string", "description": "Proposed new content (show)" },
                "contextLines": { "type": "number", "description": "Context lines, default 3 (show)" },
                "patch": { "type": "string", "description": "Unified diff patch (apply)" },
                "dryRun": { "type": "boolean", "description": "Preview without modifying (apply)" }
            },
            "required": ["action", "file"]
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }
    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        match params["action"].as_str() {
            Some("show") => DiffShowTool.execute(call_id, params, ctx).await,
            Some("apply") => DiffApplyTool.execute(call_id, params, ctx).await,
            Some(other) => Ok(ToolOutput::error(format!("Unknown diff action: {other}"))),
            None => Ok(ToolOutput::error("Missing 'action' parameter")),
        }
    }
}

// ── diff_show ───────────────────────────────────────────────────────

pub struct DiffShowTool;

#[async_trait]
impl Tool for DiffShowTool {
    fn name(&self) -> &str {
        "diff_show"
    }
    fn label(&self) -> &str {
        "Show Diff"
    }
    fn description(&self) -> &str {
        "Show a unified diff between a file's current content and proposed new content."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "Current file path" },
                "newContent": { "type": "string", "description": "Proposed new content" },
                "contextLines": { "type": "number", "description": "Number of context lines (default: 3)" }
            },
            "required": ["file", "newContent"]
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let raw_path = match params["file"].as_str() {
            Some(p) => p,
            None => return Ok(ToolOutput::error("Missing required parameter: file")),
        };
        let new_content = match params["newContent"].as_str() {
            Some(c) => c,
            None => return Ok(ToolOutput::error("Missing required parameter: newContent")),
        };
        let context_lines = params["contextLines"].as_u64().unwrap_or(3) as usize;

        let path = resolve_path(&ctx.cwd, raw_path);

        if !path.exists() {
            return Ok(ToolOutput::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let old_content = tokio::fs::read_to_string(&path).await?;
        let diff_text = generate_unified_diff(raw_path, &old_content, new_content, context_lines);

        if diff_text.is_empty() {
            return Ok(ToolOutput::text("No changes."));
        }

        Ok(ToolOutput::text(diff_text))
    }
}

// ── diff_apply ──────────────────────────────────────────────────────

pub struct DiffApplyTool;

#[async_trait]
impl Tool for DiffApplyTool {
    fn name(&self) -> &str {
        "diff_apply"
    }
    fn label(&self) -> &str {
        "Apply Diff"
    }
    fn description(&self) -> &str {
        "Apply a unified diff patch to a file."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "File path to apply patch to" },
                "patch": { "type": "string", "description": "Unified diff patch content" },
                "dryRun": { "type": "boolean", "description": "Preview changes without modifying file" }
            },
            "required": ["file", "patch"]
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let raw_path = match params["file"].as_str() {
            Some(p) => p,
            None => return Ok(ToolOutput::error("Missing required parameter: file")),
        };
        let patch = match params["patch"].as_str() {
            Some(p) => p,
            None => return Ok(ToolOutput::error("Missing required parameter: patch")),
        };
        let dry_run = params["dryRun"].as_bool().unwrap_or(false);

        let path = resolve_path(&ctx.cwd, raw_path);

        if !path.exists() {
            return Ok(ToolOutput::error(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let hunks = match parse_unified_diff(patch) {
            Ok(h) => h,
            Err(e) => return Ok(ToolOutput::error(format!("Failed to parse patch: {e}"))),
        };

        if hunks.is_empty() {
            return Ok(ToolOutput::error("No hunks found in patch"));
        }

        let lines: Vec<&str> = content.lines().collect();
        let apply_result = apply_hunks(&lines, &hunks);

        let mut report = String::new();
        let mut all_ok = true;

        for (i, hr) in apply_result.hunk_results.iter().enumerate() {
            match hr {
                HunkResult::Applied { offset } => {
                    if *offset != 0 {
                        report
                            .push_str(&format!("Hunk {}: applied with offset {offset:+}\n", i + 1));
                    } else {
                        report.push_str(&format!("Hunk {}: applied cleanly\n", i + 1));
                    }
                }
                HunkResult::Conflict { reason } => {
                    all_ok = false;
                    report.push_str(&format!("Hunk {}: FAILED — {reason}\n", i + 1));
                }
            }
        }

        if !all_ok {
            return Ok(ToolOutput::error(report));
        }

        let new_content = apply_result.output.join("\n");
        // Preserve trailing newline if original had one
        let new_content = if content.ends_with('\n') && !new_content.ends_with('\n') {
            format!("{new_content}\n")
        } else {
            new_content
        };

        if dry_run {
            let diff = generate_unified_diff(raw_path, &content, &new_content, 3);
            report.push_str("\n--- Dry run result ---\n");
            report.push_str(&diff);
            Ok(ToolOutput::text(report))
        } else {
            tokio::fs::write(&path, &new_content).await?;
            let diff = generate_unified_diff(raw_path, &content, &new_content, 3);
            report.push_str(&diff);
            Ok(ToolOutput::text(report))
        }
    }
}

// ── Diff generation ─────────────────────────────────────────────────

/// Generate a unified diff with configurable context lines.
pub fn generate_unified_diff(
    file_path: &str,
    old: &str,
    new: &str,
    context_lines: usize,
) -> String {
    let diff = TextDiff::from_lines(old, new);

    // Check if there are any changes
    let has_changes = diff
        .ops()
        .iter()
        .any(|op| !matches!(op, similar::DiffOp::Equal { .. }));
    if !has_changes {
        return String::new();
    }

    let mut output = String::new();
    output.push_str(&format!("--- {file_path}\n"));
    output.push_str(&format!("+++ {file_path}\n"));

    for hunk in diff
        .unified_diff()
        .context_radius(context_lines)
        .iter_hunks()
    {
        output.push_str(&format!("{hunk}"));
    }

    output
}

// ── Unified diff parser ─────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Hunk {
    /// 0-indexed start line in the old file
    old_start: usize,
    lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
enum DiffLine {
    Context(String),
    Add(String),
    Remove(String),
}

/// Parse a unified diff into hunks.
fn parse_unified_diff(patch: &str) -> std::result::Result<Vec<Hunk>, String> {
    let mut hunks = Vec::new();
    let lines: Vec<&str> = patch.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip --- and +++ headers
        if line.starts_with("---") || line.starts_with("+++") {
            i += 1;
            continue;
        }

        // Parse @@ header
        if line.starts_with("@@") {
            let (old_start, _) =
                parse_hunk_header(line).ok_or_else(|| format!("Invalid hunk header: {line}"))?;

            let mut hunk_lines = Vec::new();
            i += 1;

            while i < lines.len() {
                let l = lines[i];
                if l.starts_with("@@") || l.starts_with("---") || l.starts_with("+++") {
                    break;
                }
                if let Some(rest) = l.strip_prefix('+') {
                    hunk_lines.push(DiffLine::Add(rest.to_string()));
                } else if let Some(rest) = l.strip_prefix('-') {
                    hunk_lines.push(DiffLine::Remove(rest.to_string()));
                } else if let Some(rest) = l.strip_prefix(' ') {
                    hunk_lines.push(DiffLine::Context(rest.to_string()));
                } else if l == "\\ No newline at end of file" {
                    // Skip this marker
                } else {
                    // Treat unrecognized lines as context (some diffs omit leading space)
                    hunk_lines.push(DiffLine::Context(l.to_string()));
                }
                i += 1;
            }

            hunks.push(Hunk {
                old_start: old_start.saturating_sub(1), // convert 1-indexed to 0-indexed
                lines: hunk_lines,
            });
        } else {
            i += 1;
        }
    }

    Ok(hunks)
}

/// Parse `@@ -old_start,old_count +new_start,new_count @@` header.
/// Returns (old_start, old_count).
fn parse_hunk_header(header: &str) -> Option<(usize, usize)> {
    // Find the range between @@ markers
    let after_at = header.strip_prefix("@@")?;
    let end = after_at.find("@@")?;
    let range_str = after_at[..end].trim();

    // Parse -old_start,old_count
    let parts: Vec<&str> = range_str.split_whitespace().collect();
    let old_part = parts.first()?;
    let old_part = old_part.strip_prefix('-')?;

    let (start_str, count_str): (&str, &str) = if let Some(pos) = old_part.find(',') {
        (&old_part[..pos], &old_part[pos + 1..])
    } else {
        (old_part, "1")
    };

    let start: usize = start_str.parse().ok()?;
    let count: usize = count_str.parse().ok()?;

    Some((start, count))
}

// ── Patch application ───────────────────────────────────────────────

const MAX_FUZZY_OFFSET: i64 = 20;

struct ApplyResult {
    output: Vec<String>,
    hunk_results: Vec<HunkResult>,
}

enum HunkResult {
    Applied { offset: i64 },
    Conflict { reason: String },
}

/// Apply hunks to the file lines, returning new content and per-hunk results.
fn apply_hunks(original_lines: &[&str], hunks: &[Hunk]) -> ApplyResult {
    let mut output: Vec<String> = original_lines.iter().map(|l| l.to_string()).collect();
    let mut hunk_results = Vec::new();
    // Track cumulative line offset from previous hunk insertions/deletions
    let mut cumulative_offset: i64 = 0;

    for hunk in hunks {
        let target_start = (hunk.old_start as i64 + cumulative_offset) as usize;

        // Extract context and removed lines from the hunk (what we expect in the old file)
        let expected: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|l| match l {
                DiffLine::Context(s) | DiffLine::Remove(s) => Some(s.as_str()),
                DiffLine::Add(_) => None,
            })
            .collect();

        // Try exact match at the target location
        let match_result = find_hunk_location(&output, &expected, target_start);

        match match_result {
            Some((actual_start, offset)) => {
                // Apply the hunk at actual_start
                let replacement = build_replacement(&hunk.lines);
                let old_len = expected.len();

                // Replace the range
                let end = (actual_start + old_len).min(output.len());
                output.splice(actual_start..end, replacement.iter().cloned());

                // Update cumulative offset
                let new_len = replacement.len();
                cumulative_offset += new_len as i64 - old_len as i64;

                hunk_results.push(HunkResult::Applied { offset });
            }
            None => {
                hunk_results.push(HunkResult::Conflict {
                    reason: format!(
                        "context lines don't match at line {} (±{MAX_FUZZY_OFFSET})",
                        hunk.old_start + 1
                    ),
                });
            }
        }
    }

    ApplyResult {
        output,
        hunk_results,
    }
}

/// Find where a hunk's expected lines exist in the output.
/// First tries exact position, then searches ±MAX_FUZZY_OFFSET lines.
/// Returns (actual_start_index, offset_from_target).
fn find_hunk_location(lines: &[String], expected: &[&str], target: usize) -> Option<(usize, i64)> {
    if expected.is_empty() {
        return Some((target.min(lines.len()), 0));
    }

    // Try exact match first
    if matches_at(lines, expected, target) {
        return Some((target, 0));
    }

    // Fuzzy search: try offsets ±1, ±2, ... ±MAX_FUZZY_OFFSET
    for delta in 1..=MAX_FUZZY_OFFSET {
        // Try forward
        let forward = target as i64 + delta;
        if forward >= 0 {
            let fwd = forward as usize;
            if matches_at(lines, expected, fwd) {
                return Some((fwd, delta));
            }
        }

        // Try backward
        let backward = target as i64 - delta;
        if backward >= 0 {
            let bwd = backward as usize;
            if matches_at(lines, expected, bwd) {
                return Some((bwd, -delta));
            }
        }
    }

    None
}

/// Check if expected lines match at the given position.
fn matches_at(lines: &[String], expected: &[&str], start: usize) -> bool {
    if start + expected.len() > lines.len() {
        return false;
    }
    lines[start..start + expected.len()]
        .iter()
        .zip(expected.iter())
        .all(|(a, b)| a == b)
}

/// Build the replacement lines from a hunk (context + add lines).
fn build_replacement(hunk_lines: &[DiffLine]) -> Vec<String> {
    hunk_lines
        .iter()
        .filter_map(|l| match l {
            DiffLine::Context(s) | DiffLine::Add(s) => Some(s.clone()),
            DiffLine::Remove(_) => None,
        })
        .collect()
}

fn resolve_path(cwd: &Path, raw: &str) -> std::path::PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::sync::Arc;

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
        }
    }

    fn extract_text(output: &ToolOutput) -> String {
        output
            .content
            .iter()
            .filter_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ── diff_show tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn diff_show_generates_unified_diff() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "line1\nline2\nline3\nline4\nline5\n").unwrap();

        let tool = DiffShowTool;
        let result = tool
            .execute(
                "c1",
                json!({
                    "file": "test.txt",
                    "newContent": "line1\nline2\nmodified\nline4\nline5\n"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);

        // Verify unified diff format
        assert!(text.contains("--- test.txt"));
        assert!(text.contains("+++ test.txt"));
        assert!(text.contains("@@"));
        assert!(text.contains("-line3"));
        assert!(text.contains("+modified"));
    }

    #[tokio::test]
    async fn diff_show_no_changes() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("same.txt");
        std::fs::write(&file, "hello\nworld\n").unwrap();

        let tool = DiffShowTool;
        let result = tool
            .execute(
                "c2",
                json!({
                    "file": "same.txt",
                    "newContent": "hello\nworld\n"
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("No changes"));
    }

    #[tokio::test]
    async fn diff_show_custom_context_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ctx.txt");
        let content = (1..=20)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&file, &content).unwrap();

        let new_content = content.replace("line10", "CHANGED");

        let tool = DiffShowTool;
        let result = tool
            .execute(
                "c3",
                json!({
                    "file": "ctx.txt",
                    "newContent": new_content,
                    "contextLines": 1
                }),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        // With context=1, line8 should NOT be in context (only line9 and line11)
        assert!(text.contains("line9"));
        assert!(text.contains("line11"));
        assert!(!text.contains("line8"));
        assert!(!text.contains("line12"));
    }

    #[tokio::test]
    async fn diff_show_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let tool = DiffShowTool;
        let result = tool
            .execute(
                "c4",
                json!({"file": "nope.txt", "newContent": "x"}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(extract_text(&result).contains("File not found"));
    }

    // ── diff_apply tests ────────────────────────────────────────────

    #[tokio::test]
    async fn diff_apply_simple_patch() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("apply.txt");
        std::fs::write(&file, "line1\nline2\nline3\nline4\nline5\n").unwrap();

        let patch = "\
--- apply.txt
+++ apply.txt
@@ -1,5 +1,5 @@
 line1
 line2
-line3
+modified
 line4
 line5
";

        let tool = DiffApplyTool;
        let result = tool
            .execute(
                "c1",
                json!({"file": "apply.txt", "patch": patch}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(
            !result.is_error,
            "Expected success: {}",
            extract_text(&result)
        );
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("modified"));
        assert!(!written.contains("line3"));
    }

    #[tokio::test]
    async fn diff_apply_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("dry.txt");
        std::fs::write(&file, "aaa\nbbb\nccc\n").unwrap();

        let patch = "\
--- dry.txt
+++ dry.txt
@@ -1,3 +1,3 @@
 aaa
-bbb
+xxx
 ccc
";

        let tool = DiffApplyTool;
        let result = tool
            .execute(
                "c2",
                json!({"file": "dry.txt", "patch": patch, "dryRun": true}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("Dry run"));

        // File should be unchanged
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "aaa\nbbb\nccc\n");
    }

    #[tokio::test]
    async fn diff_apply_fuzzy_offset() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("fuzzy.txt");
        // File has extra lines at the top compared to what the patch expects
        std::fs::write(
            &file,
            "extra1\nextra2\nextra3\nline1\nline2\nline3\nline4\nline5\n",
        )
        .unwrap();

        // Patch targets line1 at position 1 (no extra lines)
        let patch = "\
--- fuzzy.txt
+++ fuzzy.txt
@@ -1,5 +1,5 @@
 line1
 line2
-line3
+CHANGED
 line4
 line5
";

        let tool = DiffApplyTool;
        let result = tool
            .execute(
                "c3",
                json!({"file": "fuzzy.txt", "patch": patch}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(
            !result.is_error,
            "Expected success: {}",
            extract_text(&result)
        );
        let text = extract_text(&result);
        // Should mention the offset
        assert!(text.contains("offset"));

        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("CHANGED"));
        assert!(!written.contains("line3"));
        // Extra lines should still be there
        assert!(written.contains("extra1"));
    }

    #[tokio::test]
    async fn diff_apply_conflict_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("conflict.txt");
        std::fs::write(&file, "aaa\nbbb\nccc\n").unwrap();

        // Patch references lines that don't exist
        let patch = "\
--- conflict.txt
+++ conflict.txt
@@ -1,3 +1,3 @@
 xxx
-yyy
+zzz
 www
";

        let tool = DiffApplyTool;
        let result = tool
            .execute(
                "c4",
                json!({"file": "conflict.txt", "patch": patch}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("FAILED"));
    }

    #[tokio::test]
    async fn diff_apply_multiple_hunks() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("multi.txt");
        let content = (1..=20)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&file, &content).unwrap();

        let patch = "\
--- multi.txt
+++ multi.txt
@@ -2,3 +2,3 @@
 line2
-line3
+FIRST
 line4
@@ -18,3 +18,3 @@
 line18
-line19
+SECOND
 line20
";

        let tool = DiffApplyTool;
        let result = tool
            .execute(
                "c5",
                json!({"file": "multi.txt", "patch": patch}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(
            !result.is_error,
            "Expected success: {}",
            extract_text(&result)
        );
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("FIRST"));
        assert!(written.contains("SECOND"));
        assert!(!written.contains("line3"));
        assert!(!written.contains("line19"));
    }

    // ── Unit tests for parser ───────────────────────────────────────

    #[test]
    fn parse_hunk_header_standard() {
        let (start, count) = parse_hunk_header("@@ -10,5 +10,7 @@").unwrap();
        assert_eq!(start, 10);
        assert_eq!(count, 5);
    }

    #[test]
    fn parse_hunk_header_single_line() {
        let (start, count) = parse_hunk_header("@@ -1 +1 @@").unwrap();
        assert_eq!(start, 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn parse_unified_diff_basic() {
        let patch = "\
--- a.txt
+++ a.txt
@@ -1,3 +1,3 @@
 hello
-world
+universe
 end
";
        let hunks = parse_unified_diff(patch).unwrap();
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 0); // 1-indexed -> 0-indexed
                                           // 3 diff lines + 1 trailing empty context line from the heredoc
        assert_eq!(hunks[0].lines.len(), 4);
    }

    #[test]
    fn generate_diff_format() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nchanged\nline3\n";
        let diff = generate_unified_diff("test.txt", old, new, 3);
        assert!(diff.starts_with("--- test.txt\n+++ test.txt\n"));
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+changed"));
    }
}
