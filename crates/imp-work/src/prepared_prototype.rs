use crate::context_pack::{ContextCompiler, ContextRenderer, RenderedContextPack};
use crate::model::{ContextPack, RunOutcome, WorkId};
use crate::prototype::{Prototype, PrototypeObservation, PrototypeRecordPolicy};
use crate::scheduler::WorkOutcome;
use crate::store::{WorkStore, WorkerPersistence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrototypeRequest {
    pub prototype: Prototype,
    pub context_pack: ContextPack,
    pub record: PrototypeRecordPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrototypeLaunch {
    pub prototype_id: String,
    pub context: RenderedContextPack,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrototypeResult {
    pub observation: PrototypeObservation,
    pub recorded_path: Option<std::path::PathBuf>,
    pub parent_persistence: Option<WorkerPersistence>,
}

#[derive(Debug, thiserror::Error)]
pub enum PreparedPrototypeError {
    #[error(
        "context pack `{pack_id}` belongs to `{pack_work_id}`, not prototype `{prototype_id}`"
    )]
    ContextPrototypeMismatch {
        pack_id: String,
        pack_work_id: String,
        prototype_id: String,
    },
    #[error("context pack `{0}` is stale")]
    StaleContext(String),
    #[error("prototype worker failed: {0}")]
    Worker(String),
    #[error("failed to persist prototype result: {0}")]
    Persist(#[from] crate::Error),
}

pub struct PreparedPrototypeLoop;

impl PreparedPrototypeLoop {
    pub fn run_once<F>(
        request: PreparedPrototypeRequest,
        store: &WorkStore,
        worker: F,
    ) -> Result<PreparedPrototypeResult, PreparedPrototypeError>
    where
        F: FnOnce(PreparedPrototypeLaunch) -> Result<PrototypeObservation, String>,
    {
        if request.context_pack.work_id.0 != request.prototype.id {
            return Err(PreparedPrototypeError::ContextPrototypeMismatch {
                pack_id: request.context_pack.id.0,
                pack_work_id: request.context_pack.work_id.0,
                prototype_id: request.prototype.id,
            });
        }
        let rendered = ContextRenderer::render(&request.context_pack);
        if rendered.stale {
            return Err(PreparedPrototypeError::StaleContext(
                request.context_pack.id.0,
            ));
        }

        let launch = PreparedPrototypeLaunch {
            prototype_id: request.prototype.id.clone(),
            context: rendered,
            prompt: render_prototype_launch_prompt(&request.prototype, &request.context_pack),
        };
        let observation = worker(launch).map_err(PreparedPrototypeError::Worker)?;
        let recorded_path = store.record_prototype_observation(request.record, &observation)?;
        let parent_persistence = persist_parent_learning_if_needed(store, &observation)?;

        Ok(PreparedPrototypeResult {
            observation,
            recorded_path,
            parent_persistence,
        })
    }
}

pub fn compile_prepared_prototype_context(prototype: &Prototype) -> ContextPack {
    ContextCompiler::compile_prototype(prototype, Vec::new())
}

fn render_prototype_launch_prompt(prototype: &Prototype, pack: &ContextPack) -> String {
    let rendered = ContextRenderer::render(pack);
    let mut prompt = String::new();
    prompt.push_str("You are a prepared imp prototype worker. Answer the prototype question with bounded disposable code evidence.\n\n");
    prompt.push_str(&format!("prototype_id: {}\n", prototype.id));
    prompt.push_str(&format!("question: {}\n", prototype.question));
    if let Some(hypothesis) = &prototype.hypothesis {
        prompt.push_str(&format!("hypothesis: {}\n", hypothesis));
    }
    prompt.push_str(&format!("context_pack: {}\n", pack.id));
    prompt.push_str(&format!(
        "stable_prefix_hash: {}\n\n",
        rendered.stable_prefix_hash
    ));
    prompt.push_str("Return a PrototypeObservation with hypothesis_result, evidence, learnings, followups, and promote/discard/iterate recommendation. Prototype code is disposable unless explicitly promoted.\n");
    prompt
}

fn persist_parent_learning_if_needed(
    store: &WorkStore,
    observation: &PrototypeObservation,
) -> Result<Option<WorkerPersistence>, crate::Error> {
    let Some(parent_work) = &observation.parent_work else {
        return Ok(None);
    };
    if observation.learnings.is_empty() && observation.followups.is_empty() {
        return Ok(None);
    }

    let outcome = WorkOutcome {
        work_id: WorkId::from(parent_work.as_str()),
        outcome: RunOutcome::DoneWithConcerns,
        summary: observation.summary.clone(),
        changed_paths: observation.artifacts.clone(),
        checks_passed: observation.evidence.len(),
        checks_failed: 0,
        memory_updates: observation.learnings.clone(),
        followups: observation.followups.clone(),
    };
    let run = crate::model::Run {
        id: WorkId::new("R"),
        work_id: Some(outcome.work_id.clone()),
        context_pack_id: None,
        outcome: outcome.outcome,
        summary: outcome.summary.clone(),
        changed_paths: outcome.changed_paths.clone(),
        checks: Vec::new(),
    };
    let summary = crate::scheduler::CoordinatorSummary {
        done: 1,
        recent_outcomes: vec![outcome.clone()],
        ..crate::scheduler::CoordinatorSummary::default()
    };
    store
        .persist_worker_result(&run, &outcome, &summary)
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ContextPackStatus, Task};
    use crate::prototype::{HypothesisResult, PrototypeEvidence, PrototypeOutcome};

    #[test]
    fn prepared_prototype_records_observation_and_stales_parent_context() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut parent_task = Task::new("Implement context renderer");
        parent_task.id = WorkId::from("T-parent-context");
        let parent_pack = ContextCompiler::compile_task(&parent_task, Vec::new(), Vec::new());
        let parent_pack_id = parent_pack.id.0.clone();
        store.write_context_pack(&parent_pack).unwrap();

        let prototype = Prototype::new(
            "Prototype cache-stable rendering",
            "Can stable and dynamic blocks preserve prefix hashes?",
            tmp.path().join("sandbox"),
        )
        .with_parent_work("T-parent-context")
        .with_hypothesis("Dynamic blocks do not change stable prefix hash.")
        .with_evidence_required(vec!["stable prefix hash unchanged".into()]);
        let context_pack = compile_prepared_prototype_context(&prototype);

        let result = PreparedPrototypeLoop::run_once(
            PreparedPrototypeRequest {
                prototype: prototype.clone(),
                context_pack,
                record: PrototypeRecordPolicy::Prototype,
            },
            &store,
            |launch| {
                assert_eq!(launch.prototype_id, prototype.id);
                assert!(launch.prompt.contains("stable_prefix_hash"));
                Ok(PrototypeObservation {
                    prototype_id: prototype.id.clone(),
                    question: prototype.question.clone(),
                    parent_work: Some("T-parent-context".into()),
                    hypothesis: prototype.hypothesis.clone(),
                    hypothesis_result: HypothesisResult::Supported,
                    outcome: PrototypeOutcome::Promote,
                    summary: "Stable/dynamic split worked.".into(),
                    evidence_required: prototype.evidence_required.clone(),
                    evidence: vec![PrototypeEvidence {
                        claim: "stable prefix".into(),
                        proof: "hash unchanged".into(),
                        artifact: None,
                    }],
                    learnings: vec!["Keep run metadata out of cacheable context blocks.".into()],
                    followups: vec!["Implement block stability metadata.".into()],
                    sandbox: tmp.path().join("sandbox"),
                    artifacts: vec![],
                })
            },
        )
        .unwrap();

        assert!(result.recorded_path.unwrap().exists());
        assert!(result
            .parent_persistence
            .unwrap()
            .followup_task_path
            .is_some());
        let stale = store.load_context_pack(&parent_pack_id).unwrap().unwrap();
        assert_eq!(stale.status, ContextPackStatus::Stale);
    }

    #[test]
    fn prepared_prototype_rejects_stale_context() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let prototype = Prototype::new(
            "Prototype stale",
            "Should not run stale context?",
            tmp.path(),
        );
        let mut context_pack = compile_prepared_prototype_context(&prototype);
        context_pack.status = ContextPackStatus::Stale;

        let error = PreparedPrototypeLoop::run_once(
            PreparedPrototypeRequest {
                prototype,
                context_pack,
                record: PrototypeRecordPolicy::None,
            },
            &store,
            |_| unreachable!("stale prototype should not launch"),
        )
        .unwrap_err();

        assert!(matches!(error, PreparedPrototypeError::StaleContext(_)));
    }
}
