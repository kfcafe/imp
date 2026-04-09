//! Canonical single-unit mana worker runtime.
//!
//! Provides the reusable substrate for executing one mana unit:
//! loading the unit via canonical mana-core APIs, assembling execution
//! context (task prompt, prefill, dependency summaries), and reporting
//! structured outcomes.
//!
//! This module is consumed by:
//! - `imp run <unit-id>` (imp-cli) — the preferred single-unit CLI path
//! - legacy `mana run` compatibility flows — transitional dispatch into imp workers
//! - imp's native mana tool — the first-class orchestration UX
//!
//! ## Architecture
//!
//! ```text
//! imp native mana tool        = first-class orchestration UX
//! imp run (this module)       = canonical single-unit worker runtime
//! legacy mana run compatibility = transitional parallel dispatch into imp workers
//! ```

use std::path::{Path, PathBuf};

use mana_core::api;
pub use tower_contracts::worker::{
    WorkerAssignment, WorkerAttempt, WorkerResult, WorkerStatus,
};

use crate::context_prefill::{self, AssembledContext, FileSpec, PrefillConfig};
use crate::system_prompt::{Attempt, Dependency, TaskContext};

// ---------------------------------------------------------------------------
// Shared contract re-exports
// ---------------------------------------------------------------------------

// Canonical worker assignment/outcome vocabulary now lives in tower-contracts.
// Re-export it here to keep current imp-core call sites stable during migration.

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Load a worker assignment from a mana unit using canonical mana-core APIs.
///
/// This replaces the ad hoc markdown-scanning `load_mana_unit()` that lived
/// in imp-cli. It uses `mana_core::api::get_unit()` for canonical resolution
/// and `mana_core::discovery::find_mana_dir()` for `.mana/` discovery.
pub fn load_assignment(
    cwd: &Path,
    unit_id: &str,
) -> Result<WorkerAssignment, Box<dyn std::error::Error>> {
    load_assignment_with_mana_dir(cwd, unit_id, None)
}

/// Load a worker assignment with an explicit mana dir override.
pub fn load_assignment_with_mana_dir(
    cwd: &Path,
    unit_id: &str,
    mana_dir_override: Option<&Path>,
) -> Result<WorkerAssignment, Box<dyn std::error::Error>> {
    let mana_dir = match mana_dir_override {
        Some(dir) => dir.to_path_buf(),
        None => mana_core::discovery::find_mana_dir(cwd).map_err(|e| {
            format!(
                "Could not find .mana directory while walking up from {}: {e}",
                cwd.display()
            )
        })?,
    };

    let workspace_root = mana_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.to_path_buf());

    let unit = api::get_unit(&mana_dir, unit_id).map_err(|e| {
        format!("Failed to load mana unit {unit_id}: {e}")
    })?;

    // Read the unit file body (description may be in the markdown body
    // after the frontmatter, which mana-core merges into unit.description).
    // mana-core's Unit already handles frontmatter+body merging in from_file(),
    // so unit.description contains the full combined text.
    let description = unit
        .description
        .clone()
        .unwrap_or_default();

    // Also read the markdown body from the unit file for any content
    // after the frontmatter that mana-core stores separately.
    let unit_path = mana_core::discovery::find_unit_file(&mana_dir, unit_id).ok();
    let body = unit_path.as_ref().and_then(|path| {
        let content = std::fs::read_to_string(path).ok()?;
        let body = extract_markdown_body(&content)?;
        if body.trim().is_empty() {
            None
        } else {
            Some(body)
        }
    });

    let full_description = match body {
        Some(body_text) if !description.is_empty() => {
            format!("{}\n\n{}", description.trim(), body_text.trim())
        }
        Some(body_text) => body_text.trim().to_string(),
        None => description,
    };

    let attempts = unit
        .attempt_log
        .iter()
        .map(|record| WorkerAttempt {
            number: record.num,
            outcome: format!("{:?}", record.outcome).to_lowercase(),
            summary: record
                .notes
                .clone()
                .unwrap_or_default(),
        })
        .collect();

    // Extract explicit file references from paths field.
    let files: Vec<String> = Vec::new(); // Unit doesn't have a separate `files` field in mana-core

    Ok(WorkerAssignment {
        id: unit.id.clone(),
        title: unit.title.clone(),
        description: full_description,
        acceptance: unit.acceptance.clone(),
        verify: unit.verify.clone(),
        notes: unit.notes.clone(),
        decisions: unit.decisions.clone(),
        dependencies: unit.dependencies.clone(),
        paths: unit.paths.clone(),
        files,
        attempts,
        workspace_root,
        model: unit.model.clone(),
    })
}

// ---------------------------------------------------------------------------
// Context assembly
// ---------------------------------------------------------------------------

