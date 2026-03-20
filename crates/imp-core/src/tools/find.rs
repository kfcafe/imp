use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str { "find" }
    fn label(&self) -> &str { "Find Files" }
    fn description(&self) -> &str {
        "Search for files by glob pattern. Returns matching file paths."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern to match files" },
                "path": { "type": "string", "description": "Directory to search in" },
                "limit": { "type": "number", "description": "Maximum number of results" }
            },
            "required": ["pattern"]
        })
    }
    fn is_readonly(&self) -> bool { true }

    async fn execute(&self, _call_id: &str, _params: serde_json::Value, _ctx: ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::error("find tool not yet implemented"))
    }
}
