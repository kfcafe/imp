use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::prototype::{Prototype, PrototypeStatus};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkId(pub String);

impl WorkId {
    pub fn new(prefix: &str) -> Self {
        Self(format!("{}-{}", prefix, uuid::Uuid::new_v4().simple()))
    }
}

impl std::fmt::Display for WorkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for WorkId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for WorkId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    Epic,
    Task,
    Memory,
    Decision,
    Prototype,
    Check,
    Run,
    Lease,
    ContextPack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    Ready,
    Active,
    Blocked,
    Review,
    Done,
    Dropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Fact,
    Preference,
    Decision,
    FollowUp,
    Note,
    PrototypeLearning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Command,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunOutcome {
    Done,
    DoneWithConcerns,
    Blocked,
    NeedsContext,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPackStatus {
    Draft,
    Ready,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextBlockStability {
    GlobalStatic,
    ProjectStable,
    EpicStable,
    TaskVersionStable,
    RunDynamic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub kind: LinkKind,
    pub target: WorkId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Parent,
    DependsOn,
    Blocks,
    RelatesTo,
    Verifies,
    Supersedes,
    DerivedFrom,
    AttemptedBy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub kind: SourceKind,
    pub reference: String,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    FileRange,
    Memory,
    WorkItem,
    Attempt,
    CommandOutput,
    Conversation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: WorkId,
    pub title: String,
    pub parent: Option<WorkId>,
    pub status: TaskStatus,
    pub acceptance: Vec<String>,
    pub checks: Vec<Check>,
    pub context_pack: Option<WorkId>,
    pub links: Vec<Link>,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Epic {
    pub id: WorkId,
    pub title: String,
    pub status: TaskStatus,
    pub intent: Option<String>,
    pub acceptance: Vec<String>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: WorkId,
    pub kind: MemoryKind,
    pub text: String,
    pub topics: Vec<String>,
    pub parent_work: Option<WorkId>,
    pub paths: Vec<PathBuf>,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub id: WorkId,
    pub title: String,
    pub status: DecisionStatus,
    pub rationale: Option<String>,
    pub parent_work: Option<WorkId>,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    pub id: WorkId,
    pub kind: CheckKind,
    pub description: String,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPack {
    pub id: WorkId,
    pub work_id: WorkId,
    pub version: u32,
    pub status: ContextPackStatus,
    pub token_budget: Option<u32>,
    pub blocks: Vec<ContextBlock>,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBlock {
    pub title: String,
    pub body: String,
    pub stability: ContextBlockStability,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkRunStatus {
    Planning,
    Running,
    Paused,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRunPolicy {
    pub max_jobs: usize,
    pub path_conflicts: String,
    pub require_context: bool,
    pub keep_going: bool,
}

impl Default for WorkRunPolicy {
    fn default() -> Self {
        Self {
            max_jobs: 1,
            path_conflicts: "block".to_string(),
            require_context: false,
            keep_going: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRunAssignment {
    pub work_id: WorkId,
    pub lease_id: Option<WorkId>,
    pub worker_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRun {
    pub id: WorkId,
    pub root_work_id: WorkId,
    pub status: WorkRunStatus,
    pub policy: WorkRunPolicy,
    pub current_wave: u32,
    pub started_at: String,
    pub updated_at: String,
    pub assignments: Vec<WorkRunAssignment>,
    pub blocked: Vec<WorkId>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRunEvent {
    pub sequence: u64,
    pub run_id: WorkId,
    pub timestamp: String,
    pub kind: WorkRunEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WorkRunEventKind {
    RunCreated {
        root_work_id: WorkId,
    },
    WavePlanned {
        wave: u32,
        assigned: Vec<WorkId>,
        blocked: Vec<WorkId>,
    },
    WorkerLeased {
        work_id: WorkId,
        lease_id: WorkId,
    },
    WorkerCompleted {
        work_id: WorkId,
        outcome: RunOutcome,
    },
    HandoffRecorded {
        work_id: WorkId,
        summary: String,
    },
    RunPaused,
    RunResumed,
    RunCompleted,
    RunFailed {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub id: WorkId,
    pub work_id: Option<WorkId>,
    pub context_pack_id: Option<WorkId>,
    pub outcome: RunOutcome,
    pub summary: String,
    pub changed_paths: Vec<PathBuf>,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: Option<WorkId>,
    pub command: Option<String>,
    pub passed: bool,
    pub output_ref: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub id: WorkId,
    pub work_id: WorkId,
    pub worker_id: String,
    pub worktree: Option<PathBuf>,
    pub path_locks: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "item_kind", rename_all = "snake_case")]
pub enum WorkItem {
    Epic(Epic),
    Task(Task),
    Memory(MemoryItem),
    Decision(Decision),
    Prototype(Prototype),
    Check(Check),
    ContextPack(ContextPack),
    Run(Run),
    Lease(Lease),
}

impl WorkItem {
    pub fn id(&self) -> &str {
        match self {
            Self::Epic(item) => &item.id.0,
            Self::Task(item) => &item.id.0,
            Self::Memory(item) => &item.id.0,
            Self::Decision(item) => &item.id.0,
            Self::Prototype(item) => &item.id,
            Self::Check(item) => &item.id.0,
            Self::ContextPack(item) => &item.id.0,
            Self::Run(item) => &item.id.0,
            Self::Lease(item) => &item.id.0,
        }
    }

    pub fn kind(&self) -> WorkKind {
        match self {
            Self::Epic(_) => WorkKind::Epic,
            Self::Task(_) => WorkKind::Task,
            Self::Memory(_) => WorkKind::Memory,
            Self::Decision(_) => WorkKind::Decision,
            Self::Prototype(_) => WorkKind::Prototype,
            Self::Check(_) => WorkKind::Check,
            Self::ContextPack(_) => WorkKind::ContextPack,
            Self::Run(_) => WorkKind::Run,
            Self::Lease(_) => WorkKind::Lease,
        }
    }
}

impl Task {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: WorkId::new("T"),
            title: title.into(),
            parent: None,
            status: TaskStatus::Todo,
            acceptance: Vec::new(),
            checks: Vec::new(),
            context_pack: None,
            links: Vec::new(),
            source_refs: Vec::new(),
        }
    }
}

impl Epic {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: WorkId::new("E"),
            title: title.into(),
            status: TaskStatus::Todo,
            intent: None,
            acceptance: Vec::new(),
            links: Vec::new(),
        }
    }
}

impl MemoryItem {
    pub fn new(kind: MemoryKind, text: impl Into<String>) -> Self {
        Self {
            id: WorkId::new("M"),
            kind,
            text: text.into(),
            topics: Vec::new(),
            parent_work: None,
            paths: Vec::new(),
            source_refs: Vec::new(),
        }
    }
}

impl From<PrototypeStatus> for TaskStatus {
    fn from(status: PrototypeStatus) -> Self {
        match status {
            PrototypeStatus::Planned => Self::Todo,
            PrototypeStatus::Running => Self::Active,
            PrototypeStatus::Observed => Self::Review,
            PrototypeStatus::Promoted => Self::Done,
            PrototypeStatus::Discarded => Self::Dropped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_item_exposes_id_and_kind() {
        let task = WorkItem::Task(Task::new("Build context packs"));
        assert!(task.id().starts_with("T-"));
        assert_eq!(task.kind(), WorkKind::Task);
    }

    #[test]
    fn memory_items_can_link_to_conversation_source() {
        let mut memory = MemoryItem::new(
            MemoryKind::PrototypeLearning,
            "Prototype learning is durable even if scratch code is deleted.",
        );
        memory.topics.push("prototype".into());
        memory.source_refs.push(SourceRef {
            kind: SourceKind::Conversation,
            reference: "session:current".into(),
            fingerprint: None,
        });

        assert_eq!(memory.kind, MemoryKind::PrototypeLearning);
        assert_eq!(memory.source_refs[0].kind, SourceKind::Conversation);
    }
}
