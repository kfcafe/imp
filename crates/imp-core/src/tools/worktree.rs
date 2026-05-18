use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;

use super::{resolve_path, Tool, ToolContext, ToolOutput};
use crate::config::AgentMode;
use crate::error::Result;

pub struct WorktreeTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorktreeActionClass {
    ReadOnly,
    Mutating,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedWorktreeEntry {
    path: String,
    branch: Option<String>,
    is_bare: bool,
    is_detached: bool,
}

#[async_trait]
impl Tool for WorktreeTool {
    fn name(&self) -> &str {
        "worktree"
    }

    fn label(&self) -> &str {
        "Worktree"
    }

    fn description(&self) -> &str {
        "Git worktree list/add/remove."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "add", "remove"],
                    "description": "Worktree action"
                },
                "path": {
                    "type": "string",
                    "description": "Repo/worktree path"
                },
                "worktree_path": {
                    "type": "string",
                    "description": "Worktree path"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name"
                },
                "start_point": {
                    "type": "string",
                    "description": "Starting ref"
                },
                "force": {
                    "type": "boolean",
                    "description": "Force remove"
                },
                "delete_branch": {
                    "type": "boolean",
                    "description": "Also delete branch"
                }
            },
            "required": ["action"]
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
        let action = match params["action"].as_str() {
            Some(action) => action,
            None => return Ok(ToolOutput::error("Missing required parameter: action")),
        };

        let Some(class) = action_class(action) else {
            return Ok(ToolOutput::error(format!(
                "Unknown worktree action \"{action}\""
            )));
        };

        if matches!(class, WorktreeActionClass::Mutating)
            && !matches!(ctx.mode, AgentMode::Full | AgentMode::Worker)
        {
            return Ok(ToolOutput::error(format!(
                "worktree action `{action}` is not permitted in {:?} mode; mutating worktree actions are limited to full/worker execution",
                ctx.mode
            )));
        }

        let cwd = match resolve_git_cwd(&ctx.cwd, params.get("path").and_then(|v| v.as_str())) {
            Ok(path) => path,
            Err(err) => return Ok(ToolOutput::error(err)),
        };

        let repo_root = match repo_root(&cwd).await {
            Ok(path) => path,
            Err(err) => return Ok(ToolOutput::error(err)),
        };

        match action {
            "list" => list_action(&cwd, &repo_root).await,
            "add" => add_action(&cwd, &repo_root, &params).await,
            "remove" => remove_action(&cwd, &repo_root, &params).await,
            _ => Ok(ToolOutput::error(format!(
                "Unsupported worktree action `{action}`"
            ))),
        }
    }
}

fn action_class(action: &str) -> Option<WorktreeActionClass> {
    match action {
        "list" => Some(WorktreeActionClass::ReadOnly),
        "add" | "remove" => Some(WorktreeActionClass::Mutating),
        _ => None,
    }
}

fn resolve_git_cwd(session_cwd: &Path, raw: Option<&str>) -> std::result::Result<PathBuf, String> {
    let path = match raw {
        Some(raw) if !raw.trim().is_empty() => resolve_path(session_cwd, raw),
        _ => session_cwd.to_path_buf(),
    };

    if path.is_dir() {
        return Ok(path);
    }

    if path.is_file() {
        return path.parent().map(Path::to_path_buf).ok_or_else(|| {
            format!(
                "Could not determine a working directory from file path: {}",
                path.display()
            )
        });
    }

    Err(format!(
        "git path not found or not accessible: {}",
        path.display()
    ))
}

async fn repo_root(cwd: &Path) -> std::result::Result<PathBuf, String> {
    let output = run_git(cwd, ["rev-parse", "--show-toplevel"])
        .await
        .map_err(|err| format!("Failed to run git in {}: {err}", cwd.display()))?;
    if !output.status.success() {
        return Err(not_git_repo_message(cwd, &output));
    }

    let root = stdout_trimmed(&output);
    if root.is_empty() {
        return Err(format!(
            "Failed to determine git repo root from {}",
            cwd.display()
        ));
    }

    Ok(PathBuf::from(root))
}

