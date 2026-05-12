use imp_llm::{AssistantMessage, Cost, Message, StreamEvent, Usage};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::mana_review::TurnManaReview;
use crate::trace::TraceEvent;

use super::NextActionAssessment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingStage {
    ContextAssemblyStart,
    ContextAssemblyEnd,
    LlmRequestStart,
    FirstStreamEvent,
    FirstTextDelta,
    FirstToolCall,
    MessageEnd,
    ToolExecutionStart,
    ToolExecutionEnd,
    PostTurnAssessmentStart,
    PostTurnAssessmentEnd,
}

impl TimingStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContextAssemblyStart => "context_assembly_start",
            Self::ContextAssemblyEnd => "context_assembly_end",
            Self::LlmRequestStart => "llm_request_start",
            Self::FirstStreamEvent => "first_stream_event",
            Self::FirstTextDelta => "first_text_delta",
            Self::FirstToolCall => "first_tool_call",
            Self::MessageEnd => "message_end",
            Self::ToolExecutionStart => "tool_execution_start",
            Self::ToolExecutionEnd => "tool_execution_end",
            Self::PostTurnAssessmentStart => "post_turn_assessment_start",
            Self::PostTurnAssessmentEnd => "post_turn_assessment_end",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingEvent {
    pub turn: u32,
    pub stage: TimingStage,
    pub since_turn_start_ms: u64,
    pub since_llm_request_start_ms: Option<u64>,
    pub duration_ms: Option<u64>,
    pub label: Option<String>,
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCheckpointKind {
    ProviderRequestStart,
    AssistantToolCallObserved,
    AssistantMessageFinalized,
    ToolPlanCreated,
    ToolExecutionStart,
    ToolExecutionEnd,
    ToolResultAddedToContext,
    ProviderRequestCompleted,
}

impl RecoveryCheckpointKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProviderRequestStart => "provider_request_start",
            Self::AssistantToolCallObserved => "assistant_tool_call_observed",
            Self::AssistantMessageFinalized => "assistant_message_finalized",
            Self::ToolPlanCreated => "tool_plan_created",
            Self::ToolExecutionStart => "tool_execution_start",
            Self::ToolExecutionEnd => "tool_execution_end",
            Self::ToolResultAddedToContext => "tool_result_added_to_context",
            Self::ProviderRequestCompleted => "provider_request_completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCheckpoint {
    pub version: u32,
    pub turn: u32,
    pub kind: RecoveryCheckpointKind,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub args_hash: Option<String>,
    pub success: Option<bool>,
    pub error_class: Option<String>,
    pub timestamp: u64,
}

/// Events emitted by the agent during execution.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    AgentStart {
        model: String,
        timestamp: u64,
    },
    AgentEnd {
        usage: Usage,
        cost: Cost,
        status: crate::agent::RunFinalStatus,
    },
    TurnStart {
        index: u32,
    },
    TurnAssessment {
        index: u32,
        assessment: NextActionAssessment,
    },
    TurnEnd {
        index: u32,
        message: AssistantMessage,
        mana_review: TurnManaReview,
    },
    MessageStart {
        message: Message,
    },
    MessageDelta {
        delta: StreamEvent,
    },
    MessageEnd {
        message: Message,
    },
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    ToolOutputDelta {
        tool_call_id: String,
        text: String,
    },
    ToolExecutionEnd {
        tool_call_id: String,
        result: imp_llm::ToolResultMessage,
    },
    Warning {
        message: String,
    },
    Timing {
        timing: TimingEvent,
    },
    RecoveryCheckpoint {
        checkpoint: RecoveryCheckpoint,
    },
    Error {
        error: String,
    },
}

