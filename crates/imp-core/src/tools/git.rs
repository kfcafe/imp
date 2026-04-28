use std::path::{Path, PathBuf};
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::json;
use tokio::process::Command;

use super::{resolve_path, truncate_head, Tool, ToolContext, ToolOutput};
use crate::config::AgentMode;
use crate::error::Result;

const DEFAULT_LOG_LIMIT: u32 = 10;
const DISPLAY_MAX_LINES: usize = 400;
const DISPLAY_MAX_BYTES: usize = 32 * 1024;

pub struct GitTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitActionClass {
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
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git"
    }

    fn label(&self) -> &str {
        "Git"
    }

    fn description(&self) -> &str {
        "Local git operations for status, diff, log, commit, restore, and worktrees."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "status",
                        "diff",
                        "log",
                        "merge_base",
                        "worktree_info",
                        "stage",
                        "commit",
                        "restore",
                        "worktree_add",
                        "worktree_remove"
                    ],
                    "description": "Git operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "Optional repo or worktree path to run from; defaults to the session cwd"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional file paths for diff/log/stage/restore"
                },
                "all": {
                    "type": "boolean",
                    "description": "For stage: stage all changes with git add -A"
                },
                "cached": {
                    "type": "boolean",
                    "description": "For diff: compare staged changes instead of working tree"
                },
                "base": {
                    "type": "string",
                    "description": "For diff: base ref; when set without head, compares base..HEAD"
                },
                "head": {
                    "type": "string",
                    "description": "For diff: head ref when comparing two refs"
                },
                "ref1": {
                    "type": "string",
                    "description": "For merge_base: first ref"
                },
                "ref2": {
                    "type": "string",
                    "description": "For merge_base: second ref"
                },
                "limit": {
                    "type": "number",
                    "description": "For log: maximum number of entries to show (default 10)"
                },
                "message": {
                    "type": "string",
                    "description": "For commit: commit message"
                },
                "allowEmpty": {
                    "type": "boolean",
                    "description": "For commit: allow an empty commit"
                },
                "source": {
                    "type": "string",
                    "description": "For restore: optional source ref (defaults to index/HEAD behavior)"
                },
                "worktreePath": {
                    "type": "string",
                    "description": "For worktree_add/remove: path to add or remove"
                },
                "branch": {
                    "type": "string",
                    "description": "For worktree_add: branch name to create; for worktree_remove with deleteBranch=true, explicit branch to delete"
                },
                "startPoint": {
                    "type": "string",
                    "description": "For worktree_add: optional starting ref (defaults to HEAD)"
                },
                "force": {
                    "type": "boolean",
                    "description": "For worktree_remove: force removal; for deleteBranch=true, force branch deletion"
                },
                "deleteBranch": {
                    "type": "boolean",
                    "description": "For worktree_remove: also delete the associated branch after removal"
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
                "Unknown git action \"{action}\""
            )));
        };

        if matches!(class, GitActionClass::Mutating)
            && !matches!(ctx.mode, AgentMode::Full | AgentMode::Worker)
        {
            return Ok(ToolOutput::error(format!(
                "git action `{action}` is not permitted in {:?} mode; mutating git actions are limited to full/worker execution",
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
            "status" => status_action(&cwd, &repo_root).await,
            "diff" => diff_action(&cwd, &repo_root, &params).await,
            "log" => log_action(&cwd, &repo_root, &params).await,
            "merge_base" => merge_base_action(&cwd, &repo_root, &params).await,
            "worktree_info" => worktree_info_action(&cwd, &repo_root).await,
            "stage" => stage_action(&cwd, &repo_root, &params).await,
            "commit" => commit_action(&cwd, &repo_root, &params).await,
            "restore" => restore_action(&cwd, &repo_root, &params, &ctx).await,
            "worktree_add" => worktree_add_action(&cwd, &repo_root, &params).await,
            "worktree_remove" => worktree_remove_action(&cwd, &repo_root, &params).await,
            _ => Ok(ToolOutput::error(format!(
                "Unsupported git action `{action}`"
            ))),
        }
    }
}

fn action_class(action: &str) -> Option<GitActionClass> {
    match action {
        "status" | "diff" | "log" | "merge_base" | "worktree_info" => {
            Some(GitActionClass::ReadOnly)
        }
        "stage" | "commit" | "restore" | "worktree_add" | "worktree_remove" => {
            Some(GitActionClass::Mutating)
        }
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

async fn status_action(cwd: &Path, repo_root: &Path) -> Result<ToolOutput> {
    let output = run_git(cwd, ["status", "--porcelain=v1", "--branch"]).await?;
    if !output.status.success() {
        return Ok(git_failure("git status failed", &output));
    }

    let status_text = stdout_lossy(&output);
    let mut branch_summary = String::new();
    let mut entries = Vec::new();
    let mut staged = 0u32;
    let mut unstaged = 0u32;
    let mut untracked = 0u32;

    for line in status_text.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            branch_summary = rest.trim().to_string();
            continue;
        }
        if line.len() < 3 {
            continue;
        }
        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let path = line[3..].trim().to_string();
        if index_status != ' ' && index_status != '?' {
            staged += 1;
        }
        if worktree_status != ' ' && worktree_status != '?' {
            unstaged += 1;
        }
        if index_status == '?' && worktree_status == '?' {
            untracked += 1;
        }
        entries.push(json!({
            "index_status": index_status.to_string(),
            "worktree_status": worktree_status.to_string(),
            "path": path,
            "raw": line,
        }));
    }

    let head = head_sha_short(cwd)
        .await
        .unwrap_or_else(|| "unknown".to_string());
    let secondary = mana_core::worktree::detect_worktree(cwd).ok().flatten();
    let clean = entries.is_empty();

    let mut text = String::new();
    text.push_str(&format!("repo: {}\n", repo_root.display()));
    text.push_str(&format!(
        "branch: {}\n",
        display_or_unknown(&branch_summary)
    ));
    text.push_str(&format!("head: {head}\n"));
    text.push_str(&format!(
        "state: {}\n",
        if clean { "clean" } else { "dirty" }
    ));
    if let Some(info) = &secondary {
        text.push_str(&format!("worktree: secondary ({})\n", info.branch));
        text.push_str(&format!("main worktree: {}\n", info.main_path.display()));
    } else {
        text.push_str("worktree: main\n");
    }
    if !entries.is_empty() {
        text.push_str("changes:\n");
        for entry in &entries {
            if let Some(raw) = entry.get("raw").and_then(|v| v.as_str()) {
                text.push_str(raw);
                text.push('\n');
            }
        }
    }

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "branch": branch_summary,
            "head": head,
            "clean": clean,
            "counts": {
                "staged": staged,
                "unstaged": unstaged,
                "untracked": untracked,
            },
            "entries": entries,
            "secondary_worktree": secondary.as_ref().map(|info| json!({
                "main_path": info.main_path.display().to_string(),
                "worktree_path": info.worktree_path.display().to_string(),
                "branch": info.branch,
            })),
        }),
        is_error: false,
    })
}

