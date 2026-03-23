use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::config::Config;
use crate::error::Result;

pub struct SkillManageTool;

#[async_trait]
impl Tool for SkillManageTool {
    fn name(&self) -> &str {
        "skill_manage"
    }

    fn label(&self) -> &str {
        "Skill Manager"
    }

    fn description(&self) -> &str {
        "Create, update, and delete skills. Use after completing complex tasks \
         to save the approach for future reuse. Use to fix skills that are \
         incomplete or incorrect."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["action", "name"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "patch", "delete"],
                    "description": "Action: create a new skill, patch an existing one, or delete"
                },
                "name": {
                    "type": "string",
                    "description": "Skill name (lowercase, hyphens, e.g. 'deploy-k8s')"
                },
                "content": {
                    "type": "string",
                    "description": "Full SKILL.md content including frontmatter (for 'create')"
                },
                "old_text": {
                    "type": "string",
                    "description": "Text to find in the skill (for 'patch')"
                },
                "new_text": {
                    "type": "string",
                    "description": "Replacement text (for 'patch')"
                }
            }
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let action = params["action"].as_str().unwrap_or("");
        let name = params["name"].as_str().unwrap_or("");

        if action.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: action"));
        }
        if name.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: name"));
        }

        if let Some(reason) = validate_skill_name(name) {
            return Ok(ToolOutput::error(reason));
        }

        let agent_skills_dir = Config::user_config_dir().join("skills").join("agent");

        match action {
            "create" => {
                let content = params["content"].as_str().unwrap_or("");
                if content.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: content (for 'create' action)",
                    ));
                }
                create_skill(&agent_skills_dir, name, content)
            }
            "patch" => {
                let old_text = params["old_text"].as_str().unwrap_or("");
                let new_text = params["new_text"].as_str().unwrap_or("");
                if old_text.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: old_text (for 'patch' action)",
                    ));
                }
                patch_skill(&agent_skills_dir, name, old_text, new_text)
            }
            "delete" => delete_skill(&agent_skills_dir, name),
            other => Ok(ToolOutput::error(format!(
                "Unknown action \"{other}\". Use \"create\", \"patch\", or \"delete\"."
            ))),
        }
    }
}

/// Validate a skill name against the agentskills.io spec.
fn validate_skill_name(name: &str) -> Option<String> {
    if name.len() > 64 {
        return Some(format!(
            "Skill name too long ({} chars, max 64)",
            name.len()
        ));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Some("Skill name cannot start or end with a hyphen".to_string());
    }
    if name.contains("--") {
        return Some("Skill name cannot contain consecutive hyphens".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Some(
            "Skill name must contain only lowercase letters, numbers, and hyphens".to_string(),
        );
    }
    None
}

/// Validate SKILL.md frontmatter has required fields.
fn validate_frontmatter(content: &str, expected_name: &str) -> Option<String> {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return Some("SKILL.md must start with YAML frontmatter (---)".to_string());
    }

    // Find closing ---
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---");
    let Some(end) = end else {
        return Some("SKILL.md frontmatter not closed (missing ---)".to_string());
    };

    let yaml_block = &after_first[..end];

    // Check for required fields (simple string matching — no YAML parser needed)
    let has_name = yaml_block
        .lines()
        .any(|l| l.trim_start().starts_with("name:"));
    let has_desc = yaml_block
        .lines()
        .any(|l| l.trim_start().starts_with("description:"));

    if !has_name {
        return Some("Frontmatter missing required field: name".to_string());
    }
    if !has_desc {
        return Some("Frontmatter missing required field: description".to_string());
    }

    // Verify name matches
    for line in yaml_block.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name:") {
            let val = val.trim().trim_matches('"').trim_matches('\'');
            if val != expected_name {
                return Some(format!(
                    "Frontmatter name \"{val}\" doesn't match skill name \"{expected_name}\""
                ));
            }
        }
    }

    None
}

