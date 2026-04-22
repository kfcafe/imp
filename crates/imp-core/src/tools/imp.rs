use async_trait::async_trait;
use imp_llm::ThinkingLevel;
use imp_llm::{AssistantMessage, ContentBlock};
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
        "Spawn Worker"
    }

    fn description(&self) -> &str {
        "Spawn another imp worker. Supports durable mana-unit worker runs and bounded ad hoc helper sessions."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "delegate"],
                    "description": "Preferred: spawn another imp worker. `delegate` remains accepted as a compatibility alias during migration."
                },
                "mode": {
                    "type": "string",
                    "enum": ["unit", "ad_hoc"],
                    "description": "Worker mode. 'unit' runs a tracked mana unit; 'ad_hoc' runs a bounded transient helper session."
                },
                "unit_id": {
                    "type": "string",
                    "description": "Mana unit id to execute when mode='unit'"
                },
                "prompt": {
                    "type": "string",
                    "description": "Prompt to run when mode='ad_hoc'"
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
            "required": ["action", "mode"]
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

        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(action, "spawn" | "delegate") {
            return Ok(ToolOutput::error(
                "Unsupported imp action. Expected action='spawn' (preferred) or action='delegate' (compatibility alias).",
            ));
        }

        let mode = params.get("mode").and_then(|v| v.as_str()).unwrap_or("");
        match mode {
            "unit" => execute_unit_spawn(params, ctx).await,
            "ad_hoc" => execute_ad_hoc_spawn(params, ctx).await,
            _ => Ok(ToolOutput::error(
                "Unsupported imp mode. Expected mode='unit' or mode='ad_hoc'.",
            )),
        }
    }
}

struct AdHocSpawnOutcome {
    status: &'static str,
    summary: String,
    content: String,
    success: bool,
    final_text: Option<String>,
}

fn build_spawn_details(
    spawn_mode: &str,
    durable: bool,
    status: impl Into<String>,
    success: bool,
    summary: impl Into<String>,
    model: serde_json::Value,
    provider: serde_json::Value,
    idempotency_key: Option<String>,
    mode_details: serde_json::Value,
) -> serde_json::Value {
    json!({
        "tool": "imp",
        "action": "spawn",
        "spawn_mode": spawn_mode,
        "delegation_mode": spawn_mode,
        "durable": durable,
        "status": status.into(),
        "success": success,
        "summary": summary.into(),
        "model": model,
        "provider": provider,
        "idempotency_key": idempotency_key,
        "mode_details": mode_details,
    })
}

fn build_ad_hoc_spawn_outcome(final_text: Option<String>) -> AdHocSpawnOutcome {
    match final_text.filter(|text| !text.trim().is_empty()) {
        Some(text) => AdHocSpawnOutcome {
            status: "completed",
            summary: text.clone(),
            content: text.clone(),
            success: true,
            final_text: Some(text),
        },
        None => AdHocSpawnOutcome {
            status: "completed_no_output",
            summary: "Transient helper worker completed with no final text.".to_string(),
            content: "Transient helper worker completed with no final text.".to_string(),
            success: true,
            final_text: None,
        },
    }
}

fn unit_worker_status_is_error(status: tower_contracts::worker::WorkerStatus) -> bool {
    matches!(
        status,
        tower_contracts::worker::WorkerStatus::Failed
            | tower_contracts::worker::WorkerStatus::Blocked
            | tower_contracts::worker::WorkerStatus::Cancelled
    )
}

