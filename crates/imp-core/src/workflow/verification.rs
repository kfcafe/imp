use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{VerificationRequirement, VerificationRequirementKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerificationGate {
    pub id: String,
    pub name: String,
    pub kind: VerificationGateKind,
    pub requirement: VerificationGateRequirement,
    pub status: VerificationGateStatus,
    pub command: Option<VerificationCommand>,
    pub result: Option<VerificationGateResult>,
    pub artifacts: Vec<VerificationArtifactRef>,
    pub source: VerificationGateSource,
    pub reason: Option<String>,
}

impl VerificationGate {
    pub fn command(id: impl Into<String>, command: impl Into<String>) -> Self {
        let id = id.into();
        let command = command.into();
        Self {
            name: id.clone(),
            kind: VerificationGateKind::Command,
            requirement: VerificationGateRequirement::Required,
            status: VerificationGateStatus::Pending,
            command: Some(VerificationCommand::new(command)),
            source: VerificationGateSource::WorkflowContract,
            id,
            ..Self::default()
        }
    }

    pub fn from_requirement(index: usize, requirement: &VerificationRequirement) -> Self {
        let id = format!("verify-{}", index + 1);
        let mut gate = match &requirement.kind {
            VerificationRequirementKind::Command { command } => Self::command(id, command.clone()),
            VerificationRequirementKind::Diff => Self::typed(id, VerificationGateKind::Diff),
            VerificationRequirementKind::Policy => Self::typed(id, VerificationGateKind::Policy),
            VerificationRequirementKind::Manual => Self::typed(id, VerificationGateKind::Manual),
        };
        gate.requirement = if requirement.required {
            VerificationGateRequirement::Required
        } else {
            VerificationGateRequirement::Optional
        };
        if let Some(name) = &requirement.name {
            gate.name = name.clone();
        }
        gate
    }

    pub fn typed(id: impl Into<String>, kind: VerificationGateKind) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            kind,
            requirement: VerificationGateRequirement::Required,
            status: VerificationGateStatus::Pending,
            source: VerificationGateSource::WorkflowContract,
            id,
            ..Self::default()
        }
    }

    pub fn is_required(&self) -> bool {
        self.requirement == VerificationGateRequirement::Required
    }

    pub fn closeout_effect(&self) -> VerificationCloseoutEffect {
        match (self.requirement, self.status) {
            (VerificationGateRequirement::Required, VerificationGateStatus::Passed) => {
                VerificationCloseoutEffect::AllowsDone
            }
            (VerificationGateRequirement::Required, VerificationGateStatus::Failed) => {
                VerificationCloseoutEffect::BlocksDoneWithConcerns
            }
            (
                VerificationGateRequirement::Required,
                VerificationGateStatus::Skipped
                | VerificationGateStatus::Pending
                | VerificationGateStatus::Running,
            ) => VerificationCloseoutEffect::BlocksDoneWithConcerns,
            (VerificationGateRequirement::Required, VerificationGateStatus::Blocked) => {
                VerificationCloseoutEffect::BlocksDone
            }
            (VerificationGateRequirement::Optional | VerificationGateRequirement::Advisory, _) => {
                VerificationCloseoutEffect::AllowsDone
            }
        }
    }

    pub fn mark_running(&mut self) {
        self.status = VerificationGateStatus::Running;
    }

    pub fn mark_passed(&mut self, result: VerificationGateResult) {
        self.status = VerificationGateStatus::Passed;
        self.result = Some(result);
    }

    pub fn mark_failed(&mut self, result: VerificationGateResult) {
        self.status = VerificationGateStatus::Failed;
        self.result = Some(result);
    }

    pub fn mark_skipped(&mut self, reason: impl Into<String>) {
        self.status = VerificationGateStatus::Skipped;
        self.reason = Some(reason.into());
    }

    pub fn mark_blocked(&mut self, reason: impl Into<String>) {
        self.status = VerificationGateStatus::Blocked;
        self.reason = Some(reason.into());
    }
}

