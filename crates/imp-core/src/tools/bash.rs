use std::process::Stdio;
use std::sync::OnceLock;

use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::{
    truncate_head, truncate_tail, Tool, ToolContext, ToolOutput, ToolUpdate, TruncationResult,
};
use crate::error::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;

/// Check whether the rush backend should be used.
///
/// Returns true when the `rush-backend` feature is compiled in AND the env var
/// `IMP_SHELL_BACKEND` is either unset or set to `"rush"`. Setting
/// `IMP_SHELL_BACKEND=sh` forces the traditional `sh -c` path even when rush
/// is available.
#[cfg(feature = "rush-backend")]
fn use_rush_backend() -> bool {
    match std::env::var("IMP_SHELL_BACKEND") {
        Ok(val) => val.eq_ignore_ascii_case("rush"),
        // Feature compiled in → rush is the default.
        Err(_) => true,
    }
}

/// Execute a command via rush's in-process library API. Returns `None` if rush
/// fails so the caller can fall back to `sh`.
#[cfg(feature = "rush-backend")]
fn run_via_rush(
    command: &str,
    timeout_secs: u64,
    cwd: &std::path::Path,
    json_output: bool,
) -> Option<(String, i32, bool, bool)> {
    let result = rush::run(
        command,
        &rush::RunOptions {
            cwd: Some(cwd.to_path_buf()),
            timeout: Some(timeout_secs),
            json_output,
            max_output_bytes: Some(MAX_OUTPUT_BYTES),
            ..Default::default()
        },
    );

    match result {
        Ok(r) => {
            let mut output = r.stdout;
            if !r.stderr.is_empty() {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&r.stderr);
            }
            Some((output, r.exit_code, r.timed_out, r.truncated))
        }
        Err(_) => None,
    }
}

/// Detect which shell to use for command execution.
/// Prefers rush if available on PATH, falls back to sh.
fn detect_shell() -> String {
    // IMP_SHELL overrides everything (also used by tests to force sh)
    if let Ok(shell) = std::env::var("IMP_SHELL") {
        return shell;
    }
    // Prefer rush — dogfood it as the default shell backend.
    // Cached after first PATH lookup.
    use std::sync::OnceLock;
    static RUSH_PATH: OnceLock<Option<String>> = OnceLock::new();
    if let Some(path) = RUSH_PATH.get_or_init(|| {
        std::process::Command::new("which")
            .arg("rush")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty())
    }) {
        return path.clone();
    }
    "sh".to_string()
}

fn sanitize_output_text(text: &str) -> String {
    static ANSI_RE: OnceLock<Regex> = OnceLock::new();
    let re = ANSI_RE.get_or_init(|| {
        Regex::new(r"\x1B\[[0-9;?]*[ -/]*[@-~]").expect("valid ansi regex")
    });
    re.replace_all(text, "").replace('\r', "")
}

fn looks_like_search_command(command: &str) -> bool {
    let trimmed = command.trim_start();
    trimmed.starts_with("grep ")
        || trimmed.starts_with("grep\n")
        || trimmed.starts_with("find ")
        || trimmed == "find"
        || trimmed.starts_with("ls ")
        || trimmed == "ls"
}

#[cfg(feature = "rush-backend")]
fn should_try_rush_json(command: &str) -> bool {
    if command.contains("|")
        || command.contains("&&")
        || command.contains("||")
        || command.contains(';')
        || command.contains('>')
        || command.contains('<')
    {
        return false;
    }

    looks_like_search_command(command)
}