async fn list_action(cwd: &Path, repo_root: &Path) -> Result<ToolOutput> {
    let output = run_git(cwd, ["worktree", "list", "--porcelain"]).await?;
    if !output.status.success() {
        return Ok(git_failure("git worktree list failed", &output));
    }

    let entries = parse_worktree_list(&stdout_lossy(&output));
    let current_secondary = mana_core::worktree::detect_worktree(cwd).ok().flatten();
    let mut text = String::new();
    text.push_str(&format!("repo: {}\n", repo_root.display()));
    match &current_secondary {
        Some(info) => {
            text.push_str(&format!(
                "current worktree: secondary ({}) at {}\n",
                info.branch,
                info.worktree_path.display()
            ));
            text.push_str(&format!("main worktree: {}\n", info.main_path.display()));
        }
        None => {
            text.push_str("current worktree: main\n");
        }
    }
    if entries.is_empty() {
        text.push_str("registered worktrees: none\n");
    } else {
        text.push_str("registered worktrees:\n");
        for entry in &entries {
            let branch = entry.branch.as_deref().unwrap_or("(detached)");
            let mut flags = Vec::new();
            if entry.is_bare {
                flags.push("bare");
            }
            if entry.is_detached {
                flags.push("detached");
            }
            if flags.is_empty() {
                text.push_str(&format!("- {} [{}]\n", entry.path, branch));
            } else {
                text.push_str(&format!(
                    "- {} [{}] ({})\n",
                    entry.path,
                    branch,
                    flags.join(", ")
                ));
            }
        }
    }

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "list",
            "repo_root": repo_root.display().to_string(),
            "current_secondary_worktree": current_secondary.as_ref().map(|info| json!({
                "main_path": info.main_path.display().to_string(),
                "worktree_path": info.worktree_path.display().to_string(),
                "branch": info.branch,
            })),
            "worktrees": entries.iter().map(|entry| json!({
                "path": entry.path,
                "branch": entry.branch,
                "is_bare": entry.is_bare,
                "is_detached": entry.is_detached,
            })).collect::<Vec<_>>(),
        }),
        is_error: false,
    })
}

async fn add_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let Some(raw_worktree_path) = non_empty_param(params, "worktree_path") else {
        return Ok(ToolOutput::error(
            "Missing required parameter: worktree_path",
        ));
    };
    if let Err(err) = validate_path_string(raw_worktree_path, "worktree_path") {
        return Ok(ToolOutput::error(err.to_string()));
    }
    let Some(branch) = non_empty_param(params, "branch") else {
        return Ok(ToolOutput::error("Missing required parameter: branch"));
    };
    if let Err(err) = validate_ref(branch, "branch") {
        return Ok(ToolOutput::error(err.to_string()));
    }

    let start_point = non_empty_param(params, "start_point").unwrap_or("HEAD");
    if let Err(err) = validate_ref(start_point, "start_point") {
        return Ok(ToolOutput::error(err.to_string()));
    }
    let worktree_path = resolve_path(cwd, raw_worktree_path);

    let output = run_git_owned(
        cwd,
        vec![
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            branch.to_string(),
            worktree_path.display().to_string(),
            start_point.to_string(),
        ],
    )
    .await?;

    if !output.status.success() {
        return Ok(git_failure("git worktree add failed", &output));
    }

    let summary = format!(
        "Created worktree {} on branch {}",
        worktree_path.display(),
        branch
    );

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: summary.clone(),
        }],
        details: json!({
            "action": "add",
            "repo_root": repo_root.display().to_string(),
            "worktree_path": worktree_path.display().to_string(),
            "branch": branch,
            "start_point": start_point,
            "recovery": {
                "undo": "worktree remove",
                "worktree_path": worktree_path.display().to_string(),
                "branch": branch,
                "delete_branch": true,
            },
            "summary": summary,
        }),
        is_error: false,
    })
}

