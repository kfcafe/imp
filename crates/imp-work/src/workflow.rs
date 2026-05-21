use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    CheckResult, LinkKind, MemoryItem, MemoryKind, Run, RunOutcome, Task, TaskStatus, WorkId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkTree {
    pub nodes: Vec<WorkTreeNode>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkTreeNode {
    pub id: WorkId,
    pub title: String,
    pub status: TaskStatus,
    pub parent: Option<WorkId>,
    pub dependencies: Vec<WorkId>,
    pub children: Vec<WorkTreeNode>,
    pub readiness: Readiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Readiness {
    pub ready: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifySummary {
    pub passed: bool,
    pub results: Vec<CheckResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseRequest {
    pub verify: Option<VerifySummary>,
    pub force_reason: Option<String>,
    pub summary: String,
    pub changed_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseResult {
    pub task: Task,
    pub run: Run,
    pub forced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailResult {
    pub task: Task,
    pub memory: MemoryItem,
}

pub fn build_work_tree(tasks: &[Task]) -> WorkTree {
    let known_ids = tasks
        .iter()
        .map(|task| task.id.clone())
        .collect::<BTreeSet<_>>();
    let mut warnings = Vec::new();
    for task in tasks {
        for dependency in dependency_ids(task) {
            if !known_ids.contains(&dependency) {
                warnings.push(format!(
                    "{} depends on missing task {}",
                    task.id, dependency
                ));
            }
        }
    }

    let mut child_ids = BTreeSet::new();
    for task in tasks {
        if let Some(parent) = &task.parent {
            child_ids.insert((parent.clone(), task.id.clone()));
        }
    }

    let task_map = tasks
        .iter()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    let roots = tasks
        .iter()
        .filter(|task| task.parent.is_none())
        .map(|task| build_node(task, tasks, &task_map, &mut Vec::new()))
        .collect();

    WorkTree {
        nodes: roots,
        warnings,
    }
}

pub fn readiness_for(task: &Task, tasks: &[Task]) -> Readiness {
    let task_map = tasks
        .iter()
        .map(|task| (task.id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    readiness_for_with_stack(task, &task_map, &mut Vec::new())
}

pub fn summarize_checks(results: Vec<CheckResult>) -> VerifySummary {
    VerifySummary {
        passed: results.iter().all(|result| result.passed),
        results,
    }
}

pub fn close_task_with_conventions(
    mut task: Task,
    request: CloseRequest,
) -> Result<CloseResult, String> {
    let has_checks = !task.checks.is_empty();
    let verify_passed = request.verify.as_ref().is_some_and(|verify| verify.passed);
    let forced = request
        .force_reason
        .as_ref()
        .is_some_and(|reason| !reason.trim().is_empty());

    if has_checks && !verify_passed && !forced {
        return Err(
            "task has checks; close requires passing verify results or a force reason".to_string(),
        );
    }

    task.status = TaskStatus::Done;
    let run = Run {
        id: WorkId::new("R"),
        work_id: Some(task.id.clone()),
        context_pack_id: task.context_pack.clone(),
        outcome: if forced {
            RunOutcome::DoneWithConcerns
        } else {
            RunOutcome::Done
        },
        summary: request.summary,
        changed_paths: request.changed_paths,
        checks: request
            .verify
            .map(|verify| verify.results)
            .unwrap_or_default(),
    };

    Ok(CloseResult { task, run, forced })
}

pub fn fail_task_with_conventions(
    mut task: Task,
    reason: impl Into<String>,
    next_action: Option<String>,
) -> FailResult {
    let reason = reason.into();
    task.status = TaskStatus::Blocked;
    let text = match next_action {
        Some(next_action) if !next_action.trim().is_empty() => {
            format!("BLOCKED: {reason}\nNext action: {next_action}")
        }
        _ => format!("BLOCKED: {reason}"),
    };
    let mut memory = MemoryItem::new(MemoryKind::FollowUp, text);
    memory.parent_work = Some(task.id.clone());
    memory.topics = vec!["failure".to_string(), "blocker".to_string()];
    FailResult { task, memory }
}

fn build_node(
    task: &Task,
    all_tasks: &[Task],
    task_map: &BTreeMap<WorkId, &Task>,
    stack: &mut Vec<WorkId>,
) -> WorkTreeNode {
    let mut children = Vec::new();
    for child in all_tasks
        .iter()
        .filter(|candidate| candidate.parent.as_ref() == Some(&task.id))
    {
        children.push(build_node(child, all_tasks, task_map, stack));
    }
    WorkTreeNode {
        id: task.id.clone(),
        title: task.title.clone(),
        status: task.status,
        parent: task.parent.clone(),
        dependencies: dependency_ids(task),
        children,
        readiness: readiness_for_with_stack(task, task_map, stack),
    }
}

fn readiness_for_with_stack(
    task: &Task,
    task_map: &BTreeMap<WorkId, &Task>,
    stack: &mut Vec<WorkId>,
) -> Readiness {
    let mut blockers = Vec::new();
    if matches!(task.status, TaskStatus::Blocked | TaskStatus::Dropped) {
        blockers.push(format!("task status is {:?}", task.status));
    }
    if stack.contains(&task.id) {
        blockers.push("dependency cycle detected".to_string());
        return Readiness {
            ready: false,
            blockers,
        };
    }
    stack.push(task.id.clone());
    for dependency in dependency_ids(task) {
        match task_map.get(&dependency) {
            Some(dependency_task) if dependency_task.status == TaskStatus::Done => {}
            Some(dependency_task) => blockers.push(format!(
                "dependency {} is {:?}",
                dependency_task.id, dependency_task.status
            )),
            None => blockers.push(format!("dependency {dependency} is missing")),
        }
    }
    stack.pop();
    Readiness {
        ready: blockers.is_empty(),
        blockers,
    }
}

fn dependency_ids(task: &Task) -> Vec<WorkId> {
    task.links
        .iter()
        .filter(|link| link.kind == LinkKind::DependsOn)
        .map(|link| link.target.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Check, CheckKind, Link};

    fn task(id: &str, status: TaskStatus) -> Task {
        let mut task = Task::new(id);
        task.id = WorkId(id.to_string());
        task.status = status;
        task
    }

    #[test]
    fn dependency_tree_reports_readiness_and_blockers() {
        let dependency = task("T-dep", TaskStatus::Done);
        let mut ready = task("T-ready", TaskStatus::Ready);
        ready.links.push(Link {
            kind: LinkKind::DependsOn,
            target: dependency.id.clone(),
        });
        let mut blocked = task("T-blocked", TaskStatus::Ready);
        blocked.links.push(Link {
            kind: LinkKind::DependsOn,
            target: WorkId("T-missing".to_string()),
        });

        let tree = build_work_tree(&[dependency, ready, blocked]);

        assert!(
            tree.nodes
                .iter()
                .find(|node| node.id.0 == "T-ready")
                .expect("ready node")
                .readiness
                .ready
        );
        let blocked_node = tree
            .nodes
            .iter()
            .find(|node| node.id.0 == "T-blocked")
            .expect("blocked node");
        assert!(!blocked_node.readiness.ready);
        assert!(blocked_node.readiness.blockers[0].contains("missing"));
        assert!(tree
            .warnings
            .iter()
            .any(|warning| warning.contains("missing")));
    }

    #[test]
    fn dependency_tree_preserves_parent_child_hierarchy() {
        let parent = task("T-parent", TaskStatus::Ready);
        let mut child = task("T-child", TaskStatus::Ready);
        child.parent = Some(parent.id.clone());

        let tree = build_work_tree(&[parent.clone(), child]);

        let parent_node = tree
            .nodes
            .iter()
            .find(|node| node.id == parent.id)
            .expect("parent node");
        assert_eq!(parent_node.children.len(), 1);
        assert_eq!(parent_node.children[0].id, WorkId("T-child".to_string()));
    }

    #[test]
    fn close_requires_verify_for_checked_tasks_unless_forced() {
        let mut task = task("T-verify", TaskStatus::Review);
        task.checks.push(Check {
            id: WorkId("C-verify".to_string()),
            kind: CheckKind::Command,
            description: "cargo test".to_string(),
            command: Some("cargo test".to_string()),
        });

        let denied = close_task_with_conventions(
            task.clone(),
            CloseRequest {
                verify: None,
                force_reason: None,
                summary: "done".to_string(),
                changed_paths: Vec::new(),
            },
        );
        assert!(denied.is_err());

        let closed = close_task_with_conventions(
            task.clone(),
            CloseRequest {
                verify: Some(summarize_checks(vec![CheckResult {
                    check_id: Some(WorkId("C-verify".to_string())),
                    command: Some("cargo test".to_string()),
                    passed: true,
                    output_ref: None,
                }])),
                force_reason: None,
                summary: "verified".to_string(),
                changed_paths: Vec::new(),
            },
        )
        .expect("verified close");
        assert_eq!(closed.task.status, TaskStatus::Done);
        assert_eq!(closed.run.outcome, RunOutcome::Done);

        let forced = close_task_with_conventions(
            task,
            CloseRequest {
                verify: None,
                force_reason: Some("manual migration override".to_string()),
                summary: "forced".to_string(),
                changed_paths: Vec::new(),
            },
        )
        .expect("forced close");
        assert_eq!(forced.run.outcome, RunOutcome::DoneWithConcerns);
        assert!(forced.forced);
    }

    #[test]
    fn fail_records_blocker_evidence_and_next_action() {
        let task = task("T-fail", TaskStatus::Active);

        let failed = fail_task_with_conventions(
            task,
            "missing migration parity",
            Some("implement outcome bridge".to_string()),
        );

        assert_eq!(failed.task.status, TaskStatus::Blocked);
        assert_eq!(
            failed.memory.parent_work,
            Some(WorkId("T-fail".to_string()))
        );
        assert!(failed.memory.text.contains("missing migration parity"));
        assert!(failed.memory.text.contains("implement outcome bridge"));
    }
}
