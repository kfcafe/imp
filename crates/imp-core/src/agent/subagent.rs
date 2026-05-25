use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Runtime-local identifier for a bounded subagent execution.
///
/// This is scoped to the parent imp run. It is intentionally not a durable work
/// item id, lease id, or scheduler id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubagentRunId(pub String);

impl SubagentRunId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Runtime-local identifier for the parent execution that spawned a subagent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParentRunId(pub String);

impl ParentRunId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Role a bounded subagent should play within the parent run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentRole {
    Searcher,
    Planner,
    Implementer,
    Verifier,
    Reviewer,
    Synthesizer,
    Custom(String),
}

/// Context a parent run gives to a bounded subagent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentContext {
    pub instructions: Vec<String>,
    pub messages: Vec<String>,
    pub files: Vec<SubagentFileContext>,
    pub artifacts: Vec<SubagentArtifactRef>,
}

/// A file or path-scoped context reference for a bounded subagent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentFileContext {
    pub path: PathBuf,
    pub note: Option<String>,
    pub read_only: bool,
}

/// Runtime artifact reference produced or consumed within the parent run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentArtifactRef {
    pub name: String,
    pub path: Option<PathBuf>,
    pub description: Option<String>,
}

/// Resource limits for a bounded subagent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentResourceLimits {
    pub timeout_seconds: Option<u64>,
    pub max_model_tokens: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub max_parallel_children: Option<u32>,
    pub allowed_paths: Vec<PathBuf>,
    pub writable_paths: Vec<PathBuf>,
}

/// How the parent run should consume a bounded subagent outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentMergePolicy {
    Inform,
    Verify,
    Review,
    Apply,
    Synthesize,
    Escalate,
    Custom(String),
}

impl Default for SubagentMergePolicy {
    fn default() -> Self {
        Self::Inform
    }
}

/// Input packet for a bounded subagent execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentInput {
    pub parent_run_id: ParentRunId,
    pub child_run_id: SubagentRunId,
    pub role: SubagentRole,
    pub objective: String,
    pub context: SubagentContext,
    pub resource_limits: SubagentResourceLimits,
    pub merge_policy: SubagentMergePolicy,
    pub output_contract: Option<String>,
}

/// Runtime status for a bounded subagent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentStatus {
    Pending,
    Running,
    Success,
    Incomplete,
    Blocked,
    Failed,
    Cancelled,
}

impl SubagentStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Success | Self::Incomplete | Self::Blocked | Self::Failed | Self::Cancelled
        )
    }
}

/// Structured result returned by a bounded subagent to its parent run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentOutcome {
    pub child_run_id: SubagentRunId,
    pub role: SubagentRole,
    pub status: SubagentStatus,
    pub summary: String,
    pub evidence: Vec<SubagentArtifactRef>,
    pub files_changed: Vec<PathBuf>,
    pub files_inspected: Vec<PathBuf>,
    pub verification_results: Vec<String>,
    pub blockers: Vec<String>,
    pub follow_ups: Vec<String>,
    pub diagnostics: Vec<String>,
    pub confidence: Option<SubagentConfidence>,
}

/// Coarse confidence/risk label for a bounded subagent outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentConfidence {
    Low,
    Medium,
    High,
}

/// Runtime event emitted while a bounded subagent executes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentEvent {
    Started {
        child_run_id: SubagentRunId,
        role: SubagentRole,
        objective: String,
    },
    Message {
        child_run_id: SubagentRunId,
        text: String,
    },
    ToolStarted {
        child_run_id: SubagentRunId,
        tool_name: String,
    },
    ToolCompleted {
        child_run_id: SubagentRunId,
        tool_name: String,
        success: bool,
    },
    Artifact {
        child_run_id: SubagentRunId,
        artifact: SubagentArtifactRef,
    },
    Blocked {
        child_run_id: SubagentRunId,
        reason: String,
    },
    Cancelled {
        child_run_id: SubagentRunId,
        reason: Option<String>,
    },
    Completed {
        outcome: SubagentOutcome,
    },
    Failed {
        child_run_id: SubagentRunId,
        error: String,
    },
    Merged {
        child_run_id: SubagentRunId,
        policy: SubagentMergePolicy,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subagent_status_terminal_states_are_explicit() {
        assert!(!SubagentStatus::Pending.is_terminal());
        assert!(!SubagentStatus::Running.is_terminal());
        assert!(SubagentStatus::Success.is_terminal());
        assert!(SubagentStatus::Incomplete.is_terminal());
        assert!(SubagentStatus::Blocked.is_terminal());
        assert!(SubagentStatus::Failed.is_terminal());
        assert!(SubagentStatus::Cancelled.is_terminal());
    }

    #[test]
    fn subagent_event_uses_runtime_scoped_ids() {
        let event = SubagentEvent::Started {
            child_run_id: SubagentRunId::new("child-1"),
            role: SubagentRole::Verifier,
            objective: "Check the patch".to_string(),
        };

        let json = serde_json::to_string(&event).expect("serialize event");
        assert!(json.contains("child-1"));
        assert!(!json.contains("task_id"));
        assert!(!json.contains("lease"));
        assert!(!json.contains("board"));
    }
}
