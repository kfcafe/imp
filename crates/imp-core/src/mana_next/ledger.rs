use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::workflow::{AutonomyMode, RiskLevel, WorkflowType};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowLedgerUpdate {
    pub run_id: Option<String>,
    pub status: Option<LedgerStatus>,
    pub blockers: Vec<String>,
    pub verification_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub closeout_status: Option<CloseoutStatus>,
    pub contract_artifact: Option<ArtifactRef>,
}

pub fn workflow_record_from_contract(
    id: impl Into<String>,
    contract: &crate::workflow::WorkflowContract,
    update: WorkflowLedgerUpdate,
) -> WorkflowRecord {
    let id = id.into();
    let title = contract.title.clone().unwrap_or_else(|| {
        contract
            .objective
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned()
    });

    WorkflowRecord {
        id,
        title,
        status: update.status.unwrap_or(LedgerStatus::Open),
        workflow_type: contract.workflow_type,
        risk_level: contract.risk_level,
        autonomy_mode: contract.autonomy_mode,
        parent: contract.parent_workflow_ref.clone(),
        contract_ref: Some(WorkflowContractRef {
            run_id: update.run_id.clone(),
            artifact: update.contract_artifact,
        }),
        acceptance: contract.closeout_criteria.criteria.clone(),
        closeout_criteria: contract.closeout_criteria.criteria.clone(),
        verification_refs: update.verification_refs,
        evidence_refs: update.evidence_refs,
        decision_refs: Vec::new(),
        note_refs: Vec::new(),
        child_run_refs: Vec::new(),
        blockers: update.blockers,
        final_status: update.closeout_status,
    }
}

pub fn apply_workflow_ledger_update(record: &mut WorkflowRecord, update: WorkflowLedgerUpdate) {
    if let Some(status) = update.status {
        record.status = status;
    }
    if let Some(closeout_status) = update.closeout_status {
        record.final_status = Some(closeout_status);
    }
    if let Some(contract_artifact) = update.contract_artifact {
        let contract_ref = record
            .contract_ref
            .get_or_insert_with(WorkflowContractRef::default);
        contract_ref.artifact = Some(contract_artifact);
    }
    if let Some(run_id) = update.run_id {
        let contract_ref = record
            .contract_ref
            .get_or_insert_with(WorkflowContractRef::default);
        contract_ref.run_id = Some(run_id);
    }
    extend_unique(&mut record.blockers, update.blockers);
    extend_unique(&mut record.verification_refs, update.verification_refs);
    extend_unique(&mut record.evidence_refs, update.evidence_refs);
}

fn extend_unique(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum LedgerRecord {
    Workflow(WorkflowRecord),
    Task(TaskRecord),
    Decision(DecisionRecord),
    Verification(VerificationRecord),
    Evidence(EvidenceRecord),
    Note(NoteRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowRecord {
    pub id: String,
    pub title: String,
    pub status: LedgerStatus,
    pub workflow_type: WorkflowType,
    pub risk_level: RiskLevel,
    pub autonomy_mode: AutonomyMode,
    pub parent: Option<String>,
    pub contract_ref: Option<WorkflowContractRef>,
    pub acceptance: Vec<String>,
    pub closeout_criteria: Vec<String>,
    pub verification_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub decision_refs: Vec<String>,
    pub note_refs: Vec<String>,
    pub child_run_refs: Vec<ChildRunRef>,
    pub blockers: Vec<String>,
    pub final_status: Option<CloseoutStatus>,
}

impl WorkflowRecord {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            ..Self::default()
        }
    }
}

impl Default for WorkflowRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            status: LedgerStatus::Open,
            workflow_type: WorkflowType::AdHoc,
            risk_level: RiskLevel::Unknown,
            autonomy_mode: AutonomyMode::Safe,
            parent: None,
            contract_ref: None,
            acceptance: Vec::new(),
            closeout_criteria: Vec::new(),
            verification_refs: Vec::new(),
            evidence_refs: Vec::new(),
            decision_refs: Vec::new(),
            note_refs: Vec::new(),
            child_run_refs: Vec::new(),
            blockers: Vec::new(),
            final_status: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskRecord {
    pub id: String,
    pub workflow_id: Option<String>,
    pub title: String,
    pub status: LedgerStatus,
    pub role: Option<String>,
    pub assignee: Option<String>,
    pub dependencies: Vec<String>,
    pub requires: Vec<String>,
    pub produces: Vec<String>,
    pub verification_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub closeout_status: Option<CloseoutStatus>,
}

impl TaskRecord {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            ..Self::default()
        }
    }
}

