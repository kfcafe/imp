use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::AgentMode;
use crate::policy::{RunPolicy, ToolPolicyDecision as RunToolDecision, WritePolicyDecision};
use crate::workflow::{AutonomyMode, RiskLevel, WorkflowContract, WorkflowType, WorkspaceScope};
use crate::{guardrails::GuardrailLevel, hooks::HookResult, trust::Provenance};

/// Central policy boundary for deciding whether a tool/action may proceed.
///
/// This initial type is a model-only facade. Later tasks route tool execution
/// through it while preserving current behavior.
#[derive(Debug, Clone, Default)]
pub struct ReferenceMonitor;

impl ReferenceMonitor {
    pub fn check_tool_action(
        &self,
        context: &ToolPolicyContext,
        run_policy: &RunPolicy,
    ) -> ToolPolicyDecision {
        if !context.mode.allows_tool(&context.tool_name) {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::AgentMode,
                    "agent_mode_tool_denied",
                    format!(
                        "Tool `{}` is not available in {:?} mode.",
                        context.tool_name, context.mode
                    ),
                ),
            };
        }

        if context.metadata.extension && context.metadata.secrets {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::ToolManifest,
                    "extension_secret_denied",
                    "TypeScript extension tools cannot receive secrets until explicit secret grants are implemented.",
                ),
            };
        }

        match run_policy.check_tool(&context.tool_name) {
            RunToolDecision::Allowed => {}
            RunToolDecision::Denied(message) => {
                return ToolPolicyDecision::Deny {
                    reason: PolicyReason::new(
                        PolicySource::RunPolicy,
                        "run_policy_tool_denied",
                        message,
                    ),
                };
            }
        }

        if context.metadata.workspace_write
            || matches!(
                context.action_kind,
                ToolActionKind::Write | ToolActionKind::Edit
            )
        {
            if let (Some(cwd), Some(path)) = (context.cwd.as_deref(), context.resource_scope.path())
            {
                match run_policy.check_write_path(cwd, path) {
                    WritePolicyDecision::Allowed => {}
                    WritePolicyDecision::Denied(message) => {
                        return ToolPolicyDecision::Deny {
                            reason: PolicyReason::new(
                                PolicySource::RunPolicy,
                                "run_policy_write_path_denied",
                                message,
                            ),
                        };
                    }
                }
            }
        }

        if context.metadata.extension && context.metadata.network {
            return match context.autonomy_mode {
                AutonomyMode::AllowAll => ToolPolicyDecision::Allow {
                    reasons: vec![PolicyReason::new(
                        PolicySource::WorkflowAutonomy,
                        "extension_network_allowed_allow_all",
                        "TypeScript extension network capability allowed by allow-all autonomy.",
                    )],
                },
                AutonomyMode::Suggest => self.ask_user_decision(
                    "extension_network_requires_approval",
                    "TypeScript extension network capability requires approval.",
                ),
                AutonomyMode::Safe
                | AutonomyMode::LocalAuto
                | AutonomyMode::WorktreeAuto
                | AutonomyMode::AllowAllLocal
                | AutonomyMode::Ci => ToolPolicyDecision::Deny {
                    reason: PolicyReason::new(
                        PolicySource::ToolManifest,
                        "extension_network_denied",
                        "TypeScript extension network capability is denied in this autonomy mode.",
                    ),
                },
            };
        }

        let trust_decision = self.check_trust_escalation(context);
        if !trust_decision.is_allowed() {
            return trust_decision;
        }

        let autonomy_decision = self.check_autonomy(context);
        if !autonomy_decision.is_allowed() {
            return autonomy_decision;
        }

        ToolPolicyDecision::allow()
    }

    pub fn record(
        &self,
        context: &ToolPolicyContext,
        decision: ToolPolicyDecision,
        details: Value,
    ) -> PolicyTraceRecord {
        let mut record = PolicyTraceRecord::from_context(context, decision);
        record.details = details;
        record
    }

    pub fn ask_user_record(
        &self,
        context: &ToolPolicyContext,
        message: impl Into<String>,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::AskUser {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "ask_user_required",
                    message.into(),
                ),
            },
            serde_json::json!({ "unsupported_decision": "ask_user" }),
        )
    }

    pub fn dry_run_only_record(
        &self,
        context: &ToolPolicyContext,
        message: impl Into<String>,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::DryRunOnly {
                reason: PolicyReason::new(
                    PolicySource::ToolManifest,
                    "dry_run_required",
                    message.into(),
                ),
            },
            serde_json::json!({ "unsupported_decision": "dry_run_only" }),
        )
    }

    pub fn sandbox_only_record(
        &self,
        context: &ToolPolicyContext,
        message: impl Into<String>,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::SandboxOnly {
                reason: PolicyReason::new(
                    PolicySource::ToolManifest,
                    "sandbox_required",
                    message.into(),
                ),
            },
            serde_json::json!({ "unsupported_decision": "sandbox_only" }),
        )
    }

    pub fn require_verification_record(
        &self,
        context: &ToolPolicyContext,
        message: impl Into<String>,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::RequireVerification {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "require_verification",
                    message.into(),
                ),
            },
            serde_json::json!({ "unsupported_decision": "require_verification" }),
        )
    }

    pub fn hook_blocked_record(
        &self,
        context: &ToolPolicyContext,
        hook: &HookResult,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::Hook,
                    "hook_blocked",
                    hook.reason
                        .clone()
                        .unwrap_or_else(|| "Hook blocked tool execution".into()),
                ),
            },
            serde_json::json!({ "hook": { "reason": hook.reason, "block": hook.block } }),
        )
    }

    pub fn mana_policy_record(
        &self,
        context: &ToolPolicyContext,
        decision: &crate::agent::ManaPolicyDecision,
    ) -> PolicyTraceRecord {
        let policy_decision = if decision.allowed {
            ToolPolicyDecision::Allow {
                reasons: vec![PolicyReason::new(
                    PolicySource::ManaLoop,
                    "mana_policy_allowed",
                    "Mana action allowed by active mode",
                )],
            }
        } else {
            ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::ManaLoop,
                    "mana_policy_blocked",
                    decision
                        .reason
                        .clone()
                        .unwrap_or_else(|| "Mana action blocked by active mode".into()),
                ),
            }
        };
        self.record(context, policy_decision, decision.details())
    }

    pub fn bash_equivalent_record(
        &self,
        context: &ToolPolicyContext,
        hint: &str,
    ) -> PolicyTraceRecord {
        let mut reason = PolicyReason::new(
            PolicySource::BashEquivalent,
            "policy_blocked",
            hint.to_string(),
        );
        reason.suggestion = Some("Use the native mana tool instead of shelling out to mana".into());
        self.record(
            context,
            ToolPolicyDecision::Deny { reason },
            serde_json::json!({ "bash_equivalent_hint": hint }),
        )
    }

    pub fn repeated_call_record(
        &self,
        context: &ToolPolicyContext,
        blocked: bool,
        message: &str,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            if blocked {
                ToolPolicyDecision::Deny {
                    reason: PolicyReason::new(
                        PolicySource::RepeatedCall,
                        "repeated_tool_call_blocked",
                        message,
                    ),
                }
            } else {
                ToolPolicyDecision::Allow {
                    reasons: vec![PolicyReason::new(
                        PolicySource::RepeatedCall,
                        "repeated_tool_call_warned",
                        message,
                    )],
                }
            },
            serde_json::json!({ "repeated_call": { "blocked": blocked, "message": message } }),
        )
    }

    pub fn validation_error_record(
        &self,
        context: &ToolPolicyContext,
        message: &str,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::Deny {
                reason: PolicyReason::new(PolicySource::Schema, "validation_error", message),
            },
            serde_json::json!({ "validation_error": message }),
        )
    }

    pub fn dangerous_grant_required_record(
        &self,
        context: &ToolPolicyContext,
        rail: DangerousRail,
    ) -> PolicyTraceRecord {
        self.record(
            context,
            ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::DangerousGrant,
                    rail.reason_code(),
                    rail.message(),
                ),
            },
            serde_json::json!({ "dangerous_rail": rail }),
        )
    }

    pub fn guardrail_record(
        &self,
        context: &ToolPolicyContext,
        level: GuardrailLevel,
        failed: bool,
        message: &str,
    ) -> PolicyTraceRecord {
        let decision = if failed && matches!(level, GuardrailLevel::Enforce) {
            ToolPolicyDecision::Deny {
                reason: PolicyReason::new(PolicySource::Guardrail, "guardrail_enforced", message),
            }
        } else {
            ToolPolicyDecision::Allow {
                reasons: vec![PolicyReason::new(
                    PolicySource::Guardrail,
                    if failed {
                        "guardrail_advisory_failed"
                    } else {
                        "guardrail_passed"
                    },
                    message,
                )],
            }
        };
        self.record(
            context,
            decision,
            serde_json::json!({ "guardrail": { "level": format!("{level:?}"), "failed": failed, "message": message } }),
        )
    }
    pub fn evaluate(
        &self,
        context: &ToolPolicyContext,
        run_policy: &RunPolicy,
    ) -> PolicyTraceRecord {
        let decision = self.check_tool_action(context, run_policy);
        PolicyTraceRecord::from_context(context, decision)
    }

    fn check_trust_escalation(&self, context: &ToolPolicyContext) -> ToolPolicyDecision {
        if context.supporting_provenance.is_empty() {
            return ToolPolicyDecision::allow();
        }
        if context
            .supporting_provenance
            .iter()
            .any(|provenance| !provenance.is_low_trust())
        {
            return ToolPolicyDecision::allow();
        }
        if !context.is_high_risk_action() {
            return ToolPolicyDecision::allow();
        }

        let source_summary = context
            .supporting_provenance
            .iter()
            .filter_map(|provenance| provenance.origin.as_deref())
            .collect::<Vec<_>>()
            .join(", ");
        let mut reason = PolicyReason::new(
            PolicySource::TrustLabel,
            "low_trust_escalation_denied",
            if source_summary.is_empty() {
                "Low-trust context cannot authorize this high-risk action.".to_string()
            } else {
                format!(
                    "Low-trust context cannot authorize this high-risk action. Source: {source_summary}"
                )
            },
        );
        reason.suggestion = Some(
            "Ask the user to explicitly authorize the action or provide trusted workflow policy."
                .into(),
        );
        if context.action_kind == ToolActionKind::Network {
            ToolPolicyDecision::AskUser { reason }
        } else {
            ToolPolicyDecision::Deny { reason }
        }
    }

    fn check_autonomy(&self, context: &ToolPolicyContext) -> ToolPolicyDecision {
        use AutonomyMode::*;
        match context.autonomy_mode {
            Suggest => match context.action_kind {
                ToolActionKind::Read | ToolActionKind::Search | ToolActionKind::AskUser => {
                    ToolPolicyDecision::allow()
                }
                _ => ToolPolicyDecision::Deny {
                    reason: PolicyReason::new(
                        PolicySource::WorkflowAutonomy,
                        "autonomy_suggest_side_effect_denied",
                        "Suggest mode does not execute side-effecting tools.",
                    ),
                },
            },
            Safe => ToolPolicyDecision::allow(),
            LocalAuto | WorktreeAuto => self.check_local_auto(context),
            AllowAllLocal => self.check_allow_all_local(context),
            AllowAll => self.check_allow_all(context),
            Ci => self.check_ci(context),
        }
    }

    fn check_local_auto(&self, context: &ToolPolicyContext) -> ToolPolicyDecision {
        if context.metadata.secrets || context.action_kind == ToolActionKind::Secret {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_secret_denied",
                    "Autonomy modes cannot reveal or directly access secrets.",
                ),
            };
        }
        if matches!(context.resource_scope, ResourceScope::Network { .. })
            || context.metadata.network
        {
            return self.ask_user_decision(
                "autonomy_network_requires_approval",
                "Network actions require approval in local-auto mode.",
            );
        }
        if self.is_outside_workspace(context) {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_outside_workspace_denied",
                    "Autonomous writes outside the workspace are denied.",
                ),
            };
        }
        if context.autonomy_mode == AutonomyMode::WorktreeAuto
            && !matches!(context.workspace_scope, WorkspaceScope::Worktree { .. })
        {
            return ToolPolicyDecision::SandboxOnly {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_worktree_required",
                    "worktree-auto requires an isolated worktree. Worktree execution lands in 394.9; run in an existing worktree context or choose local-auto/safe for current-workspace execution.",
                ),
            };
        }
        ToolPolicyDecision::allow()
    }

    fn check_allow_all_local(&self, context: &ToolPolicyContext) -> ToolPolicyDecision {
        if context.metadata.secrets || context.action_kind == ToolActionKind::Secret {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_secret_denied",
                    "Allow-all modes still deny secret reveal or direct secret access.",
                ),
            };
        }
        if context.metadata.network
            || matches!(context.resource_scope, ResourceScope::Network { .. })
        {
            return self.ask_user_decision(
                "autonomy_network_requires_approval",
                "Network actions require approval in allow-all-local mode.",
            );
        }
        if self.is_outside_workspace(context) {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_outside_workspace_denied",
                    "allow-all-local is scoped to the workspace/worktree.",
                ),
            };
        }
        ToolPolicyDecision::allow()
    }

    fn check_allow_all(&self, context: &ToolPolicyContext) -> ToolPolicyDecision {
        if context.metadata.secrets || context.action_kind == ToolActionKind::Secret {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_secret_denied",
                    "Allow-all still denies secret reveal or direct secret access.",
                ),
            };
        }
        if self.is_outside_workspace(context) && context.metadata.workspace_write {
            return self.ask_user_decision(
                "autonomy_outside_workspace_requires_approval",
                "Outside-workspace writes require explicit approval in allow-all mode.",
            );
        }
        ToolPolicyDecision::allow()
    }

    fn check_ci(&self, context: &ToolPolicyContext) -> ToolPolicyDecision {
        if context.metadata.secrets || context.action_kind == ToolActionKind::Secret {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_secret_denied",
                    "CI mode cannot reveal or directly access secrets.",
                ),
            };
        }
        if context.metadata.network
            || matches!(context.resource_scope, ResourceScope::Network { .. })
        {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_ci_network_denied",
                    "CI mode denies network actions unless future trusted configuration grants them.",
                ),
            };
        }
        if context.metadata.requires_approval || context.metadata.default_requires_approval {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_ci_approval_denied",
                    "CI mode fails closed when an action would require approval.",
                ),
            };
        }
        if self.is_outside_workspace(context) {
            return ToolPolicyDecision::Deny {
                reason: PolicyReason::new(
                    PolicySource::WorkflowAutonomy,
                    "autonomy_outside_workspace_denied",
                    "CI mode denies outside-workspace writes.",
                ),
            };
        }
        ToolPolicyDecision::allow()
    }

    fn ask_user_decision(&self, code: &'static str, message: &'static str) -> ToolPolicyDecision {
        ToolPolicyDecision::AskUser {
            reason: PolicyReason::new(PolicySource::WorkflowAutonomy, code, message),
        }
    }

    fn is_outside_workspace(&self, context: &ToolPolicyContext) -> bool {
        let Some(cwd) = context.cwd.as_deref() else {
            return false;
        };
        let Some(path) = context.resource_scope.path() else {
            return false;
        };
        !path.starts_with(cwd)
    }
}

