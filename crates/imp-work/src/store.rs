use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::context_pack::{ContextCompiler, ContextRenderer};
use crate::memory::{
    capture_conversation_memory, ConversationMemoryIndex, ConversationMemoryInput,
    ConversationMemoryQuery,
};
use crate::model::{
    Check, CheckKind, ContextPack, Decision, DecisionStatus, Epic, Link, LinkKind, MemoryItem,
    MemoryKind, Run, Task, TaskStatus, WorkId, WorkItem,
};
use crate::prototype::{
    Prototype, PrototypeJournal, PrototypeObservation, PrototypeRecordPolicy, PrototypeStatus,
};
use crate::scheduler::{CoordinatorSummary, Scheduler, WorkOutcome};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerPersistence {
    pub run_path: PathBuf,
    pub outcome_path: PathBuf,
    pub summary_path: PathBuf,
    pub memory_paths: Vec<PathBuf>,
    pub followup_task_path: Option<PathBuf>,
    pub stale_context_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoordinatorSnapshot {
    pub runs: Vec<Run>,
    pub outcomes: Vec<WorkOutcome>,
    pub summary: Option<CoordinatorSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkValidationReport {
    pub issues: Vec<WorkValidationIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkValidationIssue {
    pub severity: WorkValidationSeverity,
    pub code: String,
    pub message: String,
    pub item_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkValidationSeverity {
    Error,
    Warning,
}

impl WorkValidationReport {
    pub fn is_ok(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == WorkValidationSeverity::Error)
    }
}

#[derive(Debug, Clone)]
pub struct WorkStore {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkLayout {
    pub root: PathBuf,
    pub work_dir: PathBuf,
    pub contexts_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub runs_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub tasks_file: PathBuf,
    pub memory_file: PathBuf,
    pub prototypes_file: PathBuf,
    pub decisions_file: PathBuf,
}

impl WorkStore {
    pub fn open(project_root: impl Into<PathBuf>) -> Self {
        Self {
            root: project_root.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn layout(&self) -> WorkLayout {
        let work_dir = self.root.join(".imp").join("work");
        WorkLayout {
            root: self.root.clone(),
            contexts_dir: work_dir.join("contexts"),
            cache_dir: work_dir.join(".cache"),
            runs_dir: work_dir.join("runs"),
            logs_dir: work_dir.join("logs"),
            tasks_file: work_dir.join("tasks.md"),
            memory_file: work_dir.join("memory.md"),
            prototypes_file: work_dir.join("prototypes.md"),
            decisions_file: work_dir.join("decisions.md"),
            work_dir,
        }
    }

    pub fn ensure_layout(&self) -> Result<WorkLayout> {
        let layout = self.layout();
        fs::create_dir_all(&layout.contexts_dir)?;
        fs::create_dir_all(&layout.cache_dir)?;
        fs::create_dir_all(&layout.runs_dir)?;
        fs::create_dir_all(&layout.logs_dir)?;
        ensure_markdown_file(&layout.tasks_file, "# Tasks\n\n")?;
        ensure_markdown_file(&layout.memory_file, "# Memory\n\n")?;
        ensure_markdown_file(&layout.prototypes_file, "# Prototypes\n\n")?;
        ensure_markdown_file(&layout.decisions_file, "# Decisions\n\n")?;
        Ok(layout)
    }

    pub fn append_memory(&self, memory: &MemoryItem) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        let mut entry = String::new();
        entry.push_str(&format!(
            "- {} @memory @{}\n",
            one_line(&memory.text),
            format_memory_kind(memory.kind)
        ));
        entry.push_str(&format!("  id: {}\n", memory.id));
        if let Some(parent_work) = &memory.parent_work {
            entry.push_str(&format!("  parent_work: {}\n", parent_work));
        }
        if !memory.topics.is_empty() {
            entry.push_str("  topics:\n");
            for topic in &memory.topics {
                entry.push_str(&format!("    - {}\n", one_line(topic)));
            }
        }
        if !memory.paths.is_empty() {
            entry.push_str("  paths:\n");
            for path in &memory.paths {
                entry.push_str(&format!("    - {}\n", path.display()));
            }
        }
        entry.push('\n');
        append_to_file(&layout.memory_file, &entry)?;
        Ok(layout.memory_file)
    }

    pub fn capture_conversation_memory(
        &self,
        input: ConversationMemoryInput,
    ) -> Result<MemoryItem> {
        let memory = capture_conversation_memory(input);
        self.append_memory(&memory)?;
        Ok(memory)
    }

    pub fn load_memory_index(&self) -> Result<ConversationMemoryIndex> {
        let layout = self.ensure_layout()?;
        let content = fs::read_to_string(&layout.memory_file)?;
        Ok(ConversationMemoryIndex::from_items(parse_memory_file(
            &content,
        )))
    }

    pub fn retrieve_memory(
        &self,
        query: ConversationMemoryQuery,
    ) -> Result<Vec<crate::memory::ConversationMemoryMatch>> {
        Ok(self.load_memory_index()?.retrieve(query))
    }

    pub fn compile_task_context_with_memory(
        &self,
        task: &Task,
        query: ConversationMemoryQuery,
        prior_attempts: Vec<String>,
    ) -> Result<ContextPack> {
        let memory = self
            .retrieve_memory(query)?
            .into_iter()
            .map(|memory_match| memory_match.memory)
            .collect();
        Ok(ContextCompiler::compile_task(task, memory, prior_attempts))
    }

    pub fn compile_prototype_context_with_memory(
        &self,
        prototype: &Prototype,
        query: ConversationMemoryQuery,
    ) -> Result<ContextPack> {
        let memory = self
            .retrieve_memory(query)?
            .into_iter()
            .map(|memory_match| memory_match.memory)
            .collect();
        Ok(ContextCompiler::compile_prototype(prototype, memory))
    }

    pub fn write_context_pack(&self, pack: &ContextPack) -> Result<(PathBuf, PathBuf)> {
        let layout = self.ensure_layout()?;
        let json_path = layout.contexts_dir.join(format!("{}.json", pack.id));
        let md_path = layout.contexts_dir.join(format!("{}.md", pack.id));
        fs::write(&json_path, serde_json::to_vec_pretty(pack)?)?;
        fs::write(&md_path, ContextRenderer::render_markdown(pack))?;
        Ok((json_path, md_path))
    }

    pub fn load_context_pack(&self, context_pack_id: &str) -> Result<Option<ContextPack>> {
        let layout = self.ensure_layout()?;
        let path = layout.contexts_dir.join(format!("{context_pack_id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
    }

    pub fn load_context_packs(&self) -> Result<Vec<ContextPack>> {
        let layout = self.ensure_layout()?;
        let mut packs = Vec::new();
        for entry in fs::read_dir(&layout.contexts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            packs.push(serde_json::from_slice(&fs::read(path)?)?);
        }
        packs.sort_by(|left: &ContextPack, right: &ContextPack| left.id.0.cmp(&right.id.0));
        Ok(packs)
    }

    pub fn mark_context_pack_stale(&self, context_pack_id: &str) -> Result<Option<PathBuf>> {
        let Some(mut pack) = self.load_context_pack(context_pack_id)? else {
            return Ok(None);
        };
        pack.status = crate::model::ContextPackStatus::Stale;
        let (json_path, _) = self.write_context_pack(&pack)?;
        Ok(Some(json_path))
    }

    pub fn refresh_task_context_with_memory(
        &self,
        previous: &ContextPack,
        task: &Task,
        query: ConversationMemoryQuery,
        prior_attempts: Vec<String>,
    ) -> Result<ContextPack> {
        let memory = self
            .retrieve_memory(query)?
            .into_iter()
            .map(|memory_match| memory_match.memory)
            .collect();
        let mut next = ContextCompiler::compile(crate::context_pack::ContextCompileRequest {
            work_id: task.id.clone(),
            version: previous.version + 1,
            token_budget: previous.token_budget,
            objective: task.title.clone(),
            non_goals: Vec::new(),
            acceptance: task.acceptance.clone(),
            memory,
            stream_history: Vec::new(),
            checks: task
                .checks
                .iter()
                .map(|check| {
                    check
                        .command
                        .clone()
                        .unwrap_or_else(|| check.description.clone())
                })
                .collect(),
            prior_attempts,
            source_refs: task.source_refs.clone(),
            launch_kind: crate::context_pack::ContextLaunchKind::Task,
        });
        next.id = crate::model::WorkId(format!("CTX-{}-v{}", task.id, next.version));
        self.write_context_pack(&next)?;
        Ok(next)
    }

    pub fn mark_contexts_stale_after_outcome(&self, outcome: &WorkOutcome) -> Result<Vec<PathBuf>> {
        if outcome.memory_updates.is_empty() && outcome.followups.is_empty() {
            return Ok(Vec::new());
        }
        let layout = self.ensure_layout()?;
        let mut changed = Vec::new();
        for entry in fs::read_dir(&layout.contexts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let mut pack: ContextPack = serde_json::from_slice(&fs::read(&path)?)?;
            if pack.work_id == outcome.work_id
                && pack.status != crate::model::ContextPackStatus::Stale
            {
                pack.status = crate::model::ContextPackStatus::Stale;
                fs::write(&path, serde_json::to_vec_pretty(&pack)?)?;
                let md_path = layout.contexts_dir.join(format!("{}.md", pack.id));
                fs::write(md_path, ContextRenderer::render_markdown(&pack))?;
                changed.push(path);
            }
        }
        Ok(changed)
    }

    pub fn append_prototype(&self, prototype: &Prototype) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        append_to_file(&layout.prototypes_file, &render_prototype_entry(prototype))?;
        Ok(layout.prototypes_file)
    }

    pub fn load_prototypes(&self) -> Result<Vec<Prototype>> {
        let layout = self.ensure_layout()?;
        let content = fs::read_to_string(&layout.prototypes_file)?;
        Ok(parse_prototypes_file(&content))
    }

    pub fn update_prototype_status(
        &self,
        prototype_id: &str,
        status: PrototypeStatus,
    ) -> Result<Option<Prototype>> {
        let layout = self.ensure_layout()?;
        let mut prototypes = self.load_prototypes()?;
        let mut updated = None;
        for prototype in &mut prototypes {
            if prototype.id == prototype_id {
                prototype.status = status;
                updated = Some(prototype.clone());
                break;
            }
        }
        let Some(updated_prototype) = updated else {
            return Ok(None);
        };
        rewrite_prototypes_file(&layout, &prototypes)?;
        Ok(Some(updated_prototype))
    }

    pub fn record_prototype_observation(
        &self,
        policy: PrototypeRecordPolicy,
        observation: &PrototypeObservation,
    ) -> Result<Option<PathBuf>> {
        self.ensure_layout()?;
        PrototypeJournal::open(&self.root).record(policy, observation)
    }

    pub fn append_run(&self, run: &Run) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        let path = layout.runs_dir.join(format!("{}.json", run.id));
        fs::write(&path, serde_json::to_vec_pretty(run)?)?;
        Ok(path)
    }

    pub fn append_outcome(&self, outcome: &WorkOutcome) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        let path = layout.runs_dir.join("outcomes.jsonl");
        let mut line = serde_json::to_string(outcome)?;
        line.push('\n');
        append_to_file(&path, &line)?;
        Ok(path)
    }

    pub fn write_coordinator_summary(&self, summary: &CoordinatorSummary) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        let path = layout.runs_dir.join("summary.json");
        fs::write(&path, serde_json::to_vec_pretty(summary)?)?;
        Ok(path)
    }

    pub fn record_outcome_memory_updates(&self, outcome: &WorkOutcome) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for update in &outcome.memory_updates {
            let mut memory = MemoryItem::new(MemoryKind::Note, update.clone());
            memory.parent_work = Some(outcome.work_id.clone());
            paths.push(self.append_memory(&memory)?);
        }
        Ok(paths)
    }

    pub fn record_outcome_followups(&self, outcome: &WorkOutcome) -> Result<Option<PathBuf>> {
        if outcome.followups.is_empty() {
            return Ok(None);
        }
        let layout = self.ensure_layout()?;
        let mut entry = String::new();
        for followup in &outcome.followups {
            entry.push_str(&format!("- {} @task @todo\n", one_line(followup)));
            entry.push_str(&format!("  id: {}\n", crate::model::WorkId::new("T")));
            entry.push_str(&format!("  parent_work: {}\n", outcome.work_id));
            entry.push_str("  source: worker_outcome\n");
            entry.push_str(&format!(
                "  source_summary: {}\n\n",
                one_line(&outcome.summary)
            ));
        }
        append_to_file(&layout.tasks_file, &entry)?;
        Ok(Some(layout.tasks_file))
    }

    pub fn persist_worker_result(
        &self,
        run: &Run,
        outcome: &WorkOutcome,
        summary: &CoordinatorSummary,
    ) -> Result<WorkerPersistence> {
        let run_path = self.append_run(run)?;
        let outcome_path = self.append_outcome(outcome)?;
        let summary_path = self.write_coordinator_summary(summary)?;
        let memory_paths = self.record_outcome_memory_updates(outcome)?;
        let followup_task_path = self.record_outcome_followups(outcome)?;
        let stale_context_paths = self.mark_contexts_stale_after_outcome(outcome)?;
        Ok(WorkerPersistence {
            run_path,
            outcome_path,
            summary_path,
            memory_paths,
            followup_task_path,
            stale_context_paths,
        })
    }

    pub fn load_run(&self, run_id: &str) -> Result<Option<Run>> {
        let layout = self.ensure_layout()?;
        let path = layout.runs_dir.join(format!("{run_id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
    }

    pub fn load_runs(&self) -> Result<Vec<Run>> {
        let layout = self.ensure_layout()?;
        let mut runs = Vec::new();
        for entry in fs::read_dir(&layout.runs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) == Some("summary.json") {
                continue;
            }
            runs.push(serde_json::from_slice(&fs::read(path)?)?);
        }
        runs.sort_by(|left: &Run, right: &Run| left.id.0.cmp(&right.id.0));
        Ok(runs)
    }

    pub fn load_outcomes(&self) -> Result<Vec<WorkOutcome>> {
        let layout = self.ensure_layout()?;
        let path = layout.runs_dir.join("outcomes.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).map_err(crate::Error::from))
            .collect()
    }

    pub fn load_coordinator_summary(&self) -> Result<Option<CoordinatorSummary>> {
        let layout = self.ensure_layout()?;
        let path = layout.runs_dir.join("summary.json");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
    }

    pub fn load_coordinator_snapshot(&self) -> Result<CoordinatorSnapshot> {
        Ok(CoordinatorSnapshot {
            runs: self.load_runs()?,
            outcomes: self.load_outcomes()?,
            summary: self.load_coordinator_summary()?,
        })
    }

    pub fn append_decision(&self, decision: &Decision) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        append_to_file(&layout.decisions_file, &render_decision_entry(decision))?;
        Ok(layout.decisions_file)
    }

    pub fn load_decisions(&self) -> Result<Vec<Decision>> {
        let layout = self.ensure_layout()?;
        let content = fs::read_to_string(&layout.decisions_file)?;
        Ok(parse_decisions_file(&content))
    }

    pub fn update_decision_status(
        &self,
        decision_id: &str,
        status: DecisionStatus,
    ) -> Result<Option<Decision>> {
        let layout = self.ensure_layout()?;
        let mut decisions = self.load_decisions()?;
        let mut updated = None;
        for decision in &mut decisions {
            if decision.id.0 == decision_id {
                decision.status = status;
                updated = Some(decision.clone());
                break;
            }
        }
        let Some(updated_decision) = updated else {
            return Ok(None);
        };
        rewrite_decisions_file(&layout, &decisions)?;
        Ok(Some(updated_decision))
    }

    pub fn append_epic(&self, epic: &Epic) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        append_to_file(&layout.tasks_file, &render_epic_entry(epic))?;
        Ok(layout.tasks_file)
    }

    pub fn load_epics(&self) -> Result<Vec<Epic>> {
        let layout = self.ensure_layout()?;
        let content = fs::read_to_string(&layout.tasks_file)?;
        Ok(parse_epics_file(&content))
    }

    pub fn append_task(&self, task: &Task) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        append_to_file(&layout.tasks_file, &render_task_entry(task))?;
        Ok(layout.tasks_file)
    }

    pub fn load_tasks(&self) -> Result<Vec<Task>> {
        let layout = self.ensure_layout()?;
        let content = fs::read_to_string(&layout.tasks_file)?;
        Ok(parse_tasks_file(&content))
    }

    pub fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<Option<Task>> {
        let layout = self.ensure_layout()?;
        let mut tasks = self.load_tasks()?;
        let mut updated = None;
        for task in &mut tasks {
            if task.id.0 == task_id {
                task.status = status;
                updated = Some(task.clone());
                break;
            }
        }
        let Some(updated_task) = updated else {
            return Ok(None);
        };
        rewrite_tasks_file(&layout, &self.load_epics()?, &tasks)?;
        Ok(Some(updated_task))
    }

    pub fn update_task_context_pack(
        &self,
        task_id: &str,
        context_pack_id: WorkId,
    ) -> Result<Option<Task>> {
        let layout = self.ensure_layout()?;
        let mut tasks = self.load_tasks()?;
        let mut updated = None;
        for task in &mut tasks {
            if task.id.0 == task_id {
                task.context_pack = Some(context_pack_id.clone());
                updated = Some(task.clone());
                break;
            }
        }
        let Some(updated_task) = updated else {
            return Ok(None);
        };
        rewrite_tasks_file(&layout, &self.load_epics()?, &tasks)?;
        Ok(Some(updated_task))
    }

    pub fn add_task_dependency(
        &self,
        task_id: &str,
        dependency_id: WorkId,
    ) -> Result<Option<Task>> {
        let layout = self.ensure_layout()?;
        let mut tasks = self.load_tasks()?;
        let mut updated = None;
        for task in &mut tasks {
            if task.id.0 == task_id {
                if !task
                    .links
                    .iter()
                    .any(|link| link.kind == LinkKind::DependsOn && link.target == dependency_id)
                {
                    task.links.push(Link {
                        kind: LinkKind::DependsOn,
                        target: dependency_id.clone(),
                    });
                }
                updated = Some(task.clone());
                break;
            }
        }
        let Some(updated_task) = updated else {
            return Ok(None);
        };
        rewrite_tasks_file(&layout, &self.load_epics()?, &tasks)?;
        Ok(Some(updated_task))
    }

    pub fn remove_task_dependency(
        &self,
        task_id: &str,
        dependency_id: &WorkId,
    ) -> Result<Option<Task>> {
        let layout = self.ensure_layout()?;
        let mut tasks = self.load_tasks()?;
        let mut updated = None;
        for task in &mut tasks {
            if task.id.0 == task_id {
                task.links.retain(|link| {
                    !(link.kind == LinkKind::DependsOn && link.target == *dependency_id)
                });
                updated = Some(task.clone());
                break;
            }
        }
        let Some(updated_task) = updated else {
            return Ok(None);
        };
        rewrite_tasks_file(&layout, &self.load_epics()?, &tasks)?;
        Ok(Some(updated_task))
    }

    pub fn load_scheduler(&self) -> Result<Scheduler> {
        let mut scheduler = Scheduler::new();
        for task in self.load_tasks()? {
            scheduler.add_task(task);
        }
        Ok(scheduler)
    }

    pub fn append_work_item(&self, item: &WorkItem) -> Result<PathBuf> {
        match item {
            WorkItem::Epic(epic) => self.append_epic(epic),
            WorkItem::Task(task) => self.append_task(task),
            WorkItem::Memory(memory) => self.append_memory(memory),
            WorkItem::Decision(decision) => self.append_decision(decision),
            WorkItem::Prototype(prototype) => self.append_prototype(prototype),
            WorkItem::Check(_)
            | WorkItem::ContextPack(_)
            | WorkItem::Run(_)
            | WorkItem::Lease(_) => self.append_jsonl_item(item),
        }
    }

    pub fn load_work_items(&self) -> Result<Vec<WorkItem>> {
        let mut items = Vec::new();
        items.extend(self.load_epics()?.into_iter().map(WorkItem::Epic));
        items.extend(self.load_tasks()?.into_iter().map(WorkItem::Task));
        items.extend(
            self.load_memory_index()?
                .recent(usize::MAX)
                .into_iter()
                .map(WorkItem::Memory),
        );
        items.extend(self.load_decisions()?.into_iter().map(WorkItem::Decision));
        items.extend(self.load_prototypes()?.into_iter().map(WorkItem::Prototype));
        items.extend(
            self.load_context_packs()?
                .into_iter()
                .map(WorkItem::ContextPack),
        );
        items.extend(self.load_cached_work_items()?);
        Ok(items)
    }

    pub fn validate(&self) -> Result<WorkValidationReport> {
        let tasks = self.load_tasks()?;
        let memories = self.load_memory_index()?.recent(usize::MAX);
        let decisions = self.load_decisions()?;
        let prototypes = self.load_prototypes()?;
        let contexts = self.load_context_packs()?;
        let cached = self.load_cached_work_items()?;

        let mut report = WorkValidationReport::default();
        let mut ids = HashSet::new();
        for item in self.load_work_items()? {
            let id = item.id().to_string();
            if !ids.insert(id.clone()) {
                report.issues.push(WorkValidationIssue {
                    severity: WorkValidationSeverity::Error,
                    code: "duplicate_id".into(),
                    message: format!("duplicate work item id `{id}`"),
                    item_id: Some(id),
                });
            }
        }
        let known_ids = ids;
        let context_ids = contexts
            .iter()
            .map(|context| context.id.0.clone())
            .collect::<HashSet<_>>();

        for task in &tasks {
            if let Some(parent) = &task.parent {
                if !known_ids.contains(&parent.0) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Warning,
                        code: "missing_parent".into(),
                        message: format!(
                            "task `{}` references missing parent `{}`",
                            task.id, parent
                        ),
                        item_id: Some(task.id.0.clone()),
                    });
                }
            }
            for link in task
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::DependsOn)
            {
                if !known_ids.contains(&link.target.0) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Error,
                        code: "missing_dependency".into(),
                        message: format!(
                            "task `{}` depends on missing task `{}`",
                            task.id, link.target
                        ),
                        item_id: Some(task.id.0.clone()),
                    });
                }
            }
            if let Some(context_pack) = &task.context_pack {
                if !context_ids.contains(&context_pack.0) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Error,
                        code: "missing_context_pack".into(),
                        message: format!(
                            "task `{}` references missing context pack `{}`",
                            task.id, context_pack
                        ),
                        item_id: Some(task.id.0.clone()),
                    });
                }
            }
        }

        for context in &contexts {
            if context.status == crate::model::ContextPackStatus::Stale {
                report.issues.push(WorkValidationIssue {
                    severity: WorkValidationSeverity::Warning,
                    code: "stale_context_pack".into(),
                    message: format!("context pack `{}` is stale", context.id),
                    item_id: Some(context.id.0.clone()),
                });
            }
        }

        for memory in &memories {
            if let Some(parent) = &memory.parent_work {
                if !known_ids.contains(&parent.0) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Warning,
                        code: "missing_parent".into(),
                        message: format!(
                            "memory `{}` references missing parent `{}`",
                            memory.id, parent
                        ),
                        item_id: Some(memory.id.0.clone()),
                    });
                }
            }
        }
        for decision in &decisions {
            if let Some(parent) = &decision.parent_work {
                if !known_ids.contains(&parent.0) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Warning,
                        code: "missing_parent".into(),
                        message: format!(
                            "decision `{}` references missing parent `{}`",
                            decision.id, parent
                        ),
                        item_id: Some(decision.id.0.clone()),
                    });
                }
            }
        }
        for prototype in &prototypes {
            if let Some(parent) = &prototype.parent_work {
                if !known_ids.contains(parent) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Warning,
                        code: "missing_parent".into(),
                        message: format!(
                            "prototype `{}` references missing parent `{}`",
                            prototype.id, parent
                        ),
                        item_id: Some(prototype.id.clone()),
                    });
                }
            }
        }
        for item in &cached {
            if let WorkItem::Lease(lease) = item {
                if !known_ids.contains(&lease.work_id.0) {
                    report.issues.push(WorkValidationIssue {
                        severity: WorkValidationSeverity::Warning,
                        code: "missing_lease_work".into(),
                        message: format!(
                            "lease `{}` references missing work `{}`",
                            lease.id, lease.work_id
                        ),
                        item_id: Some(lease.id.0.clone()),
                    });
                }
            }
        }

        Ok(report)
    }

    pub fn load_cached_work_items(&self) -> Result<Vec<WorkItem>> {
        let layout = self.ensure_layout()?;
        let path = layout.cache_dir.join("items.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).map_err(crate::Error::from))
            .collect()
    }

    pub fn release_leases_for_work(&self, work_id: &WorkId) -> Result<Vec<WorkId>> {
        let layout = self.ensure_layout()?;
        let path = layout.cache_dir.join("items.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path)?;
        let mut kept = Vec::new();
        let mut released = Vec::new();
        for line in content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let item: WorkItem = serde_json::from_str(line)?;
            match &item {
                WorkItem::Lease(lease) if lease.work_id == *work_id => {
                    released.push(lease.id.clone());
                }
                _ => kept.push(item),
            }
        }
        let mut rewritten = String::new();
        for item in kept {
            rewritten.push_str(&serde_json::to_string(&item)?);
            rewritten.push('\n');
        }
        fs::write(&path, rewritten)?;
        Ok(released)
    }

    pub fn append_jsonl_item(&self, item: &WorkItem) -> Result<PathBuf> {
        let layout = self.ensure_layout()?;
        let path = layout.cache_dir.join("items.jsonl");
        let mut line = serde_json::to_string(item)?;
        line.push('\n');
        append_to_file(&path, &line)?;
        Ok(path)
    }
}