async fn execute_unit_spawn(params: serde_json::Value, ctx: ToolContext) -> Result<ToolOutput> {
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

    let assignment =
        mana_worker::load_assignment_with_mana_dir(&ctx.cwd, unit_id, mana_dir_override.as_deref())
            .map_err(|e| Error::Tool(e.to_string()))?;

    let options = WorkerRunOptions {
        cwd: ctx.cwd.clone(),
        model_override: None,
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
        lua_loader: ctx.lua_tool_loader.clone(),
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
        .unwrap_or_else(|| format!("Spawned worker for unit {} finished.", assignment.id));

    let content = outcome
        .result
        .summary
        .clone()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| match outcome.result.status {
            tower_contracts::worker::WorkerStatus::Completed => {
                format!(
                    "Spawned worker for unit {} completed successfully.",
                    assignment.id
                )
            }
            tower_contracts::worker::WorkerStatus::AwaitingVerify => {
                format!(
                    "Spawned worker for unit {} completed and is awaiting verify.",
                    assignment.id
                )
            }
            tower_contracts::worker::WorkerStatus::Failed => {
                format!("Spawned worker for unit {} failed.", assignment.id)
            }
            tower_contracts::worker::WorkerStatus::Blocked => {
                format!("Spawned worker for unit {} is blocked.", assignment.id)
            }
            tower_contracts::worker::WorkerStatus::Cancelled => {
                format!("Spawned worker for unit {} was cancelled.", assignment.id)
            }
        });

    let success = !unit_worker_status_is_error(outcome.result.status);

    Ok(ToolOutput {
        content: vec![ContentBlock::Text { text: content }],
        details: build_spawn_details(
            "unit",
            true,
            status,
            success,
            summary,
            json!(outcome.result.model),
            json!(params.get("provider").and_then(|v| v.as_str())),
            idempotency_key,
            json!({
                "unit_id": assignment.id,
                "verify_passed": outcome.verify_passed,
                "verify_output": outcome.verify_output,
                "verifier_result": outcome.verifier_result,
                "closed_after_verify": outcome.closed_after_verify,
                "prefilled_file_count": outcome.prefilled_files.len(),
            }),
        ),
        is_error: !success,
    })
}

async fn execute_ad_hoc_spawn(params: serde_json::Value, ctx: ToolContext) -> Result<ToolOutput> {
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
        model_override: None,
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
    let outcome = build_ad_hoc_spawn_outcome(final_text);

    Ok(ToolOutput {
        content: vec![ContentBlock::Text {
            text: outcome.content,
        }],
        details: build_spawn_details(
            "ad_hoc",
            false,
            outcome.status,
            outcome.success,
            outcome.summary,
            json!(session.model().meta.id.clone()),
            json!(session.model().meta.provider.clone()),
            idempotency_key,
            json!({
                "final_text": outcome.final_text,
            }),
        ),
        is_error: false,
    })
}