impl Default for VerificationGate {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: VerificationGateKind::Manual,
            requirement: VerificationGateRequirement::Required,
            status: VerificationGateStatus::Pending,
            command: None,
            result: None,
            artifacts: Vec::new(),
            source: VerificationGateSource::WorkflowContract,
            reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum VerificationGateKind {
    Command,
    Diff,
    Policy,
    Manual,
    Custom { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationGateRequirement {
    Required,
    Optional,
    Advisory,
}

impl Default for VerificationGateRequirement {
    fn default() -> Self {
        Self::Required
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationGateStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    Blocked,
}

impl Default for VerificationGateStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerificationCommand {
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub timeout: Option<Duration>,
}

impl VerificationCommand {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            cwd: None,
            timeout: None,
        }
    }
}

impl Default for VerificationCommand {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct VerificationGateResult {
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub summary: Option<String>,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
}

impl VerificationGateResult {
    pub fn passed(exit_code: i32) -> Self {
        Self {
            exit_code: Some(exit_code),
            ..Self::default()
        }
    }

    pub fn failed(exit_code: i32) -> Self {
        Self {
            exit_code: Some(exit_code),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerificationArtifactRef {
    pub kind: String,
    pub path: PathBuf,
    pub summary: Option<String>,
    pub bytes: Option<u64>,
    pub redaction: Option<String>,
}

impl VerificationArtifactRef {
    pub fn new(kind: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
            summary: None,
            bytes: None,
            redaction: None,
        }
    }
}

impl Default for VerificationArtifactRef {
    fn default() -> Self {
        Self::new("artifact", PathBuf::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "source")]
pub enum VerificationGateSource {
    WorkflowContract,
    ManaTask { unit_id: Option<String> },
    User,
    Inferred,
    Policy,
    Extension { id: String },
}

impl Default for VerificationGateSource {
    fn default() -> Self {
        Self::WorkflowContract
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationCloseoutEffect {
    AllowsDone,
    BlocksDoneWithConcerns,
    BlocksDone,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_gate_command_defaults_to_required_pending() {
        let gate = VerificationGate::command("unit-tests", "cargo test -p imp-core");
        assert_eq!(gate.id, "unit-tests");
        assert_eq!(gate.name, "unit-tests");
        assert_eq!(gate.kind, VerificationGateKind::Command);
        assert_eq!(gate.requirement, VerificationGateRequirement::Required);
        assert_eq!(gate.status, VerificationGateStatus::Pending);
        assert!(gate.is_required());
        assert_eq!(
            gate.command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("cargo test -p imp-core")
        );
    }

    #[test]
    fn verification_gate_serde_roundtrip_preserves_status_and_artifacts() {
        let mut gate = VerificationGate::command("fmt", "cargo fmt --check");
        gate.source = VerificationGateSource::ManaTask {
            unit_id: Some("394.7.2".into()),
        };
        gate.artifacts.push(VerificationArtifactRef::new(
            "stdout",
            ".imp/runs/run_1/verification/fmt/stdout.log",
        ));
        gate.mark_failed(VerificationGateResult::failed(1));

        let json = serde_json::to_string(&gate).unwrap();
        let decoded: VerificationGate = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, gate);
        assert_eq!(decoded.status, VerificationGateStatus::Failed);
        assert_eq!(
            decoded.closeout_effect(),
            VerificationCloseoutEffect::BlocksDoneWithConcerns
        );
    }

    #[test]
    fn verification_gate_from_requirement_maps_required_and_optional() {
        let required = VerificationRequirement::command("cargo test");
        let gate = VerificationGate::from_requirement(0, &required);
        assert_eq!(gate.id, "verify-1");
        assert_eq!(gate.requirement, VerificationGateRequirement::Required);
        assert_eq!(gate.kind, VerificationGateKind::Command);

        let optional = VerificationRequirement {
            name: Some("manual smoke".into()),
            kind: VerificationRequirementKind::Manual,
            required: false,
        };
        let gate = VerificationGate::from_requirement(1, &optional);
        assert_eq!(gate.id, "verify-2");
        assert_eq!(gate.name, "manual smoke");
        assert_eq!(gate.kind, VerificationGateKind::Manual);
        assert_eq!(gate.requirement, VerificationGateRequirement::Optional);
        assert_eq!(
            gate.closeout_effect(),
            VerificationCloseoutEffect::AllowsDone
        );
    }

    #[test]
    fn verification_gate_status_transitions_update_closeout_effect() {
        let mut gate = VerificationGate::command("test", "cargo test");
        assert_eq!(
            gate.closeout_effect(),
            VerificationCloseoutEffect::BlocksDoneWithConcerns
        );
        gate.mark_running();
        assert_eq!(gate.status, VerificationGateStatus::Running);
        gate.mark_passed(VerificationGateResult::passed(0));
        assert_eq!(gate.status, VerificationGateStatus::Passed);
        assert_eq!(
            gate.closeout_effect(),
            VerificationCloseoutEffect::AllowsDone
        );

        gate.mark_failed(VerificationGateResult::failed(101));
        assert_eq!(gate.status, VerificationGateStatus::Failed);
        assert_eq!(
            gate.closeout_effect(),
            VerificationCloseoutEffect::BlocksDoneWithConcerns
        );

        gate.mark_blocked("missing cargo");
        assert_eq!(gate.status, VerificationGateStatus::Blocked);
        assert_eq!(gate.reason.as_deref(), Some("missing cargo"));
        assert_eq!(
            gate.closeout_effect(),
            VerificationCloseoutEffect::BlocksDone
        );
    }
}
