use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

/// Reference content embedded from the skills guide.
const SKILL_REFERENCE: &str = include_str!("../../skills/writing-skills/REFERENCE.md");

pub struct WriteSkillTool;

#[async_trait]
impl Tool for WriteSkillTool {
    fn name(&self) -> &str {
        "write_skill"
    }

    fn label(&self) -> &str {
        "Write Skill"
    }

    fn description(&self) -> &str {
        "Get the imp skill authoring reference. Call this before creating or \
         modifying a SKILL.md file. Returns the format, naming rules, description \
         writing guide, and porting patterns. Use skill_manage to create/patch/delete \
         the actual files."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _call_id: &str,
        _params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolOutput> {
        Ok(ToolOutput::text(SKILL_REFERENCE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_ctx() -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: PathBuf::from("/tmp"),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
        }
    }

    #[tokio::test]
    async fn write_skill_returns_reference() {
        let tool = WriteSkillTool;
        let result = tool.execute("c1", json!({}), test_ctx()).await.unwrap();

        assert!(!result.is_error);
        let text: String = result
            .content
            .iter()
            .filter_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();

        assert!(text.contains("SKILL.md"));
        assert!(text.contains("frontmatter"));
        assert!(text.contains("description"));
    }

    #[test]
    fn write_skill_is_readonly() {
        assert!(WriteSkillTool.is_readonly());
    }
}
