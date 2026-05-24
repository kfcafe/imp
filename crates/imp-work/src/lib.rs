pub mod context_pack;
pub mod event;
pub mod global_store;
pub mod mana_shadow;
pub mod memory;
pub mod model;
pub mod prepared_prototype;
pub mod prepared_worker;
pub mod prototype;
pub mod run_engine;
pub mod runtime;
pub mod scheduler;
pub mod store;
pub mod workflow;

pub use context_pack::{
    ContextCompileRequest, ContextCompiler, ContextFreshness, ContextLaunchKind, ContextRenderer,
    RenderedContextBlock, RenderedContextPack,
};
pub use event::{
    ArtifactKind, ArtifactRef, EventCursor, EventLog, EvidenceSummary, WorkEvent, WorkEventKind,
};
pub use global_store::{
    GlobalWorkStore, ProjectScopedDecision, ProjectScopedMemory, ProjectScopedTask,
    ProjectWorkStream, StreamEvent, StreamRelation,
};
pub use mana_shadow::{
    dry_run_mana_migration, dry_run_project_local_migration, import_mana_unit_shadow,
    migrate_mana_to_store, migrate_project_local_to_global, ManaHistoryRef, ManaMigrationReport,
    ManaShadowImport, ManaShadowUnit, ProjectLocalMigrationReport,
};
pub use memory::{
    capture_conversation_memory, classify_memory_kind, ConversationMemoryIndex,
    ConversationMemoryInput, ConversationMemoryMatch, ConversationMemoryQuery,
};
pub use model::{
    Check, CheckKind, CheckResult, ContextBlock, ContextBlockStability, ContextPack,
    ContextPackStatus, Decision, DecisionStatus, Epic, Lease, Link, LinkKind, MemoryItem,
    MemoryKind, Run, RunOutcome, SourceKind, SourceRef, Task, TaskStatus, WorkId, WorkItem,
    WorkKind, WorkRun, WorkRunAssignment, WorkRunEvent, WorkRunEventKind, WorkRunPolicy,
    WorkRunStatus,
};
pub use prepared_prototype::{
    compile_prepared_prototype_context, PreparedPrototypeError, PreparedPrototypeLaunch,
    PreparedPrototypeLoop, PreparedPrototypeRequest, PreparedPrototypeResult,
};
pub use prepared_worker::{
    PreparedWorkerError, PreparedWorkerLaunch, PreparedWorkerLoop, PreparedWorkerRequest,
    PreparedWorkerResult,
};
pub use prototype::{
    HypothesisResult, Prototype, PrototypeDecision, PrototypeEvidence, PrototypeJournal,
    PrototypeObservation, PrototypeOutcome, PrototypeRecordPolicy, PrototypeStatus,
};
pub use run_engine::{
    active_work_run_status, next_actions_for_run, WorkRunEngine, WorkRunPlan, WorkRunPolicyInput,
    WorkRunView,
};
pub use runtime::{PrototypeExecutor, RuntimeExecutionResult, TaskExecutor, WorkRuntime};
pub use scheduler::{
    CoordinatorSummary, DispatchBlocker, DispatchBlockerReason, DispatchPlan, LeaseRecord,
    LeaseRequest, MultiAgentRunPlan, MultiAgentRunResult, PathConflictPolicy, RunPolicy, Scheduler,
    SchedulerError, WorkOutcome, WorkerCompletion, WorkerProfile,
};
pub use store::{
    CoordinatorSnapshot, WorkLayout, WorkStore, WorkValidationIssue, WorkValidationReport,
    WorkValidationSeverity, WorkerPersistence,
};
pub use workflow::{
    build_work_tree, close_task_with_conventions, fail_task_with_conventions, readiness_for,
    summarize_checks, CloseRequest, CloseResult, FailResult, Readiness, VerifySummary, WorkTree,
    WorkTreeNode,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("prototype error: {0}")]
    Prototype(String),
}

pub type Result<T> = std::result::Result<T, Error>;
