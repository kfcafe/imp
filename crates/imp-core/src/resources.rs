use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::storage;

/// Discovered AGENTS.md content.
#[derive(Debug, Clone)]
pub struct AgentsMd {
    pub path: PathBuf,
    pub content: String,
}

/// Discovered skill.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
}

/// Discovered prompt template.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub path: PathBuf,
    pub content: String,
}

impl PromptTemplate {
    /// Expand `{{variable}}` placeholders with the given values.
    pub fn expand(&self, vars: &HashMap<String, String>) -> String {
        let mut result = self.content.clone();
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Discovered soul document.
#[derive(Debug, Clone)]
pub struct SoulDoc {
    pub path: PathBuf,
    pub content: String,
}

/// Discover the nearest project soul document by walking up from cwd.
pub fn discover_project_soul(cwd: &Path) -> Option<SoulDoc> {
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        let path = storage::project_soul_path(d);
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(SoulDoc { path, content });
        }
        dir = d.parent();
    }
    None
}

/// Suggest where a new project soul should be created.
///
/// Prefers the nearest ancestor that looks like a project root. Falls back to `cwd/.imp/soul.md`.
pub fn suggested_project_soul_path(cwd: &Path) -> PathBuf {
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        let looks_like_project_root = d.join(".imp").exists()
            || d.join(".git").exists()
            || d.join("Cargo.toml").exists()
            || d.join("package.json").exists()
            || d.join("pyproject.toml").exists()
            || d.join("go.mod").exists()
            || d.join("AGENTS.md").exists()
            || d.join("CLAUDE.md").exists();
        if looks_like_project_root {
            return storage::project_soul_path(d);
        }
        dir = d.parent();
    }

    cwd.join(".imp").join("soul.md")
}

/// Discover the active soul document.
///
/// Precedence:
/// 1. nearest project `.imp/soul.md` while walking up from cwd
/// 2. global `<user_config_dir>/soul.md`
pub fn discover_soul(cwd: &Path, user_config_dir: &Path) -> Option<SoulDoc> {
    if let Some(project) = discover_project_soul(cwd) {
        return Some(project);
    }

    let global = user_config_dir.join("soul.md");
    std::fs::read_to_string(&global)
        .ok()
        .map(|content| SoulDoc {
            path: global,
            content,
        })
}

fn global_agents_candidates(user_config_dir: &Path) -> [PathBuf; 3] {
    [
        user_config_dir.join("agents.md"),
        user_config_dir.join("AGENTS.md"),
        user_config_dir.join("CLAUDE.md"),
    ]
}

fn project_agents_candidates(project_dir: &Path) -> [PathBuf; 3] {
    [
        storage::project_agents_path(project_dir),
        project_dir.join("AGENTS.md"),
        project_dir.join("CLAUDE.md"),
    ]
}

fn push_agents_md_if_unique(
    results: &mut Vec<AgentsMd>,
    seen_paths: &mut HashSet<PathBuf>,
    seen_content: &mut HashSet<String>,
    path: PathBuf,
) {
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };

    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.clone());
    if !seen_paths.insert(canonical_path) {
        return;
    }

    if !seen_content.insert(content.clone()) {
        return;
    }

    results.push(AgentsMd { path, content });
}

/// Discover instruction documents by walking up from cwd.
///
/// Canonical imp-native files are `.imp/agents.md` at global and project scope.
/// Legacy compatibility files (`AGENTS.md`, `CLAUDE.md`) are still read after the
/// canonical file at each scope level.
pub fn discover_agents_md(cwd: &Path, user_config_dir: &Path) -> Vec<AgentsMd> {
    let mut results = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut seen_content = HashSet::new();

    for path in global_agents_candidates(user_config_dir) {
        push_agents_md_if_unique(&mut results, &mut seen_paths, &mut seen_content, path);
    }

    let mut dir = Some(cwd);
    while let Some(d) = dir {
        for path in project_agents_candidates(d) {
            push_agents_md_if_unique(&mut results, &mut seen_paths, &mut seen_content, path);
        }
        dir = d.parent();
    }

    results
}

