//! Mana/work-graph compatibility support for imp workflow integration.

#[cfg(feature = "mana-integration")]
use std::path::{Path, PathBuf};

use crate::agent::{
    mana_loop::{self, ManaActionClass},
    Agent, AgentEvent, ContentBlock, ContinueReason, ContinueRecommendation, LoopDecision, Message,
    StopReason,
};
use crate::config::AgentMode;
use crate::mana_review::{
    ManaMutationAction, ManaMutationRecord, ManaReviewScope, ManaReviewScopeKind, ManaReviewState,
    ManaUnitSnapshot, TurnManaReview, TurnManaReviewAccumulator,
};
use crate::storage;
use imp_llm::AssistantMessage;

use super::super::{
    assistant_message_contains_mana_tool_call, assistant_message_text,
    bash_result_is_successful_check,
};

#[cfg(feature = "mana-integration")]
fn find_mana_dir(cwd: &Path) -> Option<PathBuf> {
    let mut current = if cwd.is_file() { cwd.parent()? } else { cwd };
    loop {
        let candidate = current.join(".mana");
        if candidate.is_dir() {
            return Some(candidate);
        }
        current = current.parent()?;
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

pub(crate) fn mana_run_status_from_result(
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

impl Agent {
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

    pub(in crate::agent) fn mark_workflow_externalization_nudge_queued(&mut self) {
        self.workflow_layer.mark_externalization_nudge_queued();
    }

    pub(in crate::agent) fn workflow_pre_turn_follow_up_hint(
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

    pub(in crate::agent) fn workflow_externalization_follow_up(
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

    pub(in crate::agent) fn should_queue_workflow_externalization_for_test(
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

    pub(in crate::agent) fn workflow_post_turn_signals(
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
                prompt: mana_workflow_follow_up_text().to_string(),
                reason: ContinueReason::ManaWorkflowProgress,
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
        mana_orchestration_run_id(tool_results)
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
        mana_review: &TurnManaReview,
    ) -> Option<StopReason> {
        mana_review_stop_reason(mana_review, self.mode)
    }

    pub(in crate::agent) fn turn_mana_review_accumulator(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>> {
        self.workflow_layer.turn_mana_review()
    }

    pub(in crate::agent) fn begin_turn_mana_review(&self, turn: u32) {
        if let Ok(mut review) = self.workflow_layer.turn_mana_review.lock() {
            review.begin_turn(turn);
        }
    }

    pub(in crate::agent) fn record_turn_mana_mutations(
        &self,
        tool_results: &[imp_llm::ToolResultMessage],
    ) {
        let Ok(mut review) = self.workflow_layer.turn_mana_review.lock() else {
            return;
        };

        for result in tool_results {
            if let Some(record) = mutation_record_from_mana_result(result) {
                review.push(record);
            }
        }
    }

    pub(in crate::agent) fn finish_turn_mana_review(&self, turn: u32) -> TurnManaReview {
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

        #[cfg(feature = "mana-integration")]
        {
            let Some(mana_dir) = find_mana_dir(&self.cwd) else {
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

        #[cfg(not(feature = "mana-integration"))]
        self.workflow_layer.controller_mut().skip_bootstrap(
            "mana bootstrap unavailable: imp-core built without mana-integration".to_string(),
        );
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

    pub(in crate::agent) fn workflow_controller_continue_decision(&self) -> Option<LoopDecision> {
        match self.workflow_layer.controller().decide_next() {
            crate::workflow::WorkflowControllerDecision::Continue { prompt, reason } => {
                Some(LoopDecision::Continue { prompt, reason })
            }
            crate::workflow::WorkflowControllerDecision::Stop { .. } => None,
        }
    }
}