async fn remove_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let Some(raw_worktree_path) = non_empty_param(params, "worktree_path") else {
        return Ok(ToolOutput::error(
            "Missing required parameter: worktree_path",
        ));
    };
    if let Err(err) = validate_path_string(raw_worktree_path, "worktree_path") {
        return Ok(ToolOutput::error(err.to_string()));
    }
    let worktree_path = resolve_path(cwd, raw_worktree_path);
    let force = params["force"].as_bool().unwrap_or(false);
    let delete_branch = params["delete_branch"].as_bool().unwrap_or(false);

    if same_path(&worktree_path, repo_root) {
        return Ok(ToolOutput::error(
            "Refusing to remove the main worktree/root checkout",
        ));
    }
    if same_path(&worktree_path, cwd) {
        return Ok(ToolOutput::error(
            "Refusing to remove the current working directory worktree",
        ));
    }

    let entries_output = run_git(cwd, ["worktree", "list", "--porcelain"]).await?;
    if !entries_output.status.success() {
        return Ok(git_failure("git worktree list failed", &entries_output));
    }
    let entries = parse_worktree_list(&stdout_lossy(&entries_output));
    let explicit_branch = non_empty_param(params, "branch");
    if let Some(branch) = explicit_branch {
        if let Err(err) = validate_ref(branch, "branch") {
            return Ok(ToolOutput::error(err.to_string()));
        }
    }
    if delete_branch && explicit_branch.is_none() {
        return Ok(ToolOutput::error(
            "delete_branch=true requires explicit branch",
        ));
    }
    let matched_branch = explicit_branch.map(str::to_string).or_else(|| {
        entries
            .iter()
            .find(|entry| same_path(Path::new(&entry.path), &worktree_path))
            .and_then(|entry| entry.branch.clone())
    });

    let mut args = vec!["worktree".to_string(), "remove".to_string()];
    if force {
        args.push("--force".to_string());
    }
    args.push(worktree_path.display().to_string());

    let output = run_git_owned(cwd, args).await?;
    if !output.status.success() {
        return Ok(git_failure("git worktree remove failed", &output));
    }

    let mut branch_deleted = false;
    if delete_branch {
        if let Some(branch) = matched_branch.as_deref() {
            let branch_output = run_git_owned(
                cwd,
                vec![
                    "branch".to_string(),
                    if force { "-D" } else { "-d" }.to_string(),
                    branch.to_string(),
                ],
            )
            .await?;
            if !branch_output.status.success() {
                return Ok(git_failure("git branch delete failed", &branch_output));
            }
            branch_deleted = true;
        }
    }

    let summary = if branch_deleted {
        format!(
            "Removed worktree {} and deleted branch {}",
            worktree_path.display(),
            matched_branch.as_deref().unwrap_or("(unknown)")
        )
    } else {
        format!("Removed worktree {}", worktree_path.display())
    };

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: summary.clone(),
        }],
        details: json!({
            "action": "remove",
            "repo_root": repo_root.display().to_string(),
            "worktree_path": worktree_path.display().to_string(),
            "force": force,
            "delete_branch": delete_branch,
            "branch": matched_branch,
            "branch_deleted": branch_deleted,
            "recovery": {
                "guidance": "Recreate removed worktree with worktree add if needed; deleted branches may be recoverable from reflog.",
                "worktree_path": worktree_path.display().to_string(),
                "branch_deleted": branch_deleted,
            },
            "summary": summary,
        }),
        is_error: false,
    })
}

fn non_empty_param<'a>(params: &'a serde_json::Value, field_name: &str) -> Option<&'a str> {
    params
        .get(field_name)?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn validate_path_string(
    value: &str,
    field_name: &str,
) -> std::result::Result<(), crate::error::Error> {
    if value.chars().any(|c| c == '\0' || c.is_control()) {
        return Err(crate::error::Error::Tool(format!(
            "{field_name} must be a safe path string"
        )));
    }
    Ok(())
}

fn validate_ref(value: &str, field_name: &str) -> std::result::Result<(), crate::error::Error> {
    if value.starts_with('-') || value.chars().any(|c| c == '\0' || c.is_control()) {
        return Err(crate::error::Error::Tool(format!(
            "{field_name} must be a safe git ref"
        )));
    }
    Ok(())
}

async fn run_git<I, S>(cwd: &Path, args: I) -> std::io::Result<std::process::Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.output().await
}

