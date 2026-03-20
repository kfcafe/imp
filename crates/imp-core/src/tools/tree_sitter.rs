use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

// TODO: Add tree-sitter dependency and implement native AST parsing.
// For now, these are stubs that will be filled in by the tree-sitter child bean.

pub struct ProbeSearchTool;

#[async_trait]
impl Tool for ProbeSearchTool {
    fn name(&self) -> &str { "probe_search" }
    fn label(&self) -> &str { "Semantic Code Search" }
    fn description(&self) -> &str {
        "Semantic code search using ripgrep + tree-sitter AST parsing."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "path": { "type": "string", "description": "Directory or file to search" },
                "language": { "type": "string", "description": "Limit to language" },
                "maxResults": { "type": "number", "description": "Maximum number of results" }
            },
            "required": ["query"]
        })
    }
    fn is_readonly(&self) -> bool { true }

    async fn execute(&self, _call_id: &str, _params: serde_json::Value, _ctx: ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::error("probe_search tool not yet implemented"))
    }
}

pub struct ProbeExtractTool;

#[async_trait]
impl Tool for ProbeExtractTool {
    fn name(&self) -> &str { "probe_extract" }
    fn label(&self) -> &str { "Extract Code Block" }
    fn description(&self) -> &str {
        "Extract complete code blocks from files using tree-sitter AST parsing."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "targets": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "File:line or file#symbol targets"
                },
                "context": { "type": "number", "description": "Context lines" }
            },
            "required": ["targets"]
        })
    }
    fn is_readonly(&self) -> bool { true }

    async fn execute(&self, _call_id: &str, _params: serde_json::Value, _ctx: ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::error("probe_extract tool not yet implemented"))
    }
}

pub struct ScanTool;

#[async_trait]
impl Tool for ScanTool {
    fn name(&self) -> &str { "scan" }
    fn label(&self) -> &str { "Scan Code Structure" }
    fn description(&self) -> &str {
        "Extract code structure (types, functions, imports) from source files."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["extract", "build", "scan"] },
                "files": { "type": "array", "items": { "type": "string" } },
                "directory": { "type": "string" },
                "task": { "type": "string" }
            },
            "required": ["action"]
        })
    }
    fn is_readonly(&self) -> bool { true }

    async fn execute(&self, _call_id: &str, _params: serde_json::Value, _ctx: ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::error("scan tool not yet implemented"))
    }
}

pub struct AstGrepTool;

#[async_trait]
impl Tool for AstGrepTool {
    fn name(&self) -> &str { "ast_grep" }
    fn label(&self) -> &str { "AST Pattern Search" }
    fn description(&self) -> &str {
        "Structural code search using AST patterns."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "AST pattern to search for" },
                "path": { "type": "string", "description": "File or directory to search" },
                "lang": { "type": "string", "description": "Language" },
                "replace": { "type": "string", "description": "Replacement pattern" }
            },
            "required": ["pattern"]
        })
    }
    fn is_readonly(&self) -> bool {
        // Mixed: readonly for search, write for replace
        false
    }

    async fn execute(&self, _call_id: &str, _params: serde_json::Value, _ctx: ToolContext) -> Result<ToolOutput> {
        Ok(ToolOutput::error("ast_grep tool not yet implemented"))
    }
}