fn create_skill(agent_dir: &Path, name: &str, content: &str) -> Result<ToolOutput> {
    let skill_dir = agent_dir.join(name);
    let skill_file = skill_dir.join("SKILL.md");

    if skill_file.exists() {
        return Ok(ToolOutput::error(format!(
            "Skill \"{name}\" already exists. Use 'patch' to update it."
        )));
    }

    if let Some(reason) = validate_frontmatter(content, name) {
        return Ok(ToolOutput::error(reason));
    }

    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(&skill_file, content)?;

    Ok(ToolOutput::text(format!(
        "Created skill \"{name}\" at {}",
        skill_file.display()
    )))
}

fn patch_skill(agent_dir: &Path, name: &str, old_text: &str, new_text: &str) -> Result<ToolOutput> {
    // Search agent-created skills first, then fall back to all user skills
    let agent_path = agent_dir.join(name).join("SKILL.md");
    let skill_path = if agent_path.exists() {
        agent_path
    } else {
        // Check parent skills directory (non-agent skills)
        let parent_dir = agent_dir.parent().unwrap_or(agent_dir);
        let alt_path = parent_dir.join(name).join("SKILL.md");
        if alt_path.exists() {
            alt_path
        } else {
            return Ok(ToolOutput::error(format!(
                "Skill \"{name}\" not found. Use 'create' first."
            )));
        }
    };

    let content = std::fs::read_to_string(&skill_path)?;
    let count = content.matches(old_text).count();

    match count {
        0 => Ok(ToolOutput::error(format!(
            "Text not found in skill \"{name}\""
        ))),
        1 => {
            let updated = content.replacen(old_text, new_text, 1);
            std::fs::write(&skill_path, &updated)?;
            Ok(ToolOutput::text(format!("Patched skill \"{name}\"")))
        }
        n => Ok(ToolOutput::error(format!(
            "Text matches {n} times in skill \"{name}\". Provide a more specific old_text."
        ))),
    }
}

