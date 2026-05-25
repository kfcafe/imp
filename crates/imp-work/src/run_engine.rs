use crate::model::{
    RunOutcome, Task, WorkId, WorkRun, WorkRunAssignment, WorkRunEvent, WorkRunEventKind,
    WorkRunPolicy, WorkRunStatus,
};
use crate::scheduler::{DispatchPlan, PathConflictPolicy, RunPolicy, Scheduler};
use crate::store::WorkStore;
use crate::Result;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct WorkRunPolicyInput {
    pub max_jobs: usize,
    pub path_conflicts: PathConflictPolicy,
    pub require_context: bool,
    pub keep_going: bool,
}

impl Default for WorkRunPolicyInput {
    fn default() -> Self {
        Self {
            max_jobs: 1,
            path_conflicts: PathConflictPolicy::Block,
            require_context: false,
            keep_going: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkRunPlan {
    pub root_work_id: WorkId,
    pub root_title: Option<String>,
    pub policy: WorkRunPolicyInput,
    pub dispatch: DispatchPlan,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkRunView {
    pub run: WorkRun,
    pub root_title: Option<String>,
    pub events: Vec<WorkRunEvent>,
    pub next_actions: Vec<String>,
}

pub struct WorkRunEngine<'a> {
    store: &'a WorkStore,
}

impl<'a> WorkRunEngine<'a> {
    pub fn new(store: &'a WorkStore) -> Self {
        Self { store }
    }

    pub fn plan(
        &self,
        root_work_id: WorkId,
        root_title: Option<String>,
        tasks: Vec<Task>,
        policy: WorkRunPolicyInput,
    ) -> WorkRunPlan {
        let mut scheduler = Scheduler::new();
        for task in tasks {
            scheduler.add_task(task);
        }
        let dispatch = scheduler.plan_dispatch(&RunPolicy {
            max_jobs: policy.max_jobs.max(1),
            path_conflicts: policy.path_conflicts,
        });
        WorkRunPlan {
            root_work_id,
            root_title,
            policy,
            dispatch,
        }
    }

    pub fn start_or_resume(&self, plan: &WorkRunPlan) -> Result<WorkRunView> {
        if let Some(existing) =
            self.store.load_work_runs()?.into_iter().find(|run| {
                run.root_work_id == plan.root_work_id && active_work_run_status(run.status)
            })
        {
            return self.view(existing, plan.root_title.clone());
        }

        let now = work_run_timestamp();
        let run_id = WorkId::from(format!("WR-{}", work_run_id_suffix()));
        let assignments = plan
            .dispatch
            .dispatchable
            .iter()
            .map(|work_id| WorkRunAssignment {
                work_id: work_id.clone(),
                lease_id: Some(WorkId::from(format!("L-{}-{}", run_id.0, work_id.0))),
                worker_id: None,
                status: "leased".to_string(),
            })
            .collect::<Vec<_>>();
        let blocked = plan
            .dispatch
            .blocked
            .iter()
            .map(|blocker| blocker.work_id.clone())
            .collect::<Vec<_>>();
        let status = if assignments.is_empty() && !blocked.is_empty() {
            WorkRunStatus::Blocked
        } else if assignments.is_empty() {
            WorkRunStatus::Completed
        } else {
            WorkRunStatus::Running
        };
        let run = WorkRun {
            id: run_id.clone(),
            root_work_id: plan.root_work_id.clone(),
            status,
            policy: WorkRunPolicy {
                max_jobs: plan.policy.max_jobs.max(1),
                path_conflicts: match plan.policy.path_conflicts {
                    PathConflictPolicy::Block => "block".to_string(),
                    PathConflictPolicy::Ignore => "ignore".to_string(),
                },
                require_context: plan.policy.require_context,
                keep_going: plan.policy.keep_going,
            },
            current_wave: 1,
            started_at: now.clone(),
            updated_at: now.clone(),
            assignments,
            blocked: blocked.clone(),
            summary: plan
                .root_title
                .as_ref()
                .map(|title| format!("Run for {title}")),
        };
        self.store.create_work_run(&run)?;
        self.append_event(
            &run.id,
            &now,
            WorkRunEventKind::WavePlanned {
                wave: 1,
                assigned: plan.dispatch.dispatchable.clone(),
                blocked,
            },
        )?;
        for assignment in &run.assignments {
            if let Some(lease_id) = &assignment.lease_id {
                self.append_event(
                    &run.id,
                    &now,
                    WorkRunEventKind::WorkerLeased {
                        work_id: assignment.work_id.clone(),
                        lease_id: lease_id.clone(),
                    },
                )?;
            }
        }
        if run.status == WorkRunStatus::Completed {
            self.append_event(&run.id, &now, WorkRunEventKind::RunCompleted)?;
        }
        self.view(run, plan.root_title.clone())
    }

    pub fn state(&self, run_id: &str, root_title: Option<String>) -> Result<Option<WorkRunView>> {
        self.store
            .load_work_run(run_id)?
            .map(|run| self.view(run, root_title))
            .transpose()
    }

    pub fn view(&self, run: WorkRun, root_title: Option<String>) -> Result<WorkRunView> {
        let events = self.store.load_work_run_events(&run.id)?;
        let next_actions = next_actions_for_run(&run);
        Ok(WorkRunView {
            run,
            root_title,
            events,
            next_actions,
        })
    }

    pub fn record_assignment_outcome(
        &self,
        work_id: &WorkId,
        outcome: RunOutcome,
        summary: &str,
        tasks: Vec<Task>,
    ) -> Result<Option<WorkRunView>> {
        let Some(mut run) = self.store.load_work_runs()?.into_iter().find(|run| {
            active_work_run_status(run.status)
                && run
                    .assignments
                    .iter()
                    .any(|assignment| &assignment.work_id == work_id)
        }) else {
            return Ok(None);
        };
        let now = work_run_timestamp();
        let mut found = false;
        for assignment in &mut run.assignments {
            if &assignment.work_id == work_id {
                assignment.status =
                    if matches!(outcome, RunOutcome::Done | RunOutcome::DoneWithConcerns) {
                        "completed".to_string()
                    } else {
                        "failed".to_string()
                    };
                found = true;
            }
        }
        if !found {
            return Ok(None);
        }
        run.updated_at = now.clone();
        let succeeded = matches!(outcome, RunOutcome::Done | RunOutcome::DoneWithConcerns);
        let has_failed = run
            .assignments
            .iter()
            .any(|assignment| assignment.status == "failed");
        let all_finished = run.assignments.iter().all(|assignment| {
            assignment.status == "completed"
                || assignment.status == "failed"
                || assignment.status == "cancelled"
                || assignment.status == "superseded"
        });
        if all_finished && has_failed && !run.policy.keep_going {
            run.status = WorkRunStatus::Blocked;
        } else if all_finished && succeeded {
            self.plan_next_wave(&mut run, &tasks, &now)?;
        } else if has_failed {
            run.status = WorkRunStatus::Blocked;
        }
        self.store.update_work_run(&run)?;
        self.append_event(
            &run.id,
            &now,
            WorkRunEventKind::WorkerCompleted {
                work_id: work_id.clone(),
                outcome,
            },
        )?;
        self.append_event(
            &run.id,
            &now,
            WorkRunEventKind::HandoffRecorded {
                work_id: work_id.clone(),
                summary: summary.to_string(),
            },
        )?;
        if run.status == WorkRunStatus::Completed {
            self.append_event(&run.id, &now, WorkRunEventKind::RunCompleted)?;
        }
        self.view(run, None).map(Some)
    }

    pub fn retry_assignment(&self, run_id: &str, work_id: &WorkId) -> Result<Option<WorkRunView>> {
        let Some(mut run) = self.store.load_work_run(run_id)? else {
            return Ok(None);
        };
        let now = work_run_timestamp();
        let mut found = false;
        for assignment in &mut run.assignments {
            if &assignment.work_id == work_id {
                if assignment.status != "failed" && assignment.status != "cancelled" {
                    return Ok(None);
                }
                assignment.status = "leased".to_string();
                found = true;
            }
        }
        if !found {
            return Ok(None);
        }
        if !matches!(
            run.status,
            WorkRunStatus::Cancelled | WorkRunStatus::Completed
        ) {
            run.status = WorkRunStatus::Running;
        }
        run.updated_at = now.clone();
        self.store.update_work_run(&run)?;
        self.append_event(
            &run.id,
            &now,
            WorkRunEventKind::WorkerLeased {
                work_id: work_id.clone(),
                lease_id: WorkId::from(format!("L-{}-{}", run.id.0, work_id.0)),
            },
        )?;
        self.view(run, None).map(Some)
    }

    pub fn pause(&self, run_id: &str) -> Result<Option<WorkRunView>> {
        self.transition_run(run_id, WorkRunStatus::Paused, WorkRunEventKind::RunPaused)
    }

    pub fn resume(&self, run_id: &str) -> Result<Option<WorkRunView>> {
        self.transition_run(run_id, WorkRunStatus::Running, WorkRunEventKind::RunResumed)
    }

    pub fn cancel(&self, run_id: &str, reason: Option<String>) -> Result<Option<WorkRunView>> {
        let event = WorkRunEventKind::RunFailed {
            reason: reason.unwrap_or_else(|| "cancelled".to_string()),
        };
        self.transition_run(run_id, WorkRunStatus::Cancelled, event)
    }

    fn transition_run(
        &self,
        run_id: &str,
        status: WorkRunStatus,
        event: WorkRunEventKind,
    ) -> Result<Option<WorkRunView>> {
        let Some(mut run) = self.store.load_work_run(run_id)? else {
            return Ok(None);
        };
        if matches!(
            run.status,
            WorkRunStatus::Completed | WorkRunStatus::Cancelled
        ) {
            return self.view(run, None).map(Some);
        }
        let now = work_run_timestamp();
        run.status = status;
        run.updated_at = now.clone();
        self.store.update_work_run(&run)?;
        self.append_event(&run.id, &now, event)?;
        self.view(run, None).map(Some)
    }

    fn plan_next_wave(&self, run: &mut WorkRun, tasks: &[Task], timestamp: &str) -> Result<()> {
        let mut scheduler = Scheduler::new();
        for task in tasks.iter().cloned() {
            scheduler.add_task(task);
        }
        let path_conflicts = match run.policy.path_conflicts.as_str() {
            "ignore" => PathConflictPolicy::Ignore,
            _ => PathConflictPolicy::Block,
        };
        let dispatch = scheduler.plan_dispatch(&RunPolicy {
            max_jobs: run.policy.max_jobs.max(1),
            path_conflicts,
        });
        let already_assigned = run
            .assignments
            .iter()
            .map(|assignment| assignment.work_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let new_assignments = dispatch
            .dispatchable
            .iter()
            .filter(|work_id| !already_assigned.contains(*work_id))
            .cloned()
            .collect::<Vec<_>>();
        let blocked = dispatch
            .blocked
            .iter()
            .map(|blocker| blocker.work_id.clone())
            .collect::<Vec<_>>();
        run.blocked = blocked.clone();
        if new_assignments.is_empty() {
            run.status = if blocked.is_empty() {
                WorkRunStatus::Completed
            } else {
                WorkRunStatus::Blocked
            };
            return Ok(());
        }

        run.current_wave += 1;
        run.status = WorkRunStatus::Running;
        let assigned = new_assignments.clone();
        for work_id in new_assignments {
            let lease_id = WorkId::from(format!("L-{}-{}", run.id.0, work_id.0));
            run.assignments.push(WorkRunAssignment {
                work_id: work_id.clone(),
                lease_id: Some(lease_id.clone()),
                worker_id: None,
                status: "leased".to_string(),
            });
            self.append_event(
                &run.id,
                timestamp,
                WorkRunEventKind::WorkerLeased { work_id, lease_id },
            )?;
        }
        self.append_event(
            &run.id,
            timestamp,
            WorkRunEventKind::WavePlanned {
                wave: run.current_wave,
                assigned,
                blocked,
            },
        )?;
        Ok(())
    }

    fn append_event(&self, run_id: &WorkId, timestamp: &str, kind: WorkRunEventKind) -> Result<()> {
        self.store.append_work_run_event(&WorkRunEvent {
            sequence: self.store.next_work_run_event_sequence(run_id)?,
            run_id: run_id.clone(),
            timestamp: timestamp.to_string(),
            kind,
        })?;
        Ok(())
    }
}

pub fn active_work_run_status(status: WorkRunStatus) -> bool {
    matches!(
        status,
        WorkRunStatus::Planning
            | WorkRunStatus::Running
            | WorkRunStatus::Paused
            | WorkRunStatus::Blocked
    )
}

pub fn next_actions_for_run(run: &WorkRun) -> Vec<String> {
    match run.status {
        WorkRunStatus::Running => {
            vec!["Record outcomes for assigned work, then re-check this run state.".into()]
        }
        WorkRunStatus::Blocked => {
            vec!["Inspect blockers or retry after dependencies/path conflicts are resolved.".into()]
        }
        WorkRunStatus::Completed => vec!["No action required; this run is complete.".into()],
        WorkRunStatus::Paused => vec!["Resume this run before assigning more work.".into()],
        WorkRunStatus::Failed => {
            vec!["Inspect failure events and retry or start a new run.".into()]
        }
        WorkRunStatus::Cancelled => vec!["Start a new run if this work should continue.".into()],
        WorkRunStatus::Planning => vec!["Continue planning or start the first wave.".into()],
    }
}

fn work_run_timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_secs()),
        Err(_) => "0".to_string(),
    }
}

fn work_run_id_suffix() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos()),
        Err(_) => "0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Link, LinkKind, TaskStatus};

    fn task(id: &str, title: &str) -> Task {
        let mut task = Task::new(title);
        task.id = WorkId::from(id);
        task.status = TaskStatus::Ready;
        task
    }

    fn completed_task(id: &str, title: &str) -> Task {
        let mut task = task(id, title);
        task.status = TaskStatus::Done;
        task
    }

    #[test]
    fn engine_starts_run_and_records_assignment_outcome() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let engine = WorkRunEngine::new(&store);
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![task("T-one", "One")],
            WorkRunPolicyInput::default(),
        );
        let view = engine.start_or_resume(&plan).unwrap();
        assert_eq!(view.run.status, WorkRunStatus::Running);
        assert_eq!(view.run.assignments.len(), 1);

        let updated = engine
            .record_assignment_outcome(
                &WorkId::from("T-one"),
                RunOutcome::Done,
                "done",
                vec![completed_task("T-one", "One")],
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.run.status, WorkRunStatus::Completed);
        assert!(updated
            .events
            .iter()
            .any(|event| matches!(event.kind, WorkRunEventKind::WorkerCompleted { .. })));
        assert!(updated
            .events
            .iter()
            .any(|event| matches!(event.kind, WorkRunEventKind::HandoffRecorded { .. })));
        assert!(updated
            .events
            .iter()
            .any(|event| matches!(event.kind, WorkRunEventKind::RunCompleted)));
    }

    #[test]
    fn engine_supports_pause_resume_and_cancel_transitions() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let engine = WorkRunEngine::new(&store);
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![task("T-one", "One")],
            WorkRunPolicyInput::default(),
        );
        let view = engine.start_or_resume(&plan).unwrap();
        let run_id = view.run.id.0.clone();

        let paused = engine.pause(&run_id).unwrap().unwrap();
        assert_eq!(paused.run.status, WorkRunStatus::Paused);
        assert!(paused
            .events
            .iter()
            .any(|event| matches!(event.kind, WorkRunEventKind::RunPaused)));

        let resumed = engine.resume(&run_id).unwrap().unwrap();
        assert_eq!(resumed.run.status, WorkRunStatus::Running);
        assert!(resumed
            .events
            .iter()
            .any(|event| matches!(event.kind, WorkRunEventKind::RunResumed)));

        let cancelled = engine
            .cancel(&run_id, Some("user requested".into()))
            .unwrap()
            .unwrap();
        assert_eq!(cancelled.run.status, WorkRunStatus::Cancelled);
        assert!(cancelled.events.iter().any(|event| matches!(
            &event.kind,
            WorkRunEventKind::RunFailed { reason } if reason == "user requested"
        )));
    }

    #[test]
    fn engine_retries_failed_assignment() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let engine = WorkRunEngine::new(&store);
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![task("T-one", "One")],
            WorkRunPolicyInput::default(),
        );
        let view = engine.start_or_resume(&plan).unwrap();
        let run_id = view.run.id.0.clone();
        let failed = engine
            .record_assignment_outcome(
                &WorkId::from("T-one"),
                RunOutcome::Failed,
                "failed",
                vec![task("T-one", "One")],
            )
            .unwrap()
            .unwrap();
        assert_eq!(failed.run.status, WorkRunStatus::Blocked);

        let retried = engine
            .retry_assignment(&run_id, &WorkId::from("T-one"))
            .unwrap()
            .unwrap();
        assert_eq!(retried.run.status, WorkRunStatus::Running);
        assert_eq!(retried.run.assignments[0].status, "leased".to_string());
        assert!(retried.events.iter().any(|event| matches!(
            &event.kind,
            WorkRunEventKind::WorkerLeased { work_id, .. } if work_id == &WorkId::from("T-one")
        )));
    }

    #[test]
    fn engine_does_not_retry_active_assignment() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let engine = WorkRunEngine::new(&store);
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![task("T-one", "One")],
            WorkRunPolicyInput::default(),
        );
        let view = engine.start_or_resume(&plan).unwrap();
        let run_id = view.run.id.0.clone();

        let leased_events_before_retry = view
            .events
            .iter()
            .filter(|event| matches!(
                &event.kind,
                WorkRunEventKind::WorkerLeased { work_id, .. } if work_id == &WorkId::from("T-one")
            ))
            .count();

        let retried = engine
            .retry_assignment(&run_id, &WorkId::from("T-one"))
            .unwrap();
        assert!(retried.is_none());

        let unchanged = engine.state(&run_id, None).unwrap().unwrap();
        assert_eq!(unchanged.run.status, WorkRunStatus::Running);
        assert_eq!(unchanged.run.assignments[0].status, "leased".to_string());
        let leased_events_after_retry = unchanged
            .events
            .iter()
            .filter(|event| matches!(
                &event.kind,
                WorkRunEventKind::WorkerLeased { work_id, .. } if work_id == &WorkId::from("T-one")
            ))
            .count();
        assert_eq!(leased_events_after_retry, leased_events_before_retry);
    }

    #[test]
    fn engine_advances_to_next_wave_after_outcome_unlocks_dependency() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let engine = WorkRunEngine::new(&store);
        let first = task("T-one", "One");
        let mut second = task("T-two", "Two");
        second.links.push(Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-one"),
        });
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![first, second.clone()],
            WorkRunPolicyInput::default(),
        );
        let view = engine.start_or_resume(&plan).unwrap();
        assert_eq!(view.run.assignments.len(), 1);
        assert_eq!(view.run.assignments[0].work_id, WorkId::from("T-one"));

        let updated = engine
            .record_assignment_outcome(
                &WorkId::from("T-one"),
                RunOutcome::Done,
                "done",
                vec![completed_task("T-one", "One"), second],
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.run.status, WorkRunStatus::Running);
        assert_eq!(updated.run.current_wave, 2);
        assert!(updated.run.assignments.iter().any(|assignment| {
            assignment.work_id == WorkId::from("T-two") && assignment.status == "leased".to_string()
        }));
        assert!(updated
            .events
            .iter()
            .any(|event| matches!(event.kind, WorkRunEventKind::WavePlanned { wave: 2, .. })));
    }

    #[test]
    fn engine_blocks_dependent_task_until_dependency_done() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let engine = WorkRunEngine::new(&store);
        let first = task("T-one", "One");
        let mut second = task("T-two", "Two");
        second.links.push(Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-one"),
        });
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![first, second],
            WorkRunPolicyInput::default(),
        );
        assert_eq!(plan.dispatch.dispatchable, vec![WorkId::from("T-one")]);
        assert_eq!(plan.dispatch.blocked.len(), 0);

        let mut first = task("T-one", "One");
        first.status = TaskStatus::Done;
        let mut second = task("T-two", "Two");
        second.links.push(Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-one"),
        });
        let plan = engine.plan(
            WorkId::from("E-root"),
            Some("Root work".into()),
            vec![first, second],
            WorkRunPolicyInput::default(),
        );
        assert_eq!(plan.dispatch.dispatchable, vec![WorkId::from("T-two")]);
    }
}