impl AgentEvent {
    pub fn to_trace_event(&self, run_id: impl Into<String>) -> TraceEvent {
        let run_id = run_id.into();
        match self {
            AgentEvent::AgentStart { model, timestamp } => TraceEvent::new(
                run_id,
                "agent.start",
                json!({ "model": model, "source_timestamp": timestamp }),
            ),
            AgentEvent::AgentEnd {
                usage,
                cost,
                status,
            } => TraceEvent::new(
                run_id,
                "agent.end",
                json!({
                    "usage": format!("{usage:?}"),
                    "cost": format!("{cost:?}"),
                    "status": format!("{status:?}"),
                }),
            ),
            AgentEvent::TurnStart { index } => {
                TraceEvent::new(run_id, "turn.start", json!({ "index": index })).with_turn(*index)
            }
            AgentEvent::TurnAssessment { index, assessment } => TraceEvent::new(
                run_id,
                "turn.assessment",
                json!({ "index": index, "assessment": format!("{assessment:?}") }),
            )
            .with_turn(*index),
            AgentEvent::TurnEnd {
                index,
                message,
                mana_review,
            } => TraceEvent::new(
                run_id,
                "turn.end",
                json!({
                    "index": index,
                    "message": format!("{message:?}"),
                    "mana_review": format!("{mana_review:?}"),
                }),
            )
            .with_turn(*index),
            AgentEvent::MessageStart { message } => TraceEvent::new(
                run_id,
                "message.start",
                json!({ "message": format!("{message:?}") }),
            ),
            AgentEvent::MessageDelta { delta } => TraceEvent::new(
                run_id,
                "message.delta",
                json!({ "delta": format!("{delta:?}") }),
            ),
            AgentEvent::MessageEnd { message } => TraceEvent::new(
                run_id,
                "message.end",
                json!({ "message": format!("{message:?}") }),
            ),
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => TraceEvent::new(
                run_id,
                "tool.execution.start",
                json!({ "tool_call_id": tool_call_id, "tool_name": tool_name, "args": args }),
            )
            .with_tool_call_id(tool_call_id.clone()),
            AgentEvent::ToolOutputDelta { tool_call_id, text } => TraceEvent::new(
                run_id,
                "tool.output.delta",
                json!({ "tool_call_id": tool_call_id, "text": text }),
            )
            .with_tool_call_id(tool_call_id.clone()),
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                result,
            } => TraceEvent::new(
                run_id,
                "tool.execution.end",
                json!({ "tool_call_id": tool_call_id, "result": format!("{result:?}") }),
            )
            .with_tool_call_id(tool_call_id.clone()),
            AgentEvent::Warning { message } => {
                TraceEvent::new(run_id, "warning", json!({ "message": message }))
            }
            AgentEvent::Timing { timing } => TraceEvent::new(
                run_id,
                "timing",
                json!({
                    "turn": timing.turn,
                    "stage": timing.stage.as_str(),
                    "since_turn_start_ms": timing.since_turn_start_ms,
                    "since_llm_request_start_ms": timing.since_llm_request_start_ms,
                    "duration_ms": timing.duration_ms,
                    "label": timing.label,
                    "success": timing.success,
                }),
            )
            .with_turn(timing.turn),
            AgentEvent::RecoveryCheckpoint { checkpoint } => {
                let mut event = TraceEvent::new(
                    run_id,
                    "recovery.checkpoint",
                    json!({
                        "version": checkpoint.version,
                        "turn": checkpoint.turn,
                        "kind": checkpoint.kind.as_str(),
                        "tool_call_id": checkpoint.tool_call_id,
                        "tool_name": checkpoint.tool_name,
                        "args_hash": checkpoint.args_hash,
                        "success": checkpoint.success,
                        "error_class": checkpoint.error_class,
                        "checkpoint_timestamp": checkpoint.timestamp,
                    }),
                )
                .with_turn(checkpoint.turn);
                if let Some(tool_call_id) = &checkpoint.tool_call_id {
                    event = event.with_tool_call_id(tool_call_id.clone());
                }
                event
            }
            AgentEvent::Error { error } => {
                TraceEvent::new(run_id, "error", json!({ "error": error }))
            }
        }
    }
}

#[cfg(test)]
mod trace_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn agent_events_convert_to_trace_events() {
        let start = AgentEvent::AgentStart {
            model: "test-model".into(),
            timestamp: 123,
        }
        .to_trace_event("run-1");
        assert_eq!(start.kind, "agent.start");
        assert_eq!(start.run_id, "run-1");
        assert_eq!(start.payload["model"], "test-model");

        let tool = AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".into(),
            tool_name: "read".into(),
            args: json!({"path": "README.md"}),
        }
        .to_trace_event("run-1");
        assert_eq!(tool.kind, "tool.execution.start");
        assert_eq!(tool.correlation.tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(tool.payload["tool_name"], "read");

        let timing = AgentEvent::Timing {
            timing: TimingEvent {
                turn: 2,
                stage: TimingStage::LlmRequestStart,
                since_turn_start_ms: 10,
                since_llm_request_start_ms: None,
                duration_ms: None,
                label: None,
                success: None,
            },
        }
        .to_trace_event("run-1");
        assert_eq!(timing.kind, "timing");
        assert_eq!(timing.turn, Some(2));
        assert_eq!(timing.payload["stage"], "llm_request_start");
    }

    #[test]
    fn recovery_checkpoint_conversion_preserves_correlation() {
        let event = AgentEvent::RecoveryCheckpoint {
            checkpoint: RecoveryCheckpoint {
                version: 1,
                turn: 3,
                kind: RecoveryCheckpointKind::ToolExecutionEnd,
                tool_call_id: Some("call-2".into()),
                tool_name: Some("bash".into()),
                args_hash: Some("abc".into()),
                success: Some(false),
                error_class: Some("timeout".into()),
                timestamp: 456,
            },
        }
        .to_trace_event("run-1");

        assert_eq!(event.kind, "recovery.checkpoint");
        assert_eq!(event.turn, Some(3));
        assert_eq!(event.correlation.tool_call_id.as_deref(), Some("call-2"));
        assert_eq!(event.payload["kind"], "tool_execution_end");
        assert_eq!(event.payload["error_class"], "timeout");
    }
}