fn delete_skill(agent_dir: &Path, name: &str) -> Result<ToolOutput> {
    let skill_dir = agent_dir.join(name);

    if !skill_dir.exists() {
        // Check if it exists outside agent dir — refuse to delete
        let parent = agent_dir.parent().unwrap_or(agent_dir);
        if parent.join(name).exists() {
            return Ok(ToolOutput::error(format!(
                "Skill \"{name}\" exists but is not agent-created. \
                 Only agent-created skills (in agent/ directory) can be deleted."
            )));
        }
        return Ok(ToolOutput::error(format!("Skill \"{name}\" not found")));
    }

    std::fs::remove_dir_all(&skill_dir)?;
    Ok(ToolOutput::text(format!("Deleted skill \"{name}\"")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let agent_dir = dir.path().join("skills").join("agent");
        std::fs::create_dir_all(&agent_dir).unwrap();
        (dir, agent_dir)
    }

    fn valid_skill_content(name: &str) -> String {
        format!("---\nname: {name}\ndescription: A test skill\n---\n\n# Test Skill\n\nDo things.\n")
    }

    // --- Name validation ---

    #[test]
    fn skill_manage_valid_names() {
        assert!(validate_skill_name("deploy-k8s").is_none());
        assert!(validate_skill_name("a").is_none());
        assert!(validate_skill_name("my-skill-123").is_none());
    }

    #[test]
    fn skill_manage_rejects_uppercase() {
        assert!(validate_skill_name("Deploy").is_some());
    }

    #[test]
    fn skill_manage_rejects_leading_hyphen() {
        assert!(validate_skill_name("-bad").is_some());
    }

    #[test]
    fn skill_manage_rejects_trailing_hyphen() {
        assert!(validate_skill_name("bad-").is_some());
    }

    #[test]
    fn skill_manage_rejects_consecutive_hyphens() {
        assert!(validate_skill_name("bad--name").is_some());
    }

    #[test]
    fn skill_manage_rejects_spaces() {
        assert!(validate_skill_name("bad name").is_some());
    }

    #[test]
    fn skill_manage_rejects_too_long() {
        let long = "a".repeat(65);
        assert!(validate_skill_name(&long).is_some());
    }

    // --- Frontmatter validation ---

    #[test]
    fn skill_manage_valid_frontmatter() {
        let content = "---\nname: test\ndescription: A test\n---\n\n# Body\n";
        assert!(validate_frontmatter(content, "test").is_none());
    }

    #[test]
    fn skill_manage_frontmatter_missing_name() {
        let content = "---\ndescription: A test\n---\n\n# Body\n";
        assert!(validate_frontmatter(content, "test").is_some());
    }

    #[test]
    fn skill_manage_frontmatter_missing_description() {
        let content = "---\nname: test\n---\n\n# Body\n";
        assert!(validate_frontmatter(content, "test").is_some());
    }

    #[test]
    fn skill_manage_frontmatter_name_mismatch() {
        let content = "---\nname: wrong\ndescription: A test\n---\n";
        let r = validate_frontmatter(content, "test");
        assert!(r.is_some());
        assert!(r.unwrap().contains("doesn't match"));
    }

    #[test]
    fn skill_manage_no_frontmatter() {
        let content = "# Just a heading\nNo frontmatter here.";
        assert!(validate_frontmatter(content, "test").is_some());
    }

    // --- Create ---

    #[test]
    fn skill_manage_create() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("my-skill");
        let r = create_skill(&agent_dir, "my-skill", &content).unwrap();
        assert!(!r.is_error, "Expected success: {:?}", r.text_content());

        let file = agent_dir.join("my-skill").join("SKILL.md");
        assert!(file.exists());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), content);
    }

    #[test]
    fn skill_manage_create_duplicate() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("dup");
        create_skill(&agent_dir, "dup", &content).unwrap();

        let r = create_skill(&agent_dir, "dup", &content).unwrap();
        assert!(r.is_error);
        assert!(r.text_content().unwrap().contains("already exists"));
    }

    // --- Patch ---

    #[test]
    fn skill_manage_patch() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("patchme");
        create_skill(&agent_dir, "patchme", &content).unwrap();

        let r = patch_skill(&agent_dir, "patchme", "Do things.", "Do better things.").unwrap();
        assert!(!r.is_error, "Expected success: {:?}", r.text_content());

        let updated = std::fs::read_to_string(agent_dir.join("patchme").join("SKILL.md")).unwrap();
        assert!(updated.contains("Do better things."));
        assert!(!updated.contains("Do things."));
    }

    #[test]
    fn skill_manage_patch_not_found() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("patchme");
        create_skill(&agent_dir, "patchme", &content).unwrap();

        let r = patch_skill(&agent_dir, "patchme", "NONEXISTENT", "replacement").unwrap();
        assert!(r.is_error);
        assert!(r.text_content().unwrap().contains("not found"));
    }

    #[test]
    fn skill_manage_patch_nonexistent_skill() {
        let (_dir, agent_dir) = setup();
        let r = patch_skill(&agent_dir, "nope", "old", "new").unwrap();
        assert!(r.is_error);
        assert!(r.text_content().unwrap().contains("not found"));
    }

    // --- Delete ---

    #[test]
    fn skill_manage_delete() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("deleteme");
        create_skill(&agent_dir, "deleteme", &content).unwrap();
        assert!(agent_dir.join("deleteme").exists());

        let r = delete_skill(&agent_dir, "deleteme").unwrap();
        assert!(!r.is_error);
        assert!(!agent_dir.join("deleteme").exists());
    }

    #[test]
    fn skill_manage_delete_nonexistent() {
        let (_dir, agent_dir) = setup();
        let r = delete_skill(&agent_dir, "nope").unwrap();
        assert!(r.is_error);
        assert!(r.text_content().unwrap().contains("not found"));
    }

    #[test]
    fn skill_manage_delete_non_agent_skill_refused() {
        let (dir, agent_dir) = setup();
        // Create a skill outside the agent dir (simulating a user-installed skill)
        let parent = agent_dir.parent().unwrap();
        let user_skill = parent.join("user-skill");
        std::fs::create_dir_all(&user_skill).unwrap();
        std::fs::write(user_skill.join("SKILL.md"), "content").unwrap();

        let r = delete_skill(&agent_dir, "user-skill").unwrap();
        assert!(r.is_error);
        assert!(r.text_content().unwrap().contains("not agent-created"));
    }
}