/// Context supplied to the reference monitor for a single tool/action decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolPolicyContext {
    pub run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub turn: Option<u32>,
    pub tool_call_id: Option<String>,
    pub tool_name: String,
    pub action_kind: ToolActionKind,
    pub args: Value,
    pub args_hash: Option<String>,
    pub cwd: Option<PathBuf>,
    pub resource_scope: ResourceScope,
    pub mode: AgentMode,
    pub autonomy_mode: AutonomyMode,
    pub workflow_type: WorkflowType,
    pub risk_level: RiskLevel,
    pub workspace_scope: WorkspaceScope,
    pub trust_scope: TrustScopeContext,
    pub trust_labels: Vec<String>,
    pub supporting_provenance: Vec<Provenance>,
    pub metadata: ToolMetadata,
}

impl ToolPolicyContext {
    pub fn new(tool_name: impl Into<String>, action_kind: ToolActionKind) -> Self {
        let tool_name = tool_name.into();
        Self {
            metadata: ToolMetadata::new(tool_name.clone(), action_kind),
            run_id: None,
            workflow_id: None,
            turn: None,
            tool_call_id: None,
            tool_name,
            action_kind,
            args: Value::Null,
            args_hash: None,
            cwd: None,
            resource_scope: ResourceScope::default(),
            mode: AgentMode::default(),
            autonomy_mode: AutonomyMode::default(),
            workflow_type: WorkflowType::default(),
            risk_level: RiskLevel::default(),
            workspace_scope: WorkspaceScope::default(),
            trust_scope: TrustScopeContext::default(),
            trust_labels: Vec::new(),
            supporting_provenance: Vec::new(),
        }
    }
    pub fn apply_workflow_contract(&mut self, contract: &WorkflowContract) {
        self.workflow_id = contract
            .id
            .clone()
            .or_else(|| contract.mana_unit_ref.clone());
        self.autonomy_mode = contract.autonomy_mode;
        self.workflow_type = contract.workflow_type;
        self.risk_level = contract.risk_level;
        self.workspace_scope = contract.workspace_scope.clone();
        self.trust_scope = TrustScopeContext::from_contract(contract);
        self.trust_labels = self.trust_scope.labels();
    }

