use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Input used to construct a lightweight implicit workflow contract for
/// existing imp runs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImplicitWorkflowContractInput {
    pub objective: String,
    pub cwd: Option<PathBuf>,
    pub autonomy_mode: Option<AutonomyMode>,
    pub workflow_type: Option<WorkflowType>,
    pub risk_level: Option<RiskLevel>,
    pub mana_unit_ref: Option<String>,
}

impl ImplicitWorkflowContractInput {
    pub fn prompt(objective: impl Into<String>) -> Self {
        Self {
            objective: objective.into(),
            ..Self::default()
        }
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn autonomy_mode(mut self, autonomy_mode: AutonomyMode) -> Self {
        self.autonomy_mode = Some(autonomy_mode);
        self
    }

    pub fn workflow_type(mut self, workflow_type: WorkflowType) -> Self {
        self.workflow_type = Some(workflow_type);
        self
    }

    pub fn risk_level(mut self, risk_level: RiskLevel) -> Self {
        self.risk_level = Some(risk_level);
        self
    }

    pub fn mana_unit_ref(mut self, mana_unit_ref: impl Into<String>) -> Self {
        self.mana_unit_ref = Some(mana_unit_ref.into());
        self
    }
}

/// A runtime-readable declaration of what a workflow is trying to do and what
/// constraints/proof obligations apply to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowContract {
    pub id: Option<String>,
    pub title: Option<String>,
    pub objective: String,
    pub workflow_type: WorkflowType,
    pub risk_level: RiskLevel,
    pub autonomy_mode: AutonomyMode,
    pub workspace_scope: WorkspaceScope,
    pub tool_permissions: ToolPermissionSet,
    pub required_verification: Vec<VerificationRequirement>,
    pub approval_requirements: Vec<ApprovalRequirement>,
    pub trust_scope: TrustScope,
    pub closeout_criteria: CloseoutCriteria,
    pub mana_unit_ref: Option<String>,
    pub parent_workflow_ref: Option<String>,
    pub role: Option<String>,
}

impl WorkflowContract {
    pub fn implicit(objective: impl Into<String>) -> Self {
        Self::implicit_from(ImplicitWorkflowContractInput::prompt(objective))
    }

    pub fn implicit_from(input: ImplicitWorkflowContractInput) -> Self {
        let workspace_scope = input
            .cwd
            .as_deref()
            .map(workspace_scope_for_cwd)
            .unwrap_or_default();

        Self {
            title: title_from_objective(&input.objective),
            objective: input.objective,
            workflow_type: input.workflow_type.unwrap_or_default(),
            risk_level: input.risk_level.unwrap_or_default(),
            autonomy_mode: input.autonomy_mode.unwrap_or_default(),
            workspace_scope,
            mana_unit_ref: input.mana_unit_ref,
            ..Self::default()
        }
    }

    pub fn with_workspace_scope(mut self, workspace_scope: WorkspaceScope) -> Self {
        self.workspace_scope = workspace_scope;
        self
    }

    pub fn with_autonomy_mode(mut self, autonomy_mode: AutonomyMode) -> Self {
        self.autonomy_mode = autonomy_mode;
        self
    }

    pub fn with_mana_unit_ref(mut self, mana_unit_ref: impl Into<String>) -> Self {
        self.mana_unit_ref = Some(mana_unit_ref.into());
        self
    }
}

