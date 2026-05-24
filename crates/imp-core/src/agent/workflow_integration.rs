//! Host/workflow integration around the core message loop.
//!
//! This module intentionally isolates mana/work-graph controller behavior from
//! the main agent runtime definition. The core loop can still call these hooks
//! while the durable workflow graph is being moved to an external harness.

use crate::agent::{
    autonomy::{
        edited_files_verification_obligation, failed_command_recovery_obligation, ObligationKind,
    },
    mana_loop::{self, ManaActionClass},
    Agent, AgentEvent, ContentBlock, ContinueReason, ContinueRecommendation, LoopDecision, Message,
    RunFinalStatus, StopReason,
};
use crate::config::AgentMode;
use crate::eval_candidate::{redact_eval_candidate, EvalArtifactRef};
use crate::eval_candidate_closeout::{eval_candidate_for_closeout, EvalCandidateCloseoutContext};
use crate::evidence::{
    EvidenceActions, EvidenceArtifact, EvidencePacket, EvidencePolicy, EvidenceTrustSummary,
    EvidenceVerificationGate,
};
use crate::mana_review::{
    ManaMutationAction, ManaMutationRecord, ManaReviewScope, ManaReviewScopeKind, ManaReviewState,
    ManaUnitSnapshot, TurnManaReview, TurnManaReviewAccumulator,
};
use crate::storage;
use crate::trust::{Provenance, RiskLabel, TrustLabel};
use crate::workflow::{AutonomyMode, WorkflowContract, WorkflowRunController, WorktreeRunMetadata};
use imp_llm::AssistantMessage;

use super::{
    assistant_message_contains_mana_tool_call, assistant_message_text,
    bash_result_is_successful_check, tool_results_include_successful_check,
    tool_results_include_successful_edit, tool_results_indicate_failed_bash_command,
};

#[derive(Debug)]
pub(crate) struct WorkflowRuntimeLayer {
    controller: WorkflowRunController,
    contract: WorkflowContract,
    turn_mana_review: std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>>,
    has_mana_skill: bool,
    has_mana_basics_skill: bool,
    has_mana_delegation_skill: bool,
    queued_mana_externalization_nudge: bool,
}

#[derive(Debug, Default)]
pub(super) struct WorkflowPostTurnSignals {
    pub(super) execution_debt: bool,
    pub(super) execution_evidence: bool,
    pub(super) orchestration_run_id: Option<String>,
    pub(super) orchestration_started: bool,
    pub(super) durable_workflow_progress: bool,
    pub(super) stop_reason: Option<StopReason>,
}

pub(crate) fn workflow_layer_may_override_finish(decision: &LoopDecision) -> bool {
    matches!(
        decision,
        LoopDecision::Finish {
            status: crate::agent::RunFinalStatus::Done { .. }
                | crate::agent::RunFinalStatus::DoneWithConcerns { .. }
        }
    )
}

impl WorkflowRuntimeLayer {
    pub(crate) fn new(contract: WorkflowContract) -> Self {
        Self {
            controller: WorkflowRunController::new(),
            contract,
            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
                TurnManaReviewAccumulator::default(),
            )),
            has_mana_skill: false,
            has_mana_basics_skill: false,
            has_mana_delegation_skill: false,
            queued_mana_externalization_nudge: false,
        }
    }

    pub(crate) fn controller(&self) -> &WorkflowRunController {
        &self.controller
    }

    pub(crate) fn controller_mut(&mut self) -> &mut WorkflowRunController {
        &mut self.controller
    }

    pub(crate) fn contract(&self) -> &WorkflowContract {
        &self.contract
    }

    pub(crate) fn contract_mut(&mut self) -> &mut WorkflowContract {
        &mut self.contract
    }

    pub(crate) fn replace_contract(&mut self, contract: WorkflowContract) {
        self.contract = contract;
    }

    pub(crate) fn turn_mana_review(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>> {
        self.turn_mana_review.clone()
    }

    pub(crate) fn set_mana_skill_available(&mut self, available: bool) {
        self.has_mana_skill = available;
    }

    pub(crate) fn set_mana_basics_skill_available(&mut self, available: bool) {
        self.has_mana_basics_skill = available;
    }

    pub(crate) fn set_mana_delegation_skill_available(&mut self, available: bool) {
        self.has_mana_delegation_skill = available;
    }

    pub(crate) fn mark_externalization_nudge_queued(&mut self) {
        self.queued_mana_externalization_nudge = true;
    }

    pub(crate) fn replace_controller(&mut self, controller: WorkflowRunController) {
        self.controller = controller;
    }
}

fn mana_review_scope_from_result(result: &imp_llm::ToolResultMessage) -> ManaReviewScope {
    let display = result
        .details
        .get("path")
        .or_else(|| result.details.get("mana_dir"))
        .and_then(|value| value.as_str())
        .unwrap_or("auto")
        .to_string();

    ManaReviewScope {
        kind: if display == "auto" {
            ManaReviewScopeKind::None
        } else {
            ManaReviewScopeKind::ExplicitPath
        },
        display,
    }
}

fn unit_snapshot_from_value(value: &serde_json::Value) -> Option<ManaUnitSnapshot> {
    serde_json::from_value(value.clone()).ok()
}

fn unit_snapshot_from_result(result: &imp_llm::ToolResultMessage) -> Option<ManaUnitSnapshot> {
    result
        .details
        .get("unit")
        .and_then(unit_snapshot_from_value)
}