    pub fn with_workflow_contract(mut self, contract: &WorkflowContract) -> Self {
        self.apply_workflow_contract(contract);
        self
    }
    pub fn with_supporting_provenance(mut self, provenance: Provenance) -> Self {
        self.supporting_provenance.push(provenance);
        self
    }

    fn is_high_risk_action(&self) -> bool {
        self.metadata.workspace_write
            || self.metadata.external_side_effect
            || self.metadata.network
            || self.metadata.secrets
            || matches!(
                self.action_kind,
                ToolActionKind::Write
                    | ToolActionKind::Edit
                    | ToolActionKind::Execute
                    | ToolActionKind::Network
                    | ToolActionKind::Git
                    | ToolActionKind::Mana
                    | ToolActionKind::Secret
                    | ToolActionKind::Extension
            )
            || self.resource_scope.path().is_some_and(|path| {
                self.cwd
                    .as_deref()
                    .is_some_and(|cwd| !path.starts_with(cwd))
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrustScopeContext {
    pub allow_external_context: bool,
    pub allow_durable_memory_writes: bool,
    pub low_trust_requires_review: bool,
}

impl TrustScopeContext {
    pub fn from_contract(contract: &WorkflowContract) -> Self {
        Self {
            allow_external_context: contract.trust_scope.allow_external_context,
            allow_durable_memory_writes: contract.trust_scope.allow_durable_memory_writes,
            low_trust_requires_review: contract.trust_scope.low_trust_requires_review,
        }
    }

    pub fn labels(&self) -> Vec<String> {
        let mut labels = Vec::new();
        labels.push(
            if self.allow_external_context {
                "external-context-allowed"
            } else {
                "external-context-blocked"
            }
            .to_string(),
        );
        labels.push(
            if self.allow_durable_memory_writes {
                "durable-memory-writes-allowed"
            } else {
                "durable-memory-writes-blocked"
            }
            .to_string(),
        );
        labels.push(
            if self.low_trust_requires_review {
                "low-trust-review-required"
            } else {
                "low-trust-review-not-required"
            }
            .to_string(),
        );
        labels
    }
}

impl Default for TrustScopeContext {
    fn default() -> Self {
        Self {
            allow_external_context: true,
            allow_durable_memory_writes: true,
            low_trust_requires_review: true,
        }
    }
}

/// Coarse kind of action a tool can perform.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolActionKind {
    Read,
    Write,
    Edit,
    Execute,
    Search,
    Network,
    Git,
    Mana,
    AskUser,
    Secret,
    Extension,
    #[default]
    Unknown,
}

/// Minimal tool manifest subset needed by the reference monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub action_kind: ToolActionKind,
    pub readonly: bool,
    pub workspace_write: bool,
    pub external_side_effect: bool,
    pub network: bool,
    pub secrets: bool,
    pub extension: bool,
    pub default_requires_approval: bool,
    pub resource_scopes: Vec<ResourceScope>,
    pub supports_dry_run: bool,
    pub supports_sandbox: bool,
    pub requires_approval: bool,
    pub extension_id: Option<String>,
    pub manifest_version: Option<String>,
}

impl ToolMetadata {
    pub fn new(name: impl Into<String>, action_kind: ToolActionKind) -> Self {
        Self {
            name: name.into(),
            action_kind,
            readonly: matches!(
                action_kind,
                ToolActionKind::Read | ToolActionKind::Search | ToolActionKind::AskUser
            ),
            workspace_write: matches!(action_kind, ToolActionKind::Write | ToolActionKind::Edit),
            external_side_effect: matches!(
                action_kind,
                ToolActionKind::Execute
                    | ToolActionKind::Network
                    | ToolActionKind::Git
                    | ToolActionKind::Mana
                    | ToolActionKind::Secret
                    | ToolActionKind::Extension
            ),
            network: matches!(action_kind, ToolActionKind::Network),
            secrets: matches!(action_kind, ToolActionKind::Secret),
            extension: matches!(action_kind, ToolActionKind::Extension),
            default_requires_approval: false,
            resource_scopes: Vec::new(),
            supports_dry_run: false,
            supports_sandbox: false,
            requires_approval: false,
            extension_id: None,
            manifest_version: None,
        }
    }

    pub fn resource_scope_for_args(
        &self,
        cwd: Option<&std::path::Path>,
        args: &Value,
    ) -> ResourceScope {
        let path_arg = args
            .get("path")
            .or_else(|| args.get("file"))
            .or_else(|| args.get("directory"))
            .and_then(Value::as_str);
        if let Some(path) = path_arg {
            let path = PathBuf::from(path);
            let path = match cwd {
                Some(cwd) if path.is_relative() => cwd.join(path),
                _ => path,
            };
            return match self.action_kind {
                ToolActionKind::Search => ResourceScope::Directory { path },
                _ => ResourceScope::File { path },
            };
        }
        if self.action_kind == ToolActionKind::Execute {
            if let Some(command) = args.get("command").and_then(Value::as_str) {
                let program = command
                    .split_whitespace()
                    .next()
                    .unwrap_or(command)
                    .to_string();
                return ResourceScope::Command { program };
            }
        }
        if self.action_kind == ToolActionKind::Mana {
            return ResourceScope::Mana {
                action: args
                    .get("action")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            };
        }
        if self.action_kind == ToolActionKind::Network {
            return ResourceScope::Network {
                host: args
                    .get("url")
                    .and_then(Value::as_str)
                    .and_then(extract_host),
            };
        }
        self.resource_scopes
            .first()
            .cloned()
            .unwrap_or(ResourceScope::None)
    }

    pub fn for_tool_name(name: impl Into<String>, readonly: bool) -> Self {
        let name = name.into();
        let mut metadata = Self::new(name.clone(), ToolActionKind::from_tool_name(&name));
        metadata.readonly = readonly || metadata.readonly;
        match name.as_str() {
            "read" => {
                metadata.resource_scopes.push(ResourceScope::File {
                    path: PathBuf::new(),
                });
            }
            "write" | "edit" | "multi_edit" => {
                metadata.workspace_write = true;
                metadata.resource_scopes.push(ResourceScope::File {
                    path: PathBuf::new(),
                });
            }
            "bash" => {
                metadata.external_side_effect = true;
                metadata.resource_scopes.push(ResourceScope::Command {
                    program: String::new(),
                });
            }
            "git" => {
                metadata.external_side_effect = true;
                metadata.workspace_write = true;
            }
            "mana" => {
                metadata.external_side_effect = true;
                metadata
                    .resource_scopes
                    .push(ResourceScope::Mana { action: None });
            }
            "web" => {
                metadata.network = true;
                metadata.external_side_effect = true;
                metadata
                    .resource_scopes
                    .push(ResourceScope::Network { host: None });
            }
            "extend" => {
                metadata.workspace_write = true;
                metadata.extension = true;
                metadata.external_side_effect = true;
            }
            name if name.starts_with("lua:") || name.starts_with("extension:") => {
                metadata.extension = true;
                metadata.extension_id = Some(name.to_string());
                metadata.external_side_effect = !metadata.readonly;
            }
            _ => {}
        }
        metadata.default_requires_approval = metadata.external_side_effect && !metadata.readonly;
        metadata
    }
}

impl ToolActionKind {
    pub fn from_tool_name(name: &str) -> Self {
        match name {
            "read" => Self::Read,
            "scan" | "search" | "session_search" | "memory" => Self::Search,
            "write" => Self::Write,
            "edit" | "multi_edit" => Self::Edit,
            "bash" | "shell" => Self::Execute,
            "git" | "worktree" => Self::Git,
            "mana" => Self::Mana,
            "web" => Self::Network,
            "ask" | "ask_user" => Self::AskUser,
            "extend" => Self::Extension,
            name if name.starts_with("lua:") || name.starts_with("extension:") => Self::Extension,
            _ => Self::Unknown,
        }
    }
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self::new("unknown", ToolActionKind::Unknown)
    }
}

/// Resource touched by a tool action.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResourceScope {
    #[default]
    None,
    File {
        path: PathBuf,
    },
    Directory {
        path: PathBuf,
    },
    Command {
        program: String,
    },
    Network {
        host: Option<String>,
    },
    Mana {
        action: Option<String>,
    },
    Secret {
        name: Option<String>,
    },
    Extension {
        id: String,
    },
}