impl Default for TaskRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            workflow_id: None,
            title: String::new(),
            status: LedgerStatus::Open,
            role: None,
            assignee: None,
            dependencies: Vec::new(),
            requires: Vec::new(),
            produces: Vec::new(),
            verification_refs: Vec::new(),
            evidence_refs: Vec::new(),
            blockers: Vec::new(),
            closeout_status: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DecisionRecord {
    pub id: String,
    pub workflow_id: Option<String>,
    pub question: String,
    pub status: DecisionStatus,
    pub options: Vec<String>,
    pub outcome: Option<String>,
    pub rationale: Option<String>,
    pub blocks: Vec<String>,
}

impl Default for DecisionRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            workflow_id: None,
            question: String::new(),
            status: DecisionStatus::Open,
            options: Vec::new(),
            outcome: None,
            rationale: None,
            blocks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerificationRecord {
    pub id: String,
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub name: Option<String>,
    pub gate_type: VerificationGateType,
    pub required: bool,
    pub status: VerificationStatus,
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub artifact_refs: Vec<String>,
}

impl VerificationRecord {
    pub fn required_command(id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            gate_type: VerificationGateType::Command,
            required: true,
            command: Some(command.into()),
            ..Self::default()
        }
    }
}

impl Default for VerificationRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            workflow_id: None,
            task_id: None,
            name: None,
            gate_type: VerificationGateType::Manual,
            required: true,
            status: VerificationStatus::Pending,
            command: None,
            exit_code: None,
            artifact_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvidenceRecord {
    pub id: String,
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub evidence_type: EvidenceType,
    pub trust_label: Option<String>,
    pub summary: String,
    pub artifact: Option<ArtifactRef>,
    pub produced_by: Option<String>,
}

