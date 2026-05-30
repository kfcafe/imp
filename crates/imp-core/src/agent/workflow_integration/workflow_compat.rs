//! Durable workflow compatibility support for imp workflow integration.

use crate::agent::{
    Agent, AgentEvent, ContentBlock, ContinueReason, ContinueRecommendation, LoopDecision, Message,
    StopReason,
};
use crate::config::AgentMode;
use crate::storage;
use crate::workflow_review::{
    TurnWorkflowReview, TurnWorkflowReviewAccumulator, WorkflowMutationAction,
    WorkflowMutationRecord, WorkflowReviewScope, WorkflowReviewScopeKind, WorkflowReviewState,
    WorkflowUnitSnapshot,
};
use imp_llm::AssistantMessage;

use super::super::{
    assistant_message_contains_workflow_tool_call, assistant_message_text,
    bash_result_is_successful_check,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowActionClass {
    ReadHelp,
    Inspect,
    ProgressCheckpoint,
    GraphMutation,
    DecisionFact,
    Lifecycle,
    Orchestration,
    Destructive,
    Unknown,
}

fn classify_workflow_action(action: &str) -> WorkflowActionClass {
    match action {
        "guide" | "template" => WorkflowActionClass::ReadHelp,
        "status" | "list" | "show" | "logs" | "agents" | "next" | "tree" | "search" | "runs"
        | "validate" | "scope" => WorkflowActionClass::Inspect,
        "update" | "notes_append" | "context" | "refresh_context" | "outcome"
        | "prototype_outcome" | "remember" => WorkflowActionClass::ProgressCheckpoint,
        "create" | "dep_add" | "dep_remove" | "reparent" => WorkflowActionClass::GraphMutation,
        "decision_add" | "decision_resolve" | "fact_create" | "fact_verify" => {
            WorkflowActionClass::DecisionFact
        }
        "claim" | "release" | "verify" | "close" | "reopen" | "fail" => {
            WorkflowActionClass::Lifecycle
        }
        "run" | "evaluate" | "run_state" => WorkflowActionClass::Orchestration,
        "delete" => WorkflowActionClass::Destructive,
        _ => WorkflowActionClass::Unknown,
    }
}

#[derive(Debug, Default)]
pub(crate) struct WorkflowPostTurnSignals {
    pub(crate) execution_debt: bool,
    pub(crate) execution_evidence: bool,
    pub(crate) orchestration_run_id: Option<String>,
    pub(crate) orchestration_started: bool,
    pub(crate) durable_workflow_progress: bool,
    pub(crate) stop_reason: Option<StopReason>,
}

fn workflow_review_scope_from_result(result: &imp_llm::ToolResultMessage) -> WorkflowReviewScope {
    let display = result
        .details
        .get("path")
        .or_else(|| result.details.get("workflow_dir"))
        .and_then(|value| value.as_str())
        .unwrap_or("auto")
        .to_string();

    WorkflowReviewScope {
        kind: if display == "auto" {
            WorkflowReviewScopeKind::None
        } else {
            WorkflowReviewScopeKind::ExplicitPath
        },
        display,
    }
}

fn unit_snapshot_from_value(value: &serde_json::Value) -> Option<WorkflowUnitSnapshot> {
    serde_json::from_value(value.clone()).ok()
}

fn unit_snapshot_from_result(result: &imp_llm::ToolResultMessage) -> Option<WorkflowUnitSnapshot> {
    result
        .details
        .get("unit")
        .and_then(unit_snapshot_from_value)
}

fn workflow_mutation_action(action: &str) -> Option<WorkflowMutationAction> {
    match action {
        "create" => Some(WorkflowMutationAction::Create),
        "close" => Some(WorkflowMutationAction::Close),
        "update" => Some(WorkflowMutationAction::Update),
        "notes_append" => Some(WorkflowMutationAction::NotesAppend),
        "decision_add" => Some(WorkflowMutationAction::DecisionAdd),
        "decision_resolve" => Some(WorkflowMutationAction::DecisionResolve),
        "reopen" => Some(WorkflowMutationAction::Reopen),
        "fail" => Some(WorkflowMutationAction::Fail),
        "delete" => Some(WorkflowMutationAction::Delete),
        "dep_add" => Some(WorkflowMutationAction::DepAdd),
        "dep_remove" => Some(WorkflowMutationAction::DepRemove),
        "fact_create" => Some(WorkflowMutationAction::FactCreate),
        _ => None,
    }
}

fn mutation_record_from_workflow_result(
    result: &imp_llm::ToolResultMessage,
) -> Option<WorkflowMutationRecord> {
    if result.is_error || result.tool_name != "workflow" {
        return None;
    }

    let action_name = workflow_result_action(result)?;
    let action = workflow_mutation_action(action_name)?;
    let after_unit = unit_snapshot_from_result(result);
    let deleted_unit = if action == WorkflowMutationAction::Delete {
        let id = result.details.get("id")?.as_str()?.to_string();
        let title = result
            .details
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or(&id)
            .to_string();
        Some(crate::workflow_review::WorkflowUnitRef::new(
            id, title, None,
        ))
    } else {
        None
    };

    if after_unit.is_none()
        && deleted_unit.is_none()
        && !matches!(
            action,
            WorkflowMutationAction::DepAdd | WorkflowMutationAction::DepRemove
        )
    {
        return None;
    }

    Some(WorkflowMutationRecord {
        action,
        scope: workflow_review_scope_from_result(result),
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

fn durable_workflow_follow_up_text() -> &'static str {
    "An imp-work task was closed, verified, or materially advanced, but that only proves the current task changed. Inspect the active work scope with work(action=\"next\") or work(action=\"validate\"), continue any ready work, and only stop when the requested outcome is complete, blocked by a concrete decision, or no runnable work remains."
}

fn durable_workflow_action(result: &imp_llm::ToolResultMessage) -> Option<&str> {
    match result.tool_name.as_str() {
        "workflow" | "work" => result
            .details
            .get("action")
            .and_then(|value| value.as_str())
            .or_else(|| {
                result
                    .details
                    .get("workflow_loop_policy")
                    .and_then(|policy| policy.get("action"))
                    .and_then(|value| value.as_str())
            }),
        _ => None,
    }
}

fn durable_workflow_action_class(action: &str) -> WorkflowActionClass {
    match action {
        // Native imp-work names that do not exist in legacy workflow but mutate or
        // materially advance durable work state.
        "context" | "refresh_context" | "outcome" | "prototype_outcome" | "remember" => {
            WorkflowActionClass::ProgressCheckpoint
        }
        // Native imp-work inspection/read-only actions.
        "search" | "runs" | "validate" | "scope" => WorkflowActionClass::Inspect,
        _ => classify_workflow_action(action),
    }
}

fn durable_workflow_mutation_is_execution_debt(action: &str) -> bool {
    matches!(
        durable_workflow_action_class(action),
        WorkflowActionClass::ProgressCheckpoint
            | WorkflowActionClass::GraphMutation
            | WorkflowActionClass::DecisionFact
    )
}

fn durable_workflow_lifecycle_is_execution_evidence(action: &str) -> bool {
    matches!(
        durable_workflow_action_class(action),
        WorkflowActionClass::Lifecycle | WorkflowActionClass::Orchestration
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

fn workflow_result_action(result: &imp_llm::ToolResultMessage) -> Option<&str> {
    durable_workflow_action(result)
}

fn workflow_unit_id_from_result(result: &imp_llm::ToolResultMessage) -> Option<String> {
    result
        .details
        .get("id")
        .or_else(|| result.details.get("unit_id"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn workflow_result_parent_id(result: &imp_llm::ToolResultMessage) -> Option<String> {
    result
        .details
        .get("parent")
        .or_else(|| result.details.get("parent_id"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

pub(crate) fn workflow_run_status_from_result(
    result: &imp_llm::ToolResultMessage,
) -> Option<(String, crate::workflow::WorkflowChildRunStatus)> {
    if result.is_error || result.tool_name != "workflow" {
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
        crate::workflow::WorkflowChildRunStatus::from_workflow_run_status(status, total_failed),
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
            && matches!(result.tool_name.as_str(), "workflow" | "work")
            && durable_workflow_action(result)
                .is_some_and(durable_workflow_mutation_is_execution_debt)
    })
}

fn workflow_orchestration_run_id(tool_results: &[imp_llm::ToolResultMessage]) -> Option<String> {
    tool_results.iter().find_map(|result| {
        if result.is_error || !matches!(result.tool_name.as_str(), "workflow" | "work") {
            return None;
        }
        let action = durable_workflow_action(result)?;
        if durable_workflow_action_class(action) != WorkflowActionClass::Orchestration {
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
            && matches!(result.tool_name.as_str(), "workflow" | "work")
            && durable_workflow_action(result).is_some_and(|action| {
                durable_workflow_action_class(action) == WorkflowActionClass::Orchestration
            })
    })
}

pub(super) fn tool_results_indicate_execution_evidence(
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
            "workflow" | "work" => durable_workflow_action(result)
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
        if result.is_error || !matches!(result.tool_name.as_str(), "workflow" | "work") {
            return false;
        }

        let action = durable_workflow_action(result);

        matches!(action, Some("close"))
            || matches!(action, Some("verify") if durable_workflow_verify_passed(result))
            || durable_workflow_result_has_closed_item(result)
    })
}

fn workflow_review_stop_reason(
    workflow_review: &TurnWorkflowReview,
    mode: AgentMode,
) -> Option<StopReason> {
    match workflow_review.state {
        WorkflowReviewState::NeedsDecision => Some(StopReason::UserBlocker),
        WorkflowReviewState::Changed if matches!(mode, AgentMode::Planner) => {
            if !workflow_review.proposed_children.is_empty()
                || !workflow_review.touched_units.is_empty()
                || !workflow_review.material_field_changes.is_empty()
                || !workflow_review.notes_appended.is_empty()
                || !workflow_review.decision_events.is_empty()
            {
                Some(StopReason::DecompositionCompleted)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn should_queue_workflow_externalization_follow_up(
    message: &AssistantMessage,
    user_prompt: &str,
    mode: AgentMode,
    _has_workflow_skill: bool,
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

    if assistant_message_contains_workflow_tool_call(message) {
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

fn workflow_externalization_follow_up_text() -> &'static str {
    "Before you continue: the user explicitly asked for durable work structure. Externalize the plan or decomposition you just described into native imp-work now. Create or update the relevant task(s) with native work actions, prefer global project scope, and avoid extra chat restatement when the work tool/UI already makes the delta obvious."
}

fn workflow_skill_follow_up_hint(
    prompt: &str,
    mode: AgentMode,
    tools_available: bool,
    _has_workflow_skill: bool,
    _has_workflow_basics_skill: bool,
    _has_workflow_delegation_skill: bool,
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
            Some(
                "Before you continue: use native imp-work `guide` when you need extra help with task design, decomposition, retries, or worker handoff.",
            )
        }
        AgentMode::Worker | AgentMode::Auditor if work_signal => Some(
            "Before you continue: use the native work tool and stay within this mode's allowed imp-work workflow. Use the `guide` action if you need help.",
        ),
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

impl Agent {
    pub fn set_workflow_skill_available(&mut self, available: bool) {
        self.workflow_layer.set_workflow_skill_available(available);
    }

    pub fn set_workflow_basics_skill_available(&mut self, available: bool) {
        self.workflow_layer
            .set_workflow_basics_skill_available(available);
    }

    pub fn set_workflow_delegation_skill_available(&mut self, available: bool) {
        self.workflow_layer
            .set_workflow_delegation_skill_available(available);
    }

    pub(in crate::agent) fn mark_workflow_externalization_nudge_queued(&mut self) {
        self.workflow_layer.mark_externalization_nudge_queued();
    }

    pub(in crate::agent) fn workflow_pre_turn_follow_up_hint(
        &self,
        prompt: &str,
        tools_available: bool,
    ) -> Option<&'static str> {
        workflow_skill_follow_up_hint(
            prompt,
            self.mode,
            tools_available,
            self.workflow_layer.has_workflow_skill,
            self.workflow_layer.has_workflow_basics_skill,
            self.workflow_layer.has_workflow_delegation_skill,
        )
    }

    pub(in crate::agent) fn workflow_externalization_follow_up(
        &self,
        message: &AssistantMessage,
    ) -> Option<&'static str> {
        should_queue_workflow_externalization_follow_up(
            message,
            self.initial_user_prompt(),
            self.mode,
            self.workflow_layer.has_workflow_skill,
            self.workflow_layer.queued_workflow_externalization_nudge,
        )
        .then(workflow_externalization_follow_up_text)
    }

    #[cfg(test)]
    pub(in crate::agent) fn should_queue_workflow_externalization_for_test(
        &self,
        message: &AssistantMessage,
        user_prompt: &str,
    ) -> bool {
        should_queue_workflow_externalization_follow_up(
            message,
            user_prompt,
            self.mode,
            self.workflow_layer.has_workflow_skill,
            self.workflow_layer.queued_workflow_externalization_nudge,
        )
    }

    pub(in crate::agent) fn workflow_post_turn_signals(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
        workflow_review: &TurnWorkflowReview,
    ) -> WorkflowPostTurnSignals {
        let orchestration_run_id = workflow_orchestration_run_id(tool_results);
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
            stop_reason: workflow_review_stop_reason(workflow_review, self.mode),
        }
    }

    pub(in crate::agent) fn workflow_continue_recommendation(
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
                prompt: durable_workflow_follow_up_text().to_string(),
                reason: ContinueReason::WorkflowProgress,
            })
        } else {
            None
        }
    }

    #[cfg(test)]
    pub(in crate::agent) fn workflow_execution_debt_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_execution_debt(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(in crate::agent) fn workflow_execution_evidence_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_execution_evidence(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(in crate::agent) fn workflow_orchestration_run_id_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> Option<String> {
        workflow_orchestration_run_id(tool_results)
    }

    #[cfg(test)]
    pub(in crate::agent) fn workflow_orchestration_started_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_orchestration_started(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(in crate::agent) fn workflow_durable_progress_for_test(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) -> bool {
        tool_results_indicate_durable_workflow_progress(tool_results, self.mode)
    }

    #[cfg(test)]
    pub(in crate::agent) fn workflow_review_stop_reason_for_test(
        &self,
        workflow_review: &TurnWorkflowReview,
    ) -> Option<StopReason> {
        workflow_review_stop_reason(workflow_review, self.mode)
    }

    pub(in crate::agent) fn turn_workflow_review_accumulator(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<TurnWorkflowReviewAccumulator>> {
        self.workflow_layer.turn_workflow_review()
    }

    pub(in crate::agent) fn begin_turn_workflow_review(&self, turn: u32) {
        if let Ok(mut review) = self.workflow_layer.turn_workflow_review.lock() {
            review.begin_turn(turn);
        }
    }

    pub(in crate::agent) fn record_turn_workflow_mutations(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) {
        let Ok(mut review) = self.workflow_layer.turn_workflow_review.lock() else {
            return;
        };

        for result in tool_results {
            if let Some(record) = mutation_record_from_workflow_result(result) {
                review.push(record);
            }
        }
    }

    pub(in crate::agent) fn finish_turn_workflow_review(&self, turn: u32) -> TurnWorkflowReview {
        match self.workflow_layer.turn_workflow_review.lock() {
            Ok(review) => {
                let review = review.finalize();
                if review.turn_index == turn {
                    review
                } else {
                    TurnWorkflowReview::no_change(turn)
                }
            }
            Err(_) => TurnWorkflowReview::no_change(turn),
        }
    }

    pub(in crate::agent) async fn begin_workflow_turn(
        &mut self,
        prompt: &str,
        run_artifacts: Option<&storage::RunArtifacts>,
    ) {
        self.bootstrap_workflow_if_required(prompt).await;
        self.workflow_layer.controller_mut().tick_turn();
        self.persist_workflow_controller_snapshot(run_artifacts)
            .await;
    }

    pub(in crate::agent) async fn persist_workflow_controller_snapshot(
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

    pub(in crate::agent) fn enforce_workflow_closeout_status(
        &self,
        status: crate::agent::RunFinalStatus,
    ) -> crate::agent::RunFinalStatus {
        self.workflow_layer
            .controller()
            .enforce_closeout_status(status)
    }

    pub(in crate::agent) async fn bootstrap_workflow_if_required(&mut self, prompt: &str) {
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
        self.workflow_layer
            .controller_mut()
            .skip_bootstrap("workflow bootstrap now uses native workflow artifacts".to_string());
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

    pub(in crate::agent) fn initial_user_prompt(&self) -> &str {
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

    pub(in crate::agent) fn record_workflow_obligations_from_tool_results(
        &mut self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) {
        for result in tool_results {
            if result.is_error {
                continue;
            }

            match result.tool_name.as_str() {
                "workflow" | "work" => self.record_workflow_obligation(result),
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

    fn record_workflow_obligation(&mut self, result: &imp_llm::ToolResultMessage) {
        let Some(action) = workflow_result_action(result) else {
            return;
        };

        match durable_workflow_action_class(action) {
            WorkflowActionClass::Orchestration => {
                if matches!(action, "run_state" | "evaluate") {
                    if let Some((run_id, status)) = workflow_run_status_from_result(result) {
                        self.workflow_layer
                            .controller_mut()
                            .update_child_run_status(&run_id, status);
                    }
                } else {
                    self.workflow_layer
                        .controller_mut()
                        .record_workflow_orchestration_started(workflow_orchestration_run_id(
                            std::slice::from_ref(result),
                        ));
                }
            }
            WorkflowActionClass::ProgressCheckpoint
            | WorkflowActionClass::GraphMutation
            | WorkflowActionClass::DecisionFact => {
                if matches!(action, "create") {
                    if let Some(unit_id) = workflow_unit_id_from_result(result) {
                        if workflow_result_parent_id(result).as_deref()
                            == self.workflow_layer.controller().workflow_root_id.as_deref()
                        {
                            self.workflow_layer
                                .controller_mut()
                                .record_child_unit(unit_id);
                        } else {
                            self.workflow_layer
                                .controller_mut()
                                .bind_workflow_root(unit_id);
                        }
                    }
                }
                self.workflow_layer
                    .controller_mut()
                    .record_workflow_graph_changed();
            }
            WorkflowActionClass::Lifecycle => {
                if matches!(action, "verify" | "close") {
                    if let Some(unit_id) = workflow_unit_id_from_result(result) {
                        self.workflow_layer.controller_mut().complete_unit(&unit_id);
                    }
                    self.workflow_layer.controller_mut().record_closeout_ready();
                } else {
                    self.workflow_layer
                        .controller_mut()
                        .record_workflow_graph_changed();
                }
            }
            WorkflowActionClass::ReadHelp
            | WorkflowActionClass::Inspect
            | WorkflowActionClass::Destructive
            | WorkflowActionClass::Unknown => {}
        }
    }

    pub(in crate::agent) fn workflow_controller_continue_decision(&self) -> Option<LoopDecision> {
        match self.workflow_layer.controller().decide_next() {
            crate::workflow::WorkflowControllerDecision::Continue { prompt, reason } => {
                Some(LoopDecision::Continue { prompt, reason })
            }
            crate::workflow::WorkflowControllerDecision::Stop { .. } => None,
        }
    }
}
