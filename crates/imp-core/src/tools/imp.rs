use async_trait::async_trait;
use imp_llm::{AssistantMessage, ContentBlock};
use imp_llm::ThinkingLevel;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::config::AgentMode;
use crate::error::{Error, Result};
use crate::imp_session::{ImpSession, SessionChoice, SessionOptions};
use crate::mana_worker::{self, WorkerRunOptions};

pub struct ImpTool;

#[async_trait]
impl Tool for ImpTool {
    fn name(&self) -> &str {
        "imp"
    }

    fn label(&self) -> &str {
        "Delegate Work"
    }

    fn description(&self) -> &str {
        "Delegate work to another imp worker. Supports durable mana-unit delegation and bounded ad hoc helper delegation."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["delegate"],
                    "description": "Delegate work to another imp worker"
                },
                "mode": {
                    "type": "string",
                    "enum": ["unit", "ad_hoc"],
                    "description": "Delegation mode. 'unit' is implemented now; 'ad_hoc' is reserved for follow-up work."
                },
                "unit_id": {
                    "type": "string",
                    "description": "Mana unit id to execute when mode='unit'"
                },
                "mana_dir": {
                    "type": "string",
                    "description": "Optional explicit mana directory or project root"
                },
                "defer_verify": {
                    "type": "boolean",
                    "description": "Skip inline verify/close when true"
                },
                "model": { "type": "string" },
                "provider": { "type": "string" },
                "thinking": { "type": "string" },
                "max_turns": { "type": "number" },
                "max_tokens": { "type": "number" },
                "system_prompt": { "type": "string" },
                "no_tools": { "type": "boolean" },
                "idempotency_key": {
                    "type": "string",
                    "description": "Optional caller-supplied dedupe key"
                }
            },
            "required": ["action", "mode"],
            "allOf": [
                {
                    "if": { "properties": { "mode": { "const": "unit" } } },
                    "then": { "required": ["unit_id"] }
                },
                {
                    "if": { "properties": { "mode": { "const": "ad_hoc" } } },
                    "then": { "required": ["prompt"] }
                }
            ]
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
        if !matches!(ctx.mode, AgentMode::Full | AgentMode::Orchestrator) {
            return Ok(ToolOutput::error(
                "The imp tool is only available in Full or Orchestrator mode.",
            ));
        }

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if action != "delegate" {
            return Ok(ToolOutput::error(
                "Unsupported imp action. Expected action='delegate'.",
            ));
        }

        let mode = params.get("mode").and_then(|v| v.as_str()).unwrap_or("");
        match mode {
            "unit" => execute_unit_delegate(params, ctx).await,
            "ad_hoc" => execute_ad_hoc_delegate(params, ctx).await,
            _ => Ok(ToolOutput::error(
                "Unsupported imp mode. Expected mode='unit' or mode='ad_hoc'.",
            )),
        }
    }
}

async fn execute_unit_delegate(
    params: serde_json::Value,
    ctx: ToolContext,
) -> Result<ToolOutput> {
    let unit_id = params
        .get("unit_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Tool("Missing required parameter: unit_id".into()))?;

    let mana_dir_override = params
        .get("mana_dir")
        .and_then(|v| v.as_str())
        .map(|raw| super::resolve_path(&ctx.cwd, raw));

    let assignment = mana_worker::load_assignment_with_mana_dir(
        &ctx.cwd,
        unit_id,
        mana_dir_override.as_deref(),
    )
    .map_err(|e| Error::Tool(e.to_string()))?;

    let options = WorkerRunOptions {
        cwd: ctx.cwd.clone(),
        model: params
            .get("model")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .or_else(|| assignment.model.clone()),
        provider: params
            .get("provider")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        api_key: None,
        thinking: parse_optional_thinking(&params)?,
        max_turns: params
            .get("max_turns")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        max_tokens: params
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        system_prompt: params
            .get("system_prompt")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        no_tools: params
            .get("no_tools")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        mana_dir_override,
        defer_verify: params
            .get("defer_verify")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        lua_loader: None,
    };

    let idempotency_key = params
        .get("idempotency_key")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);

    let outcome = mana_worker::run_worker_assignment(assignment.clone(), options)
        .await
        .map_err(|e| Error::Tool(e.to_string()))?;

    let status = format!("{:?}", outcome.result.status).to_lowercase();
    let summary = outcome
        .result
        .summary
        .clone()
        .unwrap_or_else(|| format!("Delegated unit {} finished.", assignment.id));

    let content = match outcome.result.status {
        tower_contracts::worker::WorkerStatus::Completed => {
            format!("Delegated unit {} completed successfully.", assignment.id)
        }
        tower_contracts::worker::WorkerStatus::AwaitingVerify => {
            format!("Delegated unit {} completed and is awaiting verify.", assignment.id)
        }
        tower_contracts::worker::WorkerStatus::Failed => {
            format!("Delegated unit {} finished but verify failed.", assignment.id)
        }
        tower_contracts::worker::WorkerStatus::Blocked => {
            format!("Delegated unit {} is blocked.", assignment.id)
        }
        tower_contracts::worker::WorkerStatus::Cancelled => {
            format!("Delegated unit {} was cancelled.", assignment.id)
        }
    };

    Ok(ToolOutput {
        content: vec![ContentBlock::Text { text: content }],
        details: json!({
            "tool": "imp",
            "action": "delegate",
            "delegation_mode": "unit",
            "durable": true,
            "unit_id": assignment.id,
            "status": status,
            "summary": summary,
            "verify_passed": outcome.verify_passed,
            "closed_after_verify": outcome.closed_after_verify,
            "model": outcome.result.model,
            "provider": params.get("provider").and_then(|v| v.as_str()),
            "prefilled_file_count": outcome.prefilled_files.len(),
            "idempotency_key": idempotency_key,
        }),
        is_error: false,
    })
}

