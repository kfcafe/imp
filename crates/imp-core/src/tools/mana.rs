use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::config::AgentMode;
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
        "Work units: status, list, show, create, close, update."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["status", "list", "show", "create", "close", "update"] },
                "id": { "type": "string" },
                "title": { "type": "string" },
                "verify": { "type": "string", "description": "Shell command, must exit 0" },
                "description": { "type": "string" },
                "parent": { "type": "string" },
                "deps": { "type": "string", "description": "Comma-separated IDs" },
                "status": { "type": "string" },
                "notes": { "type": "string", "description": "Progress log (update)" },
                "priority": { "type": "integer" },
                "labels": { "type": "string" },
                "force": { "type": "boolean" },
                "reason": { "type": "string" },
                "all": { "type": "boolean" },
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

        // Mode-based sub-action guard. The active mode is read from the IMP_MODE
        // environment variable (set by the agent runner). Full mode (default) allows
        // everything; restricted modes block specific sub-actions at execution time.
        let mode = std::env::var("IMP_MODE")
            .ok()
            .and_then(|v| AgentMode::from_name(&v))
            .unwrap_or(AgentMode::Full);

        if !mode.allows_mana_action(action) {
            let mode_name = format!("{mode:?}").to_lowercase();
            return Ok(ToolOutput::error(format!(
                "Mana action '{action}' is not available in {mode_name} mode"
            )));
        }

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::mpsc;

    use super::ManaTool;
    use crate::tools::{FileCache, FileTracker, Tool, ToolContext, ToolUpdate};
    use crate::ui::NullInterface;

    /// Outcome of a mana tool call in test context.
    enum ManaResult {
        /// Mode guard fired — action blocked.
        ModeBlocked(String),
        /// Mode guard passed — action was attempted (may fail for other reasons).
        Attempted(crate::tools::ToolOutput),
    }

    /// Run the ManaTool with `IMP_MODE` set to `mode_name` for the duration of the call.
    ///
    /// Returns `ManaResult::ModeBlocked` if the mode guard fires, or
    /// `ManaResult::Attempted` if the action was allowed and execution proceeded
    /// (the inner ToolOutput may itself be an error from missing .mana/, etc.).
    ///
    /// The env var is reset afterwards so tests don't bleed into each other.
    /// Tests in this module must run with `--test-threads=1` because env vars are
    /// process-global (the verify gate enforces this).
    // Serialize env-var-mutating tests to prevent IMP_MODE race conditions.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    async fn run_with_mode(mode_name: &str, action: &str) -> ManaResult {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("IMP_MODE").ok();
        std::env::set_var("IMP_MODE", mode_name);

        let dir = tempfile::tempdir().unwrap();
        // Create a minimal .mana/ so the tool doesn't bail with "No .mana/ directory"
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "next_id: 1\n").unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
        };

        let tool = ManaTool;
        let outcome = tool
            .execute("call_1", json!({ "action": action }), ctx)
            .await;

        // Restore previous state
        match prev {
            Some(v) => std::env::set_var("IMP_MODE", v),
            None => std::env::remove_var("IMP_MODE"),
        }

        match outcome {
            // Infrastructure error (e.g. no .mana/ dir) — action was allowed, it just failed.
            Err(crate::error::Error::Tool(msg)) => {
                ManaResult::Attempted(crate::tools::ToolOutput::error(msg))
            }
            Err(e) => ManaResult::Attempted(crate::tools::ToolOutput::error(e.to_string())),
            Ok(output) => {
                // Distinguish mode guard errors from other ToolOutput errors.
                if output.is_error {
                    if let Some(text) = output.text_content() {
                        if text.contains("mode") && text.contains(action) {
                            return ManaResult::ModeBlocked(text.to_string());
                        }
                    }
                }
                ManaResult::Attempted(output)
            }
        }
    }

    #[tokio::test]
    async fn agent_mode_mana_worker_blocks_create() {
        match run_with_mode("worker", "create").await {
            ManaResult::ModeBlocked(_) => {} // correct
            ManaResult::Attempted(out) => {
                panic!(
                    "worker should block 'create', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn agent_mode_mana_planner_allows_create() {
        // Planner can create — it reaches find_mana_dir which fails (no .mana),
        // but the mode guard must NOT fire.
        match run_with_mode("planner", "create").await {
            ManaResult::Attempted(_) => {} // correct — mode guard passed
            ManaResult::ModeBlocked(msg) => {
                panic!("planner should allow 'create' but was blocked: {msg}")
            }
        }
    }

    #[tokio::test]
    async fn agent_mode_mana_planner_blocks_close() {
        match run_with_mode("planner", "close").await {
            ManaResult::ModeBlocked(_) => {} // correct
            ManaResult::Attempted(out) => {
                panic!(
                    "planner should block 'close', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn agent_mode_mana_auditor_allows_show() {
        // Auditor can show — error will be from missing .mana/, not mode guard.
        match run_with_mode("auditor", "show").await {
            ManaResult::Attempted(_) => {} // correct — mode guard passed
            ManaResult::ModeBlocked(msg) => {
                panic!("auditor should allow 'show' but was blocked: {msg}")
            }
        }
    }

    #[tokio::test]
    async fn agent_mode_mana_auditor_blocks_update() {
        match run_with_mode("auditor", "update").await {
            ManaResult::ModeBlocked(_) => {} // correct
            ManaResult::Attempted(out) => {
                panic!(
                    "auditor should block 'update', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn agent_mode_mana_orchestrator_allows_all() {
        // Orchestrator allows all standard actions — none should hit the mode guard.
        for action in &["status", "list", "show", "create", "close", "update"] {
            match run_with_mode("orchestrator", action).await {
                ManaResult::Attempted(_) => {} // correct
                ManaResult::ModeBlocked(msg) => {
                    panic!("orchestrator should allow '{action}' but was blocked: {msg}")
                }
            }
        }
    }
}
