use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::{Error, Result};
use crate::workflow::WorkspaceScope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorktreeRunSpec {
    pub workflow_id: Option<String>,
    pub run_id: String,
    pub slug: String,
    pub start_point: String,
    pub worktree_root: Option<PathBuf>,
    pub allow_dirty_main: bool,
}

impl WorktreeRunSpec {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            ..Self::default()
        }
    }
}

impl Default for WorktreeRunSpec {
    fn default() -> Self {
        Self {
            workflow_id: None,
            run_id: "run".into(),
            slug: "worktree-auto".into(),
            start_point: "HEAD".into(),
            worktree_root: None,
            allow_dirty_main: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorktreeRunPlan {
    pub repo_root: PathBuf,
    pub main_worktree: PathBuf,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub start_point: String,
    pub run_id: String,
    pub workflow_id: Option<String>,
    pub already_in_worktree: bool,
}

impl WorktreeRunPlan {
    pub fn workspace_scope(&self) -> WorkspaceScope {
        WorkspaceScope::Worktree {
            path: self.worktree_path.clone(),
            branch: Some(self.branch.clone()),
        }
    }
}

impl Default for WorktreeRunPlan {
    fn default() -> Self {
        Self {
            repo_root: PathBuf::new(),
            main_worktree: PathBuf::new(),
            worktree_path: PathBuf::new(),
            branch: String::new(),
            start_point: "HEAD".into(),
            run_id: String::new(),
            workflow_id: None,
            already_in_worktree: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorktreeRunMetadata {
    pub repo_root: PathBuf,
    pub main_worktree: PathBuf,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub start_point: String,
    pub run_id: String,
    pub workflow_id: Option<String>,
    pub status_path: PathBuf,
    pub stat_path: PathBuf,
    pub patch_path: PathBuf,
    pub clean: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorktreeDiffArtifacts {
    pub status_path: PathBuf,
    pub stat_path: PathBuf,
    pub patch_path: PathBuf,
    pub clean: bool,
}

impl Default for WorktreeDiffArtifacts {
    fn default() -> Self {
        Self {
            status_path: PathBuf::new(),
            stat_path: PathBuf::new(),
            patch_path: PathBuf::new(),
            clean: true,
        }
    }
}

impl From<&WorktreeRunPlan> for WorktreeRunMetadata {
    fn from(plan: &WorktreeRunPlan) -> Self {
        Self {
            repo_root: plan.repo_root.clone(),
            main_worktree: plan.main_worktree.clone(),
            worktree_path: plan.worktree_path.clone(),
            branch: plan.branch.clone(),
            start_point: plan.start_point.clone(),
            run_id: plan.run_id.clone(),
            workflow_id: plan.workflow_id.clone(),
            status_path: PathBuf::new(),
            stat_path: PathBuf::new(),
            patch_path: PathBuf::new(),
            clean: true,
        }
    }
}

impl Default for WorktreeRunMetadata {
    fn default() -> Self {
        Self {
            repo_root: PathBuf::new(),
            main_worktree: PathBuf::new(),
            worktree_path: PathBuf::new(),
            branch: String::new(),
            start_point: "HEAD".into(),
            run_id: String::new(),
            workflow_id: None,
            status_path: PathBuf::new(),
            stat_path: PathBuf::new(),
            patch_path: PathBuf::new(),
            clean: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeCloseoutAction {
    Keep,
    Discard,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorktreeCloseoutResult {
    pub action: WorktreeCloseoutAction,
    pub applied: bool,
    pub removed: bool,
    pub branch_deleted: bool,
    pub clean: bool,
    pub message: String,
}

impl Default for WorktreeCloseoutResult {
    fn default() -> Self {
        Self {
            action: WorktreeCloseoutAction::Keep,
            applied: false,
            removed: false,
            branch_deleted: false,
            clean: true,
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeRunError {
    NotGitRepo(String),
    DirtyMainWorkspace(String),
    BranchExists(String),
    WorktreePathExists(PathBuf),
    GitFailed(String),
    InvalidInput(String),
    Io(String),
}

impl std::fmt::Display for WorktreeRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotGitRepo(message)
            | Self::DirtyMainWorkspace(message)
            | Self::GitFailed(message)
            | Self::InvalidInput(message)
            | Self::Io(message) => write!(f, "{message}"),
            Self::BranchExists(branch) => write!(f, "worktree branch already exists: {branch}"),
            Self::WorktreePathExists(path) => {
                write!(f, "worktree path already exists: {}", path.display())
            }
        }
    }
}

impl std::error::Error for WorktreeRunError {}

pub type WorktreeRunResult<T> = std::result::Result<T, WorktreeRunError>;

pub async fn plan_worktree_run(
    cwd: &Path,
    spec: &WorktreeRunSpec,
) -> WorktreeRunResult<WorktreeRunPlan> {
    let repo_root = repo_root(cwd).await?;
    let main_worktree = main_worktree(cwd)
        .await?
        .unwrap_or_else(|| repo_root.clone());
    let already_in_worktree = current_is_secondary_worktree(cwd).await?;

    if !spec.allow_dirty_main {
        let dirty = git_status_short(&main_worktree).await?;
        if !dirty.trim().is_empty() {
            return Err(WorktreeRunError::DirtyMainWorkspace(format!(
                "worktree-auto blocked: main workspace has uncommitted changes. Commit/stash or use local-auto.\n{}",
                dirty.trim()
            )));
        }
    }

    let branch = branch_name(spec);
    if branch_exists(&repo_root, &branch).await? {
        return Err(WorktreeRunError::BranchExists(branch));
    }

    let root = spec.worktree_root.clone().unwrap_or_else(|| {
        main_worktree
            .parent()
            .unwrap_or(&main_worktree)
            .join(".imp-worktrees")
    });
    let worktree_path = root.join(safe_segment(&format!("{}-{}", spec.run_id, spec.slug)));
    if worktree_path.exists() {
        return Err(WorktreeRunError::WorktreePathExists(worktree_path));
    }

    Ok(WorktreeRunPlan {
        repo_root,
        main_worktree,
        worktree_path,
        branch,
        start_point: spec.start_point.clone(),
        run_id: spec.run_id.clone(),
        workflow_id: spec.workflow_id.clone(),
        already_in_worktree,
    })
}

pub async fn create_worktree_run(plan: &WorktreeRunPlan) -> WorktreeRunResult<WorktreeRunMetadata> {
    if let Some(parent) = plan.worktree_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            WorktreeRunError::Io(format!(
                "failed to create worktree root {}: {err}",
                parent.display()
            ))
        })?;
    }
    let output = run_git_owned(
        &plan.repo_root,
        vec![
            "worktree".into(),
            "add".into(),
            "-b".into(),
            plan.branch.clone(),
            plan.worktree_path.display().to_string(),
            plan.start_point.clone(),
        ],
    )
    .await?;
    if !output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git worktree add failed: {}",
            stderr_trimmed(&output)
        )));
    }
    Ok(WorktreeRunMetadata::from(plan))
}

pub async fn remove_worktree_run(
    metadata: &WorktreeRunMetadata,
    delete_branch: bool,
    force: bool,
) -> WorktreeRunResult<()> {
    if same_path(&metadata.worktree_path, &metadata.main_worktree) {
        return Err(WorktreeRunError::InvalidInput(
            "refusing to remove main worktree".into(),
        ));
    }

    let mut args = vec!["worktree".to_string(), "remove".to_string()];
    if force {
        args.push("--force".into());
    }
    args.push(metadata.worktree_path.display().to_string());
    let output = run_git_owned(&metadata.repo_root, args).await?;
    if !output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git worktree remove failed: {}",
            stderr_trimmed(&output)
        )));
    }