async fn execute_ad_hoc_delegate(
    params: serde_json::Value,
    ctx: ToolContext,
) -> Result<ToolOutput> {
    let prompt = params
        .get("prompt")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Tool("Missing required parameter: prompt".into()))?;

    let idempotency_key = params
        .get("idempotency_key")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);

    let session_options = SessionOptions {
        cwd: ctx.cwd.clone(),
        model: params
            .get("model")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        provider: params
            .get("provider")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        api_key: None,
        thinking: parse_optional_thinking(&params)?,
        mode: Some(AgentMode::Reviewer),
        max_turns: params
            .get("max_turns")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        max_tokens: params
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        system_prompt: params
            .get("system_prompt")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        no_tools: false,
        session: SessionChoice::InMemory,
        task: None,
        facts: Vec::new(),
        lua_loader: None,
        ui: Some(ctx.ui.clone()),
        auth_path: None,
        context_prefill: Vec::new(),
    };

    let mut session = ImpSession::create(session_options)
        .await
        .map_err(|e| Error::Tool(e.to_string()))?;
    session
        .prompt_and_wait(prompt)
        .await
        .map_err(|e| Error::Tool(e.to_string()))?;

    let final_text = extract_final_assistant_text(&session);
    let summary = final_text
        .clone()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| "Transient helper run completed.".to_string());

    Ok(ToolOutput {
        content: vec![ContentBlock::Text {
            text: "Transient helper run completed.".to_string(),
        }],
        details: json!({
            "tool": "imp",
            "action": "delegate",
            "delegation_mode": "ad_hoc",
            "durable": false,
            "status": "completed",
            "summary": summary,
            "final_text": final_text,
            "model": session.model().meta.id.clone(),
            "provider": session.model().meta.provider.clone(),
            "idempotency_key": idempotency_key,
        }),
        is_error: false,
    })
}

fn extract_final_assistant_text(session: &ImpSession) -> Option<String> {
    session
        .session_manager()
        .get_active_messages()
        .iter()
        .rev()
        .find_map(|message| match message {
            imp_llm::Message::Assistant(AssistantMessage { content, .. }) => {
                let text = content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            _ => None,
        })
}

fn parse_optional_thinking(params: &serde_json::Value) -> Result<Option<ThinkingLevel>> {
    let Some(raw) = params.get("thinking").and_then(|v| v.as_str()) else {
        return Ok(None);
    };

    let level = match raw.to_ascii_lowercase().as_str() {
        "off" | "none" => ThinkingLevel::Off,
        "low" => ThinkingLevel::Low,
        "medium" | "med" => ThinkingLevel::Medium,
        "high" => ThinkingLevel::High,
        other => {
            return Err(Error::Tool(format!(
                "Invalid thinking level '{other}'. Expected off, low, medium, or high.",
            )))
        }
    };

    Ok(Some(level))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    fn test_ctx(mode: AgentMode) -> ToolContext {
        let (update_tx, _update_rx) = tokio::sync::mpsc::channel(1);
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(1);
        ToolContext {
            cwd: std::env::temp_dir(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx,
            command_tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(super::super::FileCache::new()),
            checkpoint_state: Arc::new(super::super::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(super::super::FileTracker::new())),
            mode,
            read_max_lines: 0,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
        }
    }

    #[test]
    fn schema_requires_unit_id_for_unit_mode() {
        let schema = ImpTool.parameters();
        let args = json!({"action": "delegate", "mode": "unit"});
        let err = super::super::validate_tool_args(&schema, &args).unwrap_err();
        assert!(format!("{err}").contains("unit_id"));
    }

    #[test]
    fn schema_requires_prompt_for_ad_hoc_mode() {
        let schema = ImpTool.parameters();
        let args = json!({"action": "delegate", "mode": "ad_hoc"});
        let err = super::super::validate_tool_args(&schema, &args).unwrap_err();
        assert!(format!("{err}").contains("prompt"));
    }

    #[tokio::test]
    async fn blocked_modes_fail_clearly() {
        let tool = ImpTool;
        let out = tool
            .execute(
                "call-1",
                json!({"action": "delegate", "mode": "unit", "unit_id": "123"}),
                test_ctx(AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(out.is_error);
        let text = out.text_content().unwrap_or_default();
        assert!(text.contains("Full or Orchestrator"));
    }

    #[tokio::test]
    async fn ad_hoc_returns_transient_details() {
        let tool = ImpTool;
        let out = tool
            .execute(
                "call-1",
                json!({"action": "delegate", "mode": "ad_hoc", "prompt": "Say only the word transient."}),
                test_ctx(AgentMode::Orchestrator),
            )
            .await
            .unwrap();
        assert!(!out.is_error);
        assert_eq!(out.details.get("delegation_mode").and_then(|v| v.as_str()), Some("ad_hoc"));
        assert_eq!(out.details.get("durable").and_then(|v| v.as_bool()), Some(false));
        assert!(out.details.get("final_text").is_some());
    }
}