/// Build a `TaskContext` from a worker assignment for system prompt Layer 5.
pub fn build_task_context(
    assignment: &WorkerAssignment,
) -> TaskContext {
    let description = assignment.description.trim().to_string();

    let notes = assignment
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string);

    let dependencies = if assignment.dependencies.is_empty() {
        Vec::new()
    } else {
        // Try to load dependency summaries from the index
        let mana_dir = assignment.workspace_root.join(".mana");
        match api::load_index(&mana_dir) {
            Ok(index) => assignment
                .dependencies
                .iter()
                .map(|dep_id| {
                    let entry = index.units.iter().find(|e| e.id == *dep_id);
                    Dependency {
                        name: dep_id.clone(),
                        status: entry
                            .map(|e| e.status.to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        detail: entry
                            .map(|e| e.title.clone())
                            .unwrap_or_else(|| "not found in active index".to_string()),
                    }
                })
                .collect(),
            Err(_) => assignment
                .dependencies
                .iter()
                .map(|dep_id| Dependency {
                    name: dep_id.clone(),
                    status: "unknown".to_string(),
                    detail: "dependency status unavailable".to_string(),
                })
                .collect(),
        }
    };

    let mut context_paths = assignment.paths.clone();
    for file in &assignment.files {
        if !context_paths.iter().any(|path| path == file) {
            context_paths.push(file.clone());
        }
    }

    TaskContext {
        title: assignment.title.clone(),
        description,
        acceptance: assignment.acceptance.clone(),
        verify: assignment.verify.clone(),
        notes,
        attempts: assignment
            .attempts
            .iter()
            .map(|a| Attempt {
                number: a.number,
                outcome: a.outcome.clone(),
                summary: a.summary.clone(),
            })
            .collect(),
        dependencies,
        decisions: assignment.decisions.clone(),
        context_paths,
    }
}

/// Build a task prompt string from a worker assignment.
///
/// This is the user-facing prompt that starts the agent's work.
pub fn build_task_prompt(assignment: &WorkerAssignment) -> String {
    let mut prompt = format!("Task: {}", assignment.title);

    if !assignment.description.trim().is_empty() {
        prompt.push_str("\n\n");
        prompt.push_str(assignment.description.trim());
    }

    if let Some(notes) = assignment
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        prompt.push_str("\n\nNotes:\n");
        prompt.push_str(notes);
    }

    if !assignment.files.is_empty() || !assignment.paths.is_empty() {
        prompt.push_str("\n\nReferenced files:\n");
        for path in assignment.paths.iter().chain(assignment.files.iter()) {
            prompt.push_str("- ");
            prompt.push_str(path);
            prompt.push('\n');
        }
        while prompt.ends_with('\n') {
            prompt.pop();
        }
    }

    if !assignment.attempts.is_empty() {
        prompt.push_str("\n\nPrevious attempts:\n");
        for attempt in &assignment.attempts {
            prompt.push_str(&format!(
                "- Attempt {} ({}): {}\n",
                attempt.number, attempt.outcome, attempt.summary
            ));
        }
        while prompt.ends_with('\n') {
            prompt.pop();
        }
    }

    if let Some(verify) = assignment
        .verify
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        prompt.push_str("\n\nVerify command: ");
        prompt.push_str(verify);
    }

    prompt
}

/// Assemble context prefill messages from the assignment's file references.
pub fn assemble_prefill(
    assignment: &WorkerAssignment,
    cwd: &Path,
) -> AssembledContext {
    let file_specs = if !assignment.files.is_empty() {
        assignment
            .files
            .iter()
            .filter_map(|s| parse_file_spec(s))
            .collect()
    } else if !assignment.paths.is_empty() {
        assignment
            .paths
            .iter()
            .filter_map(|s| parse_file_spec(s))
            .collect()
    } else {
        context_prefill::detect_file_paths(&assignment.description)
    };

    if file_specs.is_empty() {
        return AssembledContext::empty();
    }

    let config = PrefillConfig::default();
    context_prefill::assemble_context(&file_specs, cwd, &config)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a file spec string like "src/foo.rs" or "src/foo.rs:tail:50".
fn parse_file_spec(s: &str) -> Option<FileSpec> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (path_str, suffix) = if let Some(dot_pos) = s.rfind('.') {
        let after_ext = &s[dot_pos..];
        if let Some(colon_pos) = after_ext.find(':') {
            let split_at = dot_pos + colon_pos;
            (&s[..split_at], Some(&s[split_at + 1..]))
        } else {
            (s, None)
        }
    } else {
        (s, None)
    };

    let mode = match suffix {
        Some(suf) if suf.starts_with("tail:") => suf[5..]
            .parse::<usize>()
            .ok()
            .map(context_prefill::FileMode::Tail)
            .unwrap_or(context_prefill::FileMode::Full),
        Some(suf) if suf.contains('-') => {
            let parts: Vec<&str> = suf.splitn(2, '-').collect();
            match (
                parts[0].parse::<usize>(),
                parts.get(1).and_then(|p| p.parse::<usize>().ok()),
            ) {
                (Ok(start), Some(end)) => context_prefill::FileMode::Range(start, end),
                _ => context_prefill::FileMode::Full,
            }
        }
        _ => context_prefill::FileMode::Full,
    };

    Some(FileSpec {
        path: PathBuf::from(path_str),
        mode,
    })
}