fn non_empty_param<'a>(params: &'a serde_json::Value, field_name: &str) -> Option<&'a str> {
    params
        .get(field_name)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
}

async fn diff_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let files = parse_string_array(params, "files")?;
    let cached = params["cached"].as_bool().unwrap_or(false);
    let base = non_empty_param(params, "base");
    let head = non_empty_param(params, "head");

    let mut args = vec!["diff".to_string()];
    if let Some(base) = base {
        let range = match head {
            Some(head) => format!("{base}..{head}"),
            None => format!("{base}..HEAD"),
        };
        args.push(range);
    } else if cached {
        args.push("--cached".to_string());
    }

    if !files.is_empty() {
        args.push("--".to_string());
        args.extend(files.iter().cloned());
    }

    let output = run_git_owned(cwd, args).await?;
    if !output.status.success() {
        return Ok(git_failure("git diff failed", &output));
    }

    let diff = stdout_lossy(&output);
    let (display_content, display_note, temp_file) = truncate_for_display(&diff);
    let text = if diff.trim().is_empty() {
        "No diff.".to_string()
    } else if display_note.is_empty() {
        display_content.clone()
    } else {
        format!("{display_content}\n{display_note}")
    };

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "cached": cached,
            "base": base,
            "head": head,
            "files": files,
            "display_content": display_content,
            "display_note": display_note,
            "temp_file": temp_file.map(|p| p.display().to_string()),
        }),
        is_error: false,
    })
}