fn mana_mutation_action(action: &str) -> Option<ManaMutationAction> {
    match action {
        "create" => Some(ManaMutationAction::Create),
        "close" => Some(ManaMutationAction::Close),
        "update" => Some(ManaMutationAction::Update),
        "notes_append" => Some(ManaMutationAction::NotesAppend),
        "decision_add" => Some(ManaMutationAction::DecisionAdd),
        "decision_resolve" => Some(ManaMutationAction::DecisionResolve),
        "reopen" => Some(ManaMutationAction::Reopen),
        "fail" => Some(ManaMutationAction::Fail),
        "delete" => Some(ManaMutationAction::Delete),
        "dep_add" => Some(ManaMutationAction::DepAdd),
        "dep_remove" => Some(ManaMutationAction::DepRemove),
        "fact_create" => Some(ManaMutationAction::FactCreate),
        _ => None,
    }
}

fn mutation_record_from_mana_result(
    result: &imp_llm::ToolResultMessage,
) -> Option<ManaMutationRecord> {
    if result.is_error || result.tool_name != "mana" {
        return None;
    }

    let action_name = mana_result_action(result)?;
    let action = mana_mutation_action(action_name)?;
    let after_unit = unit_snapshot_from_result(result);
    let deleted_unit = if action == ManaMutationAction::Delete {
        let id = result.details.get("id")?.as_str()?.to_string();
        let title = result
            .details
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or(&id)
            .to_string();
        Some(crate::mana_review::ManaUnitRef::new(id, title, None))
    } else {
        None
    };

    if after_unit.is_none()
        && deleted_unit.is_none()
        && !matches!(
            action,
            ManaMutationAction::DepAdd | ManaMutationAction::DepRemove
        )
    {
        return None;
    }

    Some(ManaMutationRecord {
        action,
        scope: mana_review_scope_from_result(result),
        before_unit: None,
        after_unit,
        deleted_unit,
        parent_unit: None,
        related_unit: None,
        field_changes: Vec::new(),
        notes_appended: Vec::new(),
        decision_events: Vec::new(),
    })
}

fn mana_workflow_follow_up_text() -> &'static str {
    "An imp-work task was closed, verified, or materially advanced, but that only proves the current task changed. Inspect the active work scope with work(action=\"next\") or work(action=\"validate\"), continue any ready work, and only stop when the requested outcome is complete, blocked by a concrete decision, or no runnable work remains."
}

fn durable_workflow_action(result: &imp_llm::ToolResultMessage) -> Option<&str> {
    match result.tool_name.as_str() {
        "mana" | "work" => result
            .details
            .get("action")
            .and_then(|value| value.as_str())
            .or_else(|| {
                result
                    .details
                    .get("mana_loop_policy")
                    .and_then(|policy| policy.get("action"))
                    .and_then(|value| value.as_str())
            }),
        _ => None,
    }
}

fn durable_workflow_action_class(action: &str) -> mana_loop::ManaActionClass {
    match action {
        // Native imp-work names that do not exist in legacy mana but mutate or
        // materially advance durable work state.
        "context" | "refresh_context" | "outcome" | "prototype_outcome" | "remember" => {
            mana_loop::ManaActionClass::ProgressCheckpoint
        }
        // Native imp-work inspection/read-only actions.
        "search" | "runs" | "validate" | "scope" => mana_loop::ManaActionClass::Inspect,
        _ => mana_loop::classify_mana_action(action),
    }
}

fn durable_workflow_mutation_is_execution_debt(action: &str) -> bool {
    matches!(
        durable_workflow_action_class(action),
        mana_loop::ManaActionClass::ProgressCheckpoint
            | mana_loop::ManaActionClass::GraphMutation
            | mana_loop::ManaActionClass::DecisionFact
    )
}

fn durable_workflow_lifecycle_is_execution_evidence(action: &str) -> bool {
    matches!(
        durable_workflow_action_class(action),
        mana_loop::ManaActionClass::Lifecycle | mana_loop::ManaActionClass::Orchestration
    )
}

fn durable_workflow_result_has_closed_item(result: &imp_llm::ToolResultMessage) -> bool {
    result
        .details
        .get("item")
        .and_then(|item| item.get("status"))
        .and_then(|v| v.as_str())
        .is_some_and(|status| matches!(status, "done" | "closed"))
        || result
            .details
            .get("unit")
            .and_then(|unit| unit.get("status"))
            .and_then(|v| v.as_str())
            .is_some_and(|status| matches!(status, "done" | "closed"))
}

fn durable_workflow_verify_passed(result: &imp_llm::ToolResultMessage) -> bool {
    result.details.get("passed").and_then(|v| v.as_bool()) == Some(true)
}

fn mana_result_action(result: &imp_llm::ToolResultMessage) -> Option<&str> {
    durable_workflow_action(result)
}

