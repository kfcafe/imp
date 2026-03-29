use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::config::Config;
use crate::error::Result;
use crate::memory::MemoryStore;

const DEFAULT_MEMORY_LIMIT: usize = 2200;
const DEFAULT_USER_LIMIT: usize = 1400;

pub struct MemoryTool;

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn label(&self) -> &str {
        "Memory"
    }

    fn description(&self) -> &str {
        "Manage persistent memory across sessions. Use to save environment facts, \
         user preferences, and lessons learned. Target 'memory' for agent notes, \
         'user' for user profile."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["action", "target"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "replace", "remove"],
                    "description": "Action to perform"
                },
                "target": {
                    "type": "string",
                    "enum": ["memory", "user"],
                    "description": "Which store: 'memory' for agent notes, 'user' for user profile"
                },
                "content": {
                    "type": "string",
                    "description": "Content to add or replacement text"
                },
                "old_text": {
                    "type": "string",
                    "description": "Unique substring identifying the entry to replace or remove"
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
        let target = params["target"].as_str().unwrap_or("");

        if action.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: action"));
        }
        if target.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: target"));
        }

        let config_dir = Config::user_config_dir();
        let (path, char_limit) = match target {
            "memory" => (config_dir.join("memory.md"), DEFAULT_MEMORY_LIMIT),
            "user" => (config_dir.join("user.md"), DEFAULT_USER_LIMIT),
            other => {
                return Ok(ToolOutput::error(format!(
                    "Unknown target \"{other}\". Use \"memory\" or \"user\"."
                )));
            }
        };

        let mut store = match MemoryStore::load(&path, char_limit) {
            Ok(s) => s,
            Err(e) => return Ok(ToolOutput::error(format!("Failed to load memory: {e}"))),
        };

        let result = match action {
            "add" => {
                let content = params["content"].as_str().unwrap_or("");
                if content.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: content (for 'add' action)",
                    ));
                }
                store.add(content)?
            }
            "replace" => {
                let old_text = params["old_text"].as_str().unwrap_or("");
                let content = params["content"].as_str().unwrap_or("");
                if old_text.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: old_text (for 'replace' action)",
                    ));
                }
                if content.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: content (for 'replace' action)",
                    ));
                }
                store.replace(old_text, content)?
            }
            "remove" => {
                let old_text = params["old_text"].as_str().unwrap_or("");
                if old_text.is_empty() {
                    return Ok(ToolOutput::error(
                        "Missing required parameter: old_text (for 'remove' action)",
                    ));
                }
                store.remove(old_text)?
            }
            other => {
                return Ok(ToolOutput::error(format!(
                    "Unknown action \"{other}\". Use \"add\", \"replace\", or \"remove\"."
                )));
            }
        };

        let json_text = serde_json::to_string_pretty(&result.to_json()).unwrap_or_default();
        if result.success {
            Ok(ToolOutput::text(json_text))
        } else {
            Ok(ToolOutput::error(json_text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::sync::Arc;

    fn test_ctx() -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let dir = std::env::temp_dir();
        ToolContext {
            cwd: dir,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
        }
    }

    #[tokio::test]
    async fn memory_tool_validates_params() {
        let tool = MemoryTool;

        // Missing action
        let r = tool
            .execute("c1", json!({"target": "memory"}), test_ctx())
            .await
            .unwrap();
        assert!(r.is_error);

        // Missing target
        let r = tool
            .execute("c2", json!({"action": "add"}), test_ctx())
            .await
            .unwrap();
        assert!(r.is_error);

        // Missing content for add
        let r = tool
            .execute(
                "c3",
                json!({"action": "add", "target": "memory"}),
                test_ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
    }
}
