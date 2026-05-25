use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{WorkflowGraphShape, WorkflowRunController};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowIntent {
    Durable,
    Simple,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowIntentDecision {
    pub intent: WorkflowIntent,
    pub reason: String,
}

pub fn classify_workflow_intent(prompt: &str, force_durable: bool) -> WorkflowIntentDecision {
    let normalized = prompt.to_ascii_lowercase();
    if force_durable {
        return WorkflowIntentDecision {
            intent: WorkflowIntent::Durable,
            reason: "runtime mode requires durable workflow".to_string(),
        };
    }

    let durable_phrases = [
        "make it happen",
        "implement",
        "build",
        "fix",
        "continue",
        "ship",
        "refactor",
        "wire",
        "add tests",
        "autonomous",
        "multiple hours",
    ];
    if durable_phrases
        .iter()
        .any(|phrase| normalized.contains(phrase))
    {
        return WorkflowIntentDecision {
            intent: WorkflowIntent::Durable,
            reason: "prompt matches durable work rule".to_string(),
        };
    }

    WorkflowIntentDecision {
        intent: WorkflowIntent::Simple,
        reason: "no durable workflow rule matched".to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphShapeDecision {
    pub shape: WorkflowGraphShape,
    pub reason: String,
}

pub fn classify_graph_shape(prompt: &str) -> WorkflowGraphShapeDecision {
    let normalized = prompt.to_ascii_lowercase();
    let decomposition_markers = [
        "multiple hours",
        "autonomous",
        "decompose",
        "plan",
        "architecture",
        "migration",
        "multi-component",
        "across the repo",
        "end-to-end",
        "several",
        "many files",
    ];

    if decomposition_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return WorkflowGraphShapeDecision {
            shape: WorkflowGraphShape::NeedsDecomposition,
            reason: "prompt matches decomposition rule".to_string(),
        };
    }

    WorkflowGraphShapeDecision {
        shape: WorkflowGraphShape::RootOnly,
        reason: "no decomposition rule matched".to_string(),
    }
}

pub fn apply_graph_shape_to_controller(
    controller: &mut WorkflowRunController,
    decision: &WorkflowGraphShapeDecision,
) {
    controller.set_graph_shape(decision.shape);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowBootstrapRequest {
    pub title: String,
    pub description: String,
    pub acceptance: String,
    pub labels: Vec<String>,
    pub paths: Vec<String>,
}

impl WorkflowBootstrapRequest {
    pub fn from_prompt(prompt: &str, cwd: &Path) -> Self {
        let title = summarize_goal_title(prompt);
        Self {
            title,
            description: format!("Durable workflow goal from user prompt:\n\n{prompt}"),
            acceptance: "Requested outcome is implemented or answered; relevant verification has run; blockers or concerns are recorded before closeout.".to_string(),
            labels: vec!["imp".to_string(), "workflow".to_string(), "autonomy".to_string()],
            paths: cwd
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| vec![name.to_string()])
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowBootstrapResult {
    pub mana_root_id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkProductTaskSpec {
    pub title: String,
    pub description: String,
    pub acceptance: String,
    pub paths: Vec<String>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedWorkProductTask {
    pub unit_id: String,
    pub path: PathBuf,
}

pub fn validate_work_product_task_spec(spec: &WorkProductTaskSpec) -> Result<(), String> {
    if spec.title.trim().is_empty() {
        return Err("work product task title is required".to_string());
    }
    if spec.acceptance.trim().is_empty() {
        return Err("work product task acceptance criteria are required".to_string());
    }
    let normalized = format!("{}\n{}", spec.title, spec.description).to_ascii_lowercase();
    let lifecycle_only = [
        "verify",
        "verification",
        "closeout",
        "close out",
        "run tests",
        "test the work",
        "review diff",
        "review the diff",
        "report results",
        "summarize",
        "summary",
        "update status",
        "status update",
        "check work",
    ];
    if lifecycle_only
        .iter()
        .any(|phrase| normalized.trim() == *phrase || normalized.contains(phrase))
    {
        return Err("work product task appears to be lifecycle-only, not durable work".to_string());
    }
    Ok(())
}

#[cfg(feature = "mana-integration")]
pub fn create_child_work_product_tasks(
    mana_dir: &Path,
    parent_id: &str,
    specs: &[WorkProductTaskSpec],
) -> Result<Vec<CreatedWorkProductTask>, String> {
    let mut created = Vec::new();
    for spec in specs {
        validate_work_product_task_spec(spec)?;
        let result = mana_core::api::create_unit(
            mana_dir,
            mana_core::ops::create::CreateParams {
                title: spec.title.clone(),
                handle: None,
                description: Some(spec.description.clone()),
                acceptance: Some(spec.acceptance.clone()),
                notes: Some("Created by imp controlled workflow decomposition.".to_string()),
                design: None,
                verify: None,
                priority: Some(1),
                labels: spec.labels.clone(),
                assignee: None,
                dependencies: Vec::new(),
                parent: Some(parent_id.to_string()),
                produces: Vec::new(),
                requires: Vec::new(),
                paths: spec.paths.clone(),
                on_fail: None,
                fail_first: false,
                feature: false,
                kind: Some(mana_core::unit::UnitType::Task),
                verify_timeout: None,
                decisions: Vec::new(),
                force: false,
            },
        )
        .map_err(|err| err.to_string())?;
        created.push(CreatedWorkProductTask {
            unit_id: result.unit.id,
            path: result.path,
        });
    }
    Ok(created)
}

pub fn record_created_work_products(
    controller: &mut WorkflowRunController,
    created: &[CreatedWorkProductTask],
) {
    for task in created {
        controller.record_child_unit(task.unit_id.clone());
    }
}

#[cfg(feature = "mana-integration")]
pub fn create_native_mana_root(
    mana_dir: &Path,
    request: WorkflowBootstrapRequest,
) -> Result<WorkflowBootstrapResult, String> {
    let result = mana_core::api::create_unit(
        mana_dir,
        mana_core::ops::create::CreateParams {
            title: request.title,
            handle: None,
            description: Some(request.description),
            acceptance: Some(request.acceptance),
            notes: Some(
                "Created by imp workflow bootstrap before autonomous execution.".to_string(),
            ),
            design: None,
            verify: None,
            priority: Some(1),
            labels: request.labels,
            assignee: None,
            dependencies: Vec::new(),
            parent: None,
            produces: Vec::new(),
            requires: Vec::new(),
            paths: request.paths,
            on_fail: None,
            fail_first: false,
            feature: true,
            kind: Some(mana_core::unit::UnitType::Task),
            verify_timeout: None,
            decisions: Vec::new(),
            force: false,
        },
    )
    .map_err(|err| err.to_string())?;

    Ok(WorkflowBootstrapResult {
        mana_root_id: result.unit.id,
        path: result.path,
    })
}

pub fn apply_intent_to_controller(
    controller: &mut WorkflowRunController,
    decision: &WorkflowIntentDecision,
) {
    match decision.intent {
        WorkflowIntent::Durable => controller.require_bootstrap(),
        WorkflowIntent::Simple => controller.skip_bootstrap(decision.reason.clone()),
    }
}

fn summarize_goal_title(prompt: &str) -> String {
    let title = prompt
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Workflow goal")
        .trim();
    let mut chars = title.chars();
    let truncated: String = chars.by_ref().take(80).collect();
    if chars.next().is_some() {
        format!("{}…", truncated.trim_end())
    } else if truncated.is_empty() {
        "Workflow goal".to_string()
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_real_work_product_task_spec() {
        let spec = WorkProductTaskSpec {
            title: "Implement workflow controller persistence".to_string(),
            description: "Persist controller state for recovery.".to_string(),
            acceptance: "Controller state round-trips from disk.".to_string(),
            paths: vec!["crates/imp-core/src/workflow".to_string()],
            labels: vec!["workflow".to_string()],
        };

        assert!(validate_work_product_task_spec(&spec).is_ok());
    }

    #[test]
    fn rejects_lifecycle_only_task_spec() {
        let spec = WorkProductTaskSpec {
            title: "Verify and closeout".to_string(),
            description: "Run tests and report results.".to_string(),
            acceptance: "Tests pass.".to_string(),
            paths: Vec::new(),
            labels: Vec::new(),
        };

        assert!(validate_work_product_task_spec(&spec).is_err());
    }

    #[test]
    fn records_created_work_products_on_controller() {
        let mut controller = WorkflowRunController::new().with_mana_root_id("28.1");
        controller.set_graph_shape(WorkflowGraphShape::NeedsDecomposition);
        record_created_work_products(
            &mut controller,
            &[
                CreatedWorkProductTask {
                    unit_id: "28.1.1".to_string(),
                    path: PathBuf::from("28.1.1.md"),
                },
                CreatedWorkProductTask {
                    unit_id: "28.1.2".to_string(),
                    path: PathBuf::from("28.1.2.md"),
                },
            ],
        );

        assert_eq!(controller.active_unit_id.as_deref(), Some("28.1.1"));
        assert_eq!(
            controller.planning,
            crate::workflow::WorkflowPlanningState::Decomposed {
                child_unit_ids: vec!["28.1.1".to_string(), "28.1.2".to_string()],
                completed_child_unit_ids: Vec::new(),
            }
        );
    }

    #[test]
    fn classifies_implementation_as_durable() {
        let decision = classify_workflow_intent("implement the workflow bootstrap", false);
        assert_eq!(decision.intent, WorkflowIntent::Durable);
    }

    #[test]
    fn classifies_focused_work_as_root_only() {
        let decision = classify_graph_shape("implement a focused parser fix");
        assert_eq!(decision.shape, WorkflowGraphShape::RootOnly);
    }

    #[test]
    fn classifies_broad_work_as_needing_decomposition() {
        let decision =
            classify_graph_shape("build an autonomous workflow architecture across the repo");
        assert_eq!(decision.shape, WorkflowGraphShape::NeedsDecomposition);
    }

    #[test]
    fn applies_graph_shape_to_controller() {
        let mut controller = WorkflowRunController::new();
        let decision = classify_graph_shape("multiple hours of autonomous work");
        apply_graph_shape_to_controller(&mut controller, &decision);
        assert_eq!(
            controller.graph_shape,
            WorkflowGraphShape::NeedsDecomposition
        );
        assert_eq!(
            controller.snapshot().graph_shape,
            WorkflowGraphShape::NeedsDecomposition
        );
    }

    #[test]
    fn classifies_question_as_simple_by_default() {
        let decision = classify_workflow_intent("what work remains?", false);
        assert_eq!(decision.intent, WorkflowIntent::Simple);
    }

    #[test]
    fn force_durable_overrides_prompt() {
        let decision = classify_workflow_intent("what is this?", true);
        assert_eq!(decision.intent, WorkflowIntent::Durable);
    }

    #[test]
    fn applies_intent_to_controller() {
        let mut controller = WorkflowRunController::new();
        apply_intent_to_controller(
            &mut controller,
            &WorkflowIntentDecision {
                intent: WorkflowIntent::Durable,
                reason: "test".to_string(),
            },
        );
        assert!(controller.bootstrap_required());
    }
}