impl Default for EvidenceRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            workflow_id: None,
            task_id: None,
            run_id: None,
            evidence_type: EvidenceType::Other,
            trust_label: None,
            summary: String::new(),
            artifact: None,
            produced_by: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NoteRecord {
    pub id: String,
    pub workflow_id: Option<String>,
    pub task_id: Option<String>,
    pub source: NoteSource,
    pub trust_label: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorkflowContractRef {
    pub run_id: Option<String>,
    pub artifact: Option<ArtifactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArtifactRef {
    pub id: Option<String>,
    pub run_id: Option<String>,
    pub kind: ArtifactKind,
    pub path: PathBuf,
    pub media_type: Option<String>,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
}

impl ArtifactRef {
    pub fn new(kind: ArtifactKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
            ..Self::default()
        }
    }
}

impl Default for ArtifactRef {
    fn default() -> Self {
        Self {
            id: None,
            run_id: None,
            kind: ArtifactKind::Other,
            path: PathBuf::new(),
            media_type: None,
            sha256: None,
            bytes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ChildRunRef {
    pub child_id: String,
    pub role: Option<String>,
    pub status: LedgerStatus,
    pub workflow_id: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LedgerStatus {
    #[default]
    Open,
    Claimed,
    Planned,
    Executing,
    WaitingForApproval,
    Verifying,
    Blocked,
    Done,
    DoneWithConcerns,
    NeedsContext,
    Cancelled,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CloseoutStatus {
    #[default]
    Done,
    DoneWithConcerns,
    Blocked,
    NeedsContext,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DecisionStatus {
    #[default]
    Open,
    Resolved,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationGateType {
    Command,
    Diff,
    Policy,
    #[default]
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationStatus {
    #[default]
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceType {
    Trace,
    EvidencePacket,
    Diff,
    TestOutput,
    PolicyDecision,
    ToolObservation,
    ManualReview,
    ChildResult,
    EvalCandidate,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    Trace,
    EvidencePacket,
    Diff,
    VerifyLog,
    PolicyLog,
    WorkflowContract,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NoteSource {
    User,
    Agent,
    Tool,
    System,
    #[default]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mana_workflow_ledger_adapter_builds_record_from_contract() {
        let contract = crate::workflow::WorkflowContract::implicit("Implement adapter")
            .with_autonomy_mode(AutonomyMode::LocalAuto)
            .with_mana_unit_ref("394.3.5");
        let record = workflow_record_from_contract(
            "394.3.5",
            &contract,
            WorkflowLedgerUpdate {
                run_id: Some("run_1".into()),
                status: Some(LedgerStatus::Executing),
                blockers: vec!["waiting on schema".into()],
                verification_refs: vec!["verify_1".into()],
                evidence_refs: vec!["evidence_1".into()],
                closeout_status: None,
                contract_artifact: Some(ArtifactRef::new(
                    ArtifactKind::WorkflowContract,
                    ".imp/runs/run_1/workflow-contract.json",
                )),
            },
        );

        assert_eq!(record.id, "394.3.5");
        assert_eq!(record.autonomy_mode, AutonomyMode::LocalAuto);
        assert_eq!(record.status, LedgerStatus::Executing);
        assert_eq!(record.blockers, vec!["waiting on schema"]);
        assert_eq!(record.verification_refs, vec!["verify_1"]);
        assert_eq!(record.evidence_refs, vec!["evidence_1"]);
        assert_eq!(
            record
                .contract_ref
                .as_ref()
                .and_then(|r| r.run_id.as_deref()),
            Some("run_1")
        );
    }

    #[test]
    fn mana_workflow_ledger_adapter_updates_record_without_duplicate_refs() {
        let mut record = WorkflowRecord::new("394.3.5", "Adapter");
        apply_workflow_ledger_update(
            &mut record,
            WorkflowLedgerUpdate {
                status: Some(LedgerStatus::Blocked),
                blockers: vec!["needs storage decision".into()],
                verification_refs: vec!["verify_1".into()],
                evidence_refs: vec!["evidence_1".into()],
                ..WorkflowLedgerUpdate::default()
            },
        );
        apply_workflow_ledger_update(
            &mut record,
            WorkflowLedgerUpdate {
                status: Some(LedgerStatus::Done),
                blockers: vec!["needs storage decision".into()],
                verification_refs: vec!["verify_1".into(), "verify_2".into()],
                evidence_refs: vec!["evidence_1".into()],
                closeout_status: Some(CloseoutStatus::Done),
                ..WorkflowLedgerUpdate::default()
            },
        );

        assert_eq!(record.status, LedgerStatus::Done);
        assert_eq!(record.final_status, Some(CloseoutStatus::Done));
        assert_eq!(record.blockers, vec!["needs storage decision"]);
        assert_eq!(record.verification_refs, vec!["verify_1", "verify_2"]);
        assert_eq!(record.evidence_refs, vec!["evidence_1"]);
    }

    #[test]
    fn mana_workflow_ledger_round_trips_workflow_record() {
        let record = LedgerRecord::Workflow(WorkflowRecord {
            id: "394.3".into(),
            title: "Streamline mana".into(),
            status: LedgerStatus::Executing,
            workflow_type: WorkflowType::CodeChange,
            autonomy_mode: AutonomyMode::LocalAuto,
            verification_refs: vec!["verify_1".into()],
            evidence_refs: vec!["evidence_1".into()],
            child_run_refs: vec![ChildRunRef {
                child_id: "child_1".into(),
                role: Some("verifier".into()),
                status: LedgerStatus::Done,
                workflow_id: Some("394.3.child.1".into()),
                evidence_refs: vec!["evidence_child_1".into()],
            }],
            final_status: Some(CloseoutStatus::Done),
            ..WorkflowRecord::default()
        });

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("workflow"));
        let decoded: LedgerRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn mana_workflow_ledger_represents_task_decision_verification_evidence_and_note() {
        let records = vec![
            LedgerRecord::Task(TaskRecord::new("394.3.4", "Implement ledger types")),
            LedgerRecord::Decision(DecisionRecord {
                id: "dec_1".into(),
                question: "Use sidecars?".into(),
                options: vec!["frontmatter".into(), "sidecars".into()],
                ..DecisionRecord::default()
            }),
            LedgerRecord::Verification(VerificationRecord::required_command(
                "verify_1",
                "cargo test -p imp-core mana_workflow_ledger",
            )),
            LedgerRecord::Evidence(EvidenceRecord {
                id: "evidence_1".into(),
                evidence_type: EvidenceType::EvidencePacket,
                summary: "Evidence packet written".into(),
                artifact: Some(ArtifactRef::new(
                    ArtifactKind::EvidencePacket,
                    ".imp/runs/run_1/evidence.md",
                )),
                ..EvidenceRecord::default()
            }),
            LedgerRecord::Note(NoteRecord {
                id: "note_1".into(),
                source: NoteSource::Agent,
                body: "Compatibility mapping drafted".into(),
                ..NoteRecord::default()
            }),
        ];

        for record in records {
            let value = serde_json::to_value(&record).unwrap();
            let decoded: LedgerRecord = serde_json::from_value(value).unwrap();
            assert_eq!(decoded, record);
        }
    }
}