fn rewrite_decisions_file(layout: &WorkLayout, decisions: &[Decision]) -> Result<()> {
    let mut content = String::from("# Decisions\n\n");
    for decision in decisions {
        content.push_str(&render_decision_entry(decision));
    }
    fs::write(&layout.decisions_file, content)?;
    Ok(())
}

fn render_decision_entry(decision: &Decision) -> String {
    let mut entry = String::new();
    entry.push_str(&format!(
        "- {} @decision @{}\n",
        one_line(&decision.title),
        format_decision_status(decision.status)
    ));
    entry.push_str(&format!("  id: {}\n", decision.id));
    if let Some(parent_work) = &decision.parent_work {
        entry.push_str(&format!("  parent_work: {}\n", parent_work));
    }
    if let Some(rationale) = &decision.rationale {
        entry.push_str(&format!("  rationale: {}\n", one_line(rationale)));
    }
    entry.push('\n');
    entry
}

fn rewrite_prototypes_file(layout: &WorkLayout, prototypes: &[Prototype]) -> Result<()> {
    let mut content = String::from("# Prototypes\n\n");
    for prototype in prototypes {
        content.push_str(&render_prototype_entry(prototype));
    }
    fs::write(&layout.prototypes_file, content)?;
    Ok(())
}

fn render_prototype_entry(prototype: &Prototype) -> String {
    let mut entry = String::new();
    entry.push_str(&format!(
        "- {} @prototype @{}\n",
        one_line(&prototype.question),
        format_prototype_status(prototype.status)
    ));
    entry.push_str(&format!("  id: {}\n", prototype.id));
    entry.push_str(&format!("  title: {}\n", one_line(&prototype.title)));
    if let Some(parent_work) = &prototype.parent_work {
        entry.push_str(&format!("  parent_work: {}\n", one_line(parent_work)));
    }
    if let Some(hypothesis) = &prototype.hypothesis {
        entry.push_str(&format!("  hypothesis: {}\n", one_line(hypothesis)));
    }
    entry.push_str(&format!("  sandbox: {}\n", prototype.sandbox.display()));
    entry.push_str(&format!(
        "  timebox_seconds: {}\n",
        prototype.timebox_seconds
    ));
    if !prototype.evidence_required.is_empty() {
        entry.push_str("  evidence_required:\n");
        for item in &prototype.evidence_required {
            entry.push_str(&format!("    - {}\n", one_line(item)));
        }
    }
    entry.push('\n');
    entry
}

