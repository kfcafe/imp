use std::path::Path;
use std::process::Command;

pub(super) fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(format!("git {} failed: {detail}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(super) fn compact_git_label(cwd: &Path) -> Option<String> {
    let branch = run_git(cwd, &["branch", "--show-current"]).ok()?;
    let branch = if branch.trim().is_empty() {
        run_git(cwd, &["rev-parse", "--short", "HEAD"]).ok()?
    } else {
        branch
    };
    let status = run_git(cwd, &["status", "--short"]).unwrap_or_default();
    let dirty = status.lines().count();
    let mut label = if dirty == 0 {
        format!("git {branch}")
    } else {
        format!("git {branch} ±{dirty}")
    };
    if let Ok(counts) = run_git(cwd, &["rev-list", "--left-right", "--count", "HEAD...@{u}"]) {
        let mut parts = counts.split_whitespace();
        if let (Some(ahead), Some(behind)) = (parts.next(), parts.next()) {
            if ahead != "0" || behind != "0" {
                label.push_str(&format!(" ↑{ahead}↓{behind}"));
            }
        }
    }
    Some(label)
}