    if delete_branch {
        let output = run_git_owned(
            &metadata.repo_root,
            vec![
                "branch".into(),
                if force { "-D" } else { "-d" }.into(),
                metadata.branch.clone(),
            ],
        )
        .await?;
        if !output.status.success() {
            return Err(WorktreeRunError::GitFailed(format!(
                "git branch delete failed: {}",
                stderr_trimmed(&output)
            )));
        }
    }
    Ok(())
}

pub async fn capture_worktree_diff_artifacts(
    metadata: &mut WorktreeRunMetadata,
    artifact_dir: &Path,
) -> WorktreeRunResult<WorktreeDiffArtifacts> {
    tokio::fs::create_dir_all(artifact_dir)
        .await
        .map_err(|err| {
            WorktreeRunError::Io(format!(
                "failed to create worktree artifact dir {}: {err}",
                artifact_dir.display()
            ))
        })?;

    let status_output = run_git(&metadata.worktree_path, ["status", "--short"]).await?;
    if !status_output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git status --short failed: {}",
            stderr_trimmed(&status_output)
        )));
    }
    let stat_output = run_git(&metadata.worktree_path, ["diff", "--stat"]).await?;
    if !stat_output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git diff --stat failed: {}",
            stderr_trimmed(&stat_output)
        )));
    }
    let patch_output = run_git(
        &metadata.worktree_path,
        ["diff", "--binary", "--no-ext-diff", "--"],
    )
    .await?;
    if !patch_output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git diff --binary failed: {}",
            stderr_trimmed(&patch_output)
        )));
    }

    let status_path = artifact_dir.join("status.txt");
    let stat_path = artifact_dir.join("diff.stat");
    let patch_path = artifact_dir.join("diff.patch");
    tokio::fs::write(&status_path, &status_output.stdout)
        .await
        .map_err(|err| {
            WorktreeRunError::Io(format!("failed to write {}: {err}", status_path.display()))
        })?;
    tokio::fs::write(&stat_path, &stat_output.stdout)
        .await
        .map_err(|err| {
            WorktreeRunError::Io(format!("failed to write {}: {err}", stat_path.display()))
        })?;
    tokio::fs::write(&patch_path, &patch_output.stdout)
        .await
        .map_err(|err| {
            WorktreeRunError::Io(format!("failed to write {}: {err}", patch_path.display()))
        })?;

    let artifacts = WorktreeDiffArtifacts {
        status_path,
        stat_path,
        patch_path,
        clean: status_output.stdout.is_empty(),
    };
    metadata.status_path = artifacts.status_path.clone();
    metadata.stat_path = artifacts.stat_path.clone();
    metadata.patch_path = artifacts.patch_path.clone();
    metadata.clean = artifacts.clean;
    Ok(artifacts)
}