impl ResourceScope {
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            ResourceScope::File { path } | ResourceScope::Directory { path } => {
                Some(path.as_path())
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DangerousRail {
    SecretExfiltration,
    PrivateKeyRead,
    OutsideWorkspaceDestructiveWrite,
    ForcePush,
    GlobalGitConfigMutation,
    ProductionDeploy,
    CloudResourceDeletion,
    AuditLogDisable,
}

impl DangerousRail {
    pub fn reason_code(self) -> &'static str {
        match self {
            Self::SecretExfiltration => "dangerous_secret_exfiltration",
            Self::PrivateKeyRead => "dangerous_private_key_read",
            Self::OutsideWorkspaceDestructiveWrite => {
                "dangerous_outside_workspace_destructive_write"
            }
            Self::ForcePush => "dangerous_force_push",
            Self::GlobalGitConfigMutation => "dangerous_global_git_config_mutation",
            Self::ProductionDeploy => "dangerous_production_deploy",
            Self::CloudResourceDeletion => "dangerous_cloud_resource_deletion",
            Self::AuditLogDisable => "dangerous_audit_log_disable",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::SecretExfiltration => "Secret exfiltration requires an explicit dangerous grant.",
            Self::PrivateKeyRead => "Reading private keys requires an explicit dangerous grant.",
            Self::OutsideWorkspaceDestructiveWrite => {
                "Destructive writes outside the workspace require an explicit dangerous grant."
            }
            Self::ForcePush => "Force-push requires an explicit dangerous grant.",
            Self::GlobalGitConfigMutation => {
                "Global git config mutation requires an explicit dangerous grant."
            }
            Self::ProductionDeploy => "Production deploys require an explicit dangerous grant.",
            Self::CloudResourceDeletion => {
                "Cloud resource deletion requires an explicit dangerous grant."
            }
            Self::AuditLogDisable => "Disabling audit logs requires an explicit dangerous grant.",
        }
    }
}

/// Stable source/reason metadata for a policy decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyReason {
    pub source: PolicySource,
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl PolicyReason {
    pub fn new(source: PolicySource, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source,
            code: code.into(),
            message: message.into(),
            suggestion: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySource {
    AgentMode,
    RunPolicy,
    ManaLoop,
    BashEquivalent,
    RepeatedCall,
    Hook,
    Schema,
    Guardrail,
    WorkflowAutonomy,
    TrustLabel,
    ToolManifest,
    DangerousGrant,
    Unknown,
}

/// Decision returned by the monitor for a tool/action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum ToolPolicyDecision {
    Allow { reasons: Vec<PolicyReason> },
    Deny { reason: PolicyReason },
    AskUser { reason: PolicyReason },
    DryRunOnly { reason: PolicyReason },
    SandboxOnly { reason: PolicyReason },
    RequireVerification { reason: PolicyReason },
}

impl ToolPolicyDecision {
    pub fn allow() -> Self {
        Self::Allow {
            reasons: Vec::new(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

impl Default for ToolPolicyDecision {
    fn default() -> Self {
        Self::allow()
    }
}

/// Serializable policy record suitable for trace/evidence pipelines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyTraceRecord {
    pub run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub turn: Option<u32>,
    pub tool_call_id: Option<String>,
    pub tool_name: String,
    pub action_kind: ToolActionKind,
    pub decision: ToolPolicyDecision,
    pub args_hash: Option<String>,
    pub resource_scope: ResourceScope,
    pub autonomy_mode: AutonomyMode,
    pub workflow_type: WorkflowType,
    pub risk_level: RiskLevel,
    pub trust_scope: TrustScopeContext,
    pub trust_labels: Vec<String>,
    pub details: Value,
}

impl PolicyTraceRecord {
    pub fn from_context(context: &ToolPolicyContext, decision: ToolPolicyDecision) -> Self {
        Self {
            run_id: context.run_id.clone(),
            workflow_id: context.workflow_id.clone(),
            turn: context.turn,
            tool_call_id: context.tool_call_id.clone(),
            tool_name: context.tool_name.clone(),
            action_kind: context.action_kind,
            decision,
            args_hash: context.args_hash.clone(),
            resource_scope: context.resource_scope.clone(),
            autonomy_mode: context.autonomy_mode,
            workflow_type: context.workflow_type,
            risk_level: context.risk_level,
            trust_scope: context.trust_scope.clone(),
            trust_labels: context.trust_labels.clone(),
            details: Value::Null,
        }
    }
    pub fn to_trace_event(&self, run_id: impl Into<String>) -> crate::trace::TraceEvent {
        let mut event = crate::trace::TraceEvent::new(
            run_id,
            "policy.checked",
            serde_json::json!({
                "tool_name": self.tool_name,
                "action_kind": self.action_kind,
                "decision": self.decision,
                "resource_scope": self.resource_scope_summary(),
                "args_hash": self.args_hash,
                "autonomy_mode": self.autonomy_mode,
                "workflow_type": self.workflow_type,
                "risk_level": self.risk_level,
                "trust_scope": self.trust_scope,
                "trust_labels": self.trust_labels,
                "details": self.details,
            }),
        );
        event.workflow_id = self.workflow_id.clone();
        event.turn = self.turn;
        if let Some(tool_call_id) = &self.tool_call_id {
            event = event.with_tool_call_id(tool_call_id.clone());
        }
        event.redaction.contains_redactions = true;
        if self.args_hash.is_some() {
            event.redaction.content_hash = self.args_hash.clone();
        }
        event
    }

    fn resource_scope_summary(&self) -> Value {
        match &self.resource_scope {
            ResourceScope::None => Value::Null,
            ResourceScope::File { path } => {
                serde_json::json!({ "kind": "file", "path": path.display().to_string() })
            }
            ResourceScope::Directory { path } => {
                serde_json::json!({ "kind": "directory", "path": path.display().to_string() })
            }
            ResourceScope::Command { program } => {
                serde_json::json!({ "kind": "command", "program": program })
            }
            ResourceScope::Network { host } => {
                serde_json::json!({ "kind": "network", "host": host })
            }
            ResourceScope::Mana { action } => {
                serde_json::json!({ "kind": "mana", "action": action })
            }
            ResourceScope::Secret { name } => serde_json::json!({ "kind": "secret", "name": name }),
            ResourceScope::Extension { id } => serde_json::json!({ "kind": "extension", "id": id }),
        }
    }
}

fn extract_host(url: &str) -> Option<String> {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .filter(|host| !host.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod reference_monitor_types_tests {
    use super::*;

    #[test]
    fn tool_metadata_classifies_native_tools() {
        let read = ToolMetadata::for_tool_name("read", true);
        assert_eq!(read.action_kind, ToolActionKind::Read);
        assert!(read.readonly);
        assert!(!read.workspace_write);

        let write = ToolMetadata::for_tool_name("write", false);
        assert_eq!(write.action_kind, ToolActionKind::Write);
        assert!(write.workspace_write);
        assert!(!write.readonly);

        let edit = ToolMetadata::for_tool_name("edit", false);
        assert_eq!(edit.action_kind, ToolActionKind::Edit);
        assert!(edit.workspace_write);

        let bash = ToolMetadata::for_tool_name("bash", false);
        assert_eq!(bash.action_kind, ToolActionKind::Execute);
        assert!(bash.external_side_effect);
        assert!(bash.default_requires_approval);

        let git = ToolMetadata::for_tool_name("git", false);
        assert_eq!(git.action_kind, ToolActionKind::Git);
        assert!(git.external_side_effect);
        assert!(git.workspace_write);

        let mana = ToolMetadata::for_tool_name("mana", false);
        assert_eq!(mana.action_kind, ToolActionKind::Mana);
        assert!(mana.external_side_effect);

        let web = ToolMetadata::for_tool_name("web", true);
        assert_eq!(web.action_kind, ToolActionKind::Network);
        assert!(web.network);
    }

    #[test]
    fn tool_metadata_classifies_extension_placeholder() {
        let metadata = ToolMetadata::for_tool_name("lua:deploy", false);
        assert_eq!(metadata.action_kind, ToolActionKind::Extension);
        assert!(metadata.extension);
        assert_eq!(metadata.extension_id.as_deref(), Some("lua:deploy"));
        assert!(metadata.external_side_effect);
    }

    #[test]
    fn tool_metadata_extracts_resource_scope_from_args() {
        let read = ToolMetadata::for_tool_name("read", true);
        let scope = read.resource_scope_for_args(
            Some(std::path::Path::new("/repo")),
            &serde_json::json!({ "path": "src/lib.rs" }),
        );
        assert_eq!(
            scope,
            ResourceScope::File {
                path: std::path::PathBuf::from("/repo/src/lib.rs")
            }
        );

        let bash = ToolMetadata::for_tool_name("bash", false);
        assert_eq!(
            bash.resource_scope_for_args(None, &serde_json::json!({ "command": "cargo test" })),
            ResourceScope::Command {
                program: "cargo".into()
            }
        );

        let mana = ToolMetadata::for_tool_name("mana", false);
        assert_eq!(
            mana.resource_scope_for_args(None, &serde_json::json!({ "action": "close" })),
            ResourceScope::Mana {
                action: Some("close".into())
            }
        );
    }

    #[test]
    fn extension_secret_capability_is_denied_before_autonomy() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("secret_ext", ToolActionKind::Extension);
        context.metadata.extension = true;
        context.metadata.secrets = true;
        context.autonomy_mode = AutonomyMode::AllowAll;

        let decision = monitor.check_tool_action(&context, &RunPolicy::default());
        assert!(matches!(
            decision,
            ToolPolicyDecision::Deny { reason } if reason.code == "extension_secret_denied"
        ));
    }

    #[test]
    fn extension_network_capability_requires_policy_grant() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("net_ext", ToolActionKind::Extension);
        context.metadata.extension = true;
        context.metadata.network = true;
        context.autonomy_mode = AutonomyMode::Safe;

        let decision = monitor.check_tool_action(&context, &RunPolicy::default());
        assert!(matches!(
            decision,
            ToolPolicyDecision::Deny { reason } if reason.code == "extension_network_denied"
        ));

        context.autonomy_mode = AutonomyMode::AllowAll;
        let decision = monitor.check_tool_action(&context, &RunPolicy::default());
        assert!(matches!(decision, ToolPolicyDecision::Allow { .. }));
    }

    #[test]
    fn safe_mode_allows_extension_readonly_capability() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("readonly_ext", ToolActionKind::Read);
        context.metadata.extension = true;
        context.metadata.readonly = true;
        context.metadata.external_side_effect = false;
        context.metadata.workspace_write = false;
        context.autonomy_mode = AutonomyMode::Safe;

        let decision = monitor.check_tool_action(&context, &RunPolicy::default());
        assert!(matches!(decision, ToolPolicyDecision::Allow { .. }));
    }

    #[test]
    fn reference_monitor_matches_run_policy_tool_allow_and_deny() {
        let monitor = ReferenceMonitor;
        let allowed_policy = RunPolicy::new().allow_tool("read");
        let denied_policy = RunPolicy::new().deny_tool("bash");

        let read = ToolPolicyContext::new("read", ToolActionKind::Read);
        assert!(monitor
            .check_tool_action(&read, &allowed_policy)
            .is_allowed());

        let bash = ToolPolicyContext::new("bash", ToolActionKind::Execute);
        let run_policy_decision = denied_policy.check_tool("bash");
        let monitor_decision = monitor.check_tool_action(&bash, &denied_policy);
        match (run_policy_decision, monitor_decision) {
            (RunToolDecision::Denied(expected), ToolPolicyDecision::Deny { reason }) => {
                assert_eq!(reason.source, PolicySource::RunPolicy);
                assert_eq!(reason.code, "run_policy_tool_denied");
                assert_eq!(reason.message, expected);
            }
            other => panic!("unexpected decisions: {other:?}"),
        }
    }

    #[test]
    fn reference_monitor_applies_agent_mode_before_run_policy() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("write", ToolActionKind::Write);
        context.mode = AgentMode::Reviewer;
        let decision = monitor.check_tool_action(&context, &RunPolicy::new().allow_tool("write"));
        match decision {
            ToolPolicyDecision::Deny { ref reason } => {
                assert_eq!(reason.source, PolicySource::AgentMode);
                assert_eq!(reason.code, "agent_mode_tool_denied");
            }
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn reference_monitor_matches_run_policy_write_path() {
        let monitor = ReferenceMonitor;
        let policy = RunPolicy::new().allow_tool("write").allow_write("src/**");
        let cwd = std::path::PathBuf::from("/repo");
        let mut context = ToolPolicyContext::new("write", ToolActionKind::Write);
        context.cwd = Some(cwd.clone());
        context.metadata = ToolMetadata::for_tool_name("write", false);
        context.resource_scope = ResourceScope::File {
            path: std::path::PathBuf::from("/repo/README.md"),
        };

        let write_policy_decision =
            policy.check_write_path(&cwd, std::path::Path::new("/repo/README.md"));
        let monitor_decision = monitor.check_tool_action(&context, &policy);
        match (write_policy_decision, monitor_decision) {
            (WritePolicyDecision::Denied(expected), ToolPolicyDecision::Deny { reason }) => {
                assert_eq!(reason.source, PolicySource::RunPolicy);
                assert_eq!(reason.code, "run_policy_write_path_denied");
                assert_eq!(reason.message, expected);
            }
            other => panic!("unexpected decisions: {other:?}"),
        }

        context.resource_scope = ResourceScope::File {
            path: std::path::PathBuf::from("/repo/src/lib.rs"),
        };
        assert!(monitor.check_tool_action(&context, &policy).is_allowed());
    }

    #[test]
    fn reference_monitor_evaluate_returns_trace_record() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("bash", ToolActionKind::Execute);
        context.run_id = Some("run_1".into());
        let record = monitor.evaluate(&context, &RunPolicy::new().deny_tool("bash"));
        assert_eq!(record.run_id.as_deref(), Some("run_1"));
        assert_eq!(record.tool_name, "bash");
        assert!(matches!(record.decision, ToolPolicyDecision::Deny { .. }));
    }

    #[test]
    fn policy_trace_records_cover_scattered_policy_outcomes() {
        let monitor = ReferenceMonitor;
        let context = ToolPolicyContext::new("bash", ToolActionKind::Execute);

        let hook = crate::hooks::HookResult {
            block: true,
            reason: Some("blocked by hook".into()),
            modified_content: None,
        };
        assert_policy_record(
            monitor.hook_blocked_record(&context, &hook),
            PolicySource::Hook,
            "hook_blocked",
        );

        assert_policy_record(
            monitor.bash_equivalent_record(&context, "use mana tool"),
            PolicySource::BashEquivalent,
            "policy_blocked",
        );

        assert_policy_record(
            monitor.repeated_call_record(&context, true, "loop detected"),
            PolicySource::RepeatedCall,
            "repeated_tool_call_blocked",
        );

        assert_policy_record(
            monitor.repeated_call_record(&context, false, "possible loop"),
            PolicySource::RepeatedCall,
            "repeated_tool_call_warned",
        );

        assert_policy_record(
            monitor.validation_error_record(&context, "bad args"),
            PolicySource::Schema,
            "validation_error",
        );

        assert_policy_record(
            monitor.guardrail_record(
                &context,
                crate::guardrails::GuardrailLevel::Enforce,
                true,
                "guardrail failed",
            ),
            PolicySource::Guardrail,
            "guardrail_enforced",
        );
    }

    #[test]
    fn policy_trace_records_cover_mana_policy_outcomes() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("mana", ToolActionKind::Mana);
        context.mode = AgentMode::Reviewer;
        let decision = crate::agent::evaluate_mana_policy(
            context.mode,
            &serde_json::json!({ "action": "close" }),
        );
        let record = monitor.mana_policy_record(&context, &decision);
        assert_policy_record(record, PolicySource::ManaLoop, "mana_policy_blocked");
    }

    fn assert_policy_record(record: PolicyTraceRecord, source: PolicySource, code: &str) {
        match record.decision {
            ToolPolicyDecision::Allow { reasons } => {
                assert!(reasons
                    .iter()
                    .any(|reason| reason.source == source && reason.code == code));
            }
            ToolPolicyDecision::Deny { reason }
            | ToolPolicyDecision::AskUser { reason }
            | ToolPolicyDecision::DryRunOnly { reason }
            | ToolPolicyDecision::SandboxOnly { reason }
            | ToolPolicyDecision::RequireVerification { reason } => {
                assert_eq!(reason.source, source);
                assert_eq!(reason.code, code);
            }
        }
    }

    #[test]
    fn reference_monitor_context_defaults_preserve_absent_contract_behavior() {
        let context = ToolPolicyContext::new("read", ToolActionKind::Read);
        assert_eq!(context.autonomy_mode, AutonomyMode::Safe);
        assert_eq!(context.workflow_type, WorkflowType::AdHoc);
        assert_eq!(context.risk_level, RiskLevel::Unknown);
        assert_eq!(context.workspace_scope, WorkspaceScope::CurrentDirectory);
        assert_eq!(context.trust_scope, TrustScopeContext::default());
        assert_eq!(
            context.trust_labels,
            Vec::<String>::new(),
            "labels are only populated when a workflow contract is explicitly threaded"
        );
        assert!(ReferenceMonitor
            .check_tool_action(&context, &RunPolicy::new())
            .is_allowed());
    }

    #[test]
    fn reference_monitor_context_accepts_allow_all_placeholder_without_enforcing_it() {
        let mut contract = WorkflowContract::implicit("autonomous local work")
            .with_autonomy_mode(AutonomyMode::AllowAll);
        contract.id = Some("wf-allow-all".into());
        contract.workflow_type = WorkflowType::CodeChange;
        contract.risk_level = RiskLevel::High;
        contract.trust_scope.allow_external_context = false;
        contract.trust_scope.allow_durable_memory_writes = false;

        let context = ToolPolicyContext::new("bash", ToolActionKind::Execute)
            .with_workflow_contract(&contract);
        assert_eq!(context.workflow_id.as_deref(), Some("wf-allow-all"));
        assert_eq!(context.autonomy_mode, AutonomyMode::AllowAll);
        assert_eq!(context.workflow_type, WorkflowType::CodeChange);
        assert_eq!(context.risk_level, RiskLevel::High);
        assert_eq!(context.workspace_scope, contract.workspace_scope);
        assert!(!context.trust_scope.allow_external_context);
        assert!(context
            .trust_labels
            .contains(&"external-context-blocked".to_string()));
        assert!(
            ReferenceMonitor
                .check_tool_action(&context, &RunPolicy::new())
                .is_allowed(),
            "allow-all is passed through as context only in 394.5.8"
        );
    }

    #[test]
    fn policy_trace_record_includes_trust_scope_and_labels() {
        let mut contract = WorkflowContract::implicit("trusted review")
            .with_autonomy_mode(AutonomyMode::LocalAuto);
        contract.trust_scope.low_trust_requires_review = false;
        let context =
            ToolPolicyContext::new("read", ToolActionKind::Read).with_workflow_contract(&contract);
        let record = PolicyTraceRecord::from_context(&context, ToolPolicyDecision::allow());
        assert_eq!(record.autonomy_mode, AutonomyMode::LocalAuto);
        assert!(!record.trust_scope.low_trust_requires_review);
        assert!(record
            .trust_labels
            .contains(&"low-trust-review-not-required".to_string()));

        let trace = record.to_trace_event("run_1");
        assert_eq!(trace.kind, "policy.checked");
        assert_eq!(
            trace.payload["trust_scope"]["low_trust_requires_review"],
            false
        );
        assert!(trace.payload["trust_labels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| label == "low-trust-review-not-required"));
    }

    #[test]
    fn non_allow_decisions_are_serializable_policy_records() {
        let monitor = ReferenceMonitor;
        let context = ToolPolicyContext::new("bash", ToolActionKind::Execute);

        let cases = [
            (
                monitor.ask_user_record(&context, "needs approval"),
                "ask_user",
                "ask_user_required",
                "unsupported_decision",
            ),
            (
                monitor.dry_run_only_record(&context, "dry run first"),
                "dry_run_only",
                "dry_run_required",
                "unsupported_decision",
            ),
            (
                monitor.sandbox_only_record(&context, "sandbox first"),
                "sandbox_only",
                "sandbox_required",
                "unsupported_decision",
            ),
            (
                monitor.require_verification_record(&context, "verify after"),
                "require_verification",
                "require_verification",
                "unsupported_decision",
            ),
        ];

        for (record, decision_name, reason_code, detail_key) in cases {
            let json = serde_json::to_value(&record).unwrap();
            assert_eq!(json["decision"]["decision"], decision_name);
            assert_eq!(json["decision"]["reason"]["code"], reason_code);
            assert!(json["details"].get(detail_key).is_some());
            let trace = record.to_trace_event("run_1");
            assert_eq!(trace.kind, "policy.checked");
            assert_eq!(trace.payload["decision"]["decision"], decision_name);
        }
    }

    #[test]
    fn dangerous_grant_records_fail_closed_above_allow_all() {
        let monitor = ReferenceMonitor;
        let context = ToolPolicyContext::new("bash", ToolActionKind::Execute);
        let rails = [
            (
                DangerousRail::SecretExfiltration,
                "dangerous_secret_exfiltration",
            ),
            (DangerousRail::PrivateKeyRead, "dangerous_private_key_read"),
            (
                DangerousRail::OutsideWorkspaceDestructiveWrite,
                "dangerous_outside_workspace_destructive_write",
            ),
            (DangerousRail::ForcePush, "dangerous_force_push"),
            (
                DangerousRail::GlobalGitConfigMutation,
                "dangerous_global_git_config_mutation",
            ),
            (
                DangerousRail::ProductionDeploy,
                "dangerous_production_deploy",
            ),
            (
                DangerousRail::CloudResourceDeletion,
                "dangerous_cloud_resource_deletion",
            ),
            (
                DangerousRail::AuditLogDisable,
                "dangerous_audit_log_disable",
            ),
        ];

        for (rail, code) in rails {
            let record = monitor.dangerous_grant_required_record(&context, rail);
            match record.decision {
                ToolPolicyDecision::Deny { ref reason } => {
                    assert_eq!(reason.source, PolicySource::DangerousGrant);
                    assert_eq!(reason.code, code);
                    assert!(reason.message.contains("dangerous grant"));
                }
                other => panic!("dangerous rail must deny, got {other:?}"),
            }
            let json = serde_json::to_value(&record).unwrap();
            assert_eq!(
                json["details"]["dangerous_rail"],
                serde_json::to_value(rail).unwrap()
            );
        }
    }

    #[test]
    fn autonomy_reference_monitor_maps_representative_tool_classes() {
        let monitor = ReferenceMonitor;
        let policy = RunPolicy::new();

        let read = test_context(AutonomyMode::Suggest, "read", ToolActionKind::Read);
        assert!(monitor.check_tool_action(&read, &policy).is_allowed());

        let write = test_context(AutonomyMode::Suggest, "write", ToolActionKind::Write);
        assert_reason_code(
            monitor.check_tool_action(&write, &policy),
            "autonomy_suggest_side_effect_denied",
        );

        let local_write = test_context(AutonomyMode::LocalAuto, "write", ToolActionKind::Write);
        assert!(monitor
            .check_tool_action(&local_write, &policy)
            .is_allowed());

        let mut local_network =
            test_context(AutonomyMode::LocalAuto, "web", ToolActionKind::Network);
        local_network.metadata.network = true;
        local_network.resource_scope = ResourceScope::Network {
            host: Some("example.com".into()),
        };
        assert_reason_code(
            monitor.check_tool_action(&local_network, &policy),
            "autonomy_network_requires_approval",
        );

        let mut secret = test_context(AutonomyMode::AllowAll, "secret", ToolActionKind::Secret);
        secret.metadata.secrets = true;
        assert_reason_code(
            monitor.check_tool_action(&secret, &policy),
            "autonomy_secret_denied",
        );

        let mut ci_bash = test_context(AutonomyMode::Ci, "bash", ToolActionKind::Execute);
        ci_bash.metadata.default_requires_approval = true;
        assert_reason_code(
            monitor.check_tool_action(&ci_bash, &policy),
            "autonomy_ci_approval_denied",
        );
    }

    #[test]
    fn autonomy_reference_monitor_handles_outside_workspace_and_worktree_placeholder() {
        let monitor = ReferenceMonitor;
        let policy = RunPolicy::new();

        let mut outside = test_context(AutonomyMode::AllowAllLocal, "write", ToolActionKind::Write);
        outside.cwd = Some(std::path::PathBuf::from("/repo"));
        outside.resource_scope = ResourceScope::File {
            path: std::path::PathBuf::from("/tmp/file"),
        };
        assert_reason_code(
            monitor.check_tool_action(&outside, &policy),
            "autonomy_outside_workspace_denied",
        );

        let worktree = test_context(AutonomyMode::WorktreeAuto, "write", ToolActionKind::Write);
        let decision = monitor.check_tool_action(&worktree, &policy);
        assert_reason_code(decision.clone(), "autonomy_worktree_required");
        let message = policy_decision_reason(&decision).unwrap().message;
        assert!(message.contains("394.9"));
        assert!(message.contains("local-auto"));

        let read_worktree = test_context(AutonomyMode::WorktreeAuto, "read", ToolActionKind::Read);
        assert_reason_code(
            monitor.check_tool_action(&read_worktree, &policy),
            "autonomy_worktree_required",
        );

        let mut isolated = worktree.clone();
        isolated.workspace_scope = WorkspaceScope::Worktree {
            path: std::path::PathBuf::from("/repo-worktree"),
            branch: Some("workflow".into()),
        };
        assert!(monitor.check_tool_action(&isolated, &policy).is_allowed());
    }

    #[test]
    fn autonomy_safe_preserves_existing_run_policy_precedence() {
        let monitor = ReferenceMonitor;
        let mut safe = test_context(AutonomyMode::Safe, "bash", ToolActionKind::Execute);
        safe.metadata.default_requires_approval = true;
        assert!(monitor
            .check_tool_action(&safe, &RunPolicy::new())
            .is_allowed());

        assert_reason_code(
            monitor.check_tool_action(&safe, &RunPolicy::new().deny_tool("bash")),
            "run_policy_tool_denied",
        );
    }

    fn test_context(mode: AutonomyMode, name: &str, kind: ToolActionKind) -> ToolPolicyContext {
        let mut context = ToolPolicyContext::new(name, kind);
        context.autonomy_mode = mode;
        context.metadata = ToolMetadata::for_tool_name(
            name,
            matches!(kind, ToolActionKind::Read | ToolActionKind::Search),
        );
        context.cwd = Some(std::path::PathBuf::from("/repo"));
        if context.metadata.workspace_write
            || matches!(kind, ToolActionKind::Write | ToolActionKind::Edit)
        {
            context.resource_scope = ResourceScope::File {
                path: std::path::PathBuf::from("/repo/src/lib.rs"),
            };
        }
        context
    }

    fn policy_decision_reason(decision: &ToolPolicyDecision) -> Option<PolicyReason> {
        match decision {
            ToolPolicyDecision::Allow { .. } => None,
            ToolPolicyDecision::Deny { reason }
            | ToolPolicyDecision::AskUser { reason }
            | ToolPolicyDecision::DryRunOnly { reason }
            | ToolPolicyDecision::SandboxOnly { reason }
            | ToolPolicyDecision::RequireVerification { reason } => Some(reason.clone()),
        }
    }

    fn assert_reason_code(decision: ToolPolicyDecision, code: &str) {
        match decision {
            ToolPolicyDecision::Deny { reason }
            | ToolPolicyDecision::AskUser { reason }
            | ToolPolicyDecision::DryRunOnly { reason }
            | ToolPolicyDecision::SandboxOnly { reason }
            | ToolPolicyDecision::RequireVerification { reason } => assert_eq!(reason.code, code),
            other => panic!("expected non-allow decision {code}, got {other:?}"),
        }
    }

    #[test]
    fn reference_monitor_denies_low_trust_high_risk_escalation() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("bash", ToolActionKind::Execute)
            .with_supporting_provenance(
                Provenance::external_web("https://example.com/instructions")
                    .with_risk(crate::trust::RiskLabel::ContainsInstructions),
            );
        context.metadata = ToolMetadata::for_tool_name("bash", false);

        let decision = monitor.check_tool_action(&context, &RunPolicy::new());
        match decision {
            ToolPolicyDecision::Deny { reason } => {
                assert_eq!(reason.source, PolicySource::TrustLabel);
                assert_eq!(reason.code, "low_trust_escalation_denied");
                assert!(reason.message.contains("https://example.com/instructions"));
            }
            other => panic!("expected trust denial, got {other:?}"),
        }
    }

    #[test]
    fn reference_monitor_asks_user_for_low_trust_network_escalation() {
        let monitor = ReferenceMonitor;
        let mut context = ToolPolicyContext::new("web", ToolActionKind::Network)
            .with_supporting_provenance(Provenance::external_web("https://example.com"));
        context.metadata = ToolMetadata::for_tool_name("web", false);
        context.resource_scope = ResourceScope::Network {
            host: Some("api.example.com".into()),
        };

        let decision = monitor.check_tool_action(&context, &RunPolicy::new());
        match decision {
            ToolPolicyDecision::AskUser { reason } => {
                assert_eq!(reason.source, PolicySource::TrustLabel);
                assert_eq!(reason.code, "low_trust_escalation_denied");
            }
            other => panic!("expected trust approval request, got {other:?}"),
        }
    }

    #[test]
    fn reference_monitor_allows_trusted_support_and_low_risk_low_trust_context() {
        let monitor = ReferenceMonitor;
        let trusted = ToolPolicyContext::new("bash", ToolActionKind::Execute)
            .with_supporting_provenance(Provenance::user_instruction());
        assert!(monitor
            .check_tool_action(&trusted, &RunPolicy::new())
            .is_allowed());

        let low_risk = ToolPolicyContext::new("read", ToolActionKind::Read)
            .with_supporting_provenance(Provenance::external_web("https://example.com"));
        assert!(monitor
            .check_tool_action(&low_risk, &RunPolicy::new())
            .is_allowed());
    }

    #[test]
    fn reference_monitor_types_default_to_safe_unknown_context() {
        let context = ToolPolicyContext::new("mystery", ToolActionKind::Unknown);
        assert_eq!(context.tool_name, "mystery");
        assert_eq!(context.mode, AgentMode::Full);
        assert_eq!(context.autonomy_mode, AutonomyMode::default());
        assert_eq!(context.resource_scope, ResourceScope::None);
        assert_eq!(
            context.metadata,
            ToolMetadata::new("mystery", ToolActionKind::Unknown)
        );
        assert!(ToolPolicyDecision::default().is_allowed());
    }

    #[test]
    fn reference_monitor_types_serialize_decision_variants() {
        let reason = PolicyReason::new(PolicySource::RunPolicy, "deny_tool", "tool denied");
        let decision = ToolPolicyDecision::Deny { reason };
        let json = serde_json::to_value(&decision).unwrap();
        assert_eq!(json["decision"], "deny");
        assert_eq!(json["reason"]["source"], "run_policy");
        assert_eq!(json["reason"]["code"], "deny_tool");
    }

    #[test]
    fn reference_monitor_types_trace_record_carries_context() {
        let mut context = ToolPolicyContext::new("bash", ToolActionKind::Execute);
        context.run_id = Some("run_1".into());
        context.workflow_id = Some("394.5".into());
        context.turn = Some(2);
        context.tool_call_id = Some("call_1".into());
        context.resource_scope = ResourceScope::Command {
            program: "cargo".into(),
        };

        let record = PolicyTraceRecord::from_context(&context, ToolPolicyDecision::allow());
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["run_id"], "run_1");
        assert_eq!(json["workflow_id"], "394.5");
        assert_eq!(json["tool_name"], "bash");
        assert_eq!(json["action_kind"], "execute");
        assert_eq!(json["decision"]["decision"], "allow");
    }
}