fn rewrite_tasks_file(layout: &WorkLayout, epics: &[Epic], tasks: &[Task]) -> Result<()> {
    let mut content = String::from("# Tasks\n\n");
    for epic in epics {
        content.push_str(&render_epic_entry(epic));
    }
    for task in tasks {
        content.push_str(&render_task_entry(task));
    }
    fs::write(&layout.tasks_file, content)?;
    Ok(())
}

fn render_epic_entry(epic: &Epic) -> String {
    let mut entry = String::new();
    entry.push_str(&format!(
        "- {} @epic @{}\n",
        one_line(&epic.title),
        format_task_status(epic.status)
    ));
    entry.push_str(&format!("  id: {}\n", epic.id));
    if let Some(intent) = &epic.intent {
        entry.push_str(&format!("  intent: {}\n", one_line(intent)));
    }
    if !epic.acceptance.is_empty() {
        entry.push_str("  acceptance:\n");
        for acceptance in &epic.acceptance {
            entry.push_str(&format!("    - {}\n", one_line(acceptance)));
        }
    }
    entry.push('\n');
    entry
}

fn render_task_entry(task: &Task) -> String {
    let mut entry = String::new();
    entry.push_str(&format!(
        "- {} @task @{}\n",
        one_line(&task.title),
        format_task_status(task.status)
    ));
    entry.push_str(&format!("  id: {}\n", task.id));
    if let Some(parent) = &task.parent {
        entry.push_str(&format!("  parent: {}\n", parent));
    }
    if let Some(context_pack) = &task.context_pack {
        entry.push_str(&format!("  context_pack: {}\n", context_pack));
    }
    let dependencies = task
        .links
        .iter()
        .filter(|link| link.kind == LinkKind::DependsOn)
        .map(|link| link.target.to_string())
        .collect::<Vec<_>>();
    if !dependencies.is_empty() {
        entry.push_str("  depends_on:\n");
        for dependency in dependencies {
            entry.push_str(&format!("    - {}\n", dependency));
        }
    }
    if !task.acceptance.is_empty() {
        entry.push_str("  acceptance:\n");
        for acceptance in &task.acceptance {
            entry.push_str(&format!("    - {}\n", one_line(acceptance)));
        }
    }
    if !task.checks.is_empty() {
        entry.push_str("  checks:\n");
        for check in &task.checks {
            let value = check.command.as_deref().unwrap_or(&check.description);
            entry.push_str(&format!("    - {}\n", one_line(value)));
        }
    }
    entry.push('\n');
    entry
}