pub async fn closeout_worktree_run(
    metadata: &WorktreeRunMetadata,
    action: WorktreeCloseoutAction,
) -> WorktreeRunResult<WorktreeCloseoutResult> {
    match action {
        WorktreeCloseoutAction::Keep => {
            let clean = git_status_short(&metadata.worktree_path)
                .await?
                .trim()
                .is_empty();
            Ok(WorktreeCloseoutResult {
                action,
                clean,
                message: format!(
                    "kept worktree {} on branch {}",
                    metadata.worktree_path.display(),
                    metadata.branch
                ),
                ..WorktreeCloseoutResult::default()
            })
        }
        WorktreeCloseoutAction::Discard => {
            remove_worktree_run(metadata, true, true).await?;
            Ok(WorktreeCloseoutResult {
                action,
                removed: true,
                branch_deleted: true,
                clean: true,
                message: format!(
                    "discarded worktree {} and deleted branch {}",
                    metadata.worktree_path.display(),
                    metadata.branch
                ),
                ..WorktreeCloseoutResult::default()
            })
        }
        WorktreeCloseoutAction::Apply => apply_worktree_run(metadata).await,
    }
}

pub async fn apply_worktree_run(
    metadata: &WorktreeRunMetadata,
) -> WorktreeRunResult<WorktreeCloseoutResult> {
    if same_path(&metadata.worktree_path, &metadata.main_worktree) {
        return Err(WorktreeRunError::InvalidInput(
            "refusing to apply a worktree to itself".into(),
        ));
    }

    let main_status = git_status_short(&metadata.main_worktree).await?;
    if !main_status.trim().is_empty() {
        return Err(WorktreeRunError::DirtyMainWorkspace(format!(
            "worktree apply blocked: main workspace has uncommitted changes. Commit/stash first.\n{}",
            main_status.trim()
        )));
    }

    let patch_output = run_git(
        &metadata.worktree_path,
        ["diff", "--binary", "--no-ext-diff", "--"],
    )
    .await?;
    if !patch_output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git diff --binary failed: {}",
            stderr_trimmed(&patch_output)
        )));
    }
    if patch_output.stdout.is_empty() {
        return Ok(WorktreeCloseoutResult {
            action: WorktreeCloseoutAction::Apply,
            clean: true,
            message: "no worktree changes to apply".into(),
            ..WorktreeCloseoutResult::default()
        });
    }

    let check = run_git_with_stdin(
        &metadata.main_worktree,
        ["apply", "--check", "--binary", "-"],
        &patch_output.stdout,
    )
    .await?;
    if !check.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "worktree apply would conflict: {}",
            stderr_trimmed(&check)
        )));
    }

    let apply = run_git_with_stdin(
        &metadata.main_worktree,
        ["apply", "--binary", "-"],
        &patch_output.stdout,
    )
    .await?;
    if !apply.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git apply failed: {}",
            stderr_trimmed(&apply)
        )));
    }

    Ok(WorktreeCloseoutResult {
        action: WorktreeCloseoutAction::Apply,
        applied: true,
        clean: false,
        message: format!(
            "applied worktree changes from {} to {}",
            metadata.worktree_path.display(),
            metadata.main_worktree.display()
        ),
        ..WorktreeCloseoutResult::default()
    })
}

