#![recursion_limit = "256"]

pub mod agent;
pub mod builder;
pub mod compaction;
pub mod config;
pub mod context;
pub mod context_prefill;
pub mod contracts;
pub mod error;
pub mod error_display;
pub mod evidence;
pub mod guardrails;
pub mod hooks;
pub mod imp_session;
pub mod import;
pub mod learning;
pub mod mana_next;
pub mod mana_prompt_context;
pub mod mana_review;
pub mod mana_run_state;
pub mod mana_worker;
pub mod memory;
pub mod personality;
pub mod policy;
pub mod reference_monitor;
pub mod resources;
pub mod retry;
pub mod roles;
pub mod run_evidence;
pub mod runtime;
pub mod sdk;
pub mod session;
pub mod session_index;
pub mod storage;
pub mod system_prompt;
pub mod tools;
pub mod trace;
pub mod trust;
pub mod ui;
pub mod usage;
pub mod workflow;

pub use agent::{RecoveryCheckpoint, RecoveryCheckpointKind, TimingEvent, TimingStage};
pub use error::{Error, Result};
pub use error_display::format_error_for_display;
pub use imp_llm::{
    CancellationMode, ContinuationMode, PersistentSessionMode, ResumabilityMode,
    TransportCapabilities,
};
pub use mana_review::{ManaReviewState, ManaReviewUnitKind, ManaUnitRef, TurnManaReview};
pub use mana_run_state::{mana_run_summary, stop_mana_run, ManaRunSummary};
pub use sdk::*;

// Re-export imp-llm for downstream crates
pub use imp_llm;