async fn log_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let files = parse_string_array(params, "files")?;
    let limit = params["limit"]
        .as_u64()
        .unwrap_or(DEFAULT_LOG_LIMIT as u64)
        .max(1);

    let mut args = vec![
        "log".to_string(),
        "--oneline".to_string(),
        "--decorate".to_string(),
        "-n".to_string(),
        limit.to_string(),
    ];
    if !files.is_empty() {
        args.push("--".to_string());
        args.extend(files.iter().cloned());
    }

    let output = run_git_owned(cwd, args).await?;
    if !output.status.success() {
        return Ok(git_failure("git log failed", &output));
    }

    let log = stdout_lossy(&output);
    let text = if log.trim().is_empty() {
        "No commits matched.".to_string()
    } else {
        log.trim_end().to_string()
    };

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "limit": limit,
            "files": files,
        }),
        is_error: false,
    })
}

async fn merge_base_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let Some(ref1) = params["ref1"].as_str() else {
        return Ok(ToolOutput::error("Missing required parameter: ref1"));
    };
    let Some(ref2) = params["ref2"].as_str() else {
        return Ok(ToolOutput::error("Missing required parameter: ref2"));
    };

    let output = run_git_owned(
        cwd,
        vec!["merge-base".to_string(), ref1.to_string(), ref2.to_string()],
    )
    .await?;

    if !output.status.success() {
        return Ok(git_failure("git merge-base failed", &output));
    }

    let merge_base = stdout_trimmed(&output);
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: merge_base.clone(),
        }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "ref1": ref1,
            "ref2": ref2,
            "merge_base": merge_base,
        }),
        is_error: false,
    })
}

async fn worktree_info_action(cwd: &Path, repo_root: &Path) -> Result<ToolOutput> {
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

async fn stage_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let files = parse_string_array(params, "files")?;
    let all = params["all"].as_bool().unwrap_or(false);

    let args = if all {
        vec!["add".to_string(), "-A".to_string()]
    } else {
        if files.is_empty() {
            return Ok(ToolOutput::error(
                "stage requires either files[] or all=true",
            ));
        }
        let mut args = vec!["add".to_string(), "--".to_string()];
        args.extend(files.iter().cloned());
        args
    };

    let output = run_git_owned(cwd, args).await?;
    if !output.status.success() {
        return Ok(git_failure("git add failed", &output));
    }

    let summary = if all {
        "Staged all changes".to_string()
    } else {
        format!("Staged {} path(s)", files.len())
    };

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: summary.clone(),
        }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "all": all,
            "files": files,
            "summary": summary,
        }),
        is_error: false,
    })
}

async fn commit_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let Some(message) = params["message"].as_str() else {
        return Ok(ToolOutput::error("Missing required parameter: message"));
    };
    if message.trim().is_empty() {
        return Ok(ToolOutput::error("Commit message cannot be empty"));
    }

    let allow_empty = params["allowEmpty"].as_bool().unwrap_or(false);
    let mut args = vec!["commit".to_string(), "-m".to_string(), message.to_string()];
    if allow_empty {
        args.push("--allow-empty".to_string());
    }

    let output = run_git_owned(cwd, args).await?;
    if !output.status.success() {
        return Ok(git_failure("git commit failed", &output));
    }

    let head = head_sha_short(cwd)
        .await
        .unwrap_or_else(|| "unknown".to_string());
    let stdout = stdout_trimmed(&output);
    let text = if stdout.is_empty() {
        format!("Committed {head}: {message}")
    } else {
        stdout
    };

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text: text.clone() }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "message": message,
            "allow_empty": allow_empty,
            "head": head,
            "summary": text,
        }),
        is_error: false,
    })
}