async fn run_git_owned(cwd: &Path, args: Vec<String>) -> std::io::Result<std::process::Output> {
    run_git(cwd, args).await
}

fn stdout_lossy(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace('\r', "")
}

fn stderr_lossy(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).replace('\r', "")
}

fn stdout_trimmed(output: &std::process::Output) -> String {
    stdout_lossy(output).trim().to_string()
}

fn stderr_trimmed(output: &std::process::Output) -> String {
    stderr_lossy(output).trim().to_string()
}

fn not_git_repo_message(cwd: &Path, output: &std::process::Output) -> String {
    let stderr = stderr_trimmed(output);
    if stderr.is_empty() {
        format!("Not inside a git repository: {}", cwd.display())
    } else {
        format!("Not inside a git repository: {}\n{}", cwd.display(), stderr)
    }
}

fn git_failure(prefix: &str, output: &std::process::Output) -> ToolOutput {
    let stdout = stdout_trimmed(output);
    let stderr = stderr_trimmed(output);
    let combined = match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => prefix.to_string(),
        (false, true) => format!("{prefix}: {stdout}"),
        (true, false) => format!("{prefix}: {stderr}"),
        (false, false) => format!("{prefix}: {stdout}\n{stderr}"),
    };
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text: combined }],
        details: json!({
            "success": false,
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
        }),
        is_error: true,
    }
}

