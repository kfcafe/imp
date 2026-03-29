use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::config::Config;
use crate::error::Result;

const LUA_REFERENCE: &str = include_str!("../../skills/lua-tools/SKILL.md");
const SKILL_REFERENCE: &str = include_str!("../../skills/writing-skills/REFERENCE.md");

pub struct ExtendTool;

#[async_trait]
impl Tool for ExtendTool {
    fn name(&self) -> &str {
        "extend"
    }

    fn label(&self) -> &str {
        "Extend Imp"
    }

    fn description(&self) -> &str {
        "Create skills and Lua tools to extend imp. Actions: \
         'lua_reference' returns the Lua tool API, \
         'skill_reference' returns the skill authoring guide, \
         'create'/'patch'/'delete' manage skill files."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["lua_reference", "skill_reference", "create", "patch", "delete"],
                    "description": "lua_reference: Lua tool API guide. skill_reference: skill authoring guide. create/patch/delete: manage skill files."
                },
                "name": {
                    "type": "string",
                    "description": "Skill name for create/patch/delete (lowercase, hyphens, e.g. 'deploy-k8s')"
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

        match action {
            "lua_reference" => Ok(ToolOutput::text(LUA_REFERENCE)),
            "skill_reference" => Ok(ToolOutput::text(SKILL_REFERENCE)),
            "create" | "patch" | "delete" => {
                let name = params["name"].as_str().unwrap_or("");
                if name.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: name (for create/patch/delete)",
                    ));
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
                                "Missing required parameter: content (for 'create')",
                            ));
                        }
                        create_skill(&agent_skills_dir, name, content)
                    }
                    "patch" => {
                        let old_text = params["old_text"].as_str().unwrap_or("");
                        let new_text = params["new_text"].as_str().unwrap_or("");
                        if old_text.is_empty() {
                            return Ok(ToolOutput::error(
                                "Missing required parameter: old_text (for 'patch')",
                            ));
                        }
                        patch_skill(&agent_skills_dir, name, old_text, new_text)
                    }
                    "delete" => delete_skill(&agent_skills_dir, name),
                    _ => unreachable!(),
                }
            }
            "" => Ok(ToolOutput::error("Missing required parameter: action")),
            other => Ok(ToolOutput::error(format!(
                "Unknown action \"{other}\". Use: lua_reference, skill_reference, create, patch, delete"
            ))),
        }
    }
}

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

