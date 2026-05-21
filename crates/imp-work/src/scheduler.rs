use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Lease, LinkKind, Run, RunOutcome, Task, TaskStatus, WorkId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerProfile {
    pub name: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub can_write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseRequest {
    pub worker_id: String,
    pub profile: WorkerProfile,
    pub preferred_work_id: Option<WorkId>,
    pub path_locks: Vec<PathBuf>,
    pub worktree: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseRecord {
    pub lease: Lease,
    pub profile: WorkerProfile,
    pub heartbeat: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkOutcome {
    pub work_id: WorkId,
    pub outcome: RunOutcome,
    pub summary: String,
    pub changed_paths: Vec<PathBuf>,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub memory_updates: Vec<String>,
    pub followups: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CoordinatorSummary {
    pub ready: usize,
    pub leased: usize,
    pub done: usize,
    pub blocked: usize,
    pub failed: usize,
    pub needs_context: usize,
    pub path_conflicts: usize,
    pub recent_outcomes: Vec<WorkOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunPolicy {
    pub max_jobs: usize,
    pub path_conflicts: PathConflictPolicy,
}

impl Default for RunPolicy {
    fn default() -> Self {
        Self {
            max_jobs: 1,
            path_conflicts: PathConflictPolicy::Block,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathConflictPolicy {
    Block,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchPlan {
    pub dispatchable: Vec<WorkId>,
    pub blocked: Vec<DispatchBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchBlocker {
    pub work_id: WorkId,
    pub reason: DispatchBlockerReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DispatchBlockerReason {
    MaxJobsReached,
    PathConflict { path: PathBuf, held_by: WorkId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiAgentRunPlan {
    pub policy: RunPolicy,
    pub leases: Vec<LeaseRecord>,
    pub blocked: Vec<DispatchBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiAgentRunResult {
    pub runs: Vec<Run>,
    pub summary: CoordinatorSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCompletion {
    pub lease_id: WorkId,
    pub outcome: WorkOutcome,
}

#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    tasks: BTreeMap<String, Task>,
    leases: BTreeMap<String, LeaseRecord>,
    runs: Vec<Run>,
    outcomes: Vec<WorkOutcome>,
    path_locks: BTreeMap<PathBuf, String>,
    tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchedulerError {
    #[error("no ready work available")]
    NoReadyWork,
    #[error("work item `{0}` is not ready")]
    WorkNotReady(String),
    #[error("path lock conflict on `{path}` held by `{held_by}`")]
    PathConflict { path: String, held_by: String },
    #[error("lease `{0}` not found")]
    LeaseNotFound(String),
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.id.0.clone(), task);
    }

    pub fn ready_queue(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|task| task.status == TaskStatus::Ready)
            .filter(|task| self.dependencies_done(task))
            .filter(|task| !self.is_leased(&task.id))
            .collect()
    }

    pub fn plan_dispatch(&self, policy: &RunPolicy) -> DispatchPlan {
        let max_jobs = policy.max_jobs.max(1);
        let mut dispatchable = Vec::new();
        let mut blocked = Vec::new();
        let mut planned_paths: BTreeMap<PathBuf, WorkId> = BTreeMap::new();

        for task in self.ready_queue() {
            if dispatchable.len() >= max_jobs {
                blocked.push(DispatchBlocker {
                    work_id: task.id.clone(),
                    reason: DispatchBlockerReason::MaxJobsReached,
                });
                continue;
            }

            if policy.path_conflicts == PathConflictPolicy::Block {
                if let Some((path, held_by)) = first_planned_path_conflict(task, &planned_paths) {
                    blocked.push(DispatchBlocker {
                        work_id: task.id.clone(),
                        reason: DispatchBlockerReason::PathConflict { path, held_by },
                    });
                    continue;
                }
            }

            for path in task_paths(task) {
                planned_paths.insert(path, task.id.clone());
            }
            dispatchable.push(task.id.clone());
        }

        DispatchPlan {
            dispatchable,
            blocked,
        }
    }

    pub fn lease_ready(&mut self, request: LeaseRequest) -> Result<LeaseRecord, SchedulerError> {
        let task_id = if let Some(preferred) = request.preferred_work_id.clone() {
            let task = self
                .tasks
                .get(&preferred.0)
                .ok_or_else(|| SchedulerError::WorkNotReady(preferred.0.clone()))?;
            if task.status != TaskStatus::Ready
                || !self.dependencies_done(task)
                || self.is_leased(&task.id)
            {
                return Err(SchedulerError::WorkNotReady(preferred.0));
            }
            preferred
        } else {
            self.ready_queue()
                .first()
                .map(|task| task.id.clone())
                .ok_or(SchedulerError::NoReadyWork)?
        };

        for path in &request.path_locks {
            if let Some(held_by) = self.conflicting_lock(path) {
                return Err(SchedulerError::PathConflict {
                    path: path.display().to_string(),
                    held_by,
                });
            }
        }

        let lease = Lease {
            id: WorkId::new("L"),
            work_id: task_id.clone(),
            worker_id: request.worker_id.clone(),
            worktree: request.worktree.clone(),
            path_locks: request.path_locks.clone(),
        };
        for path in &lease.path_locks {
            self.path_locks.insert(path.clone(), lease.id.0.clone());
        }
        if let Some(task) = self.tasks.get_mut(&task_id.0) {
            task.status = TaskStatus::Active;
        }
        self.tick += 1;
        let record = LeaseRecord {
            lease,
            profile: request.profile,
            heartbeat: self.tick,
        };
        self.leases
            .insert(record.lease.id.0.clone(), record.clone());
        Ok(record)
    }

    pub fn heartbeat(&mut self, lease_id: &str) -> Result<(), SchedulerError> {
        let record = self
            .leases
            .get_mut(lease_id)
            .ok_or_else(|| SchedulerError::LeaseNotFound(lease_id.into()))?;
        self.tick += 1;
        record.heartbeat = self.tick;
        Ok(())
    }

    pub fn complete(
        &mut self,
        lease_id: &str,
        outcome: WorkOutcome,
    ) -> Result<Run, SchedulerError> {
        let lease = self
            .leases
            .remove(lease_id)
            .ok_or_else(|| SchedulerError::LeaseNotFound(lease_id.into()))?;
        for path in &lease.lease.path_locks {
            self.path_locks.remove(path);
        }
        if let Some(task) = self.tasks.get_mut(&outcome.work_id.0) {
            task.status = match outcome.outcome {
                RunOutcome::Done | RunOutcome::DoneWithConcerns => TaskStatus::Done,
                RunOutcome::Blocked => TaskStatus::Blocked,
                RunOutcome::NeedsContext | RunOutcome::Failed => TaskStatus::Review,
            };
        }
        let run = Run {
            id: WorkId::new("R"),
            work_id: Some(outcome.work_id.clone()),
            context_pack_id: None,
            outcome: outcome.outcome,
            summary: outcome.summary.clone(),
            changed_paths: outcome.changed_paths.clone(),
            checks: Vec::new(),
        };
        self.runs.push(run.clone());
        self.outcomes.push(outcome);
        Ok(run)
    }

    pub fn summary(&self) -> CoordinatorSummary {
        let mut summary = CoordinatorSummary {
            ready: self.ready_queue().len(),
            leased: self.leases.len(),
            path_conflicts: self.path_locks.len(),
            ..CoordinatorSummary::default()
        };
        for task in self.tasks.values() {
            match task.status {
                TaskStatus::Done => summary.done += 1,
                TaskStatus::Blocked => summary.blocked += 1,
                TaskStatus::Review => summary.needs_context += 1,
                _ => {}
            }
        }
        for outcome in &self.outcomes {
            if outcome.outcome == RunOutcome::Failed {
                summary.failed += 1;
            }
        }
        summary.recent_outcomes = self
            .outcomes
            .iter()
            .rev()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        summary
    }

    pub fn plan_multi_agent_run(
        &mut self,
        policy: RunPolicy,
        profile: WorkerProfile,
    ) -> Result<MultiAgentRunPlan, SchedulerError> {
        let dispatch = self.plan_dispatch(&policy);
        let mut leases = Vec::new();
        for work_id in &dispatch.dispatchable {
            let task = self
                .tasks
                .get(&work_id.0)
                .ok_or_else(|| SchedulerError::WorkNotReady(work_id.0.clone()))?;
            let path_locks = task_paths(task);
            leases.push(self.lease_ready(LeaseRequest {
                worker_id: format!("worker-{}", leases.len() + 1),
                profile: profile.clone(),
                preferred_work_id: Some(work_id.clone()),
                path_locks,
                worktree: None,
            })?);
        }
        Ok(MultiAgentRunPlan {
            policy,
            leases,
            blocked: dispatch.blocked,
        })
    }

    pub fn complete_multi_agent_run(
        &mut self,
        completions: Vec<WorkerCompletion>,
        keep_going: bool,
    ) -> Result<MultiAgentRunResult, SchedulerError> {
        let mut runs = Vec::new();
        for completion in completions {
            let failed = matches!(
                completion.outcome.outcome,
                RunOutcome::Failed | RunOutcome::Blocked
            );
            let lease_id = completion.lease_id.0.clone();
            runs.push(self.complete(&lease_id, completion.outcome)?);
            if failed && !keep_going {
                break;
            }
        }
        Ok(MultiAgentRunResult {
            runs,
            summary: self.summary(),
        })
    }

    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    fn dependencies_done(&self, task: &Task) -> bool {
        task.links
            .iter()
            .filter(|link| link.kind == LinkKind::DependsOn)
            .all(|link| {
                self.tasks
                    .get(&link.target.0)
                    .is_some_and(|dependency| dependency.status == TaskStatus::Done)
            })
    }

    fn is_leased(&self, work_id: &WorkId) -> bool {
        self.leases
            .values()
            .any(|record| record.lease.work_id == *work_id)
    }

    fn conflicting_lock(&self, path: &Path) -> Option<String> {
        self.path_locks
            .iter()
            .find_map(|(locked, lease_id)| paths_conflict(locked, path).then(|| lease_id.clone()))
    }
}

fn task_paths(task: &Task) -> Vec<PathBuf> {
    task.source_refs
        .iter()
        .filter(|source| source.kind == crate::model::SourceKind::FileRange)
        .map(|source| PathBuf::from(&source.reference))
        .collect()
}

fn first_planned_path_conflict(
    task: &Task,
    planned_paths: &BTreeMap<PathBuf, WorkId>,
) -> Option<(PathBuf, WorkId)> {
    task_paths(task).into_iter().find_map(|candidate| {
        planned_paths.iter().find_map(|(planned, work_id)| {
            paths_conflict(planned, &candidate).then(|| (candidate.clone(), work_id.clone()))
        })
    })
}

fn paths_conflict(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

impl WorkerProfile {
    pub fn researcher() -> Self {
        Self {
            name: "researcher".into(),
            model: None,
            tools: vec!["read".into(), "scan".into(), "web".into()],
            can_write: false,
        }
    }

    pub fn implementer() -> Self {
        Self {
            name: "implementer".into(),
            model: None,
            tools: vec![
                "read".into(),
                "edit".into(),
                "bash".into(),
                "prototype".into(),
            ],
            can_write: true,
        }
    }
}

impl WorkOutcome {
    pub fn done(work_id: WorkId, summary: impl Into<String>) -> Self {
        Self {
            work_id,
            outcome: RunOutcome::Done,
            summary: summary.into(),
            changed_paths: Vec::new(),
            checks_passed: 0,
            checks_failed: 0,
            memory_updates: Vec::new(),
            followups: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Link;

    fn ready_task(id: &str, title: &str) -> Task {
        let mut task = Task::new(title);
        task.id = WorkId::from(id);
        task.status = TaskStatus::Ready;
        task
    }

    #[test]
    fn scheduler_leases_ready_task_and_records_heartbeat() {
        let mut scheduler = Scheduler::new();
        scheduler.add_task(ready_task("T-one", "Build worker loop"));

        let lease = scheduler
            .lease_ready(LeaseRequest {
                worker_id: "worker-1".into(),
                profile: WorkerProfile::implementer(),
                preferred_work_id: None,
                path_locks: vec![PathBuf::from("crates/imp-work/src/scheduler.rs")],
                worktree: None,
            })
            .unwrap();

        assert_eq!(lease.lease.work_id, WorkId::from("T-one"));
        scheduler.heartbeat(&lease.lease.id.0).unwrap();
        assert_eq!(scheduler.summary().leased, 1);
    }

    #[test]
    fn scheduler_rejects_conflicting_path_locks() {
        let mut scheduler = Scheduler::new();
        scheduler.add_task(ready_task("T-one", "First"));
        scheduler.add_task(ready_task("T-two", "Second"));
        scheduler
            .lease_ready(LeaseRequest {
                worker_id: "worker-1".into(),
                profile: WorkerProfile::implementer(),
                preferred_work_id: Some(WorkId::from("T-one")),
                path_locks: vec![PathBuf::from("crates/imp-work/src")],
                worktree: None,
            })
            .unwrap();

        let error = scheduler
            .lease_ready(LeaseRequest {
                worker_id: "worker-2".into(),
                profile: WorkerProfile::implementer(),
                preferred_work_id: Some(WorkId::from("T-two")),
                path_locks: vec![PathBuf::from("crates/imp-work/src/scheduler.rs")],
                worktree: None,
            })
            .unwrap_err();

        assert!(matches!(error, SchedulerError::PathConflict { .. }));
    }

    #[test]
    fn scheduler_excludes_ready_tasks_with_unfinished_dependencies() {
        let mut scheduler = Scheduler::new();
        let dependency = ready_task("T-dependency", "Dependency");
        let mut dependent = ready_task("T-dependent", "Dependent");
        dependent.links.push(crate::model::Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-dependency"),
        });
        scheduler.add_task(dependency);
        scheduler.add_task(dependent);

        let ready = scheduler.ready_queue();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, WorkId::from("T-dependency"));
        let error = scheduler
            .lease_ready(LeaseRequest {
                worker_id: "worker-dependent".into(),
                profile: WorkerProfile::implementer(),
                preferred_work_id: Some(WorkId::from("T-dependent")),
                path_locks: vec![],
                worktree: None,
            })
            .unwrap_err();
        assert!(matches!(error, SchedulerError::WorkNotReady(id) if id == "T-dependent"));
    }

    #[test]
    fn scheduler_includes_ready_tasks_after_dependencies_done() {
        let mut scheduler = Scheduler::new();
        let mut dependency = ready_task("T-dependency", "Dependency");
        dependency.status = TaskStatus::Done;
        let mut dependent = ready_task("T-dependent", "Dependent");
        dependent.links.push(crate::model::Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-dependency"),
        });
        scheduler.add_task(dependency);
        scheduler.add_task(dependent);

        let ready = scheduler.ready_queue();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, WorkId::from("T-dependent"));
    }

    #[test]
    fn multi_agent_plan_leases_bounded_ready_work_and_blocks_rest() {
        let mut scheduler = Scheduler::new();
        scheduler.add_task(ready_task("T-one", "One"));
        scheduler.add_task(ready_task("T-two", "Two"));
        scheduler.add_task(ready_task("T-three", "Three"));

        let plan = scheduler
            .plan_multi_agent_run(
                RunPolicy {
                    max_jobs: 2,
                    path_conflicts: PathConflictPolicy::Block,
                },
                WorkerProfile::implementer(),
            )
            .unwrap();

        assert_eq!(plan.leases.len(), 2);
        assert_eq!(plan.blocked.len(), 1);
        assert_eq!(scheduler.summary().leased, 2);
        assert!(matches!(
            plan.blocked[0].reason,
            DispatchBlockerReason::MaxJobsReached
        ));
    }

    #[test]
    fn multi_agent_completion_aggregates_runs_deterministically() {
        let mut scheduler = Scheduler::new();
        scheduler.add_task(ready_task("T-one", "One"));
        scheduler.add_task(ready_task("T-two", "Two"));
        let plan = scheduler
            .plan_multi_agent_run(
                RunPolicy {
                    max_jobs: 2,
                    path_conflicts: PathConflictPolicy::Block,
                },
                WorkerProfile::implementer(),
            )
            .unwrap();

        let completions = plan
            .leases
            .iter()
            .map(|lease| WorkerCompletion {
                lease_id: lease.lease.id.clone(),
                outcome: WorkOutcome::done(
                    lease.lease.work_id.clone(),
                    format!("{} complete", lease.lease.work_id),
                ),
            })
            .collect();
        let result = scheduler
            .complete_multi_agent_run(completions, true)
            .unwrap();

        assert_eq!(result.runs.len(), 2);
        assert_eq!(result.summary.done, 2);
        assert_eq!(result.summary.leased, 0);
        assert_eq!(scheduler.runs().len(), 2);
    }

    #[test]
    fn multi_agent_run_respects_dependency_waves() {
        let mut scheduler = Scheduler::new();
        let dependency = ready_task("T-dep", "Dependency");
        let mut dependent = ready_task("T-next", "Dependent");
        dependent.links.push(Link {
            kind: LinkKind::DependsOn,
            target: dependency.id.clone(),
        });
        scheduler.add_task(dependency.clone());
        scheduler.add_task(dependent);

        let first_wave = scheduler
            .plan_multi_agent_run(RunPolicy::default(), WorkerProfile::implementer())
            .unwrap();
        assert_eq!(first_wave.leases.len(), 1);
        assert_eq!(first_wave.leases[0].lease.work_id, dependency.id);

        let lease = first_wave.leases[0].lease.clone();
        scheduler
            .complete_multi_agent_run(
                vec![WorkerCompletion {
                    lease_id: lease.id,
                    outcome: WorkOutcome::done(lease.work_id, "dependency complete"),
                }],
                true,
            )
            .unwrap();
        let second_wave = scheduler
            .plan_multi_agent_run(RunPolicy::default(), WorkerProfile::implementer())
            .unwrap();
        assert_eq!(second_wave.leases.len(), 1);
        assert_eq!(second_wave.leases[0].lease.work_id, WorkId::from("T-next"));
    }

    #[test]
    fn scheduler_policy_caps_dispatch_jobs() {
        let mut scheduler = Scheduler::new();
        scheduler.add_task(ready_task("T-one", "One"));
        scheduler.add_task(ready_task("T-two", "Two"));

        let plan = scheduler.plan_dispatch(&RunPolicy {
            max_jobs: 1,
            path_conflicts: PathConflictPolicy::Block,
        });

        assert_eq!(plan.dispatchable.len(), 1);
        assert_eq!(plan.blocked.len(), 1);
        assert!(matches!(
            plan.blocked[0].reason,
            DispatchBlockerReason::MaxJobsReached
        ));
    }

    #[test]
    fn scheduler_policy_blocks_conflicting_ready_paths() {
        let mut scheduler = Scheduler::new();
        let mut first = ready_task("T-one", "One");
        first.source_refs.push(crate::model::SourceRef {
            kind: crate::model::SourceKind::FileRange,
            reference: "crates/imp-work/src".to_string(),
            fingerprint: None,
        });
        let mut second = ready_task("T-two", "Two");
        second.source_refs.push(crate::model::SourceRef {
            kind: crate::model::SourceKind::FileRange,
            reference: "crates/imp-work/src/scheduler.rs".to_string(),
            fingerprint: None,
        });
        scheduler.add_task(first);
        scheduler.add_task(second);

        let plan = scheduler.plan_dispatch(&RunPolicy {
            max_jobs: 4,
            path_conflicts: PathConflictPolicy::Block,
        });

        assert_eq!(plan.dispatchable.len(), 1);
        assert_eq!(plan.blocked.len(), 1);
        assert!(matches!(
            &plan.blocked[0].reason,
            DispatchBlockerReason::PathConflict { path, held_by }
                if path == &PathBuf::from("crates/imp-work/src/scheduler.rs")
                    && held_by == &WorkId::from("T-one")
        ));
    }

    #[test]
    fn scheduler_completes_lease_and_summarizes_outcome() {
        let mut scheduler = Scheduler::new();
        scheduler.add_task(ready_task("T-one", "Build summaries"));
        let lease = scheduler
            .lease_ready(LeaseRequest {
                worker_id: "worker-1".into(),
                profile: WorkerProfile::implementer(),
                preferred_work_id: Some(WorkId::from("T-one")),
                path_locks: vec![],
                worktree: None,
            })
            .unwrap();

        let run = scheduler
            .complete(
                &lease.lease.id.0,
                WorkOutcome {
                    changed_paths: vec![PathBuf::from("crates/imp-work/src/scheduler.rs")],
                    checks_passed: 2,
                    ..WorkOutcome::done(WorkId::from("T-one"), "Scheduler summary works")
                },
            )
            .unwrap();
        let summary = scheduler.summary();

        assert_eq!(run.outcome, RunOutcome::Done);
        assert_eq!(summary.leased, 0);
        assert_eq!(summary.done, 1);
        assert_eq!(summary.recent_outcomes.len(), 1);
        assert_eq!(scheduler.runs().len(), 1);
    }
}