fn parse_worktree_list(output: &str) -> Vec<ParsedWorktreeEntry> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_detached = false;

    let push_current = |entries: &mut Vec<ParsedWorktreeEntry>,
                        current_path: &mut Option<String>,
                        current_branch: &mut Option<String>,
                        is_bare: &mut bool,
                        is_detached: &mut bool| {
        if let Some(path) = current_path.take() {
            entries.push(ParsedWorktreeEntry {
                path,
                branch: current_branch.take(),
                is_bare: *is_bare,
                is_detached: *is_detached,
            });
        }
        *is_bare = false;
        *is_detached = false;
    };

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            push_current(
                &mut entries,
                &mut current_path,
                &mut current_branch,
                &mut is_bare,
                &mut is_detached,
            );
            current_path = Some(path.to_string());
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            current_branch = Some(
                branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string(),
            );
        } else if line == "bare" {
            is_bare = true;
        } else if line == "detached" {
            is_detached = true;
        }
    }

    push_current(
        &mut entries,
        &mut current_path,
        &mut current_branch,
        &mut is_bare,
        &mut is_detached,
    );
    entries
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana_review::TurnManaReviewAccumulator;
    use crate::tools::{CheckpointState, FileCache, FileTracker};
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    fn test_ctx(dir: &Path, mode: AgentMode) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
            config: Arc::new(crate::config::Config::default()),
            run_policy: Default::default(),
            supporting_provenance: Vec::new(),
        }
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap_or_else(|e| panic!("git {:?} failed to execute: {e}", args));
        assert!(
            output.status.success(),
            "git {:?} in {} failed (exit {:?}):\nstdout: {}\nstderr: {}",
            args,
            dir.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn setup_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        run_git(dir.path(), &["init"]);
        run_git(dir.path(), &["config", "user.email", "test@test.com"]);
        run_git(dir.path(), &["config", "user.name", "Test User"]);
        fs::write(dir.path().join("note.txt"), "hello\n").unwrap();
        run_git(dir.path(), &["add", "-A"]);
        run_git(dir.path(), &["commit", "-m", "initial"]);
        dir
    }

    fn extract_text(result: &ToolOutput) -> String {
        result.text_content().unwrap_or_default().to_string()
    }

    #[test]
    fn worktree_schema_exposes_list_add_remove() {
        let schema = WorktreeTool.parameters();
        let properties = schema["properties"].as_object().unwrap();
        let actions = properties["action"]["enum"].as_array().unwrap();
        assert!(actions.iter().any(|value| value == "list"));
        assert!(actions.iter().any(|value| value == "add"));
        assert!(actions.iter().any(|value| value == "remove"));
        assert!(properties.contains_key("worktree_path"));
        assert!(properties.contains_key("start_point"));
        assert!(properties.contains_key("delete_branch"));
        assert!(!properties.contains_key("worktreePath"));
        assert!(!properties.contains_key("deleteBranch"));
    }

    #[tokio::test]
    async fn worktree_add_list_and_remove_work() {
        let dir = setup_repo();
        let tool = WorktreeTool;
        let worktree_path = dir.path().join("../repo-worktree");
        let worktree_path_str = worktree_path.display().to_string();

        let add = tool
            .execute(
                "c-add",
                json!({
                    "action": "add",
                    "worktree_path": worktree_path_str,
                    "branch": "feature/test",
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!add.is_error);
        assert!(worktree_path.exists());
        assert_eq!(add.details["recovery"]["delete_branch"], json!(true));

        let list = tool
            .execute(
                "c-list",
                json!({"action": "list"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!list.is_error);
        assert!(list.details["worktrees"].as_array().unwrap().len() >= 2);

        let remove = tool
            .execute(
                "c-remove",
                json!({
                    "action": "remove",
                    "worktree_path": worktree_path.display().to_string(),
                    "branch": "feature/test",
                    "delete_branch": true,
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!remove.is_error);
        assert!(!worktree_path.exists());
        assert_eq!(remove.details["branch_deleted"], json!(true));
    }

    #[tokio::test]
    async fn worktree_refuses_removing_main_or_current_worktree() {
        let dir = setup_repo();
        let tool = WorktreeTool;

        let main = tool
            .execute(
                "c-main",
                json!({"action": "remove", "worktree_path": dir.path().display().to_string()}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(main.is_error);
        assert!(extract_text(&main).contains("main worktree"));

        let current = tool
            .execute(
                "c-current",
                json!({"action": "remove", "worktree_path": "."}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(current.is_error);
        assert!(
            extract_text(&current).contains("main worktree")
                || extract_text(&current).contains("current working directory")
        );
    }

    #[tokio::test]
    async fn worktree_delete_branch_requires_explicit_branch() {
        let dir = setup_repo();
        let tool = WorktreeTool;
        let worktree_path = dir.path().join("../repo-worktree-no-delete");
        let add = tool
            .execute(
                "c-add",
                json!({
                    "action": "add",
                    "worktree_path": worktree_path.display().to_string(),
                    "branch": "feature/no-delete",
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!add.is_error);

        let remove = tool
            .execute(
                "c-remove",
                json!({
                    "action": "remove",
                    "worktree_path": worktree_path.display().to_string(),
                    "delete_branch": true,
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(remove.is_error);
        assert!(extract_text(&remove).contains("requires explicit branch"));

        let cleanup = tool
            .execute(
                "c-cleanup",
                json!({
                    "action": "remove",
                    "worktree_path": worktree_path.display().to_string(),
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!cleanup.is_error);
    }

    #[tokio::test]
    async fn worktree_validates_branch_and_path() {
        let dir = setup_repo();
        let tool = WorktreeTool;

        let branch = tool
            .execute(
                "c-branch",
                json!({"action": "add", "worktree_path": "../bad", "branch": "-bad"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(branch.is_error);

        let path = tool
            .execute(
                "c-path",
                json!({"action": "add", "worktree_path": "bad\npath", "branch": "feature/bad"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(path.is_error);
    }

    #[tokio::test]
    async fn planner_mode_blocks_mutating_worktree_actions() {
        let dir = setup_repo();
        let tool = WorktreeTool;

        let result = tool
            .execute(
                "c-add",
                json!({"action": "add", "worktree_path": "../blocked", "branch": "feature/blocked"}),
                test_ctx(dir.path(), AgentMode::Planner),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(extract_text(&result).contains("not permitted"));
    }

    #[test]
    fn parse_worktree_list_handles_multiple_entries() {
        let entries = parse_worktree_list(
            "worktree /repo\nHEAD abc\nbranch refs/heads/main\n\nworktree /repo-wt\nHEAD def\nbranch refs/heads/feature\ndetached\n",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/repo");
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(entries[1].branch.as_deref(), Some("feature"));
        assert!(entries[1].is_detached);
    }
}
