use serde::{Deserialize, Serialize};

/// Coarse-grained phase of a single agent turn.
///
/// This is intentionally small and mechanical: it describes where the runtime
/// is, not why the runtime should continue or stop. Policy decisions live in
/// [`LoopDecision`] / [`RunFinalStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnPhase {
    ReceiveCommands,
    PreTurn,
    BuildContext,
    SampleModel,
    FinalizeAssistantMessage,
    PlanTools,
    ExecuteTools,
    RecordObservations,
    AssessTurn,
    DecideNext,
    Finish,
}

impl TurnPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReceiveCommands => "receive_commands",
            Self::PreTurn => "pre_turn",
            Self::BuildContext => "build_context",
            Self::SampleModel => "sample_model",
            Self::FinalizeAssistantMessage => "finalize_assistant_message",
            Self::PlanTools => "plan_tools",
            Self::ExecuteTools => "execute_tools",
            Self::RecordObservations => "record_observations",
            Self::AssessTurn => "assess_turn",
            Self::DecideNext => "decide_next",
            Self::Finish => "finish",
        }
    }
}

/// Minimal visible state for a turn. This is the first slice of making turn
/// state explicit; later slices can add durable IDs and recovery cursors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnState {
    pub index: u32,
    pub phase: TurnPhase,
    pub continue_reason: Option<ContinueReason>,
    pub planned_tools: usize,
    pub completed_tools: usize,
}

impl TurnState {
    pub fn new(index: u32) -> Self {
        Self {
            index,
            phase: TurnPhase::ReceiveCommands,
            continue_reason: None,
            planned_tools: 0,
            completed_tools: 0,
        }
    }

    pub fn enter(&mut self, phase: TurnPhase) {
        self.phase = phase;
    }

    pub fn record_continue(&mut self, reason: ContinueReason) {
        self.continue_reason = Some(reason);
    }

    pub fn record_tool_plan(&mut self, planned_tools: usize) {
        self.planned_tools = planned_tools;
    }

    pub fn record_tool_results(&mut self, completed_tools: usize) {
        self.completed_tools = completed_tools;
    }
}

/// Why the agent is allowed to spend another turn.
///
/// There should be no anonymous `continue`; long-running autonomy is safe only
/// when each continuation has a concrete, inspectable reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinueReason {
    ExternalizationNeeded,
    HighConfidenceVisibleNextStep,
    ExecutionDebt,
    ToolResultsNeedInterpretation,
    QueuedUserFollowUp,
    RecoveryContinuation,
}

impl ContinueReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExternalizationNeeded => "externalization_needed",
            Self::HighConfidenceVisibleNextStep => "high_confidence_visible_next_step",
            Self::ExecutionDebt => "execution_debt",
            Self::ToolResultsNeedInterpretation => "tool_results_need_interpretation",
            Self::QueuedUserFollowUp => "queued_user_follow_up",
            Self::RecoveryContinuation => "recovery_continuation",
        }
    }
}

/// Semantic reason the runtime stopped intentionally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    NoAutomaticFollowUp,
    NoProgress,
    RepeatedAction,
    UserBlocker,
    ExecutionBlocked,
    DecompositionCompleted,
    WorkCompleted,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAutomaticFollowUp => "no_automatic_follow_up",
            Self::NoProgress => "no_progress",
            Self::RepeatedAction => "repeated_action",
            Self::UserBlocker => "user_blocker",
            Self::ExecutionBlocked => "execution_blocked",
            Self::DecompositionCompleted => "decomposition_completed",
            Self::WorkCompleted => "work_completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LoopDecision {
    Continue {
        reason: ContinueReason,
        prompt: String,
    },
    Finish {
        status: RunFinalStatus,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RunFinalStatus {
    Done {
        reason: StopReason,
    },
    DoneWithConcerns {
        reason: StopReason,
        concerns: Vec<String>,
    },
    Blocked {
        reason: StopReason,
        message: String,
    },
    NeedsUserInput {
        question: String,
    },
    Cancelled,
    Failed {
        message: String,
    },
}

impl RunFinalStatus {
    pub fn from_stop_reason(reason: StopReason) -> Self {
        match reason {
            StopReason::UserBlocker | StopReason::ExecutionBlocked | StopReason::RepeatedAction => {
                Self::Blocked {
                    reason,
                    message: reason.as_str().to_string(),
                }
            }
            StopReason::NoProgress => Self::DoneWithConcerns {
                reason,
                concerns: vec!["stopped because no justified continuation was available".into()],
            },
            _ => Self::Done { reason },
        }
    }
}

/// Tool risk visible before execution. This is deliberately conservative and
/// can be refined as tool metadata grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRisk {
    ReadOnly,
    Mutable,
    ExternalSideEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionMode {
    ParallelReadonlyThenSequentialMutable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedToolCall {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub risk: ToolRisk,
    pub retry_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPlan {
    pub mode: ToolExecutionMode,
    pub calls: Vec<PlannedToolCall>,
}

impl ToolPlan {
    pub fn empty() -> Self {
        Self {
            mode: ToolExecutionMode::ParallelReadonlyThenSequentialMutable,
            calls: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }
}