fn format_decision_status(status: DecisionStatus) -> &'static str {
    match status {
        DecisionStatus::Proposed => "proposed",
        DecisionStatus::Accepted => "accepted",
        DecisionStatus::Rejected => "rejected",
        DecisionStatus::Superseded => "superseded",
    }
}

fn parse_decision_status_from_tags(line: &str) -> DecisionStatus {
    if line.contains("@accepted") {
        DecisionStatus::Accepted
    } else if line.contains("@rejected") {
        DecisionStatus::Rejected
    } else if line.contains("@superseded") {
        DecisionStatus::Superseded
    } else {
        DecisionStatus::Proposed
    }
}

fn parse_decisions_file(content: &str) -> Vec<Decision> {
    let mut decisions = Vec::new();
    let lines = content.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.starts_with("- ") || !line.contains("@decision") {
            index += 1;
            continue;
        }

        let title = line
            .trim_start_matches("- ")
            .split(" @")
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let mut decision = Decision {
            id: crate::model::WorkId::new("D"),
            title,
            status: parse_decision_status_from_tags(line),
            rationale: None,
            parent_work: None,
            source_refs: Vec::new(),
        };

        index += 1;
        while index < lines.len() {
            let detail = lines[index];
            if detail.trim_start().starts_with("- ")
                || (!detail.starts_with(' ') && !detail.is_empty())
            {
                break;
            }
            let trimmed = detail.trim();
            if let Some(id) = trimmed.strip_prefix("id: ") {
                decision.id = id.trim().into();
            } else if let Some(parent_work) = trimmed.strip_prefix("parent_work: ") {
                decision.parent_work = Some(parent_work.trim().into());
            } else if let Some(rationale) = trimmed.strip_prefix("rationale: ") {
                decision.rationale = Some(rationale.trim().to_string());
            }
            index += 1;
        }
        decisions.push(decision);
    }
    decisions
}

fn format_prototype_status(status: PrototypeStatus) -> &'static str {
    match status {
        PrototypeStatus::Planned => "planned",
        PrototypeStatus::Running => "running",
        PrototypeStatus::Observed => "observed",
        PrototypeStatus::Promoted => "promoted",
        PrototypeStatus::Discarded => "discarded",
    }
}

fn parse_prototype_status_from_tags(line: &str) -> PrototypeStatus {
    if line.contains("@running") {
        PrototypeStatus::Running
    } else if line.contains("@observed") {
        PrototypeStatus::Observed
    } else if line.contains("@promoted") || line.contains("@promote") {
        PrototypeStatus::Promoted
    } else if line.contains("@discarded") || line.contains("@discard") {
        PrototypeStatus::Discarded
    } else {
        PrototypeStatus::Planned
    }
}