fn extract_final_assistant_text_from_messages(messages: &[imp_llm::Message]) -> Option<String> {
    messages.iter().rev().find_map(|message| match message {
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

fn extract_final_assistant_text(session: &ImpSession) -> Option<String> {
    extract_final_assistant_text_from_messages(&session.session_manager().get_active_messages())
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
            lua_tool_loader: None,
            mode,
            read_max_lines: 0,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
        }
    }

    #[test]
    fn schema_is_plain_object_without_top_level_all_of() {
        let schema = ImpTool.parameters();
        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
        assert!(schema.get("allOf").is_none());
        assert_eq!(
            schema["properties"]["prompt"]["type"].as_str(),
            Some("string")
        );
    }

    #[tokio::test]
    async fn unit_mode_requires_unit_id_at_runtime() {
        let tool = ImpTool;
        let result = tool
            .execute(
                "call-1",
                json!({"action": "spawn", "mode": "unit"}),
                test_ctx(AgentMode::Orchestrator),
            )
            .await;
        match result {
            Ok(_) => panic!("expected missing unit_id to return an error"),
            Err(err) => assert!(err.to_string().contains("unit_id")),
        }
    }

    #[tokio::test]
    async fn ad_hoc_mode_requires_prompt_at_runtime() {
        let tool = ImpTool;
        let result = tool
            .execute(
                "call-1",
                json!({"action": "spawn", "mode": "ad_hoc"}),
                test_ctx(AgentMode::Orchestrator),
            )
            .await;
        match result {
            Ok(_) => panic!("expected missing prompt to return an error"),
            Err(err) => assert!(err.to_string().contains("prompt")),
        }
    }

    #[tokio::test]
    async fn blocked_modes_fail_clearly() {
        let tool = ImpTool;
        let out = tool
            .execute(
                "call-1",
                json!({"action": "spawn", "mode": "unit", "unit_id": "123"}),
                test_ctx(AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(out.is_error);
        let text = out.text_content().unwrap_or_default();
        assert!(text.contains("Full or Orchestrator"));
    }

    #[tokio::test]
    async fn delegate_action_remains_accepted_as_compatibility_alias() {
        let tool = ImpTool;
        let result = tool
            .execute(
                "call-1",
                json!({"action": "delegate", "mode": "unit"}),
                test_ctx(AgentMode::Orchestrator),
            )
            .await;
        match result {
            Ok(_) => panic!("expected missing unit_id to return an error"),
            Err(err) => assert!(err.to_string().contains("unit_id")),
        }
    }

    #[test]
    fn build_spawn_details_keeps_shared_fields_and_groups_mode_specific_data() {
        let details = build_spawn_details(
            "ad_hoc",
            false,
            "completed",
            true,
            "summary",
            json!("model-x"),
            json!("provider-y"),
            Some("idem-1".to_string()),
            json!({"final_text": "hello"}),
        );

        assert_eq!(details.get("spawn_mode").and_then(|v| v.as_str()), Some("ad_hoc"));
        assert_eq!(details.get("delegation_mode").and_then(|v| v.as_str()), Some("ad_hoc"));
        assert_eq!(details.get("status").and_then(|v| v.as_str()), Some("completed"));
        assert_eq!(details.get("success").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            details
                .get("mode_details")
                .and_then(|v| v.get("final_text"))
                .and_then(|v| v.as_str()),
            Some("hello")
        );
    }

    #[test]
    fn build_ad_hoc_spawn_outcome_uses_final_text_when_present() {
        let outcome = build_ad_hoc_spawn_outcome(Some("transient result".to_string()));

        assert_eq!(outcome.status, "completed");
        assert!(outcome.success);
        assert_eq!(outcome.summary, "transient result");
        assert_eq!(outcome.content, "transient result");
        assert_eq!(outcome.final_text.as_deref(), Some("transient result"));
    }

    #[test]
    fn build_ad_hoc_spawn_outcome_distinguishes_missing_final_text() {
        let outcome = build_ad_hoc_spawn_outcome(None);

        assert_eq!(outcome.status, "completed_no_output");
        assert!(outcome.success);
        assert!(outcome.summary.contains("no final text"));
        assert!(outcome.content.contains("no final text"));
        assert!(outcome.final_text.is_none());
    }

    #[test]
    fn unit_worker_status_is_error_for_failed_blocked_and_cancelled_only() {
        assert!(!unit_worker_status_is_error(
            tower_contracts::worker::WorkerStatus::Completed
        ));
        assert!(!unit_worker_status_is_error(
            tower_contracts::worker::WorkerStatus::AwaitingVerify
        ));
        assert!(unit_worker_status_is_error(
            tower_contracts::worker::WorkerStatus::Failed
        ));
        assert!(unit_worker_status_is_error(
            tower_contracts::worker::WorkerStatus::Blocked
        ));
        assert!(unit_worker_status_is_error(
            tower_contracts::worker::WorkerStatus::Cancelled
        ));
    }

    #[test]
    fn extract_final_assistant_text_returns_last_non_empty_assistant_text() {
        let messages = vec![
            imp_llm::Message::Assistant(AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "first".to_string(),
                }],
                stop_reason: imp_llm::StopReason::EndTurn,
                usage: None,
                timestamp: 0,
            }),
            imp_llm::Message::Assistant(AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "   ".to_string(),
                }],
                stop_reason: imp_llm::StopReason::EndTurn,
                usage: None,
                timestamp: 0,
            }),
            imp_llm::Message::Assistant(AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "transient".to_string(),
                }],
                stop_reason: imp_llm::StopReason::EndTurn,
                usage: None,
                timestamp: 0,
            }),
        ];

        let text = extract_final_assistant_text_from_messages(&messages);

        assert_eq!(text.as_deref(), Some("transient"));
    }
}

