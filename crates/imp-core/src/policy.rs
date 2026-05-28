use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Per-run policy for constraining tool execution.
///
/// This policy is intentionally narrower than [`crate::config::AgentMode`]:
/// AgentMode establishes a coarse baseline role, while `RunPolicy` lets
/// automation further constrain a single non-interactive worker run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunPolicy {
    allowed_tools: BTreeSet<String>,
    denied_tools: BTreeSet<String>,
    allowed_write_patterns: Vec<String>,
    denied_write_patterns: Vec<String>,
}

impl RunPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_tool(mut self, name: impl AsRef<str>) -> Self {
        self.allowed_tools.insert(normalize_tool_name(name));
        self
    }

    pub fn deny_tool(mut self, name: impl AsRef<str>) -> Self {
        self.denied_tools.insert(normalize_tool_name(name));
        self
    }

    pub fn allowed_tools(&self) -> &BTreeSet<String> {
        &self.allowed_tools
    }

    pub fn denied_tools(&self) -> &BTreeSet<String> {
        &self.denied_tools
    }

    pub fn allow_write(mut self, pattern: impl Into<String>) -> Self {
        self.allowed_write_patterns.push(pattern.into());
        self
    }

    pub fn deny_write(mut self, pattern: impl Into<String>) -> Self {
        self.denied_write_patterns.push(pattern.into());
        self
    }

    pub fn allowed_write_patterns(&self) -> &[String] {
        &self.allowed_write_patterns
    }

    pub fn denied_write_patterns(&self) -> &[String] {
        &self.denied_write_patterns
    }

    pub fn is_empty(&self) -> bool {
        self.allowed_tools.is_empty()
            && self.denied_tools.is_empty()
            && self.allowed_write_patterns.is_empty()
            && self.denied_write_patterns.is_empty()
    }

    pub fn check_tool(&self, tool_name: &str) -> ToolPolicyDecision {
        let normalized = normalize_tool_name(tool_name);
        if self.denied_tools.contains(&normalized) {
            return ToolPolicyDecision::Denied(format!("Tool `{tool_name}` denied by run policy."));
        }

        if !self.allowed_tools.is_empty() && !self.allowed_tools.contains(&normalized) {
            return ToolPolicyDecision::Denied(format!(
                "Tool `{tool_name}` is not in the run policy allowlist."
            ));
        }

        ToolPolicyDecision::Allowed
    }

    pub fn check_write_path(&self, cwd: &Path, path: &Path) -> WritePolicyDecision {
        if self.allowed_write_patterns.is_empty() && self.denied_write_patterns.is_empty() {
            return WritePolicyDecision::Allowed;
        }

        let Ok(relative) = normalize_relative_path(cwd, path) else {
            return WritePolicyDecision::Denied(format!(
                "Write to `{}` denied by run policy because the path is outside the worker root `{}`.",
                path.display(),
                cwd.display()
            ));
        };
        let display = relative.to_string_lossy().replace('\\', "/");

        if matches_any(&display, &self.denied_write_patterns) {
            return WritePolicyDecision::Denied(format!(
                "Write to `{display}` denied by run policy denylist."
            ));
        }

        if !self.allowed_write_patterns.is_empty()
            && !matches_any(&display, &self.allowed_write_patterns)
        {
            return WritePolicyDecision::Denied(format!(
                "Write to `{display}` is not in the run policy write allowlist."
            ));
        }

        WritePolicyDecision::Allowed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WritePolicyDecision {
    Allowed,
    Denied(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPolicyDecision {
    Allowed,
    Denied(String),
}

fn normalize_tool_name(name: impl AsRef<str>) -> String {
    name.as_ref().trim().to_ascii_lowercase()
}

fn normalize_relative_path(cwd: &Path, path: &Path) -> Result<PathBuf, ()> {
    let root = physical_or_normalized(cwd);
    let candidate_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let candidate = physical_or_normalized(&candidate_path);
    candidate
        .strip_prefix(&root)
        .map(Path::to_path_buf)
        .map_err(|_| ())
}

fn physical_or_normalized(path: &Path) -> PathBuf {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            if let Ok(target) = std::fs::read_link(path) {
                let resolved_target = if target.is_absolute() {
                    target
                } else {
                    path.parent().unwrap_or_else(|| Path::new("")).join(target)
                };
                return physical_or_normalized(&resolved_target);
            }
        }
    }

    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }

    let mut missing_components = Vec::new();
    let mut ancestor = path;
    while let Some(parent) = ancestor.parent() {
        if let Some(name) = ancestor.file_name() {
            missing_components.push(name.to_os_string());
        }
        if let Ok(mut canonical_parent) = std::fs::canonicalize(parent) {
            for component in missing_components.iter().rev() {
                canonical_parent.push(component);
            }
            return canonical_parent;
        }
        ancestor = parent;
    }

    normalize_path(path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn matches_any(path: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| path_matches_pattern(path, pattern))
}

fn path_matches_pattern(path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().replace('\\', "/");
    if pattern == path {
        return true;
    }
    glob::Pattern::new(&pattern).is_ok_and(|glob| glob.matches(path))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{RunPolicy, ToolPolicyDecision, WritePolicyDecision};

    #[test]
    fn write_allowlist_blocks_symlink_escape_from_worker_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = root.path().join("allowed.md");
        create_symlink(outside.path().join("secret.md"), &link);

        let policy = RunPolicy::new().allow_write("allowed.md");
        assert!(matches!(
            policy.check_write_path(root.path(), &link),
            WritePolicyDecision::Denied(reason) if reason.contains("outside the worker root")
        ));
    }

    #[test]
    fn write_allowlist_allows_missing_nested_file_under_existing_root() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("docs").join("note.md");

        let policy = RunPolicy::new().allow_write("docs/*.md");
        assert_eq!(
            policy.check_write_path(root.path(), &path),
            WritePolicyDecision::Allowed
        );
    }

    #[cfg(unix)]
    fn create_symlink(target: PathBuf, link: &std::path::Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(windows)]
    fn create_symlink(target: PathBuf, link: &std::path::Path) {
        std::os::windows::fs::symlink_file(target, link).unwrap();
    }

    #[test]
    fn empty_policy_allows_tools() {
        assert_eq!(
            RunPolicy::new().check_tool("bash"),
            ToolPolicyDecision::Allowed
        );
    }

    #[test]
    fn deny_tool_blocks_even_when_allowed() {
        let policy = RunPolicy::new().allow_tool("bash").deny_tool("bash");
        assert!(matches!(
            policy.check_tool("bash"),
            ToolPolicyDecision::Denied(reason) if reason.contains("denied")
        ));
    }

    #[test]
    fn allowlist_blocks_unlisted_tools() {
        let policy = RunPolicy::new().allow_tool("read");
        assert_eq!(policy.check_tool("read"), ToolPolicyDecision::Allowed);
        assert!(matches!(
            policy.check_tool("write"),
            ToolPolicyDecision::Denied(reason) if reason.contains("allowlist")
        ));
    }

    #[test]
    fn tool_names_are_normalized() {
        let policy = RunPolicy::new().allow_tool(" Read ").deny_tool(" Git ");
        assert_eq!(policy.check_tool("read"), ToolPolicyDecision::Allowed);
        assert!(matches!(
            policy.check_tool("git"),
            ToolPolicyDecision::Denied(_)
        ));
    }
}