#[cfg(feature = "rush-backend")]
fn parse_json_lines_to_text(command: &str, output: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(output).ok()?;
    let items = value.as_array()?;

    let mut lines = Vec::new();
    let is_grep = command.trim_start().starts_with("grep");
    let is_find = command.trim_start().starts_with("find");
    let is_ls = command.trim_start().starts_with("ls");

    for item in items {
        if is_grep {
            let file = item.get("file").and_then(|v| v.as_str()).unwrap_or("");
            let line = item.get("line_number").and_then(|v| v.as_u64()).unwrap_or(0);
            let full_line = item
                .get("full_line")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_end_matches('\n');
            if !file.is_empty() && line > 0 {
                lines.push(format!("{file}:{line}:{full_line}"));
            }
        } else if is_find {
            if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                lines.push(path.to_string());
            }
        } else if is_ls {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                let suffix = match item.get("type").and_then(|v| v.as_str()) {
                    Some("directory") => "/",
                    Some("symlink") => "@",
                    _ => "",
                };
                lines.push(format!("{name}{suffix}"));
            }
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn truncate_command_output(command: &str, output: &str) -> TruncationResult {
    if looks_like_search_command(command) {
        truncate_head(output, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES)
    } else {
        truncate_tail(output, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES)
    }
}

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn label(&self) -> &str {
        "Bash"
    }
    fn description(&self) -> &str {
        "Execute a shell command in the workspace or an optional working directory. Use it for search, file discovery, builds, tests, git, scripts, and other shell-native tasks."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute, such as grep, find, ls, git, cargo, python, or project scripts" },
                "timeout": { "type": "number", "description": "Optional timeout in seconds" },
                "workdir": { "type": "string", "description": "Optional working directory for this command; defaults to the session cwd" }
            },
            "required": ["command"]
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
        let command = params["command"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'command' parameter".into()))?;

        let timeout_secs = params["timeout"].as_u64().unwrap_or(DEFAULT_TIMEOUT_SECS);

        // Support per-command workdir override
        let ctx = if let Some(workdir) = params["workdir"].as_str() {
            let wd = super::resolve_path(&ctx.cwd, workdir);
            if !wd.is_dir() {
                return Ok(ToolOutput::error(format!(
                    "workdir not found or not a directory: {}",
                    wd.display()
                )));
            }
            ToolContext { cwd: wd, ..ctx }
        } else {
            ctx
        };

        run_command(command, timeout_secs, &ctx).await
    }
}