fn parse_prototypes_file(content: &str) -> Vec<Prototype> {
    let mut prototypes = Vec::new();
    let lines = content.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.starts_with("- ") || !line.contains("@prototype") {
            index += 1;
            continue;
        }

        let question = line
            .trim_start_matches("- ")
            .split(" @")
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let mut title = question.clone();
        let mut id: Option<String> = None;
        let mut parent_work = None;
        let mut hypothesis = None;
        let mut sandbox = PathBuf::from(".tmp/imp-prototypes");
        let mut timebox_seconds = 300;
        let mut evidence_required = Vec::new();
        let status = parse_prototype_status_from_tags(line);

        index += 1;
        while index < lines.len() {
            let detail = lines[index];
            if detail.trim_start().starts_with("- ")
                || (!detail.starts_with(' ') && !detail.is_empty())
            {
                break;
            }
            let trimmed = detail.trim();
            if let Some(value) = trimmed.strip_prefix("id: ") {
                id = Some(value.trim().to_string());
            } else if let Some(value) = trimmed.strip_prefix("title: ") {
                title = value.trim().to_string();
            } else if let Some(value) = trimmed.strip_prefix("parent_work: ") {
                parent_work = Some(value.trim().to_string());
            } else if let Some(value) = trimmed.strip_prefix("hypothesis: ") {
                hypothesis = Some(value.trim().to_string());
            } else if let Some(value) = trimmed.strip_prefix("sandbox: ") {
                sandbox = PathBuf::from(value.trim());
            } else if let Some(value) = trimmed.strip_prefix("timebox_seconds: ") {
                timebox_seconds = value.trim().parse().unwrap_or(300);
            } else if trimmed == "evidence_required:" {
                index += 1;
                while index < lines.len() {
                    let item = lines[index].trim();
                    if let Some(value) = item.strip_prefix("- ") {
                        evidence_required.push(value.trim().to_string());
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            }
            index += 1;
        }

        let mut prototype = Prototype::new(title, question, sandbox)
            .with_timebox_seconds(timebox_seconds)
            .with_evidence_required(evidence_required);
        if let Some(id) = id {
            prototype.id = id;
        }
        prototype.parent_work = parent_work;
        prototype.hypothesis = hypothesis;
        prototype.status = status;
        prototypes.push(prototype);
    }
    prototypes
}

fn parse_epics_file(content: &str) -> Vec<Epic> {
    let mut epics = Vec::new();
    let lines = content.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.starts_with("- ") || !line.contains("@epic") {
            index += 1;
            continue;
        }

        let title = line
            .trim_start_matches("- ")
            .split(" @")
            .next()
            .unwrap_or_default()
            .trim();
        let mut epic = Epic::new(title);
        epic.status = parse_task_status_from_tags(line);
        index += 1;
        while index < lines.len() {
            let detail = lines[index];
            if detail.trim_start().starts_with("- ")
                || (!detail.starts_with(' ') && !detail.is_empty())
            {
                break;
            }
            let trimmed = detail.trim();
            if let Some(id) = trimmed.strip_prefix("id: ") {
                epic.id = id.trim().into();
            } else if let Some(intent) = trimmed.strip_prefix("intent: ") {
                epic.intent = Some(intent.trim().to_string());
            } else if trimmed == "acceptance:" {
                index += 1;
                while index < lines.len() {
                    let item = lines[index].trim();
                    if let Some(value) = item.strip_prefix("- ") {
                        epic.acceptance.push(value.trim().to_string());
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            }
            index += 1;
        }
        epics.push(epic);
    }
    epics
}

fn format_task_status(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::Ready => "ready",
        TaskStatus::Active => "active",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Review => "review",
        TaskStatus::Done => "done",
        TaskStatus::Dropped => "dropped",
    }
}

fn parse_tasks_file(content: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let lines = content.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.starts_with("- ") || !line.contains("@task") {
            index += 1;
            continue;
        }

        let title = line
            .trim_start_matches("- ")
            .split(" @")
            .next()
            .unwrap_or_default()
            .trim();
        let mut task = Task::new(title);
        task.status = parse_task_status_from_tags(line);
        index += 1;
        while index < lines.len() {
            let detail = lines[index];
            if detail.trim_start().starts_with("- ")
                || (!detail.starts_with(' ') && !detail.is_empty())
            {
                break;
            }
            let trimmed = detail.trim();
            if let Some(id) = trimmed.strip_prefix("id: ") {
                task.id = id.trim().into();
            } else if let Some(parent) = trimmed
                .strip_prefix("parent_work: ")
                .or_else(|| trimmed.strip_prefix("parent: "))
            {
                task.parent = Some(parent.trim().into());
            } else if let Some(context_pack) = trimmed.strip_prefix("context_pack: ") {
                task.context_pack = Some(context_pack.trim().into());
            } else if trimmed == "depends_on:" {
                index += 1;
                while index < lines.len() {
                    let item = lines[index].trim();
                    if let Some(value) = item.strip_prefix("- ") {
                        task.links.push(Link {
                            kind: LinkKind::DependsOn,
                            target: value.trim().into(),
                        });
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            } else if trimmed == "acceptance:" {
                index += 1;
                while index < lines.len() {
                    let item = lines[index].trim();
                    if let Some(value) = item.strip_prefix("- ") {
                        task.acceptance.push(value.trim().to_string());
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            } else if trimmed == "checks:" {
                index += 1;
                while index < lines.len() {
                    let item = lines[index].trim();
                    if let Some(value) = item.strip_prefix("- ") {
                        task.checks.push(Check {
                            id: crate::model::WorkId::new("C"),
                            kind: CheckKind::Command,
                            description: value.trim().to_string(),
                            command: Some(value.trim().to_string()),
                        });
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            }
            index += 1;
        }
        tasks.push(task);
    }
    tasks
}

fn parse_task_status_from_tags(line: &str) -> TaskStatus {
    if line.contains("@ready") {
        TaskStatus::Ready
    } else if line.contains("@active") {
        TaskStatus::Active
    } else if line.contains("@blocked") {
        TaskStatus::Blocked
    } else if line.contains("@review") {
        TaskStatus::Review
    } else if line.contains("@done") {
        TaskStatus::Done
    } else if line.contains("@dropped") {
        TaskStatus::Dropped
    } else {
        TaskStatus::Todo
    }
}

fn parse_memory_file(content: &str) -> Vec<MemoryItem> {
    let mut items = Vec::new();
    let lines = content.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.starts_with("- ") {
            index += 1;
            continue;
        }

        let text = line
            .trim_start_matches("- ")
            .split(" @")
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        let mut memory = MemoryItem::new(parse_memory_kind_from_tags(line), text);
        index += 1;
        while index < lines.len() {
            let detail = lines[index];
            if detail.trim_start().starts_with("- ") || !detail.starts_with(' ') {
                break;
            }
            let trimmed = detail.trim();
            if let Some(id) = trimmed.strip_prefix("id: ") {
                memory.id = id.trim().into();
            } else if let Some(parent_work) = trimmed.strip_prefix("parent_work: ") {
                memory.parent_work = Some(parent_work.trim().into());
            } else if trimmed == "topics:" {
                index += 1;
                while index < lines.len() {
                    let topic_line = lines[index].trim();
                    if let Some(topic) = topic_line.strip_prefix("- ") {
                        memory.topics.push(topic.trim().to_string());
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            } else if trimmed == "paths:" {
                index += 1;
                while index < lines.len() {
                    let path_line = lines[index].trim();
                    if let Some(path) = path_line.strip_prefix("- ") {
                        memory.paths.push(PathBuf::from(path.trim()));
                        index += 1;
                    } else {
                        index = index.saturating_sub(1);
                        break;
                    }
                }
            }
            index += 1;
        }
        items.push(memory);
    }
    items
}

fn parse_memory_kind_from_tags(line: &str) -> crate::model::MemoryKind {
    if line.contains("@prototype-learning") {
        crate::model::MemoryKind::PrototypeLearning
    } else if line.contains("@preference") {
        crate::model::MemoryKind::Preference
    } else if line.contains("@decision") {
        crate::model::MemoryKind::Decision
    } else if line.contains("@follow-up") {
        crate::model::MemoryKind::FollowUp
    } else if line.contains("@fact") {
        crate::model::MemoryKind::Fact
    } else {
        crate::model::MemoryKind::Note
    }
}

fn ensure_markdown_file(path: &Path, heading: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        fs::write(path, heading)?;
    }
    Ok(())
}

fn append_to_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_memory_kind(kind: crate::model::MemoryKind) -> &'static str {
    match kind {
        crate::model::MemoryKind::Fact => "fact",
        crate::model::MemoryKind::Preference => "preference",
        crate::model::MemoryKind::Decision => "decision",
        crate::model::MemoryKind::FollowUp => "follow-up",
        crate::model::MemoryKind::Note => "note",
        crate::model::MemoryKind::PrototypeLearning => "prototype-learning",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MemoryKind, SourceKind, SourceRef, WorkId};
    use crate::prototype::{
        HypothesisResult, PrototypeEvidence, PrototypeObservation, PrototypeOutcome,
    };

    #[test]
    fn store_ensures_work_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let layout = store.ensure_layout().unwrap();

        assert!(layout.tasks_file.exists());
        assert!(layout.memory_file.exists());
        assert!(layout.prototypes_file.exists());
        assert!(layout.decisions_file.exists());
        assert!(layout.contexts_dir.exists());
        assert!(layout.cache_dir.exists());
        assert!(layout.runs_dir.exists());
        assert!(layout.logs_dir.exists());
    }

    #[test]
    fn store_appends_memory_item() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut memory = MemoryItem::new(
            MemoryKind::PrototypeLearning,
            "Prototype code is temporary; prototype learning is durable.",
        );
        memory.parent_work = Some(WorkId::from("P-runtime-loop"));
        memory.topics.push("prototype".into());
        memory.source_refs.push(SourceRef {
            kind: SourceKind::Conversation,
            reference: "session:current".into(),
            fingerprint: None,
        });

        let path = store.append_memory(&memory).unwrap();
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("@prototype-learning"));
        assert!(content.contains("parent_work: P-runtime-loop"));
    }

    #[test]
    fn store_validation_reports_missing_dependency_and_context() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Broken graph task");
        task.id = WorkId::from("T-broken");
        task.context_pack = Some(WorkId::from("CTX-missing"));
        task.links.push(Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-missing"),
        });
        store.append_task(&task).unwrap();

        let report = store.validate().unwrap();
        let codes = report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>();

        assert!(!report.is_ok());
        assert!(codes.iter().any(|code| *code == "missing_dependency"));
        assert!(codes.iter().any(|code| *code == "missing_context_pack"));
    }

    #[test]
    fn store_validation_reports_stale_context_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Stale context task");
        task.id = WorkId::from("T-stale-context-validation");
        let mut pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        pack.status = crate::model::ContextPackStatus::Stale;
        task.context_pack = Some(pack.id.clone());
        store.append_task(&task).unwrap();
        store.write_context_pack(&pack).unwrap();

        let report = store.validate().unwrap();

        assert!(report.is_ok());
        assert!(report.issues.iter().any(|issue| {
            issue.code == "stale_context_pack" && issue.severity == WorkValidationSeverity::Warning
        }));
    }

    #[test]
    fn store_releases_cached_leases_for_work() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let work_id = WorkId::from("T-release-lease");
        let lease = WorkItem::Lease(crate::model::Lease {
            id: WorkId::from("L-release"),
            work_id: work_id.clone(),
            worker_id: "worker-release".into(),
            worktree: None,
            path_locks: vec![PathBuf::from("crates/imp-work/src/store.rs")],
        });
        let other_lease = WorkItem::Lease(crate::model::Lease {
            id: WorkId::from("L-keep"),
            work_id: WorkId::from("T-keep-lease"),
            worker_id: "worker-keep".into(),
            worktree: None,
            path_locks: vec![],
        });
        store.append_work_item(&lease).unwrap();
        store.append_work_item(&other_lease).unwrap();

        let released = store.release_leases_for_work(&work_id).unwrap();
        let cached = store.load_cached_work_items().unwrap();

        assert_eq!(released, vec![WorkId::from("L-release")]);
        assert!(!cached.iter().any(|item| item.id() == "L-release"));
        assert!(cached.iter().any(|item| item.id() == "L-keep"));
    }

    #[test]
    fn store_loads_cached_runtime_work_items() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let check = WorkItem::Check(Check {
            id: WorkId::from("C-cache"),
            kind: CheckKind::Command,
            description: "Run cached item tests".into(),
            command: Some("cargo test -p imp-work cached_work_items".into()),
        });
        let context_pack = WorkItem::ContextPack(ContextCompiler::compile_task(
            &Task::new("Cached context pack"),
            Vec::new(),
            Vec::new(),
        ));
        let run = WorkItem::Run(Run {
            id: WorkId::from("R-cache"),
            work_id: Some(WorkId::from("T-cache")),
            context_pack_id: None,
            outcome: crate::model::RunOutcome::Done,
            summary: "Cached run".into(),
            changed_paths: vec![],
            checks: vec![],
        });
        let lease = WorkItem::Lease(crate::model::Lease {
            id: WorkId::from("L-cache"),
            work_id: WorkId::from("T-cache"),
            worker_id: "worker-cache".into(),
            worktree: None,
            path_locks: vec![],
        });

        for item in [check, context_pack, run, lease] {
            store.append_work_item(&item).unwrap();
        }

        let cached = store.load_cached_work_items().unwrap();
        let all = store.load_work_items().unwrap();
        let cached_kinds = cached.iter().map(WorkItem::kind).collect::<Vec<_>>();
        let all_kinds = all.iter().map(WorkItem::kind).collect::<Vec<_>>();

        assert_eq!(cached.len(), 4);
        assert!(cached_kinds.contains(&crate::model::WorkKind::Check));
        assert!(cached_kinds.contains(&crate::model::WorkKind::ContextPack));
        assert!(cached_kinds.contains(&crate::model::WorkKind::Run));
        assert!(cached_kinds.contains(&crate::model::WorkKind::Lease));
        assert!(all_kinds.contains(&crate::model::WorkKind::Check));
        assert!(all_kinds.contains(&crate::model::WorkKind::ContextPack));
        assert!(all_kinds.contains(&crate::model::WorkKind::Run));
        assert!(all_kinds.contains(&crate::model::WorkKind::Lease));
    }

    #[test]
    fn store_appends_and_loads_mixed_work_items() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut epic = Epic::new("Aggregate work item APIs");
        epic.id = WorkId::from("E-aggregate");
        let mut task = Task::new("Round-trip aggregate task");
        task.id = WorkId::from("T-aggregate");
        task.parent = Some(epic.id.clone());
        let memory = MemoryItem::new(
            MemoryKind::Fact,
            "Aggregate APIs hide markdown file placement from integrations.",
        );
        let decision = Decision {
            id: WorkId::from("D-aggregate"),
            title: "Use append_work_item for integration seams".into(),
            status: DecisionStatus::Accepted,
            rationale: Some("Integration code should not know storage file placement.".into()),
            parent_work: Some(epic.id.clone()),
            source_refs: Vec::new(),
        };
        let mut prototype = Prototype::new(
            "Prototype aggregate API",
            "Can aggregate APIs persist prototypes?",
            tmp.path().join("sandbox"),
        );
        prototype.id = "P-aggregate".into();

        for item in [
            WorkItem::Epic(epic),
            WorkItem::Task(task),
            WorkItem::Memory(memory),
            WorkItem::Decision(decision),
            WorkItem::Prototype(prototype),
        ] {
            store.append_work_item(&item).unwrap();
        }

        let loaded = store.load_work_items().unwrap();
        let ids = loaded
            .iter()
            .map(|item| item.id().to_string())
            .collect::<Vec<_>>();
        let kinds = loaded.iter().map(WorkItem::kind).collect::<Vec<_>>();

        assert!(ids.contains(&"E-aggregate".into()));
        assert!(ids.contains(&"T-aggregate".into()));
        assert!(ids.contains(&"D-aggregate".into()));
        assert!(ids.contains(&"P-aggregate".into()));
        assert!(kinds.contains(&crate::model::WorkKind::Epic));
        assert!(kinds.contains(&crate::model::WorkKind::Task));
        assert!(kinds.contains(&crate::model::WorkKind::Memory));
        assert!(kinds.contains(&crate::model::WorkKind::Decision));
        assert!(kinds.contains(&crate::model::WorkKind::Prototype));
    }

    #[test]
    fn store_updates_decision_status() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let decision = Decision {
            id: WorkId::from("D-update"),
            title: "Update decision status".into(),
            status: DecisionStatus::Proposed,
            rationale: Some("Need to accept later.".into()),
            parent_work: Some(WorkId::from("E-decisions")),
            source_refs: Vec::new(),
        };
        store.append_decision(&decision).unwrap();

        let updated = store
            .update_decision_status("D-update", DecisionStatus::Accepted)
            .unwrap()
            .unwrap();
        let loaded = store.load_decisions().unwrap();
        let content = std::fs::read_to_string(store.layout().decisions_file).unwrap();

        assert_eq!(updated.status, DecisionStatus::Accepted);
        assert_eq!(loaded[0].status, DecisionStatus::Accepted);
        assert_eq!(
            loaded[0].rationale.as_deref(),
            Some("Need to accept later.")
        );
        assert!(content.contains("@decision @accepted"));
        assert!(store
            .update_decision_status("D-missing", DecisionStatus::Accepted)
            .unwrap()
            .is_none());
    }

    #[test]
    fn store_updates_prototype_status() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut prototype = Prototype::new(
            "Update prototype status",
            "Can prototype status be promoted?",
            tmp.path().join("sandbox"),
        )
        .with_hypothesis("Status updates preserve prototype details.")
        .with_evidence_required(vec!["status promoted".into()]);
        prototype.id = "P-update".into();
        store.append_prototype(&prototype).unwrap();

        let updated = store
            .update_prototype_status("P-update", PrototypeStatus::Promoted)
            .unwrap()
            .unwrap();
        let loaded = store.load_prototypes().unwrap();
        let content = std::fs::read_to_string(store.layout().prototypes_file).unwrap();

        assert_eq!(updated.status, PrototypeStatus::Promoted);
        assert_eq!(loaded[0].status, PrototypeStatus::Promoted);
        assert_eq!(
            loaded[0].hypothesis.as_deref(),
            Some("Status updates preserve prototype details.")
        );
        assert_eq!(loaded[0].evidence_required, vec!["status promoted"]);
        assert!(content.contains("@prototype @promoted"));
        assert!(store
            .update_prototype_status("P-missing", PrototypeStatus::Promoted)
            .unwrap()
            .is_none());
    }

    #[test]
    fn store_appends_and_loads_decisions() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let decision = Decision {
            id: WorkId::from("D-imp-work-name"),
            title: "Use imp-work as the crate name".into(),
            status: DecisionStatus::Accepted,
            rationale: Some(
                "imp-work is clearer than imp-works and matches Rust module naming.".into(),
            ),
            parent_work: Some(WorkId::from("E-imp-work")),
            source_refs: Vec::new(),
        };

        let path = store.append_decision(&decision).unwrap();
        let loaded = store.load_decisions().unwrap();

        assert!(path.exists());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, WorkId::from("D-imp-work-name"));
        assert_eq!(loaded[0].title, "Use imp-work as the crate name");
        assert_eq!(loaded[0].status, DecisionStatus::Accepted);
        assert_eq!(
            loaded[0].parent_work.as_ref(),
            Some(&WorkId::from("E-imp-work"))
        );
        assert_eq!(
            loaded[0].rationale.as_deref(),
            Some("imp-work is clearer than imp-works and matches Rust module naming.")
        );
    }

    #[test]
    fn store_loads_multiple_decision_statuses() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let layout = store.ensure_layout().unwrap();
        std::fs::write(
            &layout.decisions_file,
            "# Decisions\n\n- Proposed choice @decision @proposed\n  id: D-proposed\n\n- Rejected choice @decision @rejected\n  id: D-rejected\n\n- Superseded choice @decision @superseded\n  id: D-superseded\n",
        )
        .unwrap();

        let loaded = store.load_decisions().unwrap();

        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].status, DecisionStatus::Proposed);
        assert_eq!(loaded[1].status, DecisionStatus::Rejected);
        assert_eq!(loaded[2].status, DecisionStatus::Superseded);
    }

    #[test]
    fn store_appends_and_loads_native_prototypes() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut prototype = Prototype::new(
            "Prototype native persistence",
            "Can prototypes round-trip through prototypes.md?",
            tmp.path().join("sandbox"),
        )
        .with_parent_work("T-parent-prototype")
        .with_hypothesis("A simple markdown shape is enough for planned prototypes.")
        .with_timebox_seconds(120)
        .with_evidence_required(vec!["prototype loads with evidence_required".into()]);
        prototype.id = "P-native-prototype".into();
        prototype.status = PrototypeStatus::Running;

        let path = store.append_prototype(&prototype).unwrap();
        let loaded = store.load_prototypes().unwrap();

        assert!(path.exists());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "P-native-prototype");
        assert_eq!(loaded[0].title, "Prototype native persistence");
        assert_eq!(
            loaded[0].question,
            "Can prototypes round-trip through prototypes.md?"
        );
        assert_eq!(loaded[0].parent_work.as_deref(), Some("T-parent-prototype"));
        assert_eq!(
            loaded[0].hypothesis.as_deref(),
            Some("A simple markdown shape is enough for planned prototypes.")
        );
        assert_eq!(loaded[0].sandbox, tmp.path().join("sandbox"));
        assert_eq!(loaded[0].timebox_seconds, 120);
        assert_eq!(loaded[0].status, PrototypeStatus::Running);
        assert_eq!(
            loaded[0].evidence_required,
            vec!["prototype loads with evidence_required"]
        );
    }

    #[test]
    fn store_loads_observed_prototype_entries_as_prototypes() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let observation = PrototypeObservation {
            prototype_id: "P-observed".into(),
            question: "Can observed prototypes reload?".into(),
            parent_work: Some("T-parent".into()),
            hypothesis: Some("Observation entries are parseable.".into()),
            hypothesis_result: HypothesisResult::Supported,
            outcome: PrototypeOutcome::Promote,
            summary: "Observation recorded.".into(),
            evidence_required: vec!["entry has evidence".into()],
            evidence: vec![],
            learnings: vec![],
            followups: vec![],
            sandbox: tmp.path().join("sandbox"),
            artifacts: vec![],
        };
        store
            .record_prototype_observation(PrototypeRecordPolicy::Prototype, &observation)
            .unwrap();

        let loaded = store.load_prototypes().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "P-observed");
        assert_eq!(loaded[0].status, PrototypeStatus::Promoted);
        assert_eq!(loaded[0].question, "Can observed prototypes reload?");
    }

    #[test]
    fn store_compiles_prototype_context_with_retrieved_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let prototype = Prototype::new(
            "Prototype cache-stable context rendering",
            "Can prototype contexts include retrieved memory?",
            tmp.path().join("sandbox"),
        )
        .with_parent_work("T-prototype-memory")
        .with_hypothesis("Prototype context packs can use the same memory retrieval path as tasks.")
        .with_evidence_required(vec!["rendered context includes relevant memory".into()]);
        store
            .capture_conversation_memory(ConversationMemoryInput {
                text: "Prototype context packs should include parent task memory about cache stability.".into(),
                kind: None,
                parent_work: Some(WorkId::from("T-prototype-memory")),
                topics: vec!["prototype".into(), "context-pack".into(), "cache".into()],
                paths: vec![],
                source: None,
            })
            .unwrap();

        let pack = store
            .compile_prototype_context_with_memory(
                &prototype,
                ConversationMemoryQuery {
                    text: Some("cache stability".into()),
                    topic: Some("prototype".into()),
                    parent_work: Some(WorkId::from("T-prototype-memory")),
                    path: None,
                    limit: 5,
                },
            )
            .unwrap();
        let rendered = crate::context_pack::ContextRenderer::render_markdown(&pack);

        assert_eq!(pack.work_id, WorkId::from(prototype.id.as_str()));
        assert!(rendered.contains("Prototype context packs should include parent task memory"));
        assert!(rendered.contains("rendered context includes relevant memory"));
        assert!(rendered.contains("Prototype code is disposable"));
    }

    #[test]
    fn store_loads_persisted_context_packs_as_work_items() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let task = Task::new("List context packs");
        let pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let pack_id = pack.id.clone();
        store.write_context_pack(&pack).unwrap();

        let packs = store.load_context_packs().unwrap();
        let items = store.load_work_items().unwrap();

        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].id, pack_id);
        assert!(items.iter().any(|item| {
            item.kind() == crate::model::WorkKind::ContextPack && item.id() == pack_id.0
        }));
    }

    #[test]
    fn store_writes_loads_and_marks_context_pack_stale() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Refresh context after outcome");
        task.id = WorkId::from("T-refresh-context");
        let pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let pack_id = pack.id.0.clone();

        let (json_path, md_path) = store.write_context_pack(&pack).unwrap();
        assert!(json_path.exists());
        assert!(md_path.exists());
        assert_eq!(
            store.load_context_pack(&pack_id).unwrap().unwrap().id.0,
            pack_id
        );

        let stale_path = store.mark_context_pack_stale(&pack_id).unwrap().unwrap();
        let stale = store.load_context_pack(&pack_id).unwrap().unwrap();
        assert_eq!(stale_path, json_path);
        assert_eq!(stale.status, crate::model::ContextPackStatus::Stale);
    }

    #[test]
    fn store_refreshes_task_context_with_retrieved_memory_next_version() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Build refreshed context");
        task.id = WorkId::from("T-refresh-next");
        let previous = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        store.write_context_pack(&previous).unwrap();
        store
            .capture_conversation_memory(ConversationMemoryInput {
                text: "Refreshed contexts should include new outcome memory.".into(),
                kind: None,
                parent_work: Some(task.id.clone()),
                topics: vec!["context-pack".into(), "memory".into()],
                paths: vec![],
                source: None,
            })
            .unwrap();

        let next = store
            .refresh_task_context_with_memory(
                &previous,
                &task,
                ConversationMemoryQuery {
                    text: Some("outcome memory".into()),
                    topic: Some("memory".into()),
                    parent_work: Some(task.id.clone()),
                    path: None,
                    limit: 5,
                },
                vec!["Previous run recorded memory update.".into()],
            )
            .unwrap();
        let rendered = crate::context_pack::ContextRenderer::render_markdown(&next);

        assert_eq!(next.version, previous.version + 1);
        assert!(rendered.contains("Refreshed contexts should include new outcome memory."));
        assert!(store.load_context_pack(&next.id.0).unwrap().is_some());
    }

    #[test]
    fn store_marks_related_contexts_stale_after_outcome_updates_memory_or_followups() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Mark stale after outcome");
        task.id = WorkId::from("T-stale-after-outcome");
        let pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let pack_id = pack.id.0.clone();
        store.write_context_pack(&pack).unwrap();
        let outcome = WorkOutcome {
            work_id: task.id.clone(),
            outcome: crate::model::RunOutcome::DoneWithConcerns,
            summary: "Outcome changed context inputs.".into(),
            changed_paths: vec![],
            checks_passed: 1,
            checks_failed: 0,
            memory_updates: vec!["New memory should stale old context.".into()],
            followups: vec![],
        };

        let changed = store.mark_contexts_stale_after_outcome(&outcome).unwrap();
        let stale = store.load_context_pack(&pack_id).unwrap().unwrap();

        assert_eq!(changed.len(), 1);
        assert_eq!(stale.status, crate::model::ContextPackStatus::Stale);
    }

    #[test]
    fn store_persists_worker_result_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let run = Run {
            id: WorkId::from("R-store"),
            work_id: Some(WorkId::from("T-store")),
            context_pack_id: None,
            outcome: crate::model::RunOutcome::Done,
            summary: "Stored run".into(),
            changed_paths: vec![PathBuf::from("crates/imp-work/src/store.rs")],
            checks: vec![],
        };
        let outcome = WorkOutcome {
            work_id: WorkId::from("T-store"),
            outcome: crate::model::RunOutcome::Done,
            summary: "Stored outcome".into(),
            changed_paths: vec![PathBuf::from("crates/imp-work/src/store.rs")],
            checks_passed: 1,
            checks_failed: 0,
            memory_updates: vec!["Worker persistence writes memory updates.".into()],
            followups: vec!["Create task seed from worker follow-up.".into()],
        };
        let summary = CoordinatorSummary {
            done: 1,
            recent_outcomes: vec![outcome.clone()],
            ..CoordinatorSummary::default()
        };

        let persisted = store
            .persist_worker_result(&run, &outcome, &summary)
            .unwrap();

        assert!(persisted.run_path.exists());
        assert!(persisted.outcome_path.exists());
        assert!(persisted.summary_path.exists());
        assert_eq!(persisted.memory_paths.len(), 1);
        assert!(fs::read_to_string(persisted.run_path)
            .unwrap()
            .contains("R-store"));
        assert!(fs::read_to_string(persisted.outcome_path)
            .unwrap()
            .contains("Stored outcome"));
        assert!(fs::read_to_string(persisted.memory_paths[0].clone())
            .unwrap()
            .contains("Worker persistence writes memory updates"));
        let followup_path = persisted.followup_task_path.unwrap();
        assert!(fs::read_to_string(followup_path)
            .unwrap()
            .contains("Create task seed from worker follow-up."));
    }

    #[test]
    fn store_records_outcome_followups_as_task_seeds() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let outcome = WorkOutcome {
            work_id: WorkId::from("T-parent"),
            outcome: crate::model::RunOutcome::DoneWithConcerns,
            summary: "Implementation found a follow-up.".into(),
            changed_paths: vec![],
            checks_passed: 1,
            checks_failed: 0,
            memory_updates: vec![],
            followups: vec!["Add context refresh after memory updates.".into()],
        };

        let path = store.record_outcome_followups(&outcome).unwrap().unwrap();
        let content = fs::read_to_string(path).unwrap();

        assert!(content.contains("Add context refresh after memory updates."));
        assert!(content.contains("@task @todo"));
        assert!(content.contains("parent_work: T-parent"));
        assert!(content.contains("source: worker_outcome"));
    }

    #[test]
    fn store_appends_and_loads_native_epics() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut epic = Epic::new("Build imp-work native foundation");
        epic.id = WorkId::from("E-imp-work");
        epic.status = TaskStatus::Ready;
        epic.intent = Some("Replace mana dependency with native work primitives.".into());
        epic.acceptance
            .push("tasks can point at durable epics".into());

        let path = store.append_epic(&epic).unwrap();
        let loaded = store.load_epics().unwrap();

        assert!(path.exists());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, WorkId::from("E-imp-work"));
        assert_eq!(loaded[0].title, "Build imp-work native foundation");
        assert_eq!(loaded[0].status, TaskStatus::Ready);
        assert_eq!(
            loaded[0].intent.as_deref(),
            Some("Replace mana dependency with native work primitives.")
        );
        assert_eq!(
            loaded[0].acceptance,
            vec!["tasks can point at durable epics"]
        );
    }

    #[test]
    fn store_task_parent_can_reference_loaded_epic() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut epic = Epic::new("Orchestrate subagents");
        epic.id = WorkId::from("E-subagents");
        let mut task = Task::new("Lease ready subagent task");
        task.id = WorkId::from("T-lease-subagent");
        task.parent = Some(epic.id.clone());
        store.append_epic(&epic).unwrap();
        store.append_task(&task).unwrap();

        let epics = store.load_epics().unwrap();
        let tasks = store.load_tasks().unwrap();

        assert_eq!(epics[0].id, WorkId::from("E-subagents"));
        assert_eq!(tasks[0].parent.as_ref(), Some(&epics[0].id));
    }

    #[test]
    fn store_appends_and_loads_native_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Add native task creation API");
        task.id = WorkId::from("T-native-task");
        task.status = TaskStatus::Ready;
        task.parent = Some(WorkId::from("E-imp-work"));
        task.links.push(Link {
            kind: LinkKind::DependsOn,
            target: WorkId::from("T-prerequisite"),
        });
        task.acceptance
            .push("task round-trips through tasks.md".into());
        task.checks.push(Check {
            id: WorkId::from("C-native-task"),
            kind: CheckKind::Command,
            description: "Run store tests".into(),
            command: Some("cargo test -p imp-work store".into()),
        });

        let path = store.append_task(&task).unwrap();
        let loaded = store.load_tasks().unwrap();

        assert!(path.exists());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, WorkId::from("T-native-task"));
        assert_eq!(loaded[0].title, "Add native task creation API");
        assert_eq!(loaded[0].status, TaskStatus::Ready);
        assert_eq!(loaded[0].parent.as_ref(), Some(&WorkId::from("E-imp-work")));
        assert_eq!(
            loaded[0].acceptance,
            vec!["task round-trips through tasks.md"]
        );
        assert_eq!(
            loaded[0]
                .links
                .iter()
                .find(|link| link.kind == LinkKind::DependsOn)
                .map(|link| &link.target),
            Some(&WorkId::from("T-prerequisite"))
        );
        assert_eq!(
            loaded[0].checks[0].command.as_deref(),
            Some("cargo test -p imp-work store")
        );
    }

    #[test]
    fn store_loads_followup_task_seeds() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let outcome = WorkOutcome {
            work_id: WorkId::from("T-parent"),
            outcome: crate::model::RunOutcome::DoneWithConcerns,
            summary: "Implementation found follow-ups.".into(),
            changed_paths: vec![],
            checks_passed: 1,
            checks_failed: 0,
            memory_updates: vec![],
            followups: vec!["Add context refresh after memory updates.".into()],
        };
        store.record_outcome_followups(&outcome).unwrap();

        let tasks = store.load_tasks().unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Add context refresh after memory updates.");
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(
            tasks[0].parent.as_ref().map(|id| id.0.as_str()),
            Some("T-parent")
        );
    }

    #[test]
    fn store_loads_tasks_with_acceptance_and_checks() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let layout = store.ensure_layout().unwrap();
        std::fs::write(
            &layout.tasks_file,
            "# Tasks\n\n- Build context refresh @task @ready\n  id: T-refresh\n  parent: E-work\n  acceptance:\n    - context pack marked stale\n  checks:\n    - cargo test -p imp-work context_refresh\n",
        )
        .unwrap();

        let tasks = store.load_tasks().unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, WorkId::from("T-refresh"));
        assert_eq!(tasks[0].status, TaskStatus::Ready);
        assert_eq!(tasks[0].acceptance, vec!["context pack marked stale"]);
        assert_eq!(
            tasks[0].checks[0].command.as_deref(),
            Some("cargo test -p imp-work context_refresh")
        );
    }

    #[test]
    fn store_adds_and_removes_task_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut dependency = Task::new("Dependency to edit");
        dependency.id = WorkId::from("T-edit-dependency");
        dependency.status = TaskStatus::Ready;
        let mut dependent = Task::new("Dependent to edit");
        dependent.id = WorkId::from("T-edit-dependent");
        dependent.status = TaskStatus::Ready;
        store.append_task(&dependency).unwrap();
        store.append_task(&dependent).unwrap();

        let updated = store
            .add_task_dependency("T-edit-dependent", WorkId::from("T-edit-dependency"))
            .unwrap()
            .unwrap();
        assert_eq!(
            updated
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::DependsOn)
                .count(),
            1
        );
        // Adding twice is idempotent.
        store
            .add_task_dependency("T-edit-dependent", WorkId::from("T-edit-dependency"))
            .unwrap();
        let tasks = store.load_tasks().unwrap();
        let dependent = tasks
            .iter()
            .find(|task| task.id == WorkId::from("T-edit-dependent"))
            .unwrap();
        assert_eq!(
            dependent
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::DependsOn)
                .count(),
            1
        );
        assert_eq!(store.load_scheduler().unwrap().ready_queue().len(), 1);

        let updated = store
            .remove_task_dependency("T-edit-dependent", &WorkId::from("T-edit-dependency"))
            .unwrap()
            .unwrap();
        assert!(updated
            .links
            .iter()
            .all(|link| link.kind != LinkKind::DependsOn));
        assert_eq!(store.load_scheduler().unwrap().ready_queue().len(), 2);
        assert!(store
            .add_task_dependency("T-missing", WorkId::from("T-edit-dependency"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn store_persists_task_context_pack_links() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Own a context pack");
        task.id = WorkId::from("T-own-context");
        store.append_task(&task).unwrap();

        let updated = store
            .update_task_context_pack("T-own-context", WorkId::from("CTX-T-own-context-v1"))
            .unwrap()
            .unwrap();
        let loaded = store.load_tasks().unwrap();
        let content = std::fs::read_to_string(store.layout().tasks_file).unwrap();

        assert_eq!(
            updated.context_pack.as_ref(),
            Some(&WorkId::from("CTX-T-own-context-v1"))
        );
        assert_eq!(
            loaded[0].context_pack.as_ref(),
            Some(&WorkId::from("CTX-T-own-context-v1"))
        );
        assert!(content.contains("context_pack: CTX-T-own-context-v1"));
        assert!(store
            .update_task_context_pack("T-missing", WorkId::from("CTX-missing-v1"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn store_updates_task_status() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut epic = Epic::new("Lifecycle epic");
        epic.id = WorkId::from("E-lifecycle");
        let mut task = Task::new("Move task through lifecycle");
        task.id = WorkId::from("T-lifecycle");
        task.status = TaskStatus::Ready;
        task.parent = Some(epic.id.clone());
        task.acceptance.push("status updates to done".into());
        store.append_epic(&epic).unwrap();
        store.append_task(&task).unwrap();

        let updated = store
            .update_task_status("T-lifecycle", TaskStatus::Done)
            .unwrap()
            .unwrap();
        let tasks = store.load_tasks().unwrap();
        let epics = store.load_epics().unwrap();
        let content = std::fs::read_to_string(store.layout().tasks_file).unwrap();

        assert_eq!(updated.status, TaskStatus::Done);
        assert_eq!(tasks[0].status, TaskStatus::Done);
        assert_eq!(tasks[0].acceptance, vec!["status updates to done"]);
        assert_eq!(epics[0].id, WorkId::from("E-lifecycle"));
        assert!(content.contains("@task @done"));
        assert!(content.contains("@epic @todo"));
        assert!(store
            .update_task_status("T-missing", TaskStatus::Done)
            .unwrap()
            .is_none());
    }

    #[test]
    fn store_loads_persisted_tasks_into_scheduler_ready_queue() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut ready = Task::new("Ready persisted task");
        ready.id = WorkId::from("T-ready-persisted");
        ready.status = TaskStatus::Ready;
        let mut done = Task::new("Done persisted task");
        done.id = WorkId::from("T-done-persisted");
        done.status = TaskStatus::Done;
        store.append_task(&ready).unwrap();
        store.append_task(&done).unwrap();

        let mut scheduler = store.load_scheduler().unwrap();
        let ready_queue = scheduler.ready_queue();

        assert_eq!(ready_queue.len(), 1);
        assert_eq!(ready_queue[0].id, WorkId::from("T-ready-persisted"));
        let lease = scheduler
            .lease_ready(crate::scheduler::LeaseRequest {
                worker_id: "worker-1".into(),
                profile: crate::scheduler::WorkerProfile::implementer(),
                preferred_work_id: Some(WorkId::from("T-ready-persisted")),
                path_locks: vec![],
                worktree: None,
            })
            .unwrap();
        assert_eq!(lease.lease.work_id, WorkId::from("T-ready-persisted"));
    }

    #[test]
    fn store_loads_persisted_coordinator_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let run = Run {
            id: WorkId::from("R-reload"),
            work_id: Some(WorkId::from("T-reload")),
            context_pack_id: None,
            outcome: crate::model::RunOutcome::DoneWithConcerns,
            summary: "Reloaded run".into(),
            changed_paths: vec![],
            checks: vec![],
        };
        let outcome = WorkOutcome {
            work_id: WorkId::from("T-reload"),
            outcome: crate::model::RunOutcome::DoneWithConcerns,
            summary: "Reloaded outcome".into(),
            changed_paths: vec![],
            checks_passed: 1,
            checks_failed: 0,
            memory_updates: vec![],
            followups: vec![],
        };
        let summary = CoordinatorSummary {
            done: 1,
            recent_outcomes: vec![outcome.clone()],
            ..CoordinatorSummary::default()
        };
        store
            .persist_worker_result(&run, &outcome, &summary)
            .unwrap();

        let snapshot = store.load_coordinator_snapshot().unwrap();

        assert_eq!(snapshot.runs.len(), 1);
        assert_eq!(snapshot.runs[0].id, WorkId::from("R-reload"));
        assert_eq!(snapshot.outcomes, vec![outcome]);
        assert_eq!(snapshot.summary.unwrap().done, 1);
        assert_eq!(
            store.load_run("R-reload").unwrap().unwrap().summary,
            "Reloaded run"
        );
        assert!(store.load_run("R-missing").unwrap().is_none());
    }

    #[test]
    fn store_loads_and_retrieves_memory_index() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        store
            .capture_conversation_memory(ConversationMemoryInput {
                text: "Task context packs should include relevant conversational memory.".into(),
                kind: None,
                parent_work: Some(WorkId::from("T-context-memory")),
                topics: vec!["context-pack".into(), "memory".into()],
                paths: vec![PathBuf::from("crates/imp-work/src/context_pack.rs")],
                source: Some("conversation:test".into()),
            })
            .unwrap();

        let matches = store
            .retrieve_memory(ConversationMemoryQuery {
                text: Some("conversational memory".into()),
                topic: Some("context-pack".into()),
                parent_work: Some(WorkId::from("T-context-memory")),
                path: None,
                limit: 3,
            })
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert!(matches[0].memory.text.contains("conversational memory"));
    }

    #[test]
    fn store_compiles_task_context_with_retrieved_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let mut task = Task::new("Build memory-aware context compiler");
        task.id = WorkId::from("T-memory-context");
        task.acceptance
            .push("context includes retrieved memory".into());
        store
            .capture_conversation_memory(ConversationMemoryInput {
                text: "Memory-aware context compilation should retrieve by parent task.".into(),
                kind: None,
                parent_work: Some(task.id.clone()),
                topics: vec!["memory".into(), "context-pack".into()],
                paths: vec![],
                source: None,
            })
            .unwrap();

        let pack = store
            .compile_task_context_with_memory(
                &task,
                ConversationMemoryQuery {
                    text: Some("retrieve parent task".into()),
                    topic: Some("memory".into()),
                    parent_work: Some(task.id.clone()),
                    path: None,
                    limit: 5,
                },
                Vec::new(),
            )
            .unwrap();
        let rendered = crate::context_pack::ContextRenderer::render_markdown(&pack);

        assert!(rendered.contains("Memory-aware context compilation"));
        assert!(rendered.contains("context includes retrieved memory"));
    }

    #[test]
    fn store_records_prototype_observation() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorkStore::open(tmp.path());
        let observation = PrototypeObservation {
            prototype_id: "P-store".into(),
            question: "Can the store reconcile prototype observations?".into(),
            parent_work: Some("T-store".into()),
            hypothesis: None,
            hypothesis_result: HypothesisResult::NotAssessed,
            outcome: PrototypeOutcome::Iterate,
            summary: "Recorded append-only prototype observation.".into(),
            evidence_required: vec!["prototypes.md exists".into()],
            evidence: vec![PrototypeEvidence {
                claim: "prototype recorded".into(),
                proof: "prototypes.md contained P-store".into(),
                artifact: None,
            }],
            learnings: vec!["Store should own prototype reconciliation path.".into()],
            followups: vec![],
            sandbox: tmp.path().join("sandbox"),
            artifacts: vec![],
        };

        let path = store
            .record_prototype_observation(PrototypeRecordPolicy::Prototype, &observation)
            .unwrap()
            .unwrap();
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("P-store"));
        assert!(content.contains("Store should own prototype reconciliation path."));
    }
}