/// Discover skills from user and project directories.
pub fn discover_skills(cwd: &Path, user_config_dir: &Path) -> Vec<Skill> {
    let mut by_name = HashMap::new();
    let mut dirs = vec![user_config_dir.join("skills")];

    let mut ancestry = Vec::new();
    let mut dir = Some(cwd);
    while let Some(current) = dir {
        ancestry.push(storage::project_skills_dir(current));
        dir = current.parent();
    }
    ancestry.reverse();
    dirs.extend(ancestry);

    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let skill_dir = entry.path();
                let skill_file = skill_dir.join("SKILL.md");
                if skill_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&skill_file) {
                        let name = skill_dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let description = extract_description(&content);
                        by_name.insert(
                            name.clone(),
                            Skill {
                                name,
                                description,
                                path: skill_file,
                            },
                        );
                    }
                }
            }
        }
    }

    let mut skills: Vec<Skill> = by_name.into_values().collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Discover prompt templates.
pub fn discover_prompts(cwd: &Path, user_config_dir: &Path) -> Result<Vec<PromptTemplate>> {
    let mut prompts = Vec::new();

    let dirs = [
        user_config_dir.join("prompts"),
        storage::project_prompts_dir(cwd),
    ];

    for dir in &dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let name = path
                            .file_stem()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        prompts.push(PromptTemplate {
                            name,
                            path,
                            content,
                        });
                    }
                }
            }
        }
    }

    Ok(prompts)
}

/// Extract the first paragraph as a description from a markdown file.
pub fn extract_description(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| l.starts_with('#') || l.trim().is_empty())
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

/// Return markdown content without leading YAML frontmatter.
pub fn strip_frontmatter(content: &str) -> &str {
    let Some(rest) = content.strip_prefix("---\n") else {
        return content;
    };

    match rest.find("\n---") {
        Some(end) => rest[end + "\n---".len()..].trim_start_matches(['\n', '\r']),
        None => content,
    }
}

