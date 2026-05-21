pub mod context_pack;
pub mod mana_shadow;
pub mod memory;
pub mod model;
pub mod prepared_prototype;
pub mod prepared_worker;
pub mod prototype;
pub mod runtime;
pub mod scheduler;
pub mod store;

pub use context_pack::{
    ContextCompileRequest, ContextCompiler, ContextFreshness, ContextLaunchKind, ContextRenderer,
    RenderedContextBlock, RenderedContextPack,
};
pub use mana_shadow::{import_mana_unit_shadow, ManaShadowImport, ManaShadowUnit};
pub use memory::{
    capture_conversation_memory, classify_memory_kind, ConversationMemoryIndex,
    ConversationMemoryInput, ConversationMemoryMatch, ConversationMemoryQuery,
};
pub use model::{
    Check, CheckKind, CheckResult, ContextBlock, ContextBlockStability, ContextPack,
    ContextPackStatus, Decision, DecisionStatus, Epic, Lease, Link, LinkKind, MemoryItem,
    MemoryKind, Run, RunOutcome, SourceKind, SourceRef, Task, TaskStatus, WorkId, WorkItem,
    WorkKind,
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
pub use runtime::{PrototypeExecutor, RuntimeExecutionResult, TaskExecutor, WorkRuntime};
pub use scheduler::{
    CoordinatorSummary, LeaseRecord, LeaseRequest, Scheduler, SchedulerError, WorkOutcome,
    WorkerProfile,
};
pub use store::{
    CoordinatorSnapshot, WorkLayout, WorkStore, WorkValidationIssue, WorkValidationReport,
    WorkValidationSeverity, WorkerPersistence,
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