fn validate_frontmatter(content: &str, expected_name: &str) -> Option<String> {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return Some("SKILL.md must start with YAML frontmatter (---)".to_string());
    }

    let after_first = &trimmed[3..];
    let end = after_first.find("\n---");
    let Some(end) = end else {
        return Some("SKILL.md frontmatter not closed (missing ---)".to_string());
    };

    let yaml_block = &after_first[..end];

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
    let agent_path = agent_dir.join(name).join("SKILL.md");
    let skill_path = if agent_path.exists() {
        agent_path
    } else {
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

    fn test_ctx() -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: PathBuf::from("/tmp"),
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: std::sync::Arc::new(crate::ui::NullInterface),
            file_cache: std::sync::Arc::new(crate::tools::FileCache::new()),
            file_tracker: std::sync::Arc::new(std::sync::Mutex::new(
                crate::tools::FileTracker::new(),
            )),
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
        }
    }

    // --- References ---

    #[tokio::test]
    async fn extend_lua_reference() {
        let tool = ExtendTool;
        let result = tool
            .execute("c1", json!({"action": "lua_reference"}), test_ctx())
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.text_content().unwrap();
        assert!(text.contains("imp.register_tool"));
        assert!(text.contains("imp.exec"));
    }

    #[tokio::test]
    async fn extend_skill_reference() {
        let tool = ExtendTool;
        let result = tool
            .execute("c2", json!({"action": "skill_reference"}), test_ctx())
            .await
            .unwrap();
        assert!(!result.is_error);
        let text = result.text_content().unwrap();
        assert!(text.contains("SKILL.md"));
        assert!(text.contains("frontmatter"));
    }

    // --- Name validation ---

    #[test]
    fn extend_valid_names() {
        assert!(validate_skill_name("deploy-k8s").is_none());
        assert!(validate_skill_name("a").is_none());
        assert!(validate_skill_name("my-skill-123").is_none());
    }

    #[test]
    fn extend_rejects_bad_names() {
        assert!(validate_skill_name("Deploy").is_some());
        assert!(validate_skill_name("-bad").is_some());
        assert!(validate_skill_name("bad-").is_some());
        assert!(validate_skill_name("bad--name").is_some());
        assert!(validate_skill_name("bad name").is_some());
        assert!(validate_skill_name(&"a".repeat(65)).is_some());
    }

    // --- Frontmatter ---

    #[test]
    fn extend_valid_frontmatter() {
        let content = "---\nname: test\ndescription: A test\n---\n\n# Body\n";
        assert!(validate_frontmatter(content, "test").is_none());
    }

    #[test]
    fn extend_frontmatter_missing_fields() {
        assert!(validate_frontmatter("---\ndescription: A test\n---\n", "test").is_some());
        assert!(validate_frontmatter("---\nname: test\n---\n", "test").is_some());
    }

    #[test]
    fn extend_frontmatter_name_mismatch() {
        let r = validate_frontmatter("---\nname: wrong\ndescription: A test\n---\n", "test");
        assert!(r.is_some());
        assert!(r.unwrap().contains("doesn't match"));
    }

    #[test]
    fn extend_no_frontmatter() {
        assert!(validate_frontmatter("# Just a heading", "test").is_some());
    }

    // --- Create ---

    #[test]
    fn extend_create_skill() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("my-skill");
        let r = create_skill(&agent_dir, "my-skill", &content).unwrap();
        assert!(!r.is_error);
        assert!(agent_dir.join("my-skill").join("SKILL.md").exists());
    }

    #[test]
    fn extend_create_duplicate() {
        let (_dir, agent_dir) = setup();
        let content = valid_skill_content("dup");
        create_skill(&agent_dir, "dup", &content).unwrap();
        let r = create_skill(&agent_dir, "dup", &content).unwrap();
        assert!(r.is_error);
    }

    // --- Patch ---

    #[test]
    fn extend_patch_skill() {
        let (_dir, agent_dir) = setup();
        create_skill(&agent_dir, "patchme", &valid_skill_content("patchme")).unwrap();
        let r = patch_skill(&agent_dir, "patchme", "Do things.", "Do better things.").unwrap();
        assert!(!r.is_error);
        let updated = std::fs::read_to_string(agent_dir.join("patchme").join("SKILL.md")).unwrap();
        assert!(updated.contains("Do better things."));
    }

    #[test]
    fn extend_patch_not_found() {
        let (_dir, agent_dir) = setup();
        create_skill(&agent_dir, "patchme", &valid_skill_content("patchme")).unwrap();
        let r = patch_skill(&agent_dir, "patchme", "NONEXISTENT", "new").unwrap();
        assert!(r.is_error);
    }

    // --- Delete ---

    #[test]
    fn extend_delete_skill() {
        let (_dir, agent_dir) = setup();
        create_skill(&agent_dir, "deleteme", &valid_skill_content("deleteme")).unwrap();
        let r = delete_skill(&agent_dir, "deleteme").unwrap();
        assert!(!r.is_error);
        assert!(!agent_dir.join("deleteme").exists());
    }

    #[test]
    fn extend_delete_nonexistent() {
        let (_dir, agent_dir) = setup();
        let r = delete_skill(&agent_dir, "nope").unwrap();
        assert!(r.is_error);
    }

    #[test]
    fn extend_delete_non_agent_refused() {
        let (_dir, agent_dir) = setup();
        let parent = agent_dir.parent().unwrap();
        let user_skill = parent.join("user-skill");
        std::fs::create_dir_all(&user_skill).unwrap();
        std::fs::write(user_skill.join("SKILL.md"), "content").unwrap();
        let r = delete_skill(&agent_dir, "user-skill").unwrap();
        assert!(r.is_error);
        assert!(r.text_content().unwrap().contains("not agent-created"));
    }
}