/// Render a skill body for explicit slash-command invocation.
pub fn render_skill_invocation(name: &str, content: &str, args: &str) -> String {
    let body = strip_frontmatter(content).trim();
    let args = args.trim();
    let body = if args.is_empty() {
        body.to_string()
    } else if body.contains("$ARGUMENTS") {
        body.replace("$ARGUMENTS", args)
    } else {
        format!("{body}\n\nARGUMENTS: {args}")
    };

    format!("Use the `{name}` skill.\n\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // -- soul discovery --

    #[test]
    fn resource_discover_soul_uses_global_fallback() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        fs::write(user_dir.join("soul.md"), "# Soul\n\nglobal soul").unwrap();

        let soul = discover_soul(&cwd, &user_dir).expect("global soul should load");
        assert!(soul.content.contains("global soul"));
        assert_eq!(soul.path, user_dir.join("soul.md"));
    }

    #[test]
    fn resource_discover_soul_prefers_nearest_project_override() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let project = dir.path().join("project");
        let nested = project.join("src").join("deep");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(project.join(".imp")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(user_dir.join("soul.md"), "# Soul\n\nglobal soul").unwrap();
        fs::write(
            project.join(".imp").join("soul.md"),
            "# Soul\n\nproject soul",
        )
        .unwrap();

        let soul = discover_soul(&nested, &user_dir).expect("project soul should load");
        assert!(soul.content.contains("project soul"));
        assert_eq!(soul.path, project.join(".imp").join("soul.md"));
    }

    #[test]
    fn resource_discover_project_soul_walks_up_from_cwd() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("project");
        let nested = project.join("src").join("deep");
        fs::create_dir_all(project.join(".imp")).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            project.join(".imp").join("soul.md"),
            "# Soul\n\nproject soul",
        )
        .unwrap();

        let soul = discover_project_soul(&nested).expect("project soul should load");
        assert!(soul.content.contains("project soul"));
        assert_eq!(soul.path, project.join(".imp").join("soul.md"));
    }

    #[test]
    fn resource_suggested_project_soul_path_prefers_nearest_projectish_ancestor() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("project");
        let nested = project.join("src").join("deep");
        fs::create_dir_all(&nested).unwrap();
        fs::write(project.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        let path = suggested_project_soul_path(&nested);
        assert_eq!(path, project.join(".imp").join("soul.md"));
    }

    #[test]
    fn resource_discover_soul_empty_when_absent() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        assert!(discover_soul(&cwd, &user_dir).is_none());
    }

    // -- AGENTS.md discovery --

    #[test]
    fn resource_discover_agents_md_from_user_config() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("AGENTS.md"), "# Global rules").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let results = discover_agents_md(&cwd, &user_dir);
        assert!(results.iter().any(|a| a.content.contains("Global rules")));
    }

    #[test]
    fn resource_discover_agents_md_walks_up_from_cwd() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();

        // Create AGENTS.md at the project root
        let project = dir.path().join("project");
        let subdir = project.join("src").join("deep");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(project.join("AGENTS.md"), "# Project rules").unwrap();

        let results = discover_agents_md(&subdir, &user_dir);
        assert!(results.iter().any(|a| a.content.contains("Project rules")));
    }

    #[test]
    fn resource_discover_agents_md_finds_claude_md() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("CLAUDE.md"), "# Claude config").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let results = discover_agents_md(&cwd, &user_dir);
        assert!(results.iter().any(|a| a.content.contains("Claude config")));
    }

    #[test]
    fn resource_discover_agents_md_global_first() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let project = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&project).unwrap();

        fs::write(user_dir.join("AGENTS.md"), "global").unwrap();
        fs::write(project.join("AGENTS.md"), "project").unwrap();

        let results = discover_agents_md(&project, &user_dir);
        // Global should appear before project
        let global_idx = results.iter().position(|a| a.content == "global").unwrap();
        let project_idx = results.iter().position(|a| a.content == "project").unwrap();
        assert!(global_idx < project_idx);
    }

    #[test]
    fn resource_discover_agents_md_reads_global_imp_agents_file() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("agents.md"), "global-imp").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let results = discover_agents_md(&cwd, &user_dir);
        assert!(results.iter().any(|a| a.content == "global-imp"));
    }

    #[test]
    fn resource_discover_agents_md_prefers_project_imp_agents_file() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let project = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(project.join(".imp")).unwrap();
        fs::write(project.join(".imp").join("agents.md"), "project-imp").unwrap();
        fs::write(project.join("AGENTS.md"), "project-legacy").unwrap();

        let results = discover_agents_md(&project, &user_dir);
        let canonical_idx = results
            .iter()
            .position(|a| a.content == "project-imp")
            .unwrap();
        let legacy_idx = results
            .iter()
            .position(|a| a.content == "project-legacy")
            .unwrap();
        assert!(canonical_idx < legacy_idx);
    }

    #[test]
    fn resource_discover_agents_md_dedupes_legacy_global_copy() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("agents.md"), "same global rules").unwrap();
        fs::write(user_dir.join("AGENTS.md"), "same global rules").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let results = discover_agents_md(&cwd, &user_dir);
        assert_eq!(
            results
                .iter()
                .filter(|a| a.content == "same global rules")
                .count(),
            1
        );
    }

    #[test]
    fn resource_discover_agents_md_dedupes_global_when_home_is_ancestor() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join(".imp");
        let project = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&project).unwrap();
        fs::write(user_dir.join("agents.md"), "global rules").unwrap();

        let results = discover_agents_md(&project, &user_dir);
        assert_eq!(
            results
                .iter()
                .filter(|a| a.content == "global rules")
                .count(),
            1
        );
    }

    #[test]
    fn resource_discover_agents_md_keeps_distinct_global_and_project_rules() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let project = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&project).unwrap();
        fs::write(user_dir.join("agents.md"), "global rules").unwrap();
        fs::write(project.join("AGENTS.md"), "project rules").unwrap();

        let results = discover_agents_md(&project, &user_dir);
        assert!(results.iter().any(|a| a.content == "global rules"));
        assert!(results.iter().any(|a| a.content == "project rules"));
    }

    #[test]
    fn resource_discover_agents_md_empty_when_no_files() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let results = discover_agents_md(&cwd, &user_dir);
        assert!(results.is_empty());
    }

    // -- Skills discovery --

    #[test]
    fn resource_discover_skills_from_user_dir() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let skills_dir = user_dir.join("skills").join("my-skill");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "# My Skill\n\nDoes useful things for you.\n",
        )
        .unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let skills = discover_skills(&cwd, &user_dir);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert!(skills[0].description.contains("useful things"));
    }

    #[test]
    fn resource_discover_skills_from_project_dir() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();

        let cwd = dir.path().join("project");
        let skills_dir = cwd.join(".imp").join("skills").join("project-skill");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "# Project Skill\n\nProject-specific automation.\n",
        )
        .unwrap();

        let skills = discover_skills(&cwd, &user_dir);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "project-skill");
    }

    #[test]
    fn resource_discover_skills_from_both_dirs() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let user_skills = user_dir.join("skills").join("global-skill");
        fs::create_dir_all(&user_skills).unwrap();
        fs::write(user_skills.join("SKILL.md"), "# Global\n\nGlobal skill.\n").unwrap();

        let cwd = dir.path().join("project");
        let project_skills = cwd.join(".imp").join("skills").join("local-skill");
        fs::create_dir_all(&project_skills).unwrap();
        fs::write(project_skills.join("SKILL.md"), "# Local\n\nLocal skill.\n").unwrap();

        let skills = discover_skills(&cwd, &user_dir);
        assert_eq!(skills.len(), 2);
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"global-skill"));
        assert!(names.contains(&"local-skill"));
    }

    #[test]
    fn resource_discover_skills_walks_up_from_cwd() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();

        let project = dir.path().join("project");
        let nested = project.join("src").join("deep");
        let skills_dir = project.join(".imp").join("skills").join("project-skill");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            skills_dir.join("SKILL.md"),
            "# Project Skill\n\nProject-specific automation.\n",
        )
        .unwrap();

        let skills = discover_skills(&nested, &user_dir);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "project-skill");
    }

    #[test]
    fn resource_discover_skills_project_overrides_user_by_name() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let user_skill = user_dir.join("skills").join("mana");
        fs::create_dir_all(&user_skill).unwrap();
        fs::write(user_skill.join("SKILL.md"), "# Mana\n\nUser version.\n").unwrap();

        let project = dir.path().join("project");
        let project_skill = project.join(".imp").join("skills").join("mana");
        fs::create_dir_all(&project_skill).unwrap();
        fs::write(
            project_skill.join("SKILL.md"),
            "# Mana\n\nProject version.\n",
        )
        .unwrap();

        let skills = discover_skills(&project, &user_dir);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "mana");
        assert!(skills[0].description.contains("Project version"));
        assert_eq!(skills[0].path, project_skill.join("SKILL.md"));
    }

    #[test]
    fn resource_discover_skills_skips_dirs_without_skill_md() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let skills_dir = user_dir.join("skills").join("incomplete-skill");
        fs::create_dir_all(&skills_dir).unwrap();
        // No SKILL.md — just a random file
        fs::write(skills_dir.join("README.md"), "not a skill").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let skills = discover_skills(&cwd, &user_dir);
        assert!(skills.is_empty());
    }

    #[test]
    fn resource_discover_skills_empty_when_no_dirs() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let skills = discover_skills(&cwd, &user_dir);
        assert!(skills.is_empty());
    }

    // -- Prompt template discovery --

    #[test]
    fn resource_discover_prompts_from_user_dir() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let prompts_dir = user_dir.join("prompts");
        fs::create_dir_all(&prompts_dir).unwrap();
        fs::write(prompts_dir.join("review.md"), "Review this code: {{code}}").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let prompts = discover_prompts(&cwd, &user_dir).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "review");
        assert!(prompts[0].content.contains("{{code}}"));
    }

    #[test]
    fn resource_discover_prompts_from_project_dir() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        fs::create_dir_all(&user_dir).unwrap();

        let cwd = dir.path().join("project");
        let prompts_dir = cwd.join(".imp").join("prompts");
        fs::create_dir_all(&prompts_dir).unwrap();
        fs::write(
            prompts_dir.join("deploy.md"),
            "Deploy {{service}} to {{env}}",
        )
        .unwrap();

        let prompts = discover_prompts(&cwd, &user_dir).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "deploy");
    }

    #[test]
    fn resource_discover_prompts_ignores_non_md_files() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let prompts_dir = user_dir.join("prompts");
        fs::create_dir_all(&prompts_dir).unwrap();
        fs::write(prompts_dir.join("valid.md"), "prompt content").unwrap();
        fs::write(prompts_dir.join("ignored.txt"), "not a prompt").unwrap();
        fs::write(prompts_dir.join("also_ignored.toml"), "nope").unwrap();

        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();

        let prompts = discover_prompts(&cwd, &user_dir).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "valid");
    }

    #[test]
    fn resource_discover_prompts_empty_when_no_dirs() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let prompts = discover_prompts(&cwd, &user_dir).unwrap();
        assert!(prompts.is_empty());
    }

    // -- Template expansion --

    #[test]
    fn resource_prompt_template_expand_variables() {
        let template = PromptTemplate {
            name: "test".into(),
            path: PathBuf::from("test.md"),
            content: "Hello {{name}}, welcome to {{project}}!".into(),
        };

        let mut vars = HashMap::new();
        vars.insert("name".into(), "Alice".into());
        vars.insert("project".into(), "imp".into());

        let result = template.expand(&vars);
        assert_eq!(result, "Hello Alice, welcome to imp!");
    }

    #[test]
    fn resource_prompt_template_expand_missing_variable_left_as_is() {
        let template = PromptTemplate {
            name: "test".into(),
            path: PathBuf::from("test.md"),
            content: "Hello {{name}}, your role is {{role}}.".into(),
        };

        let mut vars = HashMap::new();
        vars.insert("name".into(), "Bob".into());
        // "role" not provided

        let result = template.expand(&vars);
        assert_eq!(result, "Hello Bob, your role is {{role}}.");
    }

    #[test]
    fn resource_prompt_template_expand_empty_vars() {
        let template = PromptTemplate {
            name: "test".into(),
            path: PathBuf::from("test.md"),
            content: "No variables here.".into(),
        };

        let vars = HashMap::new();
        let result = template.expand(&vars);
        assert_eq!(result, "No variables here.");
    }

    #[test]
    fn resource_prompt_template_expand_repeated_variable() {
        let template = PromptTemplate {
            name: "test".into(),
            path: PathBuf::from("test.md"),
            content: "{{x}} and {{x}} again".into(),
        };

        let mut vars = HashMap::new();
        vars.insert("x".into(), "hello".into());

        let result = template.expand(&vars);
        assert_eq!(result, "hello and hello again");
    }

    // -- extract_description --

    #[test]
    fn resource_extract_description_skips_headings() {
        let content = "# Title\n\nThis is the description.\nMore text here.\n\n## Section";
        let desc = extract_description(content);
        assert_eq!(desc, "This is the description. More text here.");
    }

    #[test]
    fn resource_extract_description_empty_content() {
        assert_eq!(extract_description(""), "");
    }

    #[test]
    fn resource_extract_description_truncates_at_200_chars() {
        let long_line = "A".repeat(250);
        let content = format!("# Title\n\n{}", long_line);
        let desc = extract_description(&content);
        assert_eq!(desc.len(), 200);
    }
}