async fn restore_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
    ctx: &ToolContext,
) -> Result<ToolOutput> {
    let files = parse_string_array(params, "files")?;
    if files.is_empty() {
        return Ok(ToolOutput::error("restore requires files[]"));
    }

    let snapshot_paths: Vec<PathBuf> = files.iter().map(|file| resolve_path(cwd, file)).collect();
    let checkpoint = ctx.checkpoint_state.snapshot_paths(
        &snapshot_paths,
        Some(format!("git restore in {}", cwd.display())),
    )?;

    let mut args = vec!["restore".to_string()];
    if let Some(source) = params["source"].as_str() {
        if !source.trim().is_empty() {
            args.push(format!("--source={source}"));
        }
    }
    args.push("--".to_string());
    args.extend(files.iter().cloned());

    let output = run_git_owned(cwd, args).await?;
    if !output.status.success() {
        return Ok(git_failure("git restore failed", &output));
    }

    let summary = format!("Restored {} path(s)", files.len());
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: summary.clone(),
        }],
        details: json!({
            "repo_root": repo_root.display().to_string(),
            "files": files,
            "checkpoint_id": checkpoint.as_ref().map(|c| c.id.clone()),
            "checkpoint_label": checkpoint.as_ref().and_then(|c| c.label.clone()),
            "summary": summary,
        }),
        is_error: false,
    })
}

async fn worktree_add_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let Some(raw_worktree_path) = params["worktreePath"].as_str() else {
        return Ok(ToolOutput::error(
            "Missing required parameter: worktreePath",
        ));
    };
    let Some(branch) = params["branch"].as_str() else {
        return Ok(ToolOutput::error("Missing required parameter: branch"));
    };
    if branch.trim().is_empty() {
        return Ok(ToolOutput::error("branch cannot be empty"));
    }

    let worktree_path = resolve_path(cwd, raw_worktree_path);
    let start_point = params["startPoint"].as_str().unwrap_or("HEAD");

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
            "repo_root": repo_root.display().to_string(),
            "worktree_path": worktree_path.display().to_string(),
            "branch": branch,
            "start_point": start_point,
            "summary": summary,
        }),
        is_error: false,
    })
}

async fn worktree_remove_action(
    cwd: &Path,
    repo_root: &Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let Some(raw_worktree_path) = params["worktreePath"].as_str() else {
        return Ok(ToolOutput::error(
            "Missing required parameter: worktreePath",
        ));
    };
    let worktree_path = resolve_path(cwd, raw_worktree_path);
    let force = params["force"].as_bool().unwrap_or(false);
    let delete_branch = params["deleteBranch"].as_bool().unwrap_or(false);

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
    let matched_branch = params["branch"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| {
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
            "repo_root": repo_root.display().to_string(),
            "worktree_path": worktree_path.display().to_string(),
            "force": force,
            "delete_branch": delete_branch,
            "branch": matched_branch,
            "branch_deleted": branch_deleted,
            "summary": summary,
        }),
        is_error: false,
    })
}

fn parse_string_array(
    params: &serde_json::Value,
    field_name: &str,
) -> std::result::Result<Vec<String>, crate::error::Error> {
    let Some(value) = params.get(field_name) else {
        return Ok(Vec::new());
    };
    let Some(items) = value.as_array() else {
        return Err(crate::error::Error::Tool(format!(
            "{field_name} must be an array of strings"
        )));
    };

    let mut result = Vec::with_capacity(items.len());
    for item in items {
        let Some(s) = item.as_str() else {
            return Err(crate::error::Error::Tool(format!(
                "{field_name} must contain only strings"
            )));
        };
        result.push(s.to_string());
    }
    Ok(result)
}

async fn head_sha_short(cwd: &Path) -> Option<String> {
    let output = run_git(cwd, ["rev-parse", "--short", "HEAD"]).await.ok()?;
    if !output.status.success() {
        return None;
    }
    let head = stdout_trimmed(&output);
    if head.is_empty() {
        None
    } else {
        Some(head)
    }
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
    ToolOutput::error(combined)
}

fn display_or_unknown(s: &str) -> &str {
    if s.trim().is_empty() {
        "unknown"
    } else {
        s
    }
}