impl Default for WorkflowContract {
    fn default() -> Self {
        Self {
            id: None,
            title: None,
            objective: String::new(),
            workflow_type: WorkflowType::AdHoc,
            risk_level: RiskLevel::Unknown,
            autonomy_mode: AutonomyMode::Safe,
            workspace_scope: WorkspaceScope::CurrentDirectory,
            tool_permissions: ToolPermissionSet::default(),
            required_verification: Vec::new(),
            approval_requirements: Vec::new(),
            trust_scope: TrustScope::default(),
            closeout_criteria: CloseoutCriteria::default(),
            mana_unit_ref: None,
            parent_workflow_ref: None,
            role: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowType {
    #[default]
    AdHoc,
    CodeChange,
    Investigation,
    Review,
    Planning,
    Documentation,
    Verification,
    Orchestration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AutonomyMode {
    Suggest,
    #[default]
    Safe,
    LocalAuto,
    WorktreeAuto,
    AllowAllLocal,
    AllowAll,
    Ci,
}

impl AutonomyMode {
    pub const ALL: [Self; 7] = [
        Self::Suggest,
        Self::Safe,
        Self::LocalAuto,
        Self::WorktreeAuto,
        Self::AllowAllLocal,
        Self::AllowAll,
        Self::Ci,
    ];

    pub fn canonical_name(self) -> &'static str {
        match self {
            AutonomyMode::Suggest => "suggest",
            AutonomyMode::Safe => "safe",
            AutonomyMode::LocalAuto => "local-auto",
            AutonomyMode::WorktreeAuto => "worktree-auto",
            AutonomyMode::AllowAllLocal => "allow-all-local",
            AutonomyMode::AllowAll => "allow-all",
            AutonomyMode::Ci => "ci",
        }
    }
}

impl fmt::Display for AutonomyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_name())
    }
}

