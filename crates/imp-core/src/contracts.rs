//! imp-owned worker and evidence contract types.
//!
//! These DTOs define the boundary between imp's workflow worker runtime,
//! and future runner surfaces. They used to live in the experimental
//! the earlier experimental contracts crate, but currently only imp consumes
//! them, so they stay
//! local until a real cross-repo/versioned protocol boundary is needed.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A previous attempt on a worker-assigned unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerAttempt {
    pub number: u32,
    pub outcome: String,
    pub summary: String,
}

/// Everything needed to execute a unit as a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerAssignment {
    /// The unit ID (e.g. "28.1.2").
    pub id: String,
    /// Unit title.
    pub title: String,
    /// Combined description (frontmatter + body).
    pub description: String,
    /// Supplemental design guidance, if any.
    pub design: Option<String>,
    /// Acceptance criteria, if any.
    pub acceptance: Option<String>,
    /// Verify command, if any.
    pub verify: Option<String>,
    /// Effective verify timeout in seconds, if any.
    pub verify_timeout_secs: Option<u64>,
    /// Whether the verify gate was proven failing before work started.
    pub fail_first: bool,
    /// Unit notes (progress, diagnosis, etc.).
    pub notes: Option<String>,
    /// Unresolved decisions.
    pub decisions: Vec<String>,
    /// Dependency IDs.
    pub dependencies: Vec<String>,
    /// File paths declared on the unit.
    pub paths: Vec<String>,
    /// Explicit file references for context prefill.
    pub files: Vec<String>,
    /// Structured attempt history.
    pub attempts: Vec<WorkerAttempt>,
    /// Workspace root (parent of .imp/workflows/).
    pub workspace_root: PathBuf,
    /// Model override from unit metadata, if any.
    pub model: Option<String>,
}

/// Structured outcome from a worker run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerResult {
    /// The unit ID.
    pub unit_id: String,
    /// Final status.
    pub status: WorkerStatus,
    /// Human-readable summary of what happened.
    pub summary: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Number of tool calls made.
    pub tool_count: usize,
    /// Number of agent turns.
    pub turns: usize,
    /// Total tokens used, if available.
    pub tokens: Option<u64>,
    /// Total cost, if available.
    pub cost: Option<f64>,
    /// Model used.
    pub model: Option<String>,
}

/// Worker completion status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// Agent completed work and handed back a candidate result; verify has not
    /// run yet. This is the pre-verification success stage on the path to
    /// `Completed`, not a separate completion universe.
    AwaitingVerify,
    /// Agent completed work and verify passed.
    Completed,
    /// Agent hit a blocker it could not resolve.
    Blocked,
    /// Agent or verify failed.
    Failed,
    /// Run was cancelled.
    Cancelled,
}

impl WorkerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AwaitingVerify => "awaiting_verify",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn lifecycle_label(self) -> &'static str {
        match self {
            Self::AwaitingVerify => "candidate complete · awaiting verify",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.lifecycle_label())
    }
}

/// Minimal shared verification status for cross-boundary lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierStatus {
    Passed,
    Failed,
    Skipped,
}

/// Narrow first-pass artifact kinds that other runtimes can trust cold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    VerifyOutput,
    DiffScopeSummary,
    Patch,
    ReviewRecord,
    Log,
    Other,
}

/// Reference to a durable artifact without embedding storage-heavy payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub artifact_id: String,
    pub kind: ArtifactKind,
    /// Path, URI, or storage-specific lookup token.
    pub locator: String,
    pub run_id: Option<String>,
    pub unit_id: Option<String>,
    pub stage: Option<String>,
}

/// Minimal verifier result lineage shared across imp and workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierResult {
    pub verifier_name: String,
    pub status: VerifierStatus,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub summary: Option<String>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub run_id: Option<String>,
    pub unit_id: Option<String>,
}

/// Reference-first evidence bundle shape; storage stays owned by workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundleRef {
    pub bundle_id: String,
    pub unit_id: String,
    pub run_id: Option<String>,
    pub artifact_refs: Vec<ArtifactRef>,
    pub summary: Option<String>,
}

/// Shared worker-facing contracts.
pub mod worker {
    pub use super::{WorkerAssignment, WorkerAttempt, WorkerResult, WorkerStatus};
}

/// Placeholder runner-facing contract modules.
pub mod runner {}

/// Shared evidence and artifact reference vocabulary.
pub mod evidence {
    pub use super::{ArtifactKind, ArtifactRef, EvidenceBundleRef, VerifierResult, VerifierStatus};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_status_serializes_as_snake_case() {
        let json = serde_json::to_string(&WorkerStatus::AwaitingVerify).unwrap();
        assert_eq!(json, "\"awaiting_verify\"");
    }

    #[test]
    fn verifier_result_round_trips_with_artifact_refs() {
        let verifier = VerifierResult {
            verifier_name: "unit.verify".to_string(),
            status: VerifierStatus::Failed,
            command: Some("cargo test".to_string()),
            exit_code: Some(1),
            summary: Some("verify failed".to_string()),
            artifact_refs: vec![ArtifactRef {
                artifact_id: "artifact-1".to_string(),
                kind: ArtifactKind::VerifyOutput,
                locator: "workflow://units/9/artifacts/verify-output".to_string(),
                run_id: Some("run-1".to_string()),
                unit_id: Some("9".to_string()),
                stage: Some("verify".to_string()),
            }],
            started_at: None,
            finished_at: None,
            run_id: Some("run-1".to_string()),
            unit_id: Some("9".to_string()),
        };

        let json = serde_json::to_string(&verifier).unwrap();
        let round_trip: VerifierResult = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, verifier);
    }
}
