use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

/// Reference content embedded from the skill file.
const LUA_REFERENCE: &str = include_str!("../../skills/lua-tools/SKILL.md");

pub struct WriteLuaTool;

#[async_trait]
impl Tool for WriteLuaTool {
    fn name(&self) -> &str {
        "write_lua"
    }

    fn label(&self) -> &str {
        "Write Lua Tool"
    }

    fn description(&self) -> &str {
        "Get the imp Lua extension API reference for writing custom tools, hooks, \
         and commands. Call this before creating or modifying any .lua file in \
         ~/.config/imp/lua/ or .imp/lua/. Returns the full API: imp.register_tool(), \
         imp.on(), imp.exec(), imp.events, file layout, and porting patterns."
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
        Ok(ToolOutput::text(LUA_REFERENCE))
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
    async fn write_lua_returns_reference() {
        let tool = WriteLuaTool;
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

        assert!(text.contains("imp.register_tool"));
        assert!(text.contains("imp.exec"));
        assert!(text.contains("imp.on"));
    }

    #[test]
    fn write_lua_is_readonly() {
        assert!(WriteLuaTool.is_readonly());
    }
}
