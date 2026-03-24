use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::Result;

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

/// Discover all AGENTS.md files by walking up from cwd.
pub fn discover_agents_md(cwd: &Path, user_config_dir: &Path) -> Vec<AgentsMd> {
    let mut results = Vec::new();

    // User global
    for name in &["AGENTS.md", "CLAUDE.md"] {
        let path = user_config_dir.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            results.push(AgentsMd { path, content });
        }
    }

    // Walk up from cwd
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        for name in &["AGENTS.md", "CLAUDE.md"] {
            let path = d.join(name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                results.push(AgentsMd { path, content });
            }
        }
        dir = d.parent();
    }

    results
}

/// Discover skills from user and project directories.
pub fn discover_skills(cwd: &Path, user_config_dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();

    let dirs = [
        user_config_dir.join("skills"),
        cwd.join(".imp").join("skills"),
    ];

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
                        skills.push(Skill {
                            name,
                            description,
                            path: skill_file,
                        });
                    }
                }
            }
        }
    }

    skills
}

/// Discover prompt templates.
pub fn discover_prompts(cwd: &Path, user_config_dir: &Path) -> Result<Vec<PromptTemplate>> {
    let mut prompts = Vec::new();

    let dirs = [
        user_config_dir.join("prompts"),
        cwd.join(".imp").join("prompts"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
    fn resource_discover_agents_md_empty_when_no_files() {
        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("config");
        let cwd = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(&cwd).unwrap();

        let results = discover_agents_md(&cwd, &user_dir);
        // May have results from walk-up finding nothing — just not our test files
        // In temp dir there should be no AGENTS.md above it
        assert!(results
            .iter()
            .all(|a| { a.path.starts_with(dir.path()) || !a.path.exists() }));
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
