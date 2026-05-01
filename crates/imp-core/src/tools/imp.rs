use async_trait::async_trait;
use imp_llm::ThinkingLevel;
use imp_llm::{AssistantMessage, ContentBlock};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use super::{Tool, ToolContext, ToolOutput};
use crate::config::AgentMode;
use crate::error::{Error, Result};
use crate::imp_session::{ImpSession, SessionChoice, SessionOptions};

pub struct ImpTool;

const DEFAULT_ASK_AGENT_TIMEOUT_SECS: u64 = 300;
const ASK_AGENT_CANCEL_GRACE_SECS: u64 = 5;
const MAX_ASK_AGENT_TIMEOUT_SECS: u64 = 1800;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskAgentWorkerType {
    Reviewer,
    Worker,
    NoTools,
}

impl AskAgentWorkerType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reviewer => "reviewer",
            Self::Worker => "worker",
            Self::NoTools => "no_tools",
        }
    }

    fn mode(self) -> AgentMode {
        match self {
            Self::Reviewer | Self::NoTools => AgentMode::Reviewer,
            Self::Worker => AgentMode::Worker,
        }
    }

    fn no_tools(self) -> bool {
        matches!(self, Self::NoTools)
    }
}

impl Default for AskAgentWorkerType {
    fn default() -> Self {
        Self::Reviewer
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AskAgentConfig {
    #[serde(default)]
    pub default_worker_type: AskAgentWorkerType,
    #[serde(default)]
    pub profiles: AskAgentProfiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskAgentProfiles {
    #[serde(default = "default_reviewer_profile")]
    pub reviewer: AskAgentProfile,
    #[serde(default = "default_worker_profile")]
    pub worker: AskAgentProfile,
    #[serde(default = "default_no_tools_profile")]
    pub no_tools: AskAgentProfile,
}

impl Default for AskAgentProfiles {
    fn default() -> Self {
        Self {
            reviewer: default_reviewer_profile(),
            worker: default_worker_profile(),
            no_tools: default_no_tools_profile(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskAgentProfile {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub thinking: Option<ThinkingLevel>,
    pub max_turns: Option<u32>,
    pub max_tokens: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub system_prompt: Option<String>,
}

impl Default for AskAgentProfile {
    fn default() -> Self {
        Self {
            model: None,
            provider: None,
            thinking: None,
            max_turns: None,
            max_tokens: None,
            timeout_secs: Some(DEFAULT_ASK_AGENT_TIMEOUT_SECS),
            system_prompt: None,
        }
    }
}

fn default_reviewer_profile() -> AskAgentProfile {
    AskAgentProfile {
        thinking: Some(ThinkingLevel::Medium),
        max_turns: Some(4),
        timeout_secs: Some(DEFAULT_ASK_AGENT_TIMEOUT_SECS),
        system_prompt: Some("You are a focused reviewer. Inspect, critique, and summarize. Do not make changes unless explicitly asked by the parent agent.".to_string()),
        ..AskAgentProfile::default()
    }
}

fn default_worker_profile() -> AskAgentProfile {
    AskAgentProfile {
        thinking: Some(ThinkingLevel::Low),
        max_turns: Some(6),
        timeout_secs: Some(DEFAULT_ASK_AGENT_TIMEOUT_SECS),
        system_prompt: Some("You are a bounded helper worker. Complete the requested task directly, verify narrowly, and report concise evidence.".to_string()),
        ..AskAgentProfile::default()
    }
}

fn default_no_tools_profile() -> AskAgentProfile {
    AskAgentProfile {
        thinking: Some(ThinkingLevel::High),
        max_turns: Some(2),
        timeout_secs: Some(DEFAULT_ASK_AGENT_TIMEOUT_SECS),
        system_prompt: Some("You are a no-tool reasoning helper. Think through the request and answer from the provided prompt only.".to_string()),
        ..AskAgentProfile::default()
    }
}

#[async_trait]
impl Tool for ImpTool {
    fn name(&self) -> &str {
        "ask_agent"
    }

    fn label(&self) -> &str {
        "Ask Agent"
    }

    fn description(&self) -> &str {
        "Ask a bounded helper agent for focused review, implementation, or no-tool reasoning."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Prompt for the helper agent."
                },
                "worker_type": {
                    "type": "string",
                    "enum": ["reviewer", "worker", "no_tools"],
                    "description": "Helper profile. Defaults to reviewer."
                },
                "timeout_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_ASK_AGENT_TIMEOUT_SECS,
                    "description": "Maximum wall-clock time before cancellation. Defaults from worker profile, fallback 300."
                }
            },
            "required": ["prompt"]
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
                "The ask_agent tool is only available in Full or Orchestrator mode.",
            ));
        }

        if has_unit_mode_intent(&params) {
            return Ok(ToolOutput::error(
                "ask_agent is ad_hoc-only. Use the native mana tool with action=run for durable unit execution.",
            ));
        }

        execute_ask_agent(params, ctx).await
    }
}

struct AdHocSpawnOutcome {
    status: &'static str,
    summary: String,
    content: String,
    success: bool,
    final_text: Option<String>,
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
            summary: "Helper agent completed with no final text.".to_string(),
            content: "Helper agent completed with no final text.".to_string(),
            success: true,
            final_text: None,
        },
    }
}