async fn run_command(command: &str, timeout_secs: u64, ctx: &ToolContext) -> Result<ToolOutput> {
    // Check cancellation before spawning.
    if ctx.is_cancelled() {
        return Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text {
                text: "[Command cancelled]".to_string(),
            }],
            details: json!({ "exit_code": -1, "timed_out": false, "cancelled": true, "truncated": false }),
            is_error: true,
        });
    }

    // Try the rush in-process backend when available.
    #[cfg(feature = "rush-backend")]
    if use_rush_backend() {
        let rush_json = should_try_rush_json(command);
        if let Some((output, exit_code, timed_out, truncated)) =
            run_via_rush(command, timeout_secs, &ctx.cwd, rush_json)
        {
            let transformed = if rush_json {
                parse_json_lines_to_text(command, &output).unwrap_or(output)
            } else {
                output
            };
            let sanitized = sanitize_output_text(&transformed);
            // Stream the output lines so callers see incremental progress.
            for line in sanitized.lines() {
                let _ = ctx
                    .update_tx
                    .send(ToolUpdate {
                        content: vec![imp_llm::ContentBlock::Text {
                            text: line.to_string(),
                        }],
                        details: serde_json::Value::Null,
                    })
                    .await;
            }

            let mut result_text = sanitized;
            if timed_out {
                result_text.push_str(&format!("\n[Command timed out after {timeout_secs}s]"));
            }

            return Ok(ToolOutput {
                content: vec![imp_llm::ContentBlock::Text { text: result_text }],
                details: json!({
                    "exit_code": exit_code,
                    "timed_out": timed_out,
                    "cancelled": false,
                    "truncated": truncated,
                    "backend": "rush",
                }),
                is_error: exit_code != 0,
            });
        }
        // rush failed — fall through to sh.
    }

    let mut child = {
        // Use rush if available and configured, otherwise sh
        let shell = detect_shell();
        let mut cmd = Command::new(&shell);
        cmd.arg("-c")
            .arg(command)
            .current_dir(&ctx.cwd)
            // Tool commands are non-interactive. Keep stdin disconnected so
            // subprocesses cannot consume raw terminal input (for example SGR
            // mouse reporting sequences) from the interactive TUI.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Create a new process group so we can kill the entire tree.
        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        cmd.spawn()
            .map_err(|e| crate::error::Error::Tool(format!("failed to spawn command: {e}")))?
    };

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Merge stdout and stderr into a single stream.
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut output = String::new();
    let mut timed_out = false;
    let mut stdout_done = false;
    let mut stderr_done = false;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    while !stdout_done || !stderr_done {
        tokio::select! {
            biased;

            _ = tokio::time::sleep_until(deadline) => {
                timed_out = true;
                kill_process_group(&child).await;
                break;
            }

            _ = wait_for_cancellation(&ctx.cancelled), if !ctx.is_cancelled() => {
                kill_process_group(&child).await;
                break;
            }

            line = stdout_reader.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(line)) => {
                        if !line.bytes().any(|b| b == 0) {
                            let clean = sanitize_output_text(&line);
                            if !clean.is_empty() {
                                append_line(&mut output, &clean, &ctx.update_tx).await;
                            }
                        }
                    }
                    _ => { stdout_done = true; }
                }
            }

            line = stderr_reader.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(line)) => {
                        if !line.bytes().any(|b| b == 0) {
                            let clean = sanitize_output_text(&line);
                            if !clean.is_empty() {
                                append_line(&mut output, &clean, &ctx.update_tx).await;
                            }
                        }
                    }
                    _ => { stderr_done = true; }
                }
            }
        }
    }

    // Wait for child with a timeout — don't hang if process won't exit
    let status = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
        .await
        .ok()
        .and_then(|r| r.ok());
    let exit_code = status.and_then(|s| s.code()).unwrap_or(-1);

    // Truncate from the tail (end matters more for command output).
    let TruncationResult {
        content: truncated_output,
        truncated,
        output_lines,
        total_lines,
        temp_file,
        ..
    } = truncate_command_output(command, &output);

    let mut result_text = truncated_output;

    if truncated {
        let note = if looks_like_search_command(command) {
            format!(
                "\n[Output truncated: showing first {output_lines} of {total_lines} lines{}]",
                temp_file
                    .as_ref()
                    .map(|p| format!(". Full output saved to {}", p.display()))
                    .unwrap_or_default()
            )
        } else {
            format!(
                "\n[Output truncated: showing last {output_lines} of {total_lines} lines{}]",
                temp_file
                    .as_ref()
                    .map(|p| format!(". Full output saved to {}", p.display()))
                    .unwrap_or_default()
            )
        };
        result_text.push_str(&note);
    }

    if timed_out {
        result_text.push_str(&format!("\n[Command timed out after {timeout_secs}s]"));
    }

    let cancelled = ctx.is_cancelled();
    let details = json!({
        "exit_code": exit_code,
        "timed_out": timed_out,
        "cancelled": cancelled,
        "truncated": truncated,
        "command": command,
    });

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text: result_text }],
        details,
        is_error: cancelled || exit_code != 0,
    })
}

