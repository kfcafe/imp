use imp_llm::{AssistantMessage, Cost, Message, StreamEvent, Usage};
use serde::{Deserialize, Serialize};

use crate::mana_review::TurnManaReview;

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
