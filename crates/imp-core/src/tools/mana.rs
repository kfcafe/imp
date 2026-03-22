use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

/// Find the .mana/ directory from the agent's working directory.
fn find_mana_dir(cwd: &Path) -> std::result::Result<std::path::PathBuf, String> {
    mana_core::discovery::find_mana_dir(cwd).map_err(|e| e.to_string())
}

/// Serialize any serde-serializable value to pretty JSON text output.
fn json_output(value: &impl serde::Serialize) -> ToolOutput {
    match serde_json::to_string_pretty(value) {
        Ok(json) => ToolOutput::text(json),
        Err(e) => ToolOutput::error(format!("Failed to serialize: {e}")),
    }
}

// ---------------------------------------------------------------------------
// mana_status
// ---------------------------------------------------------------------------

pub struct ManaStatusTool;

#[async_trait]
impl Tool for ManaStatusTool {
    fn name(&self) -> &str {
        "mana_status"
    }
    fn label(&self) -> &str {
        "Mana Status"
    }
    fn description(&self) -> &str {
        "Show project status: open units, blocked units, claimed units, and recent activity."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({ "type": "object", "properties": {} })
    }
    fn is_readonly(&self) -> bool {
        true
    }
    async fn execute(
        &self,
        _call_id: &str,
        _params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;
        match mana_core::api::get_status(&mana_dir) {
            Ok(status) => Ok(json_output(&status)),
            Err(e) => Ok(ToolOutput::error(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// mana_list
// ---------------------------------------------------------------------------

pub struct ManaListTool;

#[async_trait]
impl Tool for ManaListTool {
    fn name(&self) -> &str {
        "mana_list"
    }
    fn label(&self) -> &str {
        "Mana List"
    }
    fn description(&self) -> &str {
        "List units. Filter by status, priority, parent, or label."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "status": { "type": "string", "description": "Filter: open, closed, in_progress" },
                "parent": { "type": "string", "description": "Filter to children of this parent ID" },
                "all": { "type": "boolean", "description": "Include closed/archived units" },
            },
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        let list_params = mana_core::ops::list::ListParams {
            status: params["status"].as_str().map(|s| s.to_string()),
            priority: params["priority"].as_u64().map(|p| p as u8),
            parent: params["parent"].as_str().map(|s| s.to_string()),
            label: params["label"].as_str().map(|s| s.to_string()),
            assignee: None,
            current_user: None,
            include_closed: params["all"].as_bool().unwrap_or(false),
        };

        match mana_core::api::list_units(&mana_dir, &list_params) {
            Ok(entries) => Ok(json_output(&entries)),
            Err(e) => Ok(ToolOutput::error(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// mana_show
// ---------------------------------------------------------------------------

pub struct ManaShowTool;

#[async_trait]
impl Tool for ManaShowTool {
    fn name(&self) -> &str {
        "mana_show"
    }
    fn label(&self) -> &str {
        "Mana Show"
    }
    fn description(&self) -> &str {
        "Show full details of a unit by ID, including description, notes, verify, and attempt history."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Unit ID to show" },
            },
            "required": ["id"],
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let id = params["id"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'id' parameter".into()))?;
        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        match mana_core::api::get_unit(&mana_dir, id) {
            Ok(unit) => Ok(json_output(&unit)),
            Err(e) => Ok(ToolOutput::error(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// mana_create
// ---------------------------------------------------------------------------

pub struct ManaCreateTool;

#[async_trait]
impl Tool for ManaCreateTool {
    fn name(&self) -> &str {
        "mana_create"
    }
    fn label(&self) -> &str {
        "Mana Create"
    }
    fn description(&self) -> &str {
        "Create a new unit. Requires a title and verify command. Returns the created unit as JSON."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Unit title" },
                "verify": { "type": "string", "description": "Shell command that must exit 0 to close the unit" },
                "description": { "type": "string", "description": "Full description (agent prompt when dispatched)" },
                "parent": { "type": "string", "description": "Parent unit ID" },
                "deps": { "type": "string", "description": "Comma-separated dependency IDs" },
                "priority": { "type": "integer", "description": "Priority 0-4 (P0=critical, P4=low)" },
                "labels": { "type": "string", "description": "Comma-separated labels" },
            },
            "required": ["title", "verify"],
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let title = params["title"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'title' parameter".into()))?;
        let verify = params["verify"].as_str().map(|s| s.to_string());

        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        let deps: Vec<String> = params["deps"]
            .as_str()
            .map(|s| {
                s.split(',')
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let labels: Vec<String> = params["labels"]
            .as_str()
            .map(|s| {
                s.split(',')
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let create_params = mana_core::ops::create::CreateParams {
            title: title.to_string(),
            description: params["description"].as_str().map(|s| s.to_string()),
            acceptance: None,
            notes: None,
            design: None,
            verify,
            priority: params["priority"].as_u64().map(|p| p as u8),
            labels,
            assignee: None,
            dependencies: deps,
            parent: params["parent"].as_str().map(|s| s.to_string()),
            produces: Vec::new(),
            requires: Vec::new(),
            paths: Vec::new(),
            on_fail: None,
            fail_first: false,
            feature: false,
            verify_timeout: None,
            decisions: Vec::new(),
            force: true, // skip verify lint in tool context
        };

        match mana_core::api::create_unit(&mana_dir, create_params) {
            Ok(result) => Ok(ToolOutput::text(format!(
                "Created unit {}: {}",
                result.unit.id, result.unit.title
            ))),
            Err(e) => Ok(ToolOutput::error(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// mana_close
// ---------------------------------------------------------------------------

pub struct ManaCloseTool;

#[async_trait]
impl Tool for ManaCloseTool {
    fn name(&self) -> &str {
        "mana_close"
    }
    fn label(&self) -> &str {
        "Mana Close"
    }
    fn description(&self) -> &str {
        "Close a unit. Runs verify command first — must pass to close unless force=true."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Unit ID to close" },
                "force": { "type": "boolean", "description": "Skip verify check" },
                "reason": { "type": "string", "description": "Close reason" },
            },
            "required": ["id"],
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let id = params["id"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'id' parameter".into()))?;
        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        let opts = mana_core::ops::close::CloseOpts {
            reason: params["reason"].as_str().map(|s| s.to_string()),
            force: params["force"].as_bool().unwrap_or(false),
        };

        match mana_core::api::close_unit(&mana_dir, id, opts) {
            Ok(outcome) => {
                let msg = match &outcome {
                    mana_core::ops::close::CloseOutcome::Closed(r) => {
                        format!("Closed unit {}", r.unit.id)
                    }
                    mana_core::ops::close::CloseOutcome::VerifyFailed(_) => {
                        "Verify failed — unit remains open".to_string()
                    }
                    mana_core::ops::close::CloseOutcome::RejectedByHook { unit_id } => {
                        format!("Pre-close hook rejected {unit_id}")
                    }
                    mana_core::ops::close::CloseOutcome::FeatureRequiresHuman {
                        unit_id, ..
                    } => format!("Feature {unit_id} requires human review"),
                    mana_core::ops::close::CloseOutcome::CircuitBreakerTripped {
                        unit_id,
                        total_attempts,
                        max,
                        ..
                    } => format!("Circuit breaker: {unit_id} ({total_attempts}/{max} attempts)"),
                    mana_core::ops::close::CloseOutcome::MergeConflict { files, .. } => {
                        format!("Merge conflict: {}", files.join(", "))
                    }
                };
                Ok(ToolOutput::text(msg))
            }
            Err(e) => Ok(ToolOutput::error(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// mana_update
// ---------------------------------------------------------------------------

pub struct ManaUpdateTool;

#[async_trait]
impl Tool for ManaUpdateTool {
    fn name(&self) -> &str {
        "mana_update"
    }
    fn label(&self) -> &str {
        "Mana Update"
    }
    fn description(&self) -> &str {
        "Update a unit's fields. Use notes to log progress during execution."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Unit ID to update" },
                "notes": { "type": "string", "description": "Append to notes (progress logging)" },
                "status": { "type": "string", "description": "New status: open, in_progress, closed" },
                "title": { "type": "string", "description": "New title" },
                "priority": { "type": "integer", "description": "New priority 0-4" },
            },
            "required": ["id"],
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }
    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let id = params["id"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'id' parameter".into()))?;
        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        let update_params = mana_core::ops::update::UpdateParams {
            title: params["title"].as_str().map(|s| s.to_string()),
            description: params["description"].as_str().map(|s| s.to_string()),
            acceptance: params["acceptance"].as_str().map(|s| s.to_string()),
            notes: params["notes"].as_str().map(|s| s.to_string()),
            design: params["design"].as_str().map(|s| s.to_string()),
            status: params["status"].as_str().map(|s| s.to_string()),
            priority: params["priority"].as_u64().map(|p| p as u8),
            assignee: None,
            add_label: None,
            remove_label: None,
            decisions: Vec::new(),
            resolve_decisions: Vec::new(),
        };

        match mana_core::api::update_unit(&mana_dir, id, update_params) {
            Ok(result) => Ok(ToolOutput::text(format!(
                "Updated unit {}: {}",
                result.unit.id, result.unit.title
            ))),
            Err(e) => Ok(ToolOutput::error(e.to_string())),
        }
    }
}