fn truncate_for_display(text: &str) -> (String, String, Option<PathBuf>) {
    let truncated = truncate_head(text, DISPLAY_MAX_LINES, DISPLAY_MAX_BYTES);
    let content = truncated.content.trim_end().to_string();
    let note = if truncated.truncated {
        let base = format!(
            "[output truncated: showing {}/{} lines, {}/{} bytes]",
            truncated.output_lines,
            truncated.total_lines,
            truncated.output_bytes,
            truncated.total_bytes,
        );
        match &truncated.temp_file {
            Some(path) => format!("{base} full output: {}", path.display()),
            None => base,
        }
    } else {
        String::new()
    };
    (content, note, truncated.temp_file)
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
            lua_tool_loader: None,
            mode,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
            config: Arc::new(crate::config::Config::default()),
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

    #[tokio::test]
    async fn git_status_reports_clean_repo() {
        let dir = setup_repo();
        let tool = GitTool;
        let result = tool
            .execute(
                "c1",
                json!({"action": "status"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("state: clean"));
        assert_eq!(result.details["clean"], json!(true));
    }

    #[tokio::test]
    async fn git_diff_ignores_empty_ref_fields() {
        let dir = setup_repo();
        let tool = GitTool;

        let result = tool
            .execute(
                "c-diff",
                json!({"action": "diff", "base": "", "head": ""}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(extract_text(&result), "No diff.");
        assert_eq!(result.details["base"], json!(null));
        assert_eq!(result.details["head"], json!(null));
    }

    #[tokio::test]
    async fn git_stage_and_commit_work() {
        let dir = setup_repo();
        fs::write(dir.path().join("note.txt"), "hello world\n").unwrap();
        let tool = GitTool;

        let stage = tool
            .execute(
                "c-stage",
                json!({"action": "stage", "files": ["note.txt"]}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!stage.is_error);

        let commit = tool
            .execute(
                "c-commit",
                json!({"action": "commit", "message": "update note"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!commit.is_error);
        assert!(extract_text(&commit).contains("update note"));

        let status = tool
            .execute(
                "c-status",
                json!({"action": "status"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!status.is_error);
        assert_eq!(status.details["clean"], json!(true));
    }

    #[tokio::test]
    async fn git_restore_reverts_file_and_creates_checkpoint() {
        let dir = setup_repo();
        fs::write(dir.path().join("note.txt"), "changed\n").unwrap();
        let tool = GitTool;
        let ctx = test_ctx(dir.path(), AgentMode::Worker);
        let checkpoint_state = ctx.checkpoint_state.clone();

        let result = tool
            .execute(
                "c-restore",
                json!({"action": "restore", "files": ["note.txt"]}),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            fs::read_to_string(dir.path().join("note.txt")).unwrap(),
            "hello\n"
        );
        assert_eq!(checkpoint_state.checkpoints().len(), 1);
        assert!(result.details["checkpoint_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn git_worktree_add_and_remove_work() {
        let dir = setup_repo();
        let tool = GitTool;
        let worktree_path = dir.path().join("../repo-worktree");
        let worktree_path_str = worktree_path.display().to_string();

        let add = tool
            .execute(
                "c-add",
                json!({
                    "action": "worktree_add",
                    "worktreePath": worktree_path_str,
                    "branch": "feature/test",
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!add.is_error);
        assert!(worktree_path.exists());

        let info = tool
            .execute(
                "c-info",
                json!({"action": "worktree_info"}),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!info.is_error);
        assert!(info.details["worktrees"].as_array().unwrap().len() >= 2);

        let remove = tool
            .execute(
                "c-remove",
                json!({
                    "action": "worktree_remove",
                    "worktreePath": worktree_path.display().to_string(),
                    "deleteBranch": true,
                }),
                test_ctx(dir.path(), AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(!remove.is_error);
        assert!(!worktree_path.exists());
    }

    #[tokio::test]
    async fn planner_mode_blocks_mutating_git_actions() {
        let dir = setup_repo();
        let tool = GitTool;
        fs::write(dir.path().join("note.txt"), "changed\n").unwrap();

        let result = tool
            .execute(
                "c-stage",
                json!({"action": "stage", "files": ["note.txt"]}),
                test_ctx(dir.path(), AgentMode::Planner),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(extract_text(&result).contains("not permitted"));
    }

    #[tokio::test]
    async fn planner_mode_allows_readonly_git_actions() {
        let dir = setup_repo();
        let tool = GitTool;

        let result = tool
            .execute(
                "c-status",
                json!({"action": "status"}),
                test_ctx(dir.path(), AgentMode::Planner),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(extract_text(&result).contains("repo:"));
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
