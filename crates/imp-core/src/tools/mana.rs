use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;

fn find_mana_dir(cwd: &Path) -> std::result::Result<std::path::PathBuf, String> {
    mana_core::discovery::find_mana_dir(cwd).map_err(|e| e.to_string())
}

fn json_output(value: &impl serde::Serialize) -> ToolOutput {
    match serde_json::to_string_pretty(value) {
        Ok(json) => ToolOutput::text(json),
        Err(e) => ToolOutput::error(format!("Failed to serialize: {e}")),
    }
}

pub struct ManaTool;

#[async_trait]
impl Tool for ManaTool {
    fn name(&self) -> &str {
        "mana"
    }
    fn label(&self) -> &str {
        "Mana"
    }
    fn description(&self) -> &str {
        "Manage work units. Actions: status, list, show, create, close, update."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "list", "show", "create", "close", "update"],
                    "description": "Action to perform"
                },
                "id": { "type": "string", "description": "Unit ID (for show, close, update)" },
                "title": { "type": "string", "description": "Unit title (for create)" },
                "verify": { "type": "string", "description": "Verify command (for create)" },
                "description": { "type": "string", "description": "Description (for create, update)" },
                "parent": { "type": "string", "description": "Parent unit ID (for create, list)" },
                "deps": { "type": "string", "description": "Comma-separated dependency IDs (for create)" },
                "status": { "type": "string", "description": "Filter by status (for list) or set status (for update)" },
                "notes": { "type": "string", "description": "Append to notes (for update — use for progress logging)" },
                "priority": { "type": "integer", "description": "Priority 0-4 (for create, update)" },
                "labels": { "type": "string", "description": "Comma-separated labels (for create)" },
                "force": { "type": "boolean", "description": "Skip verify check (for close)" },
                "reason": { "type": "string", "description": "Close reason (for close)" },
                "all": { "type": "boolean", "description": "Include closed/archived (for list)" },
            },
            "required": ["action"],
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
        let action = params["action"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'action' parameter".into()))?;

        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        match action {
            "status" => match mana_core::api::get_status(&mana_dir) {
                Ok(status) => Ok(json_output(&status)),
                Err(e) => Ok(ToolOutput::error(e.to_string())),
            },
            "list" => {
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
            "show" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("show requires 'id'".into()))?;
                match mana_core::api::get_unit(&mana_dir, id) {
                    Ok(unit) => Ok(json_output(&unit)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "create" => {
                let title = params["title"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("create requires 'title'".into()))?;
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
                    verify: params["verify"].as_str().map(|s| s.to_string()),
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
                    force: true,
                };
                match mana_core::api::create_unit(&mana_dir, create_params) {
                    Ok(result) => Ok(ToolOutput::text(format!(
                        "Created unit {}: {}",
                        result.unit.id, result.unit.title
                    ))),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "close" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("close requires 'id'".into()))?;
                let opts = mana_core::ops::close::CloseOpts {
                    reason: params["reason"].as_str().map(|s| s.to_string()),
                    force: params["force"].as_bool().unwrap_or(false),
                    defer_verify: false,
                };
                match mana_core::api::close_unit(&mana_dir, id, opts) {
                    Ok(outcome) => {
                        use mana_core::ops::close::CloseOutcome;
                        let msg = match &outcome {
                            CloseOutcome::Closed(r) => format!("Closed unit {}", r.unit.id),
                            CloseOutcome::VerifyFailed(_) => {
                                "Verify failed — unit remains open".to_string()
                            }
                            CloseOutcome::RejectedByHook { unit_id } => {
                                format!("Hook rejected {unit_id}")
                            }
                            CloseOutcome::FeatureRequiresHuman { unit_id, .. } => {
                                format!("Feature {unit_id} requires human review")
                            }
                            CloseOutcome::CircuitBreakerTripped {
                                unit_id,
                                total_attempts,
                                max,
                                ..
                            } => format!("Circuit breaker: {unit_id} ({total_attempts}/{max})"),
                            CloseOutcome::MergeConflict { files, .. } => {
                                format!("Merge conflict: {}", files.join(", "))
                            }
                            CloseOutcome::DeferredVerify { unit_id } => {
                                format!("Deferred verify for {unit_id}")
                            }
                        };
                        Ok(ToolOutput::text(msg))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "update" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("update requires 'id'".into()))?;
                let update_params = mana_core::ops::update::UpdateParams {
                    title: params["title"].as_str().map(|s| s.to_string()),
                    description: params["description"].as_str().map(|s| s.to_string()),
                    acceptance: None,
                    notes: params["notes"].as_str().map(|s| s.to_string()),
                    design: None,
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
            other => Ok(ToolOutput::error(format!(
                "Unknown action: {other}. Use: status, list, show, create, close, update"
            ))),
        }
    }
}
