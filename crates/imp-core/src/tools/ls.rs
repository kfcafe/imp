use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }
    fn label(&self) -> &str {
        "List Directory"
    }
    fn description(&self) -> &str {
        "List directory contents."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory to list" },
                "limit": { "type": "number", "description": "Maximum number of entries" }
            }
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
        Ok(ToolOutput::error("ls tool not yet implemented"))
    }
}
