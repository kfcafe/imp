use crate::context_pack::{ContextRenderer, RenderedContextPack};
use crate::model::{ContextPack, Run, Task, WorkId};
use crate::scheduler::{LeaseRequest, Scheduler, SchedulerError, WorkOutcome, WorkerProfile};
use crate::store::{WorkStore, WorkerPersistence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWorkerRequest {
    pub worker_id: String,
    pub profile: WorkerProfile,
    pub task: Task,
    pub context_pack: ContextPack,
    pub path_locks: Vec<std::path::PathBuf>,
    pub worktree: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWorkerLaunch {
    pub worker_id: String,
    pub work_id: WorkId,
    pub lease_id: WorkId,
    pub profile: WorkerProfile,
    pub context: RenderedContextPack,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWorkerResult {
    pub run: Run,
    pub outcome: WorkOutcome,
    pub coordinator_summary: crate::scheduler::CoordinatorSummary,
    pub persistence: Option<WorkerPersistence>,
}

#[derive(Debug, thiserror::Error)]
pub enum PreparedWorkerError {
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error("context pack `{pack_id}` belongs to `{pack_work_id}`, not task `{task_id}`")]
    ContextTaskMismatch {
        pack_id: String,
        pack_work_id: String,
        task_id: String,
    },
    #[error("context pack `{0}` is stale")]
    StaleContext(String),
    #[error("failed to persist worker result: {0}")]
    Persist(#[from] crate::Error),
    #[error("worker failed: {0}")]
    Worker(String),
}

pub struct PreparedWorkerLoop {
    scheduler: Scheduler,
}

impl PreparedWorkerLoop {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
        }
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    pub fn run_once<F>(
        &mut self,
        request: PreparedWorkerRequest,
        worker: F,
    ) -> Result<PreparedWorkerResult, PreparedWorkerError>
    where
        F: FnOnce(PreparedWorkerLaunch) -> Result<WorkOutcome, String>,
    {
        self.run_once_inner(request, None, worker)
    }

    pub fn run_once_persisted<F>(
        &mut self,
        request: PreparedWorkerRequest,
        store: &WorkStore,
        worker: F,
    ) -> Result<PreparedWorkerResult, PreparedWorkerError>
    where
        F: FnOnce(PreparedWorkerLaunch) -> Result<WorkOutcome, String>,
    {
        self.run_once_inner(request, Some(store), worker)
    }

    fn run_once_inner<F>(
        &mut self,
        request: PreparedWorkerRequest,
        store: Option<&WorkStore>,
        worker: F,
    ) -> Result<PreparedWorkerResult, PreparedWorkerError>
    where
        F: FnOnce(PreparedWorkerLaunch) -> Result<WorkOutcome, String>,
    {
        if request.context_pack.work_id != request.task.id {
            return Err(PreparedWorkerError::ContextTaskMismatch {
                pack_id: request.context_pack.id.0,
                pack_work_id: request.context_pack.work_id.0,
                task_id: request.task.id.0,
            });
        }
        let rendered = ContextRenderer::render(&request.context_pack);
        if rendered.stale {
            return Err(PreparedWorkerError::StaleContext(request.context_pack.id.0));
        }

        self.scheduler.add_task(request.task.clone());
        let lease = self.scheduler.lease_ready(LeaseRequest {
            worker_id: request.worker_id.clone(),
            profile: request.profile.clone(),
            preferred_work_id: Some(request.task.id.clone()),
            path_locks: request.path_locks,
            worktree: request.worktree,
        })?;
        let prompt = render_launch_prompt(&request.task, &request.context_pack, &rendered);
        let launch = PreparedWorkerLaunch {
            worker_id: request.worker_id,
            work_id: request.task.id.clone(),
            lease_id: lease.lease.id.clone(),
            profile: request.profile,
            context: rendered,
            prompt,
        };
        let outcome = worker(launch).map_err(PreparedWorkerError::Worker)?;
        let run = self
            .scheduler
            .complete(&lease.lease.id.0, outcome.clone())?;
        let coordinator_summary = self.scheduler.summary();
        let persistence = store
            .map(|store| store.persist_worker_result(&run, &outcome, &coordinator_summary))
            .transpose()?;
        Ok(PreparedWorkerResult {
            run,
            outcome,
            coordinator_summary,
            persistence,
        })
    }
}

impl Default for PreparedWorkerLoop {
    fn default() -> Self {
        Self::new()
    }
}

fn render_launch_prompt(task: &Task, pack: &ContextPack, rendered: &RenderedContextPack) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "You are a prepared imp worker. Execute the task using the provided context pack.\n\n",
    );
    prompt.push_str(&format!("task_id: {}\n", task.id));
    prompt.push_str(&format!("task: {}\n", task.title));
    prompt.push_str(&format!("context_pack: {}\n", pack.id));
    prompt.push_str(&format!(
        "stable_prefix_hash: {}\n\n",
        rendered.stable_prefix_hash
    ));
    prompt.push_str("Return only a structured outcome: status, summary, changed_paths, checks, memory_updates, followups. Do not return a transcript unless asked.\n");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_pack::ContextCompiler;
    use crate::model::{RunOutcome, TaskStatus};

    fn ready_task() -> Task {
        let mut task = Task::new("Implement prepared worker loop");
        task.id = WorkId::from("T-prepared-worker");
        task.status = TaskStatus::Ready;
        task.acceptance
            .push("worker receives rendered context pack".into());
        task
    }

    #[test]
    fn prepared_worker_leases_task_launches_context_and_reconciles_outcome() {
        let task = ready_task();
        let context_pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let mut worker_loop = PreparedWorkerLoop::new();

        let result = worker_loop
            .run_once(
                PreparedWorkerRequest {
                    worker_id: "worker-1".into(),
                    profile: WorkerProfile::implementer(),
                    task: task.clone(),
                    context_pack,
                    path_locks: vec!["crates/imp-work/src/prepared_worker.rs".into()],
                    worktree: None,
                },
                |launch| {
                    assert_eq!(launch.work_id, WorkId::from("T-prepared-worker"));
                    assert!(launch.prompt.contains("stable_prefix_hash"));
                    assert!(!launch.context.blocks.is_empty());
                    Ok(WorkOutcome {
                        work_id: launch.work_id,
                        outcome: RunOutcome::Done,
                        summary: "Prepared worker loop reconciled outcome.".into(),
                        changed_paths: vec!["crates/imp-work/src/prepared_worker.rs".into()],
                        checks_passed: 1,
                        checks_failed: 0,
                        memory_updates: vec!["Prepared workers return compact outcomes.".into()],
                        followups: vec![],
                    })
                },
            )
            .unwrap();

        assert_eq!(result.run.outcome, RunOutcome::Done);
        assert_eq!(result.coordinator_summary.done, 1);
        assert_eq!(result.coordinator_summary.leased, 0);
        assert!(result.persistence.is_none());
        assert_eq!(worker_loop.scheduler().runs().len(), 1);
    }

    #[test]
    fn prepared_worker_can_persist_run_outcome_summary_and_memory_updates() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let task = ready_task();
        let context_pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let mut worker_loop = PreparedWorkerLoop::new();

        let result = worker_loop
            .run_once_persisted(
                PreparedWorkerRequest {
                    worker_id: "worker-1".into(),
                    profile: WorkerProfile::implementer(),
                    task,
                    context_pack,
                    path_locks: vec![],
                    worktree: None,
                },
                &store,
                |launch| {
                    Ok(WorkOutcome {
                        work_id: launch.work_id,
                        outcome: RunOutcome::DoneWithConcerns,
                        summary: "Persisted prepared worker result.".into(),
                        changed_paths: vec!["crates/imp-work/src/prepared_worker.rs".into()],
                        checks_passed: 1,
                        checks_failed: 0,
                        memory_updates: vec![
                            "Prepared worker outcomes are persisted through WorkStore.".into(),
                        ],
                        followups: vec!["Load persisted outcomes into coordinator summary.".into()],
                    })
                },
            )
            .unwrap();

        let persistence = result.persistence.unwrap();
        assert!(persistence.run_path.exists());
        assert!(persistence.outcome_path.exists());
        assert!(persistence.summary_path.exists());
        assert_eq!(persistence.memory_paths.len(), 1);
        let memory = std::fs::read_to_string(&persistence.memory_paths[0]).unwrap();
        assert!(memory.contains("Prepared worker outcomes are persisted"));
        let summary = std::fs::read_to_string(&persistence.summary_path).unwrap();
        assert!(summary.contains("leased"));
    }

    #[test]
    fn prepared_worker_rejects_stale_context() {
        let task = ready_task();
        let mut context_pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        context_pack.status = crate::model::ContextPackStatus::Stale;
        let mut worker_loop = PreparedWorkerLoop::new();

        let error = worker_loop
            .run_once(
                PreparedWorkerRequest {
                    worker_id: "worker-1".into(),
                    profile: WorkerProfile::implementer(),
                    task,
                    context_pack,
                    path_locks: vec![],
                    worktree: None,
                },
                |_| unreachable!("stale context should not launch"),
            )
            .unwrap_err();

        assert!(matches!(error, PreparedWorkerError::StaleContext(_)));
    }
}