fn mana_unit_id_from_result(result: &imp_llm::ToolResultMessage) -> Option<String> {
    result
        .details
        .get("id")
        .or_else(|| result.details.get("unit_id"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn mana_result_parent_id(result: &imp_llm::ToolResultMessage) -> Option<String> {
    result
        .details
        .get("parent")
        .or_else(|| result.details.get("parent_id"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

pub(super) fn mana_run_status_from_result(
    result: &imp_llm::ToolResultMessage,
) -> Option<(String, crate::workflow::WorkflowChildRunStatus)> {
    if result.is_error || result.tool_name != "mana" {
        return None;
    }
    let run_id = result
        .details
        .get("run_id")
        .and_then(|v| v.as_str())?
        .to_string();
    let status = result.details.get("status").and_then(|v| v.as_str())?;
    let total_failed = result
        .details
        .get("summary")
        .and_then(|summary| summary.get("total_failed"))
        .and_then(|v| v.as_u64());
    Some((
        run_id,
        crate::workflow::WorkflowChildRunStatus::from_mana_run_status(status, total_failed),
    ))
}

fn tool_results_indicate_execution_debt(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return false;
    }

    tool_results.iter().any(|result| {
        !result.is_error
            && matches!(result.tool_name.as_str(), "mana" | "work")
            && durable_workflow_action(result)
                .is_some_and(durable_workflow_mutation_is_execution_debt)
    })
}

fn mana_orchestration_run_id(tool_results: &[imp_llm::ToolResultMessage]) -> Option<String> {
    tool_results.iter().find_map(|result| {
        if result.is_error || !matches!(result.tool_name.as_str(), "mana" | "work") {
            return None;
        }
        let action = durable_workflow_action(result)?;
        if durable_workflow_action_class(action) != mana_loop::ManaActionClass::Orchestration {
            return None;
        }
        result
            .details
            .get("run_id")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
    })
}

fn tool_results_indicate_orchestration_started(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(mode, AgentMode::Full | AgentMode::Orchestrator) {
        return false;
    }

    tool_results.iter().any(|result| {
        !result.is_error
            && matches!(result.tool_name.as_str(), "mana" | "work")
            && durable_workflow_action(result).is_some_and(|action| {
                durable_workflow_action_class(action) == mana_loop::ManaActionClass::Orchestration
            })
    })
}

fn tool_results_indicate_execution_evidence(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return false;
    }

    tool_results.iter().any(|result| {
        if result.is_error {
            return false;
        }

        match result.tool_name.as_str() {
            "write" | "edit" | "multi_edit" | "openrouter_secret_run" => true,
            "bash" | "shell" => true,
            "mana" | "work" => durable_workflow_action(result)
                .is_some_and(durable_workflow_lifecycle_is_execution_evidence),
            _ => false,
        }
    })
}

fn tool_results_indicate_durable_workflow_progress(
    tool_results: &[imp_llm::ToolResultMessage],
    mode: AgentMode,
) -> bool {
    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Worker
    ) {
        return false;
    }

    tool_results.iter().any(|result| {
        if result.is_error || !matches!(result.tool_name.as_str(), "mana" | "work") {
            return false;
        }

        let action = durable_workflow_action(result);

        matches!(action, Some("close"))
            || matches!(action, Some("verify") if durable_workflow_verify_passed(result))
            || durable_workflow_result_has_closed_item(result)
    })
}

fn mana_review_stop_reason(mana_review: &TurnManaReview, mode: AgentMode) -> Option<StopReason> {
    match mana_review.state {
        ManaReviewState::NeedsDecision => Some(StopReason::UserBlocker),
        ManaReviewState::Changed if matches!(mode, AgentMode::Planner) => {
            if !mana_review.proposed_children.is_empty()
                || !mana_review.touched_units.is_empty()
                || !mana_review.material_field_changes.is_empty()
                || !mana_review.notes_appended.is_empty()
                || !mana_review.decision_events.is_empty()
            {
                Some(StopReason::DecompositionCompleted)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn should_queue_mana_externalization_follow_up(
    message: &AssistantMessage,
    user_prompt: &str,
    mode: AgentMode,
    _has_mana_skill: bool,
    already_queued: bool,
) -> bool {
    if already_queued {
        return false;
    }

    if !matches!(
        mode,
        AgentMode::Full | AgentMode::Planner | AgentMode::Orchestrator
    ) {
        return false;
    }

    if assistant_message_contains_mana_tool_call(message) {
        return false;
    }

    if !user_prompt_requests_durable_externalization(user_prompt) {
        return false;
    }

    let text = assistant_message_text(message);
    assistant_described_externalizable_plan(&text)
}

fn user_prompt_requests_durable_externalization(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    let explicit_phrase = [
        "create imp-work",
        "create work",
        "externalize",
        "imp-work task",
        "work task",
        "work tasks",
        "record this",
        "save this plan",
        "split this into tasks",
        "split this into units",
        "turn this into imp-work",
        "decompose this into tasks",
        "decompose this into units",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    explicit_phrase
        || ((lower.contains("split") || lower.contains("decompose"))
            && (lower.contains("task")
                || lower.contains("tasks")
                || lower.contains("unit")
                || lower.contains("units")))
}

fn assistant_described_externalizable_plan(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let has_plan_shape = [
        "acceptance",
        "dependency",
        "dependencies",
        "phase",
        "phase 1",
        "phase 2",
        "verify gate",
        "verification",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    let has_work_product = [
        "task",
        "tasks",
        "unit",
        "units",
        "work item",
        "work items",
        "split this",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    has_plan_shape && has_work_product
}

fn mana_externalization_follow_up_text() -> &'static str {
    "Before you continue: the user explicitly asked for durable work structure. Externalize the plan or decomposition you just described into native imp-work now. Create or update the relevant task(s) with native work actions, prefer global project scope, and avoid extra chat restatement when the work tool/UI already makes the delta obvious."
}

fn mana_skill_follow_up_hint(
    prompt: &str,
    mode: AgentMode,
    tools_available: bool,
    _has_mana_skill: bool,
    _has_mana_basics_skill: bool,
    _has_mana_delegation_skill: bool,
) -> Option<&'static str> {
    if !tools_available {
        return None;
    }

    let lower = prompt.to_ascii_lowercase();

    let orchestration_signal = [
        "decompose",
        "decomposition",
        "split this",
        "break this up",
        "break it up",
        "parallel",
        "parallel helper",
        "bounded helper",
        "orchestrate",
        "orchestration",
        "create a task",
        "create tasks",
        "work guide",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    let work_signal = [
        " imp-work ",
        "work next",
        "work list",
        "work show",
        "work update",
        "work create",
        "work guide",
    ]
    .iter()
    .any(|needle| lower.contains(needle));

    match mode {
        AgentMode::Full | AgentMode::Orchestrator | AgentMode::Planner
            if orchestration_signal || work_signal =>
        {
            Some("Before you continue: use native imp-work `guide` when you need extra help with task design, decomposition, retries, or worker handoff.")
        }
        AgentMode::Worker | AgentMode::Auditor if work_signal => {
            Some("Before you continue: use the native work tool and stay within this mode's allowed imp-work workflow. Use the `guide` action if you need help.")
        }
        _ => None,
    }
}

pub(crate) fn orchestration_follow_up_text(run_id: Option<&str>) -> String {
    if let Some(run_id) = run_id {
        return format!(
            "Orchestration has started as {run_id}, but the requested outcome is not complete yet. Inspect work(action=\"runs\", run_id=\"{run_id}\") or the relevant native imp-work run state/logs, continue coordinating ready work, retry or escalate failed tasks, and only stop when the workflow is verified complete, blocked by a concrete decision, or no runnable work remains."
        );
    }

    crate::workflow::workflow_supervision_prompt()
}

fn evidence_trust_summary_from_messages(messages: &[Message]) -> EvidenceTrustSummary {
    let mut summary = EvidenceTrustSummary::default();
    for message in messages {
        let Message::ToolResult(result) = message else {
            continue;
        };
        let Some(provenance) = result
            .details
            .get("provenance")
            .and_then(|value| serde_json::from_value::<Provenance>(value.clone()).ok())
        else {
            continue;
        };
        record_evidence_provenance(&mut summary, &provenance);
    }
    summary.sources.sort();
    summary.sources.dedup();
    summary.low_trust_influences.sort();
    summary.low_trust_influences.dedup();
    summary.warnings.sort();
    summary.warnings.dedup();
    summary
}

fn record_evidence_provenance(summary: &mut EvidenceTrustSummary, provenance: &Provenance) {
    summary.sources.push(format!(
        "source={:?}; trust={:?}; origin={}",
        provenance.source,
        provenance.trust,
        provenance.origin.as_deref().unwrap_or("unknown")
    ));
    if provenance.is_low_trust() {
        summary.low_trust_influences.push(format!(
            "low-trust source observed: {}",
            provenance.origin.as_deref().unwrap_or("unknown")
        ));
    }
    if provenance.risk.iter().any(|risk| {
        matches!(
            risk,
            RiskLabel::PossiblePromptInjection | RiskLabel::ContainsInstructions
        )
    }) {
        summary.warnings.push(format!(
            "instruction-like low-trust content observed from {}",
            provenance.origin.as_deref().unwrap_or("unknown")
        ));
    }
    if provenance.trust == TrustLabel::ExternalUntrusted {
        summary
            .warnings
            .push("external/untrusted content cannot authorize policy or tool escalation".into());
    }
}

fn evidence_verification_gate(
    gate: &crate::workflow::VerificationGate,
) -> EvidenceVerificationGate {
    EvidenceVerificationGate {
        name: if gate.name.is_empty() {
            gate.id.clone()
        } else {
            gate.name.clone()
        },
        required: gate.is_required(),
        status: format!("{:?}", gate.status).to_lowercase(),
        command: gate.command.as_ref().map(|command| command.command.clone()),
        exit_code: gate.result.as_ref().and_then(|result| result.exit_code),
        artifact_path: gate
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "status")
            .or_else(|| gate.artifacts.first())
            .map(|artifact| artifact.path.clone()),
    }
}

fn evidence_policy_for_autonomy(mode: AutonomyMode) -> EvidencePolicy {
    let mut policy = EvidencePolicy::default();
    policy.decisions.push(format!("autonomy mode: {mode}"));
    policy
        .decisions
        .push("policy.checked trace events record mode, scope, and decision context when policy checks run".into());
    policy
        .denials
        .push("hard-rail bypass: none recorded; dangerous grants are not implemented".into());
    match mode {
        AutonomyMode::LocalAuto | AutonomyMode::WorktreeAuto => {
            policy.decisions.push(
                "autonomous local actions remain subject to workspace, network, secret, and hard-rail policy".into(),
            );
            policy.approvals.push(
                "network, outside-workspace, destructive, and secret-sensitive actions require approval or are denied".into(),
            );
        }
        AutonomyMode::AllowAllLocal => {
            policy
                .decisions
                .push("allow-all-local remained scoped to local workspace/worktree actions".into());
            policy.decisions.push(
                "notable high-risk actions should be inspected in policy.checked trace events"
                    .into(),
            );
            policy.approvals.push(
                "network, outside-workspace, production, secret, and dangerous-grant actions were not silently allowed".into(),
            );
        }
        AutonomyMode::AllowAll => {
            policy.decisions.push(
                "allow-all mode was active; audit evidence and policy.checked trace events remain enabled".into(),
            );
            policy.decisions.push(
                "notable high-risk actions should be inspected in policy.checked trace events"
                    .into(),
            );
            policy.approvals.push(
                "secret exfiltration, dangerous grants, and unsupported outside-scope mutations were not silently allowed".into(),
            );
        }
        AutonomyMode::Ci => {
            policy.decisions.push(
                "ci mode fails closed for prompts/approvals not declared ahead of time".into(),
            );
        }
        AutonomyMode::Suggest | AutonomyMode::Safe => {}
    }
    policy
}

fn evidence_actions_from_messages(messages: &[Message]) -> EvidenceActions {
    let mut actions = EvidenceActions::default();
    for message in messages {
        let Message::Assistant(assistant) = message else {
            continue;
        };
        for block in &assistant.content {
            let ContentBlock::ToolCall {
                name, arguments, ..
            } = block
            else {
                continue;
            };
            actions.tools.push(name.clone());
            match name.as_str() {
                "read" => {
                    if let Some(path) = arguments.get("path").and_then(|value| value.as_str()) {
                        actions.files_inspected.push(path.to_string());
                    }
                }
                "write" | "edit" => {
                    if let Some(path) = arguments.get("path").and_then(|value| value.as_str()) {
                        actions.files_changed.push(path.to_string());
                    }
                }
                "bash" => {
                    if let Some(command) = arguments.get("command").and_then(|value| value.as_str())
                    {
                        actions.commands_run.push(command.to_string());
                    }
                }
                "scan" => actions.searches.push(format!("scan {arguments}")),
                _ => {}
            }
        }
    }
    actions.files_inspected.sort();
    actions.files_inspected.dedup();
    actions.files_changed.sort();
    actions.files_changed.dedup();
    actions.commands_run.sort();
    actions.commands_run.dedup();
    actions.searches.sort();
    actions.searches.dedup();
    actions.tools.sort();
    actions.tools.dedup();
    actions
}

impl Agent {
    pub fn workflow_contract(&self) -> &WorkflowContract {
        self.workflow_layer.contract()
    }

    pub fn workflow_contract_mut(&mut self) -> &mut WorkflowContract {
        self.workflow_layer.contract_mut()
    }

    pub fn set_workflow_contract(&mut self, contract: WorkflowContract) {
        self.workflow_layer.replace_contract(contract);
    }

    pub(super) fn write_workflow_contract_snapshot(&self, artifacts: &storage::RunArtifacts) {
        let _ = std::fs::write(
            artifacts.workflow_contract_path(),
            serde_json::to_string_pretty(self.workflow_contract()).unwrap_or_default(),
        );
    }

    pub(super) async fn write_closeout_eval_candidate(
        &self,
        run_id: &str,
        artifacts: &storage::RunArtifacts,
        prompt: &str,
        status: &RunFinalStatus,
    ) {
        let candidate_id = format!("{run_id}-closeout");
        let context = EvalCandidateCloseoutContext {
            candidate_id: candidate_id.clone(),
            run_id: Some(run_id.to_string()),
            workflow_id: self
                .workflow_contract()
                .id
                .clone()
                .or_else(|| self.workflow_contract().mana_unit_ref.clone()),
            session_id: None,
            prompt: Some(prompt.to_string()),
            expected_summary: Some(
                "Agent run should satisfy its workflow contract and required verification gates"
                    .into(),
            ),
            verifiers: Vec::new(),
            artifact_refs: vec![
                EvalArtifactRef {
                    kind: "trace".into(),
                    path: artifacts.trace_path(),
                    summary: Some("Structured runtime event trace".into()),
                    sha256: None,
                },
                EvalArtifactRef {
                    kind: "evidence".into(),
                    path: artifacts.evidence_path(),
                    summary: Some("Run evidence packet".into()),
                    sha256: None,
                },
                EvalArtifactRef {
                    kind: "workflow-contract".into(),
                    path: artifacts.workflow_contract_path(),
                    summary: Some("Workflow contract snapshot".into()),
                    sha256: None,
                },
            ],
        };
        let Some(candidate) =
            eval_candidate_for_closeout(status, &self.verification_gates, context)
        else {
            return;
        };
        let path = artifacts.eval_candidate_path(&candidate_id);
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        if let Ok(json) = serde_json::to_string_pretty(&redact_eval_candidate(candidate)) {
            let _ = std::fs::write(path, json);
        }
    }

    pub(super) async fn write_run_evidence(
        &self,
        run_id: &str,
        artifacts: &storage::RunArtifacts,
        worktree_metadata: Option<&WorktreeRunMetadata>,
        prompt: &str,
        status: &RunFinalStatus,
    ) {
        let mut packet = EvidencePacket::new(run_id, prompt);
        packet.workflow_id = self
            .workflow_contract()
            .id
            .clone()
            .or_else(|| self.workflow_contract().mana_unit_ref.clone());
        packet.workflow_type = Some(format!("{:?}", self.workflow_contract().workflow_type));
        packet.risk_level = Some(format!("{:?}", self.workflow_contract().risk_level));
        packet.autonomy_mode = Some(self.workflow_contract().autonomy_mode.to_string());
        packet.final_status = Some(format!("{:?}", status));
        packet.policy = evidence_policy_for_autonomy(self.workflow_contract().autonomy_mode);
        packet.trust = evidence_trust_summary_from_messages(&self.messages);
        packet
            .summary
            .push("Agent run completed; inspect trace.jsonl for structured event details.".into());
        packet.actions = evidence_actions_from_messages(&self.messages);
        packet.verification = self
            .verification_gates
            .iter()
            .map(evidence_verification_gate)
            .collect();
        if let Some(metadata) = worktree_metadata {
            packet.summary.push(format!(
                "Worktree-auto changes ran in {} on branch {}; inspect worktree artifacts for status and patch.",
                metadata.worktree_path.display(),
                metadata.branch
            ));
        }
        packet.artifacts = vec![
            EvidenceArtifact {
                kind: "trace".into(),
                path: artifacts.trace_path(),
                summary: Some("Structured runtime event trace".into()),
            },
            EvidenceArtifact {
                kind: "workflow-contract".into(),
                path: artifacts.workflow_contract_path(),
                summary: Some("Workflow contract snapshot".into()),
            },
        ];
        if let Some(metadata) = worktree_metadata {
            let worktree_dir = artifacts.root().join("worktree");
            packet.artifacts.extend([
                EvidenceArtifact {
                    kind: "worktree-status".into(),
                    path: metadata.status_path.clone(),
                    summary: Some(format!(
                        "Worktree status for {} ({})",
                        metadata.worktree_path.display(),
                        if metadata.clean { "clean" } else { "dirty" }
                    )),
                },
                EvidenceArtifact {
                    kind: "worktree-diff-stat".into(),
                    path: metadata.stat_path.clone(),
                    summary: Some("Worktree diff summary".into()),
                },
                EvidenceArtifact {
                    kind: "worktree-diff".into(),
                    path: metadata.patch_path.clone(),
                    summary: Some("Binary-safe worktree patch".into()),
                },
                EvidenceArtifact {
                    kind: "worktree-metadata".into(),
                    path: worktree_dir.join("worktree-metadata.json"),
                    summary: Some(format!(
                        "Worktree {} on branch {}",
                        metadata.worktree_path.display(),
                        metadata.branch
                    )),
                },
            ]);
        }
        let evidence_path = artifacts.evidence_path();
        if packet.write_markdown(&evidence_path).is_ok() {
            self.write_trace_event(&AgentEvent::EvidenceWritten {
                path: evidence_path.clone(),
            });
            let _ = self
                .event_tx
                .send(AgentEvent::EvidenceWritten {
                    path: evidence_path,
                })
                .await;
        }
    }

    pub fn set_workflow_mana_skill_available(&mut self, available: bool) {
        self.workflow_layer.set_mana_skill_available(available);
    }

    pub fn set_workflow_mana_basics_skill_available(&mut self, available: bool) {
        self.workflow_layer
            .set_mana_basics_skill_available(available);
    }

    pub fn set_workflow_mana_delegation_skill_available(&mut self, available: bool) {
        self.workflow_layer
            .set_mana_delegation_skill_available(available);
    }

    pub(super) fn mark_workflow_externalization_nudge_queued(&mut self) {
        self.workflow_layer.mark_externalization_nudge_queued();
    }

    pub(super) fn workflow_pre_turn_follow_up_hint(
        &self,
        prompt: &str,
        tools_available: bool,
    ) -> Option<&'static str> {
        mana_skill_follow_up_hint(
            prompt,
            self.mode,
            tools_available,
            self.workflow_layer.has_mana_skill,
            self.workflow_layer.has_mana_basics_skill,
            self.workflow_layer.has_mana_delegation_skill,
        )
    }

    pub(super) fn workflow_externalization_follow_up(
        &self,
        message: &AssistantMessage,
    ) -> Option<&'static str> {
        should_queue_mana_externalization_follow_up(
            message,
            self.initial_user_prompt(),
            self.mode,
            self.workflow_layer.has_mana_skill,
            self.workflow_layer.queued_mana_externalization_nudge,
        )
        .then(mana_externalization_follow_up_text)
    }

    #[cfg(test)]
    pub(super) fn should_queue_workflow_externalization_for_test(
        &self,
        message: &AssistantMessage,
        user_prompt: &str,
    ) -> bool {
        should_queue_mana_externalization_follow_up(
            message,
            user_prompt,
            self.mode,
            self.workflow_layer.has_mana_skill,
            self.workflow_layer.queued_mana_externalization_nudge,
        )
    }

    pub(super) fn workflow_post_turn_signals(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
        mana_review: &TurnManaReview,
    ) -> WorkflowPostTurnSignals {
        let orchestration_run_id = mana_orchestration_run_id(tool_results);
        WorkflowPostTurnSignals {
            execution_debt: tool_results_indicate_execution_debt(tool_results, self.mode),
            execution_evidence: tool_results_indicate_execution_evidence(tool_results, self.mode),
            orchestration_started: orchestration_run_id.is_some()
                || tool_results_indicate_orchestration_started(tool_results, self.mode),
            orchestration_run_id,
            durable_workflow_progress: tool_results_indicate_durable_workflow_progress(
                tool_results,
                self.mode,
            ),
            stop_reason: mana_review_stop_reason(mana_review, self.mode),
        }
    }

    pub(super) fn workflow_continue_recommendation(
        &self,
        signals: &WorkflowPostTurnSignals,
    ) -> Option<ContinueRecommendation> {
        if signals.orchestration_started {
            Some(ContinueRecommendation {
                prompt: orchestration_follow_up_text(signals.orchestration_run_id.as_deref()),
                reason: ContinueReason::OrchestrationProgress,
            })
        } else if signals.durable_workflow_progress {
            Some(ContinueRecommendation {
                prompt: mana_workflow_follow_up_text().to_string(),
                reason: ContinueReason::ManaWorkflowProgress,
            })
        } else {
            None
        }
    }

    #[cfg(test)]
    pub(super) fn workflow_execution_debt_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_execution_debt(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(super) fn workflow_execution_evidence_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_execution_evidence(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(super) fn workflow_orchestration_run_id_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> Option<String> {
        mana_orchestration_run_id(tool_results)
    }

    #[cfg(test)]
    pub(super) fn workflow_orchestration_started_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_orchestration_started(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(super) fn workflow_durable_progress_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_durable_workflow_progress(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(super) fn workflow_review_stop_reason_for_test(
        &self,
        mana_review: &TurnManaReview,
    ) -> Option<StopReason> {
        mana_review_stop_reason(mana_review, self.mode)
    }

    pub(super) fn turn_mana_review_accumulator(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>> {
        self.workflow_layer.turn_mana_review()
    }

    pub(super) fn begin_turn_mana_review(&self, turn: u32) {
        if let Ok(mut review) = self.workflow_layer.turn_mana_review.lock() {
            review.begin_turn(turn);
        }
    }

    pub(super) fn record_turn_mana_mutations(&self, tool_results: &[imp_llm::ToolResultMessage]) {
        let Ok(mut review) = self.workflow_layer.turn_mana_review.lock() else {
            return;
        };

        for result in tool_results {
            if let Some(record) = mutation_record_from_mana_result(result) {
                review.push(record);
            }
        }
    }

    pub(super) fn finish_turn_mana_review(&self, turn: u32) -> TurnManaReview {
        match self.workflow_layer.turn_mana_review.lock() {
            Ok(review) => {
                let review = review.finalize();
                if review.turn_index == turn {
                    review
                } else {
                    TurnManaReview::no_change(turn)
                }
            }
            Err(_) => TurnManaReview::no_change(turn),
        }
    }

    pub(super) async fn begin_workflow_turn(
        &mut self,
        prompt: &str,
        run_artifacts: Option<&storage::RunArtifacts>,
    ) {
        self.bootstrap_workflow_if_required(prompt).await;
        self.workflow_layer.controller_mut().tick_turn();
        self.persist_workflow_controller_snapshot(run_artifacts)
            .await;
    }

    pub(super) async fn persist_workflow_controller_snapshot(
        &self,
        run_artifacts: Option<&storage::RunArtifacts>,
    ) {
        if let Some(artifacts) = run_artifacts {
            let _ = self
                .workflow_layer
                .controller()
                .save_to_path(&artifacts.workflow_controller_path());
        }
        self.emit(AgentEvent::WorkflowControllerSnapshot {
            snapshot: self.workflow_layer.controller().snapshot(),
        })
        .await;
    }

    pub(super) fn enforce_workflow_closeout_status(
        &self,
        status: crate::agent::RunFinalStatus,
    ) -> crate::agent::RunFinalStatus {
        self.workflow_layer
            .controller()
            .enforce_closeout_status(status)
    }

    pub(super) async fn bootstrap_workflow_if_required(&mut self, prompt: &str) {
        if !matches!(
            self.workflow_layer.controller().bootstrap,
            crate::workflow::WorkflowBootstrapState::Unspecified
        ) {
            return;
        }

        if matches!(self.mode, AgentMode::Orchestrator) {
            let decision = crate::workflow::classify_workflow_intent(prompt, true);
            crate::workflow::apply_intent_to_controller(
                self.workflow_layer.controller_mut(),
                &decision,
            );
        }
        if !self.workflow_layer.controller().bootstrap_required() {
            return;
        }

        let shape = crate::workflow::classify_graph_shape(prompt);
        crate::workflow::apply_graph_shape_to_controller(
            self.workflow_layer.controller_mut(),
            &shape,
        );

        let Ok(mana_dir) = mana_core::discovery::find_mana_dir(&self.cwd) else {
            self.workflow_layer.controller_mut().skip_bootstrap(format!(
                "mana bootstrap unavailable: no .mana found for {}",
                self.cwd.display()
            ));
            return;
        };

        let request = crate::workflow::WorkflowBootstrapRequest::from_prompt(prompt, &self.cwd);
        match crate::workflow::create_native_mana_root(&mana_dir, request) {
            Ok(root) => {
                self.workflow_layer
                    .controller_mut()
                    .bind_mana_root(root.mana_root_id);
                self.workflow_layer
                    .controller_mut()
                    .record_mana_graph_changed();
            }
            Err(err) => {
                self.workflow_layer.controller_mut().closeout.blocker = Some(format!(
                    "workflow bootstrap failed to create mana root: {err}"
                ));
            }
        }
    }

    pub fn resume_workflow_controller_from_project_run(
        &mut self,
        run_id: &str,
    ) -> std::io::Result<()> {
        let artifacts = crate::storage::existing_project_run_artifacts(&self.cwd, run_id)?;
        self.workflow_layer.replace_controller(
            crate::workflow::WorkflowRunController::load_from_path(
                &artifacts.workflow_controller_path(),
            )?,
        );
        Ok(())
    }

    pub(super) fn initial_user_prompt(&self) -> &str {
        self.messages
            .iter()
            .find_map(|message| match message {
                Message::User(user) => user.content.iter().find_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                }),
                _ => None,
            })
            .unwrap_or("")
    }

    pub(super) fn should_retry_unanswered_execution_debt(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
        execution_evidence: bool,
    ) -> bool {
        !matches!(self.mode, AgentMode::Planner)
            && self.queued_execution_debt_follow_up_count == 1
            && !execution_evidence
            && tool_results
                .iter()
                .all(|result| result.is_error || result.tool_name == "work")
    }

    pub(super) fn record_obligations_from_tool_results(
        &mut self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) {
        if tool_results_indicate_failed_bash_command(tool_results, self.mode) {
            self.obligation_ledger
                .add(failed_command_recovery_obligation());
        }
        if tool_results_include_successful_edit(tool_results) {
            self.obligation_ledger
                .add(edited_files_verification_obligation());
        }
        if tool_results_include_successful_check(tool_results) {
            self.obligation_ledger
                .resolve_kind(ObligationKind::EditedFilesVerification);
        }
        if tool_results_indicate_execution_evidence(tool_results, self.mode) {
            self.obligation_ledger
                .resolve_kind(ObligationKind::FailedCommandRecovery);
        }
    }

    pub(super) fn record_workflow_obligations_from_tool_results(
        &mut self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) {
        for result in tool_results {
            if result.is_error {
                continue;
            }

            match result.tool_name.as_str() {
                "mana" | "work" => self.record_workflow_mana_obligation(result),
                "write" | "edit" | "multi_edit" => {
                    self.workflow_layer
                        .controller_mut()
                        .record_direct_work_changed();
                    self.workflow_layer.controller_mut().record_closeout_ready();
                }
                "read" if self.workflow_layer.controller().direct_closeout_required => {
                    self.workflow_layer.controller_mut().record_closeout_ready();
                }
                "bash" | "shell" if bash_result_is_successful_check(result) => {
                    self.workflow_layer.controller_mut().record_closeout_ready();
                }
                _ => {}
            }
        }
    }

    fn record_workflow_mana_obligation(&mut self, result: &imp_llm::ToolResultMessage) {
        let Some(action) = mana_result_action(result) else {
            return;
        };

        match durable_workflow_action_class(action) {
            ManaActionClass::Orchestration => {
                if matches!(action, "run_state" | "evaluate") {
                    if let Some((run_id, status)) = mana_run_status_from_result(result) {
                        self.workflow_layer
                            .controller_mut()
                            .update_child_run_status(&run_id, status);
                    }
                } else {
                    self.workflow_layer
                        .controller_mut()
                        .record_mana_orchestration_started(mana_orchestration_run_id(
                            std::slice::from_ref(result),
                        ));
                }
            }
            ManaActionClass::ProgressCheckpoint
            | ManaActionClass::GraphMutation
            | ManaActionClass::DecisionFact => {
                if matches!(action, "create") {
                    if let Some(unit_id) = mana_unit_id_from_result(result) {
                        if mana_result_parent_id(result).as_deref()
                            == self.workflow_layer.controller().mana_root_id.as_deref()
                        {
                            self.workflow_layer
                                .controller_mut()
                                .record_child_unit(unit_id);
                        } else {
                            self.workflow_layer.controller_mut().bind_mana_root(unit_id);
                        }
                    }
                }
                self.workflow_layer
                    .controller_mut()
                    .record_mana_graph_changed();
            }
            ManaActionClass::Lifecycle => {
                if matches!(action, "verify" | "close") {
                    if let Some(unit_id) = mana_unit_id_from_result(result) {
                        self.workflow_layer.controller_mut().complete_unit(&unit_id);
                    }
                    self.workflow_layer.controller_mut().record_closeout_ready();
                } else {
                    self.workflow_layer
                        .controller_mut()
                        .record_mana_graph_changed();
                }
            }
            ManaActionClass::ReadHelp
            | ManaActionClass::Inspect
            | ManaActionClass::Destructive
            | ManaActionClass::Unknown => {}
        }
    }

    pub(super) fn override_finish_with_workflow_decision(
        &self,
        decision: LoopDecision,
    ) -> LoopDecision {
        if !workflow_layer_may_override_finish(&decision) {
            return decision;
        }
        self.workflow_controller_continue_decision()
            .unwrap_or(decision)
    }

    pub(super) fn workflow_controller_continue_decision(&self) -> Option<LoopDecision> {
        match self.workflow_layer.controller().decide_next() {
            crate::workflow::WorkflowControllerDecision::Continue { prompt, reason } => {
                Some(LoopDecision::Continue { prompt, reason })
            }
            crate::workflow::WorkflowControllerDecision::Stop { .. } => None,
        }
    }
}
