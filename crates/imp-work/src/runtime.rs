use crate::prepared_prototype::{
    PreparedPrototypeError, PreparedPrototypeLaunch, PreparedPrototypeLoop,
    PreparedPrototypeRequest, PreparedPrototypeResult,
};
use crate::prepared_worker::{
    PreparedWorkerError, PreparedWorkerLaunch, PreparedWorkerLoop, PreparedWorkerRequest,
    PreparedWorkerResult,
};
use crate::prototype::PrototypeObservation;
use crate::scheduler::WorkOutcome;
use crate::store::WorkStore;

/// Runtime seam for executing a prepared task launch.
///
/// imp-work owns prepared context, leases, persistence, and structured reconciliation.
/// imp-core can implement this trait with a real subagent/runtime without exposing
/// provider transcripts or chat state to imp-work.
pub trait TaskExecutor {
    fn execute_task(&mut self, launch: PreparedWorkerLaunch) -> Result<WorkOutcome, String>;
}

impl<F> TaskExecutor for F
where
    F: FnMut(PreparedWorkerLaunch) -> Result<WorkOutcome, String>,
{
    fn execute_task(&mut self, launch: PreparedWorkerLaunch) -> Result<WorkOutcome, String> {
        self(launch)
    }
}

/// Runtime seam for executing a prepared prototype launch.
pub trait PrototypeExecutor {
    fn execute_prototype(
        &mut self,
        launch: PreparedPrototypeLaunch,
    ) -> Result<PrototypeObservation, String>;
}

impl<F> PrototypeExecutor for F
where
    F: FnMut(PreparedPrototypeLaunch) -> Result<PrototypeObservation, String>,
{
    fn execute_prototype(
        &mut self,
        launch: PreparedPrototypeLaunch,
    ) -> Result<PrototypeObservation, String> {
        self(launch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeExecutionResult {
    Task(PreparedWorkerResult),
    Prototype(PreparedPrototypeResult),
}

pub struct WorkRuntime {
    worker_loop: PreparedWorkerLoop,
}

impl WorkRuntime {
    pub fn new() -> Self {
        Self {
            worker_loop: PreparedWorkerLoop::new(),
        }
    }

    pub fn worker_loop(&self) -> &PreparedWorkerLoop {
        &self.worker_loop
    }

    pub fn worker_loop_mut(&mut self) -> &mut PreparedWorkerLoop {
        &mut self.worker_loop
    }

    pub fn run_task<E>(
        &mut self,
        request: PreparedWorkerRequest,
        store: Option<&WorkStore>,
        executor: &mut E,
    ) -> Result<PreparedWorkerResult, PreparedWorkerError>
    where
        E: TaskExecutor,
    {
        if let Some(store) = store {
            self.worker_loop
                .run_once_persisted(request, store, |launch| executor.execute_task(launch))
        } else {
            self.worker_loop
                .run_once(request, |launch| executor.execute_task(launch))
        }
    }

    pub fn run_prototype<E>(
        &mut self,
        request: PreparedPrototypeRequest,
        store: &WorkStore,
        executor: &mut E,
    ) -> Result<PreparedPrototypeResult, PreparedPrototypeError>
    where
        E: PrototypeExecutor,
    {
        PreparedPrototypeLoop::run_once(request, store, |launch| executor.execute_prototype(launch))
    }
}

impl Default for WorkRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_pack::ContextCompiler;
    use crate::model::{RunOutcome, Task, TaskStatus, WorkId};
    use crate::prepared_prototype::compile_prepared_prototype_context;
    use crate::prototype::{
        HypothesisResult, Prototype, PrototypeEvidence, PrototypeOutcome, PrototypeRecordPolicy,
    };
    use crate::scheduler::WorkerProfile;

    #[test]
    fn runtime_runs_task_executor_without_transcripts() {
        let mut task = Task::new("Execute through runtime seam");
        task.id = WorkId::from("T-runtime-seam");
        task.status = TaskStatus::Ready;
        let context_pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let mut runtime = WorkRuntime::new();
        let mut executor = |launch: PreparedWorkerLaunch| {
            assert_eq!(launch.work_id, WorkId::from("T-runtime-seam"));
            assert!(launch.prompt.contains("Return only a structured outcome"));
            Ok(WorkOutcome::done(
                launch.work_id,
                "Runtime seam produced outcome.",
            ))
        };

        let result = runtime
            .run_task(
                PreparedWorkerRequest {
                    worker_id: "worker-1".into(),
                    profile: WorkerProfile::implementer(),
                    task,
                    context_pack,
                    path_locks: Vec::new(),
                    worktree: None,
                },
                None,
                &mut executor,
            )
            .unwrap();

        assert_eq!(result.run.outcome, RunOutcome::Done);
        assert_eq!(result.coordinator_summary.done, 1);
    }

    #[test]
    fn runtime_runs_prototype_executor_and_records_observation() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let prototype = Prototype::new(
            "Prototype runtime seam",
            "Can prototype executors return structured observations?",
            tmp.path().join("sandbox"),
        )
        .with_hypothesis("The seam can stay transcript-free.");
        let context_pack = compile_prepared_prototype_context(&prototype);
        let mut runtime = WorkRuntime::new();
        let prototype_id = prototype.id.clone();
        let prototype_question = prototype.question.clone();
        let prototype_hypothesis = prototype.hypothesis.clone();
        let prototype_evidence_required = prototype.evidence_required.clone();
        let mut executor = |launch: PreparedPrototypeLaunch| {
            assert_eq!(launch.prototype_id, prototype_id);
            Ok(PrototypeObservation {
                prototype_id: launch.prototype_id,
                question: prototype_question.clone(),
                parent_work: None,
                hypothesis: prototype_hypothesis.clone(),
                hypothesis_result: HypothesisResult::Supported,
                outcome: PrototypeOutcome::Promote,
                summary: "Runtime seam recorded prototype observation.".into(),
                evidence_required: prototype_evidence_required.clone(),
                evidence: vec![PrototypeEvidence {
                    claim: "structured observation".into(),
                    proof: "executor returned PrototypeObservation".into(),
                    artifact: None,
                }],
                learnings: vec!["Prototype executors do not need to return transcripts.".into()],
                followups: vec![],
                sandbox: tmp.path().join("sandbox"),
                artifacts: vec![],
            })
        };

        let result = runtime
            .run_prototype(
                PreparedPrototypeRequest {
                    prototype,
                    context_pack,
                    record: PrototypeRecordPolicy::Prototype,
                },
                &store,
                &mut executor,
            )
            .unwrap();

        assert!(result.recorded_path.unwrap().exists());
        assert_eq!(result.observation.outcome, PrototypeOutcome::Promote);
    }
}