fn optional_non_empty_string(params: &serde_json::Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn has_unit_mode_intent(params: &serde_json::Value) -> bool {
    optional_non_empty_string(params, "unit_id").is_some()
        || optional_non_empty_string(params, "mana_dir").is_some()
        || params
            .get("defer_verify")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        || matches!(
            optional_non_empty_string(params, "mode").as_deref(),
            Some("unit")
        )
}

fn ask_agent_worker_type(
    params: &serde_json::Value,
    config: &AskAgentConfig,
) -> Result<AskAgentWorkerType> {
    match optional_non_empty_string(params, "worker_type") {
        Some(raw) => match raw.as_str() {
            "reviewer" => Ok(AskAgentWorkerType::Reviewer),
            "worker" => Ok(AskAgentWorkerType::Worker),
            "no_tools" => Ok(AskAgentWorkerType::NoTools),
            other => Err(Error::Tool(format!(
                "Invalid worker_type '{other}'. Expected reviewer, worker, or no_tools."
            ))),
        },
        None => Ok(config.default_worker_type),
    }
}

fn profile_for(config: &AskAgentConfig, worker_type: AskAgentWorkerType) -> &AskAgentProfile {
    match worker_type {
        AskAgentWorkerType::Reviewer => &config.profiles.reviewer,
        AskAgentWorkerType::Worker => &config.profiles.worker,
        AskAgentWorkerType::NoTools => &config.profiles.no_tools,
    }
}

fn ask_agent_timeout_secs(params: &serde_json::Value, profile: &AskAgentProfile) -> u64 {
    params
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .filter(|secs| *secs > 0)
        .unwrap_or(
            profile
                .timeout_secs
                .unwrap_or(DEFAULT_ASK_AGENT_TIMEOUT_SECS),
        )
        .min(MAX_ASK_AGENT_TIMEOUT_SECS)
}

fn ad_hoc_spawn_timeout_error(timeout_secs: u64) -> Error {
    Error::Tool(format!(
        "ask_agent timed out after {timeout_secs}s and was cancelled"
    ))
}

fn build_ask_agent_details(
    worker_type: AskAgentWorkerType,
    durable: bool,
    status: impl Into<String>,
    success: bool,
    summary: impl Into<String>,
    model: serde_json::Value,
    provider: serde_json::Value,
    timeout_secs: u64,
    final_text: Option<String>,
) -> serde_json::Value {
    json!({
        "tool": "ask_agent",
        "worker_type": worker_type.as_str(),
        "durable": durable,
        "status": status.into(),
        "success": success,
        "summary": summary.into(),
        "model": model,
        "provider": provider,
        "timeout_secs": timeout_secs,
        "final_text": final_text,
    })
}

async fn execute_ask_agent(params: serde_json::Value, ctx: ToolContext) -> Result<ToolOutput> {
    let worker_type = ask_agent_worker_type(&params, &ctx.config.ask_agent)?;
    let profile = profile_for(&ctx.config.ask_agent, worker_type);
    let timeout_secs = ask_agent_timeout_secs(&params, profile);
    let timeout = Duration::from_secs(timeout_secs);
    let cancel_grace = Duration::from_secs(ASK_AGENT_CANCEL_GRACE_SECS);
    let prompt = params
        .get("prompt")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Tool("Missing required parameter: prompt".into()))?;

    let session_options = SessionOptions {
        cwd: ctx.cwd.clone(),
        model_override: None,
        model: profile.model.clone(),
        provider: profile.provider.clone(),
        api_key: None,
        thinking: profile.thinking,
        mode: Some(worker_type.mode()),
        max_turns: profile.max_turns,
        max_tokens: profile.max_tokens,
        system_prompt: profile.system_prompt.clone(),
        no_tools: worker_type.no_tools(),
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
        .prompt(prompt)
        .await
        .map_err(|e| Error::Tool(e.to_string()))?;
    match tokio::time::timeout(timeout, session.wait()).await {
        Ok(result) => result.map_err(|e| Error::Tool(e.to_string()))?,
        Err(_) => {
            let _ = session.cancel().await;
            let mut aborted = false;
            if tokio::time::timeout(cancel_grace, session.wait())
                .await
                .is_err()
            {
                session.abort();
                aborted = true;
            }
            let error = ad_hoc_spawn_timeout_error(timeout_secs);
            return Ok(ToolOutput {
                content: vec![ContentBlock::Text {
                    text: error.to_string(),
                }],
                details: json!({
                    "tool": "ask_agent",
                    "worker_type": worker_type.as_str(),
                    "durable": false,
                    "status": "timeout",
                    "success": false,
                    "timeout_secs": timeout_secs,
                    "cancelled": true,
                    "aborted": aborted,
                }),
                is_error: true,
            });
        }
    }

    let final_text = extract_final_assistant_text(&session);
    let outcome = build_ad_hoc_spawn_outcome(final_text);

    Ok(ToolOutput {
        content: vec![ContentBlock::Text {
            text: outcome.content,
        }],
        details: build_ask_agent_details(
            worker_type,
            false,
            outcome.status,
            outcome.success,
            outcome.summary,
            json!(session.model().meta.id.clone()),
            json!(session.model().meta.provider.clone()),
            timeout_secs,
            outcome.final_text,
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
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode,
            read_max_lines: 0,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        }
    }

    #[test]
    fn schema_is_plain_object_without_top_level_all_of() {
        let schema = ImpTool.parameters();
        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
        assert!(schema.get("allOf").is_none());
        assert_eq!(
            schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(schema["required"][0].as_str(), Some("prompt"));
        assert_eq!(
            schema["properties"]["prompt"]["type"].as_str(),
            Some("string")
        );
        assert_eq!(
            schema["properties"]["timeout_secs"]["type"].as_str(),
            Some("integer")
        );
        assert!(schema["properties"].get("unit_id").is_none());
        assert!(schema["properties"].get("delegate").is_none());
    }

    #[test]
    fn worker_type_defaults_to_reviewer_and_validates_values() {
        let config = AskAgentConfig::default();
        assert_eq!(
            ask_agent_worker_type(&json!({}), &config).unwrap(),
            AskAgentWorkerType::Reviewer
        );
        assert_eq!(
            ask_agent_worker_type(&json!({"worker_type": "worker"}), &config).unwrap(),
            AskAgentWorkerType::Worker
        );
        assert!(ask_agent_worker_type(&json!({"worker_type": "bad"}), &config).is_err());
    }

    #[test]
    fn unit_payloads_are_directed_to_mana_run() {
        assert!(has_unit_mode_intent(&json!({"unit_id": "123"})));
        assert!(has_unit_mode_intent(&json!({"mode": "unit"})));
        assert!(has_unit_mode_intent(&json!({"defer_verify": true})));
        assert!(!has_unit_mode_intent(&json!({"prompt": "inspect"})));
    }

    #[test]
    fn ask_agent_timeout_uses_profile_and_clamps_explicit_value() {
        let mut profile = AskAgentProfile::default();
        profile.timeout_secs = Some(42);
        assert_eq!(ask_agent_timeout_secs(&json!({}), &profile), 42);
        assert_eq!(
            ask_agent_timeout_secs(&json!({"timeout_secs": 0}), &profile),
            42
        );
        assert_eq!(
            ask_agent_timeout_secs(&json!({"timeout_secs": 12}), &profile),
            12
        );
        assert_eq!(
            ask_agent_timeout_secs(&json!({"timeout_secs": 999_999}), &profile),
            MAX_ASK_AGENT_TIMEOUT_SECS
        );
    }

    #[test]
    fn worker_type_maps_to_modes_and_tool_access() {
        assert_eq!(AskAgentWorkerType::Reviewer.mode(), AgentMode::Reviewer);
        assert_eq!(AskAgentWorkerType::Worker.mode(), AgentMode::Worker);
        assert_eq!(AskAgentWorkerType::NoTools.mode(), AgentMode::Reviewer);
        assert!(!AskAgentWorkerType::Reviewer.no_tools());
        assert!(!AskAgentWorkerType::Worker.no_tools());
        assert!(AskAgentWorkerType::NoTools.no_tools());
    }

    #[tokio::test]
    async fn unit_mode_returns_mana_run_guidance() {
        let tool = ImpTool;
        let out = tool
            .execute(
                "call-1",
                json!({"unit_id": "missing-unit-for-validation"}),
                test_ctx(AgentMode::Orchestrator),
            )
            .await
            .unwrap();

        assert!(out.is_error);
        assert!(out
            .text_content()
            .unwrap_or_default()
            .contains("mana tool with action=run"));
    }

    #[tokio::test]
    async fn missing_prompt_returns_error() {
        let tool = ImpTool;
        let result = tool
            .execute("call-1", json!({}), test_ctx(AgentMode::Orchestrator))
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
                json!({"prompt": "inspect"}),
                test_ctx(AgentMode::Worker),
            )
            .await
            .unwrap();
        assert!(out.is_error);
        let text = out.text_content().unwrap_or_default();
        assert!(text.contains("Full or Orchestrator"));
    }

    #[test]
    fn build_ask_agent_details_keeps_stable_fields() {
        let details = build_ask_agent_details(
            AskAgentWorkerType::Reviewer,
            false,
            "completed",
            true,
            "summary",
            json!("model-x"),
            json!("provider-y"),
            300,
            Some("hello".to_string()),
        );

        assert_eq!(
            details.get("tool").and_then(|v| v.as_str()),
            Some("ask_agent")
        );
        assert_eq!(
            details.get("worker_type").and_then(|v| v.as_str()),
            Some("reviewer")
        );
        assert_eq!(
            details.get("durable").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(details.get("success").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            details.get("final_text").and_then(|v| v.as_str()),
            Some("hello")
        );
        assert!(details.get("delegation_mode").is_none());
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