/// Extract the markdown body after YAML frontmatter.
fn extract_markdown_body(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first().copied() != Some("---") {
        return None;
    }
    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, line)| (*line == "---").then_some(i))?;
    let body = lines[end + 1..].join("\n");
    Some(body)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_task_prompt_basic() {
        let assignment = WorkerAssignment {
            id: "1".to_string(),
            title: "Fix the bug".to_string(),
            description: "There is a null pointer in foo.rs".to_string(),
            acceptance: None,
            verify: Some("cargo test".to_string()),
            notes: None,
            decisions: Vec::new(),
            dependencies: Vec::new(),
            paths: Vec::new(),
            files: Vec::new(),
            attempts: Vec::new(),
            workspace_root: PathBuf::from("/tmp"),
            model: None,
        };
        let prompt = build_task_prompt(&assignment);
        assert!(prompt.contains("Task: Fix the bug"));
        assert!(prompt.contains("null pointer"));
        assert!(prompt.contains("Verify command: cargo test"));
    }

    #[test]
    fn build_task_prompt_with_attempts() {
        let assignment = WorkerAssignment {
            id: "2".to_string(),
            title: "Add test".to_string(),
            description: "Add a test for auth".to_string(),
            acceptance: None,
            verify: None,
            notes: Some("Check the fixtures module".to_string()),
            decisions: Vec::new(),
            dependencies: Vec::new(),
            paths: vec!["tests/auth.rs".to_string()],
            files: vec!["src/fixtures.rs".to_string()],
            attempts: vec![WorkerAttempt {
                number: 1,
                outcome: "fail".to_string(),
                summary: "Wrong fixture path".to_string(),
            }],
            workspace_root: PathBuf::from("/tmp"),
            model: None,
        };
        let prompt = build_task_prompt(&assignment);
        assert!(prompt.contains("Notes:"));
        assert!(prompt.contains("Check the fixtures module"));
        assert!(prompt.contains("Previous attempts:"));
        assert!(prompt.contains("Referenced files:"));
        assert!(prompt.contains("tests/auth.rs"));
        assert!(prompt.contains("src/fixtures.rs"));
        assert!(prompt.contains("Attempt 1 (fail): Wrong fixture path"));
    }

    #[test]
    fn build_task_context_populates_fields() {
        let assignment = WorkerAssignment {
            id: "3".to_string(),
            title: "Refactor module".to_string(),
            description: "Split into submodules".to_string(),
            acceptance: Some("All tests pass".to_string()),
            verify: Some("cargo test".to_string()),
            notes: Some("Prefer touching parser and module wiring first".to_string()),
            decisions: vec!["Use mod.rs or inline?".to_string()],
            dependencies: Vec::new(),
            paths: vec!["src/lib.rs".to_string()],
            files: vec!["src/parser.rs".to_string()],
            attempts: Vec::new(),
            workspace_root: PathBuf::from("/tmp"),
            model: None,
        };
        let ctx = build_task_context(&assignment);
        assert_eq!(ctx.title, "Refactor module");
        assert_eq!(ctx.acceptance.as_deref(), Some("All tests pass"));
        assert_eq!(ctx.verify.as_deref(), Some("cargo test"));
        assert_eq!(ctx.notes.as_deref(), Some("Prefer touching parser and module wiring first"));
        assert_eq!(ctx.decisions, vec!["Use mod.rs or inline?"]);
        assert_eq!(ctx.context_paths, vec!["src/lib.rs", "src/parser.rs"]);
    }

    #[test]
    fn parse_file_spec_plain() {
        let spec = parse_file_spec("src/main.rs").unwrap();
        assert_eq!(spec.path, PathBuf::from("src/main.rs"));
        assert_eq!(spec.mode, context_prefill::FileMode::Full);
    }

    #[test]
    fn parse_file_spec_tail() {
        let spec = parse_file_spec("src/main.rs:tail:50").unwrap();
        assert_eq!(spec.path, PathBuf::from("src/main.rs"));
        assert_eq!(spec.mode, context_prefill::FileMode::Tail(50));
    }

    #[test]
    fn parse_file_spec_range() {
        let spec = parse_file_spec("src/main.rs:10-20").unwrap();
        assert_eq!(spec.path, PathBuf::from("src/main.rs"));
        assert_eq!(spec.mode, context_prefill::FileMode::Range(10, 20));
    }

    #[test]
    fn parse_file_spec_empty() {
        assert!(parse_file_spec("").is_none());
        assert!(parse_file_spec("  ").is_none());
    }

    #[test]
    fn extract_markdown_body_works() {
        let content = "---\ntitle: Test\n---\n\nBody text here.";
        let body = extract_markdown_body(content).unwrap();
        assert!(body.contains("Body text here."));
    }

    #[test]
    fn extract_markdown_body_no_frontmatter() {
        assert!(extract_markdown_body("No frontmatter").is_none());
    }
}