impl FromStr for AutonomyMode {
    type Err = ParseAutonomyModeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "suggest" | "plan" | "planning" | "review" | "review-only" => Ok(Self::Suggest),
            "safe" | "default" | "interactive" => Ok(Self::Safe),
            "local" | "auto-local" | "local-auto" => Ok(Self::LocalAuto),
            "worktree" | "auto-worktree" | "worktree-auto" => Ok(Self::WorktreeAuto),
            "allow-all-local" | "all-local" | "local-all" | "yolo-local" => Ok(Self::AllowAllLocal),
            "allow-all" | "all" | "yolo" => Ok(Self::AllowAll),
            "ci" | "headless" | "noninteractive" | "non-interactive" => Ok(Self::Ci),
            _ => Err(ParseAutonomyModeError(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAutonomyModeError(String);

impl fmt::Display for ParseAutonomyModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown autonomy mode `{}`", self.0)
    }
}

impl std::error::Error for ParseAutonomyModeError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceScope {
    #[default]
    CurrentDirectory,
    Repository {
        root: PathBuf,
    },
    Worktree {
        path: PathBuf,
        branch: Option<String>,
    },
    Custom {
        root: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ToolPermissionSet {
    pub allowed_tools: BTreeSet<String>,
    pub denied_tools: BTreeSet<String>,
}

impl ToolPermissionSet {
    pub fn allow(mut self, tool: impl Into<String>) -> Self {
        self.allowed_tools.insert(normalize_tool_name(tool.into()));
        self
    }

    pub fn deny(mut self, tool: impl Into<String>) -> Self {
        self.denied_tools.insert(normalize_tool_name(tool.into()));
        self
    }

    pub fn allows_all_by_default(&self) -> bool {
        self.allowed_tools.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerificationRequirement {
    pub name: Option<String>,
    pub kind: VerificationRequirementKind,
    pub required: bool,
}

impl VerificationRequirement {
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            name: None,
            kind: VerificationRequirementKind::Command {
                command: command.into(),
            },
            required: true,
        }
    }
}

impl Default for VerificationRequirement {
    fn default() -> Self {
        Self {
            name: None,
            kind: VerificationRequirementKind::Manual,
            required: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum VerificationRequirementKind {
    Command {
        command: String,
    },
    Diff,
    Policy,
    #[default]
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApprovalRequirement {
    pub action: ApprovalAction,
    pub reason: Option<String>,
    pub required: bool,
}

impl ApprovalRequirement {
    pub fn required(action: ApprovalAction, reason: impl Into<String>) -> Self {
        Self {
            action,
            reason: Some(reason.into()),
            required: true,
        }
    }
}

impl Default for ApprovalRequirement {
    fn default() -> Self {
        Self {
            action: ApprovalAction::HighRiskTool,
            reason: None,
            required: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalAction {
    #[default]
    HighRiskTool,
    Network,
    SecretAccess,
    OutsideWorkspaceWrite,
    DestructiveShell,
    DependencyChange,
    SchemaMigration,
    Deployment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrustScope {
    pub allow_external_context: bool,
    pub allow_durable_memory_writes: bool,
    pub low_trust_requires_review: bool,
}

impl Default for TrustScope {
    fn default() -> Self {
        Self {
            allow_external_context: true,
            allow_durable_memory_writes: true,
            low_trust_requires_review: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CloseoutCriteria {
    pub require_summary: bool,
    pub require_evidence_packet: bool,
    pub require_no_unresolved_required_verification: bool,
    pub criteria: Vec<String>,
}

impl Default for CloseoutCriteria {
    fn default() -> Self {
        Self {
            require_summary: true,
            require_evidence_packet: false,
            require_no_unresolved_required_verification: true,
            criteria: Vec::new(),
        }
    }
}

fn workspace_scope_for_cwd(cwd: &Path) -> WorkspaceScope {
    match std::fs::canonicalize(cwd) {
        Ok(root) => WorkspaceScope::Repository { root },
        Err(_) => WorkspaceScope::Repository {
            root: cwd.to_path_buf(),
        },
    }
}

fn normalize_tool_name(tool: String) -> String {
    tool.trim().to_ascii_lowercase()
}

fn title_from_objective(objective: &str) -> Option<String> {
    let title = objective.lines().next().unwrap_or_default().trim();
    if title.is_empty() {
        None
    } else {
        Some(title.chars().take(80).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_contract_defaults_are_safe_and_lightweight() {
        let contract = WorkflowContract::implicit("Fix the failing auth test");
        assert_eq!(contract.objective, "Fix the failing auth test");
        assert_eq!(contract.title.as_deref(), Some("Fix the failing auth test"));
        assert_eq!(contract.workflow_type, WorkflowType::AdHoc);
        assert_eq!(contract.risk_level, RiskLevel::Unknown);
        assert_eq!(contract.autonomy_mode, AutonomyMode::Safe);
        assert_eq!(contract.workspace_scope, WorkspaceScope::CurrentDirectory);
        assert!(contract.tool_permissions.allows_all_by_default());
        assert!(contract.required_verification.is_empty());
        assert!(contract.closeout_criteria.require_summary);
    }

    #[test]
    fn implicit_workflow_contract_uses_run_context() {
        let temp = tempfile::TempDir::new().unwrap();
        let contract = WorkflowContract::implicit_from(
            ImplicitWorkflowContractInput::prompt("Fix login tests")
                .cwd(temp.path())
                .autonomy_mode(AutonomyMode::LocalAuto)
                .workflow_type(WorkflowType::CodeChange)
                .risk_level(RiskLevel::Medium),
        );

        assert_eq!(contract.objective, "Fix login tests");
        assert_eq!(contract.title.as_deref(), Some("Fix login tests"));
        assert_eq!(contract.autonomy_mode, AutonomyMode::LocalAuto);
        assert_eq!(contract.workflow_type, WorkflowType::CodeChange);
        assert_eq!(contract.risk_level, RiskLevel::Medium);
        assert!(matches!(
            contract.workspace_scope,
            WorkspaceScope::Repository { .. }
        ));
        assert!(contract.tool_permissions.allows_all_by_default());
        assert!(contract.closeout_criteria.require_summary);
    }

    #[test]
    fn implicit_workflow_contract_records_mana_unit_ref() {
        let contract = WorkflowContract::implicit_from(
            ImplicitWorkflowContractInput::prompt("Implement mana task").mana_unit_ref("394.2.2"),
        );

        assert_eq!(contract.mana_unit_ref.as_deref(), Some("394.2.2"));
        assert_eq!(contract.objective, "Implement mana task");
        assert_eq!(contract.autonomy_mode, AutonomyMode::Safe);
        assert_eq!(contract.risk_level, RiskLevel::Unknown);
    }

    #[test]
    fn workflow_contract_serializes_with_kebab_case_modes() {
        let contract = WorkflowContract::implicit("Refactor parser")
            .with_autonomy_mode(AutonomyMode::LocalAuto)
            .with_workspace_scope(WorkspaceScope::Repository {
                root: PathBuf::from("/tmp/repo"),
            });

        let json = serde_json::to_string(&contract).expect("serialize contract");
        assert!(json.contains("local-auto"));
        let decoded: WorkflowContract = serde_json::from_str(&json).expect("deserialize contract");
        assert_eq!(decoded, contract);
    }

    #[test]
    fn autonomy_modes_have_canonical_names_and_safe_default() {
        assert_eq!(AutonomyMode::default(), AutonomyMode::Safe);
        let names: Vec<_> = AutonomyMode::ALL
            .iter()
            .map(|mode| mode.canonical_name())
            .collect();
        assert_eq!(
            names,
            vec![
                "suggest",
                "safe",
                "local-auto",
                "worktree-auto",
                "allow-all-local",
                "allow-all",
                "ci"
            ]
        );
        for mode in AutonomyMode::ALL {
            assert_eq!(mode.to_string(), mode.canonical_name());
        }
    }

    #[test]
    fn autonomy_modes_parse_canonical_names_and_aliases() {
        let cases = [
            ("suggest", AutonomyMode::Suggest),
            ("plan", AutonomyMode::Suggest),
            ("planning", AutonomyMode::Suggest),
            ("review-only", AutonomyMode::Suggest),
            ("safe", AutonomyMode::Safe),
            ("default", AutonomyMode::Safe),
            ("interactive", AutonomyMode::Safe),
            ("local", AutonomyMode::LocalAuto),
            ("local_auto", AutonomyMode::LocalAuto),
            ("auto-local", AutonomyMode::LocalAuto),
            ("worktree", AutonomyMode::WorktreeAuto),
            ("worktree_auto", AutonomyMode::WorktreeAuto),
            ("auto-worktree", AutonomyMode::WorktreeAuto),
            ("allow-all-local", AutonomyMode::AllowAllLocal),
            ("all-local", AutonomyMode::AllowAllLocal),
            ("local-all", AutonomyMode::AllowAllLocal),
            ("yolo-local", AutonomyMode::AllowAllLocal),
            ("allow-all", AutonomyMode::AllowAll),
            ("all", AutonomyMode::AllowAll),
            ("yolo", AutonomyMode::AllowAll),
            ("ci", AutonomyMode::Ci),
            ("headless", AutonomyMode::Ci),
            ("noninteractive", AutonomyMode::Ci),
            ("non-interactive", AutonomyMode::Ci),
        ];

        for (input, expected) in cases {
            assert_eq!(input.parse::<AutonomyMode>().unwrap(), expected, "{input}");
            assert_eq!(
                input.to_ascii_uppercase().parse::<AutonomyMode>().unwrap(),
                expected,
                "uppercase {input}"
            );
        }
        assert!("dangerous".parse::<AutonomyMode>().is_err());
    }

    #[test]
    fn autonomy_modes_serde_roundtrip_canonical_names() {
        for mode in AutonomyMode::ALL {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{}\"", mode.canonical_name()));
            let decoded: AutonomyMode = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, mode);
        }
    }

    #[test]
    fn autonomy_mode_parses_canonical_names_and_aliases() {
        assert_eq!("safe".parse::<AutonomyMode>().unwrap(), AutonomyMode::Safe);
        assert_eq!(
            "local".parse::<AutonomyMode>().unwrap(),
            AutonomyMode::LocalAuto
        );
        assert_eq!(
            "worktree-auto".parse::<AutonomyMode>().unwrap(),
            AutonomyMode::WorktreeAuto
        );
        assert_eq!(
            "yolo".parse::<AutonomyMode>().unwrap(),
            AutonomyMode::AllowAll
        );
        assert!("nope".parse::<AutonomyMode>().is_err());
    }

    #[test]
    fn tool_permission_names_are_normalized() {
        let perms = ToolPermissionSet::default().allow(" Read ").deny(" BASH ");
        assert!(perms.allowed_tools.contains("read"));
        assert!(perms.denied_tools.contains("bash"));
    }

    #[test]
    fn verification_and_approval_requirements_round_trip() {
        let contract = WorkflowContract {
            required_verification: vec![VerificationRequirement::command("cargo test")],
            approval_requirements: vec![ApprovalRequirement::required(
                ApprovalAction::Network,
                "fetch issue details",
            )],
            closeout_criteria: CloseoutCriteria {
                criteria: vec!["targeted tests pass".to_owned()],
                ..CloseoutCriteria::default()
            },
            ..WorkflowContract::implicit("Implement feature")
        };

        let value = serde_json::to_value(&contract).expect("serialize");
        let decoded: WorkflowContract = serde_json::from_value(value).expect("deserialize");
        assert_eq!(decoded, contract);
    }
}