async fn wait_for_cancellation(cancelled: &std::sync::atomic::AtomicBool) {
    while !cancelled.load(std::sync::atomic::Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

async fn append_line(
    output: &mut String,
    line: &str,
    update_tx: &tokio::sync::mpsc::Sender<ToolUpdate>,
) {
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(line);
    let _ = update_tx
        .send(ToolUpdate {
            content: vec![imp_llm::ContentBlock::Text {
                text: line.to_string(),
            }],
            details: serde_json::Value::Null,
        })
        .await;
}

/// Kill the entire process group. Sends SIGTERM, waits briefly, then SIGKILL.
#[cfg(unix)]
async fn kill_process_group(child: &tokio::process::Child) {
    if let Some(pid) = child.id() {
        let pgid = pid as i32;

        // SIGTERM the group
        unsafe {
            libc::kill(-pgid, libc::SIGTERM);
        }

        // Brief wait, then force-kill
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        unsafe {
            libc::kill(-pgid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
async fn kill_process_group(_child: &tokio::process::Child) {
    // Best-effort on non-Unix — nothing we can do portably.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::NullInterface;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    // Tests use sh for deterministic behavior (rush has exit code bugs: rush#8)
    fn ensure_sh() {
        std::env::set_var("IMP_SHELL", "sh");
    }

    fn test_ctx(dir: &std::path::Path) -> (ToolContext, tokio::sync::mpsc::Receiver<ToolUpdate>) {
        ensure_sh();
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        let ctx = ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
        };
        (ctx, rx)
    }

    #[tokio::test]
    async fn bash_simple_command() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command("echo hello world", DEFAULT_TIMEOUT_SECS, &ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("hello world"));
        assert_eq!(result.details["exit_code"], 0);
    }

    #[tokio::test]
    async fn bash_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command("exit 42", DEFAULT_TIMEOUT_SECS, &ctx)
            .await
            .unwrap();

        assert!(result.is_error);
        assert_eq!(result.details["exit_code"], 42);
    }

    #[tokio::test]
    async fn bash_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command("sleep 60", 1, &ctx).await.unwrap();

        assert!(result.details["timed_out"].as_bool().unwrap());
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("timed out"));
    }

    #[tokio::test]
    async fn bash_cancellation() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        // Set cancelled before running — should return immediately.
        ctx.cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let result = run_command("sleep 60", DEFAULT_TIMEOUT_SECS, &ctx)
            .await
            .unwrap();

        assert!(result.details["cancelled"].as_bool().unwrap());
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("cancelled"));
    }

    #[tokio::test]
    async fn bash_cancellation_during_execution() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());
        let cancelled = Arc::clone(&ctx.cancelled);

        let task = tokio::spawn(async move { run_command("sleep 60", DEFAULT_TIMEOUT_SECS, &ctx).await });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        cancelled.store(true, std::sync::atomic::Ordering::Relaxed);

        let result = task.await.unwrap().unwrap();
        assert!(result.details["cancelled"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn bash_streaming_output() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, mut rx) = test_ctx(tmp.path());

        let handle = tokio::spawn(async move {
            run_command(
                "echo line1; echo line2; echo line3",
                DEFAULT_TIMEOUT_SECS,
                &ctx,
            )
            .await
        });

        // Collect streamed updates
        let mut updates = Vec::new();
        while let Some(update) = rx.recv().await {
            updates.push(update);
        }

        let result = handle.await.unwrap().unwrap();
        assert!(!result.is_error);
        assert!(
            !updates.is_empty(),
            "should have received streaming updates"
        );
    }

    #[tokio::test]
    async fn bash_stdout_and_stderr_merged() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command(
            "echo stdout_line; echo stderr_line >&2",
            DEFAULT_TIMEOUT_SECS,
            &ctx,
        )
        .await
        .unwrap();

        // exit code 0 → not an error
        assert!(!result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("stdout_line"));
        assert!(text.contains("stderr_line"));
    }

    #[tokio::test]
    async fn bash_writes_file_side_effect() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command(
            "echo 'side effect content' > side_effect.txt",
            DEFAULT_TIMEOUT_SECS,
            &ctx,
        )
        .await
        .unwrap();

        assert!(!result.is_error);
        let written = std::fs::read_to_string(tmp.path().join("side_effect.txt")).unwrap();
        assert!(written.contains("side effect content"));
    }

    #[tokio::test]
    async fn bash_uses_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("testfile.txt"), "content").unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command("ls testfile.txt", DEFAULT_TIMEOUT_SECS, &ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("testfile.txt"));
    }

    #[tokio::test]
    async fn bash_strips_ansi_sequences() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());

        let result = run_command("printf '\\033[1;31mred\\033[0m\\n'", DEFAULT_TIMEOUT_SECS, &ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("red"));
        assert!(!text.contains("\u{1b}[1;31m"));
        assert!(!text.contains("\u{1b}[0m"));
    }

    #[tokio::test]
    async fn bash_workdir_override_executes_in_target_dir() {
        let root = tempfile::tempdir().unwrap();
        let subdir = root.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("inside.txt"), "ok").unwrap();
        let tool = BashTool;
        let (ctx, _rx) = test_ctx(root.path());

        let result = tool
            .execute(
                "c-workdir",
                serde_json::json!({"command": "ls inside.txt", "workdir": "subdir"}),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("inside.txt"));
    }

    #[tokio::test]
    async fn bash_invalid_workdir_returns_error() {
        let root = tempfile::tempdir().unwrap();
        let tool = BashTool;
        let (ctx, _rx) = test_ctx(root.path());

        let result = tool
            .execute(
                "c-bad-workdir",
                serde_json::json!({"command": "pwd", "workdir": "missing-dir"}),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = match &result.content[0] {
            imp_llm::ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        };
        assert!(text.contains("workdir not found"));
    }


    // ── rush backend tests ──────────────────────────────────────────
    //
    // Call run_via_rush directly to avoid env-var races between
    // parallel test threads.

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_echo() {
        let tmp = tempfile::tempdir().unwrap();
        let (output, exit_code, timed_out, _truncated) =
            run_via_rush("echo hello world", DEFAULT_TIMEOUT_SECS, tmp.path(), false)
                .expect("rush should succeed");

        assert_eq!(exit_code, 0);
        assert!(!timed_out);
        assert!(output.contains("hello world"), "stdout missing: {output}");
    }

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_builtin() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("afile.txt"), "content").unwrap();

        let (output, exit_code, _, _) =
            run_via_rush("ls", DEFAULT_TIMEOUT_SECS, tmp.path(), false).expect("rush should succeed");

        assert_eq!(exit_code, 0);
        assert!(
            output.contains("afile.txt"),
            "ls should list file: {output}"
        );
    }

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_ls_json_text_transform() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("afile.txt"), "content").unwrap();

        let (output, exit_code, _, _) =
            run_via_rush("ls", DEFAULT_TIMEOUT_SECS, tmp.path(), true).expect("rush should succeed");
        let text = parse_json_lines_to_text("ls", &output).expect("json should transform");

        assert_eq!(exit_code, 0);
        assert!(text.contains("afile.txt"));
    }

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_grep_json_text_transform() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("afile.txt"), "hello needle world\n").unwrap();

        let (output, exit_code, _, _) = run_via_rush(
            "grep -r needle .",
            DEFAULT_TIMEOUT_SECS,
            tmp.path(),
            true,
        )
        .expect("rush should succeed");
        let text = parse_json_lines_to_text("grep -r needle .", &output).expect("json should transform");

        assert_eq!(exit_code, 0);
        assert!(text.contains("needle"));
        assert!(text.contains("afile.txt") || text.contains(":1:"), "unexpected grep text: {text}");
    }

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_find_json_text_transform() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("afile.txt"), "content").unwrap();

        let (output, exit_code, _, _) = run_via_rush(
            "find . -name afile.txt",
            DEFAULT_TIMEOUT_SECS,
            tmp.path(),
            true,
        )
        .expect("rush should succeed");
        let text = parse_json_lines_to_text("find . -name afile.txt", &output).expect("json should transform");

        assert_eq!(exit_code, 0);
        assert!(text.contains("afile.txt"));
    }

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_pipeline() {
        let tmp = tempfile::tempdir().unwrap();

        let (output, exit_code, _, _) =
            run_via_rush("echo foo | cat", DEFAULT_TIMEOUT_SECS, tmp.path(), false)
                .expect("rush should succeed");

        assert_eq!(exit_code, 0);
        assert!(output.contains("foo"), "pipeline output missing: {output}");
    }

    #[test]
    #[cfg(feature = "rush-backend")]
    fn test_rush_backend_exit_code() {
        let tmp = tempfile::tempdir().unwrap();

        let (_, exit_code, _, _) = run_via_rush("exit 42", DEFAULT_TIMEOUT_SECS, tmp.path(), false)
            .expect("rush should return result even on non-zero exit");

        assert_eq!(exit_code, 42);
    }
}