pub async fn write_worktree_metadata(path: &Path, metadata: &WorktreeRunMetadata) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(Error::Io)?;
    }
    let bytes = serde_json::to_vec_pretty(metadata).map_err(Error::Json)?;
    tokio::fs::write(path, bytes).await.map_err(Error::Io)
}

fn branch_name(spec: &WorktreeRunSpec) -> String {
    let id = spec.workflow_id.as_deref().unwrap_or(&spec.run_id);
    format!("imp/{}/{}", safe_segment(id), safe_segment(&spec.slug))
}

fn safe_segment(input: &str) -> String {
    let segment: String = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if segment.is_empty() {
        "run".into()
    } else {
        segment
    }
}

async fn repo_root(cwd: &Path) -> WorktreeRunResult<PathBuf> {
    let output = run_git(cwd, ["rev-parse", "--show-toplevel"]).await?;
    if !output.status.success() {
        return Err(WorktreeRunError::NotGitRepo(format!(
            "not inside a git repository: {}\n{}",
            cwd.display(),
            stderr_trimmed(&output)
        )));
    }
    Ok(PathBuf::from(stdout_trimmed(&output)))
}

async fn current_is_secondary_worktree(cwd: &Path) -> WorktreeRunResult<bool> {
    let output = run_git(cwd, ["worktree", "list", "--porcelain"]).await?;
    if !output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git worktree list failed: {}",
            stderr_trimmed(&output)
        )));
    }

    let current = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let mut first_worktree: Option<PathBuf> = None;
    for line in output.stdout.split(|byte| *byte == b'\n') {
        let line = String::from_utf8_lossy(line);
        let Some(path) = line.strip_prefix("worktree ") else {
            continue;
        };
        let path = PathBuf::from(path);
        if first_worktree.is_none() {
            first_worktree = Some(path.clone());
        }
        if same_path(&path, &current) {
            return Ok(first_worktree.as_ref() != Some(&path));
        }
    }
    Ok(false)
}
async fn main_worktree(cwd: &Path) -> WorktreeRunResult<Option<PathBuf>> {
    let output = run_git(cwd, ["worktree", "list", "--porcelain"]).await?;
    if !output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git worktree list failed: {}",
            stderr_trimmed(&output)
        )));
    }
    Ok(output.stdout.split(|byte| *byte == b'\n').find_map(|line| {
        let line = String::from_utf8_lossy(line);
        line.strip_prefix("worktree ").map(PathBuf::from)
    }))
}

async fn git_status_short(cwd: &Path) -> WorktreeRunResult<String> {
    let output = run_git(cwd, ["status", "--short"]).await?;
    if !output.status.success() {
        return Err(WorktreeRunError::GitFailed(format!(
            "git status --short failed: {}",
            stderr_trimmed(&output)
        )));
    }
    Ok(stdout_trimmed(&output))
}

async fn branch_exists(cwd: &Path, branch: &str) -> WorktreeRunResult<bool> {
    let output = run_git_owned(
        cwd,
        vec![
            "show-ref".into(),
            "--verify".into(),
            format!("refs/heads/{branch}"),
        ],
    )
    .await?;
    Ok(output.status.success())
}

async fn run_git<I, S>(cwd: &Path, args: I) -> WorktreeRunResult<std::process::Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|err| {
            WorktreeRunError::Io(format!("failed to run git in {}: {err}", cwd.display()))
        })
}

async fn run_git_owned(cwd: &Path, args: Vec<String>) -> WorktreeRunResult<std::process::Output> {
    run_git(cwd, args).await
}

async fn run_git_with_stdin<I, S>(
    cwd: &Path,
    args: I,
    stdin: &[u8],
) -> WorktreeRunResult<std::process::Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut child = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            WorktreeRunError::Io(format!("failed to run git in {}: {err}", cwd.display()))
        })?;

    if let Some(mut child_stdin) = child.stdin.take() {
        child_stdin.write_all(stdin).await.map_err(|err| {
            WorktreeRunError::Io(format!(
                "failed to write git stdin in {}: {err}",
                cwd.display()
            ))
        })?;
    }

    child.wait_with_output().await.map_err(|err| {
        WorktreeRunError::Io(format!(
            "failed to wait for git in {}: {err}",
            cwd.display()
        ))
    })
}

fn stdout_trimmed(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn stderr_trimmed(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
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
    use std::fs;

    fn run_git_sync(dir: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap_or_else(|err| panic!("git {:?} failed to execute: {err}", args));
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn setup_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        run_git_sync(dir.path(), &["init"]);
        run_git_sync(dir.path(), &["config", "user.email", "test@test.com"]);
        run_git_sync(dir.path(), &["config", "user.name", "Test User"]);
        fs::write(dir.path().join("README.md"), "hello\n").unwrap();
        run_git_sync(dir.path(), &["add", "-A"]);
        run_git_sync(dir.path(), &["commit", "-m", "initial"]);
        dir
    }

    #[tokio::test]
    async fn worktree_mode_plans_creates_and_removes_worktree() {
        let repo = setup_repo();
        let root = tempfile::tempdir().unwrap();
        let spec = WorktreeRunSpec {
            workflow_id: Some("394.9".into()),
            run_id: "run_123".into(),
            slug: "test run".into(),
            worktree_root: Some(root.path().to_path_buf()),
            ..WorktreeRunSpec::default()
        };
        let plan = plan_worktree_run(repo.path(), &spec).await.unwrap();
        assert_eq!(plan.branch, "imp/394-9/test-run");
        assert!(!plan.worktree_path.exists());
        assert!(matches!(
            plan.workspace_scope(),
            WorkspaceScope::Worktree { .. }
        ));

        let metadata = create_worktree_run(&plan).await.unwrap();
        assert!(metadata.worktree_path.exists());
        assert!(metadata.worktree_path.join("README.md").exists());
        fs::write(metadata.worktree_path.join("README.md"), "hello\nchanged\n").unwrap();
        fs::write(metadata.worktree_path.join("new.txt"), "new file\n").unwrap();
        let mut metadata = metadata;
        let diff =
            capture_worktree_diff_artifacts(&mut metadata, &repo.path().join("artifacts/worktree"))
                .await
                .unwrap();
        assert!(!diff.clean);
        assert!(diff.status_path.exists());
        assert!(diff.stat_path.exists());
        assert!(diff.patch_path.exists());
        assert!(std::fs::read_to_string(&diff.status_path)
            .unwrap()
            .contains("new.txt"));
        assert!(std::fs::read_to_string(&diff.patch_path)
            .unwrap()
            .contains("changed"));

        let metadata_path = repo.path().join("metadata/worktree.json");
        write_worktree_metadata(&metadata_path, &metadata)
            .await
            .unwrap();
        assert!(metadata_path.exists());
        let metadata_json = std::fs::read_to_string(&metadata_path).unwrap();
        assert!(metadata_json.contains("diff.patch"));

        remove_worktree_run(&metadata, true, true).await.unwrap();
        assert!(!metadata.worktree_path.exists());
    }

    #[tokio::test]
    async fn worktree_mode_rejects_non_git_repo() {
        let temp = tempfile::tempdir().unwrap();
        let err = plan_worktree_run(temp.path(), &WorktreeRunSpec::new("run_1"))
            .await
            .unwrap_err();
        assert!(matches!(err, WorktreeRunError::NotGitRepo(_)));
    }

    #[tokio::test]
    async fn worktree_mode_blocks_dirty_main_workspace() {
        let repo = setup_repo();
        fs::write(repo.path().join("dirty.txt"), "dirty\n").unwrap();
        let err = plan_worktree_run(repo.path(), &WorktreeRunSpec::new("run_1"))
            .await
            .unwrap_err();
        assert!(matches!(err, WorktreeRunError::DirtyMainWorkspace(_)));
    }

    #[tokio::test]
    async fn worktree_mode_detects_branch_and_path_collisions() {
        let repo = setup_repo();
        run_git_sync(repo.path(), &["branch", "imp/run_1/worktree-auto"]);
        let err = plan_worktree_run(repo.path(), &WorktreeRunSpec::new("run_1"))
            .await
            .unwrap_err();
        assert!(matches!(err, WorktreeRunError::BranchExists(_)));

        run_git_sync(repo.path(), &["branch", "-D", "imp/run_1/worktree-auto"]);
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("run_1-worktree-auto");
        fs::create_dir_all(&existing).unwrap();
        let spec = WorktreeRunSpec {
            worktree_root: Some(root.path().to_path_buf()),
            ..WorktreeRunSpec::new("run_1")
        };
        let err = plan_worktree_run(repo.path(), &spec).await.unwrap_err();
        assert!(matches!(err, WorktreeRunError::WorktreePathExists(_)));
    }

    #[tokio::test]
    async fn worktree_mode_keep_discard_and_apply_closeout() {
        let repo = setup_repo();
        let root = tempfile::tempdir().unwrap();
        let spec = WorktreeRunSpec {
            run_id: "run_apply".into(),
            worktree_root: Some(root.path().to_path_buf()),
            ..WorktreeRunSpec::default()
        };
        let plan = plan_worktree_run(repo.path(), &spec).await.unwrap();
        let metadata = create_worktree_run(&plan).await.unwrap();

        fs::write(metadata.worktree_path.join("README.md"), "hello\napplied\n").unwrap();
        let keep = closeout_worktree_run(&metadata, WorktreeCloseoutAction::Keep)
            .await
            .unwrap();
        assert_eq!(keep.action, WorktreeCloseoutAction::Keep);
        assert!(!keep.clean);
        assert!(metadata.worktree_path.exists());

        let apply = closeout_worktree_run(&metadata, WorktreeCloseoutAction::Apply)
            .await
            .unwrap();
        assert!(apply.applied);
        assert!(std::fs::read_to_string(repo.path().join("README.md"))
            .unwrap()
            .contains("applied"));
        assert!(metadata.worktree_path.exists());

        run_git_sync(repo.path(), &["add", "README.md"]);
        run_git_sync(repo.path(), &["commit", "-m", "apply worktree changes"]);
        let discard = closeout_worktree_run(&metadata, WorktreeCloseoutAction::Discard)
            .await
            .unwrap();
        assert!(discard.removed);
        assert!(discard.branch_deleted);
        assert!(!metadata.worktree_path.exists());
    }

    #[tokio::test]
    async fn worktree_mode_apply_reports_conflicts_without_modifying_main() {
        let repo = setup_repo();
        let root = tempfile::tempdir().unwrap();
        let spec = WorktreeRunSpec {
            run_id: "run_conflict".into(),
            worktree_root: Some(root.path().to_path_buf()),
            ..WorktreeRunSpec::default()
        };
        let plan = plan_worktree_run(repo.path(), &spec).await.unwrap();
        let metadata = create_worktree_run(&plan).await.unwrap();

        fs::write(
            metadata.worktree_path.join("README.md"),
            "hello\nfrom worktree\n",
        )
        .unwrap();
        fs::write(repo.path().join("README.md"), "hello\nfrom main\n").unwrap();
        run_git_sync(repo.path(), &["add", "README.md"]);
        run_git_sync(repo.path(), &["commit", "-m", "main diverged"]);

        let err = closeout_worktree_run(&metadata, WorktreeCloseoutAction::Apply)
            .await
            .unwrap_err();
        assert!(matches!(err, WorktreeRunError::GitFailed(_)));
        assert!(std::fs::read_to_string(repo.path().join("README.md"))
            .unwrap()
            .contains("from main"));

        closeout_worktree_run(&metadata, WorktreeCloseoutAction::Discard)
            .await
            .unwrap();
    }
}
