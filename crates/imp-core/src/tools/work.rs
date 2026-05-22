use async_trait::async_trait;
use imp_work::global_store::StreamRelation;
use imp_work::{
    build_work_tree, capture_conversation_memory, close_task_with_conventions,
    fail_task_with_conventions, summarize_checks, CheckResult, CloseRequest, ContextRenderer,
    ConversationMemoryInput, ConversationMemoryQuery, Decision, DecisionStatus, Epic,
    GlobalWorkStore, HypothesisResult, MemoryItem, MemoryKind, Prototype, PrototypeEvidence,
    PrototypeObservation, PrototypeOutcome, PrototypeRecordPolicy, PrototypeStatus, Run,
    RunOutcome, StreamEvent, Task, TaskStatus, WorkId, WorkItem, WorkKind, WorkStore,
};
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::{Error, Result};
use crate::reference_monitor::{ToolActionKind, ToolMetadata};
use crate::storage;

pub struct WorkTool;

#[async_trait]
impl Tool for WorkTool {
    fn name(&self) -> &str {
        "work"
    }

    fn label(&self) -> &str {
        "Work"
    }

    fn description(&self) -> &str {
        "Create and list native imp-work items backed by .imp/work. Use this for durable tasks, epics, memory, decisions, and prototypes without going through mana."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "list", "context", "refresh_context", "next", "show", "update", "claim", "dep_add", "dep_remove", "validate", "scope", "guide", "search", "outcome", "prototype_outcome", "runs", "tree", "verify", "close", "fail", "remember"],
                    "description": "Create, list, show, update, claim, report active store scope, show the imp-work guide, remember/search memory, record/inspect task or prototype outcomes, prepare/refresh context packs, select next ready native imp-work items, or manage native tree/verify/close/fail workflows."
                },
                "kind": {
                    "type": "string",
                    "enum": ["task", "epic", "memory", "decision", "prototype", "context_pack"],
                    "description": "Work item kind for create/list/context. Create supports task, epic, memory, decision, prototype. Context supports task/prototype. List/show support context_pack too."
                },
                "id": { "type": "string", "description": "Existing work item id for show/update/claim/context/dependency actions." },
                "dependency_id": { "type": "string", "description": "Dependency task id for dep_add/dep_remove." },
                "title": { "type": "string", "description": "Title for tasks, epics, decisions, and prototypes." },
                "text": { "type": "string", "description": "Memory text, or fallback title/question text." },
                "status": { "type": "string", "description": "Status for tasks/epics/decisions/prototypes." },
                "parent_work": { "type": "string", "description": "Optional parent work id." },
                "acceptance": { "type": "array", "items": { "type": "string" } },
                "checks": { "type": "array", "items": { "type": "string" } },
                "depends_on": { "type": "array", "items": { "type": "string" }, "description": "Task dependency ids." },
                "topics": { "type": "array", "items": { "type": "string" } },
                "topic": { "type": "string", "description": "Single topic filter for context memory retrieval or guide topic." },
                "memory_kind": {
                    "type": "string",
                    "enum": ["fact", "preference", "decision", "follow_up", "note", "prototype_learning"],
                    "description": "Memory kind for memory items."
                },
                "rationale": { "type": "string", "description": "Decision rationale." },
                "hypothesis": { "type": "string", "description": "Prototype hypothesis." },
                "evidence_required": { "type": "array", "items": { "type": "string" } },
                "sandbox": { "type": "string", "description": "Prototype sandbox path." },
                "store_source": {
                    "type": "string",
                    "enum": ["global_project_scoped"],
                    "description": "Normal work actions use the global project-scoped store. project-local stores are migration input only."
                },
                "stream_id": { "type": "string", "description": "Optional project work stream id for global project-scoped task continuity." },
                "relation": { "type": "string", "enum": ["opened", "continues", "follow_up_to", "supersedes", "related_to", "derived_from", "regression_of", "closed"], "description": "Optional project stream relation for global task create/outcome continuity." },
                "limit": { "type": "number", "description": "Maximum list items to return. Defaults to 50." },
                "query": { "type": "string", "description": "Text query for memory retrieval/search or context memory retrieval." },
                "paths": { "type": "array", "items": { "type": "string" }, "description": "Optional paths for memory items." },
                "path": { "type": "string", "description": "Optional path filter for memory search/list/remember." },
                "worker_id": { "type": "string", "description": "Worker id for claim leases. Defaults to imp." },
                "worktree": { "type": "string", "description": "Optional worktree path for claim leases." },
                "path_locks": { "type": "array", "items": { "type": "string" }, "description": "Optional path locks for claim leases." },
                "require_context": { "type": "boolean", "description": "For next/claim, require tasks to have a current non-stale context pack." },
                "outcome": {
                    "type": "string",
                    "enum": ["done", "done_with_concerns", "blocked", "needs_context", "failed"],
                    "description": "Structured task outcome for action=outcome."
                },
                "summary": { "type": "string", "description": "Outcome summary for action=outcome." },
                "changed_paths": { "type": "array", "items": { "type": "string" } },
                "memory_updates": { "type": "array", "items": { "type": "string" } },
                "followups": { "type": "array", "items": { "type": "string" } },
                "checks_passed": { "type": "number" },
                "checks_failed": { "type": "number" },
                "hypothesis_result": {
                    "type": "string",
                    "enum": ["supported", "refuted", "inconclusive", "not_assessed"],
                    "description": "Prototype hypothesis result for action=prototype_outcome."
                },
                "force": { "type": "boolean", "description": "For close, allow closing a checked task without passing verify when force_reason is provided." },
                "force_reason": { "type": "string", "description": "Required reason when force-closing a checked task without passing verify." },
                "next_action": { "type": "string", "description": "For fail, the next useful action to recover from the blocker." },
                "recommended_action": {
                    "type": "string",
                    "enum": ["promote", "discard", "iterate", "inconclusive"],
                    "description": "Prototype outcome recommendation for action=prototype_outcome."
                },
                "evidence": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "claim": { "type": "string" },
                            "proof": { "type": "string" },
                            "artifact": { "type": "string" }
                        },
                        "required": ["claim", "proof"]
                    }
                }
            },
            "required": ["action"]
        })
    }

    fn is_readonly(&self) -> bool {
        false
    }

    fn policy_metadata(&self) -> ToolMetadata {
        let mut metadata = ToolMetadata::new(self.name(), ToolActionKind::Write);
        metadata.workspace_write = true;
        metadata.external_side_effect = false;
        metadata.default_requires_approval = false;
        metadata
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let action = required_str(&params, "action")?;
        let scope = storage::WorkScope::for_project_dir(&ctx.cwd);
        let store = WorkStore::open(global_project_work_store_root(&scope));
        match action {
            "create" => create_item(&store, &ctx.cwd, &params),
            "list" => list_items(&store, &ctx.cwd, &params),
            "context" => create_context_pack(&store, &ctx.cwd, &params),
            "refresh_context" => refresh_context_pack(&store, &params),
            "next" => next_ready_tasks(&store, &params),
            "show" => show_item(&store, &ctx.cwd, &params),
            "update" => update_item(&store, &ctx.cwd, &params),
            "claim" => claim_task(&store, &params),
            "dep_add" => edit_dependency(&store, &params, true),
            "dep_remove" => edit_dependency(&store, &params, false),
            "validate" => validate_store(&store),
            "scope" => work_scope(&store, &ctx.cwd),
            "guide" => work_guide(&params),
            "search" => search_memory(&store, &params),
            "outcome" => record_outcome(&store, &ctx.cwd, &params),
            "prototype_outcome" => record_prototype_outcome(&store, &params),
            "runs" => list_runs(&store, &params),
            "tree" => work_tree(&store),
            "verify" => verify_task(&store, &params),
            "close" => close_task_action(&store, &params),
            "fail" => fail_task_action(&store, &params),
            "remember" => remember_memory(&store, &params),
            other => Err(Error::Tool(format!(
                "unsupported work action `{other}`; expected create, list, context, refresh_context, next, show, update, claim, dep_add, dep_remove, validate, scope, guide, search, outcome, prototype_outcome, runs, tree, verify, close, fail, or remember"
            ))),
        }
    }
}

fn scope_from_global_project_store(root: &std::path::Path) -> Option<storage::WorkScope> {
    let project_hash = root.file_name()?.to_str()?;
    let projects_dir = root.parent()?;
    if projects_dir.file_name()?.to_str()? != "projects" {
        return None;
    }
    let global_store_root = projects_dir.parent()?.to_path_buf();
    let mut candidates = std::env::current_dir().ok().into_iter().collect::<Vec<_>>();
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(std::path::PathBuf::from(home));
    }
    for candidate in candidates {
        let scope = storage::WorkScope::with_global_root(&candidate, global_store_root.clone());
        if global_project_hash(&scope.project_root) == project_hash {
            return Some(scope);
        }
    }
    None
}

fn global_project_hash(project_root: &std::path::Path) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project_root.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn global_project_work_store_root(scope: &storage::WorkScope) -> std::path::PathBuf {
    scope
        .global_store_root
        .join("projects")
        .join(global_project_hash(&scope.project_root))
}

fn create_item(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind = required_str(params, "kind")?;
    if kind == "task" {
        ensure_global_task_source(params)?;
        return create_global_project_scoped_item(store, cwd, params);
    }

    let item = match kind {
        "epic" => WorkItem::Epic(build_epic(params)?),
        "memory" => WorkItem::Memory(build_memory(params)?),
        "decision" => WorkItem::Decision(build_decision(params)?),
        "prototype" => WorkItem::Prototype(build_prototype(params)?),
        other => {
            return Err(Error::Tool(format!(
            "unsupported work kind `{other}`; expected task, epic, memory, decision, or prototype"
        )))
        }
    };
    let path = store
        .append_work_item(&item)
        .map_err(|error| Error::Tool(format!("failed to create work item: {error}")))?;
    let text = format!(
        "Created {} {} in {}",
        format_work_kind(item.kind()),
        item.id(),
        path.display()
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "create",
            "store_source": "global_project_scoped",
            "kind": format_work_kind(item.kind()),
            "id": item.id(),
            "path": path,
            "item": item,
        }),
        is_error: false,
    })
}

fn create_global_project_scoped_item(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind = required_str(params, "kind")?;
    if kind != "task" {
        return Err(Error::Tool(format!(
            "store_source=global_project_scoped currently supports create kind=task only; got `{kind}`"
        )));
    }
    let task = build_task(store, params)?;
    let scope = storage::WorkScope::for_project_dir(cwd);
    let global = GlobalWorkStore::open(scope.global_store_root.clone());
    let stream_id = optional_string(params, "stream_id");
    let path = global
        .append_task_in_stream(&scope.project_root, &task, stream_id.as_deref())
        .map_err(|error| Error::Tool(format!("failed to create global project task: {error}")))?;
    store.append_task(&task).map_err(|error| {
        Error::Tool(format!(
            "failed to mirror task into global work graph: {error}"
        ))
    })?;
    if let Some(stream_id) = stream_id.as_deref() {
        let relation = parse_stream_relation(params)?.unwrap_or(StreamRelation::Opened);
        append_stream_event_for_task(
            &global,
            &scope.project_root,
            stream_id,
            Some(task.id.clone()),
            relation,
            format!("Created task {}: {}", task.id, task.title),
        )?;
    }
    let text = format!(
        "Created global project task {} in {}",
        task.id,
        path.display()
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "create",
            "store_source": storage::WorkStoreSource::GlobalProjectScoped,
            "kind": "task",
            "id": task.id,
            "path": path,
            "project_root": scope.project_root,
            "global_store_root": scope.global_store_root,
            "stream_id": stream_id,
            "item": task,
        }),
        is_error: false,
    })
}

fn list_items(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind_filter = params.get("kind").and_then(|value| value.as_str());
    if kind_filter == Some("task") {
        ensure_global_task_source(params)?;
        return list_global_project_scoped_tasks(store, cwd, params);
    }

    let limit = params
        .get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(50) as usize;
    let mut items = store
        .load_work_items()
        .map_err(|error| Error::Tool(format!("failed to list work items: {error}")))?;
    if let Some(kind_filter) = kind_filter {
        let expected = parse_work_kind(kind_filter)?;
        items.retain(|item| item.kind() == expected);
    }
    if let Some(status_filter) = optional_string(params, "status") {
        items.retain(|item| item_status(item).as_deref() == Some(status_filter.as_str()));
    }
    if let Some(parent_filter) = optional_string(params, "parent_work") {
        items.retain(|item| item_parent(item).as_deref() == Some(parent_filter.as_str()));
    }
    if let Some(path_filter) = optional_string(params, "path") {
        let path = std::path::PathBuf::from(path_filter);
        items.retain(|item| item_matches_path(item, &path));
    }
    items.truncate(limit.max(1));

    let mut text = format!("{} work item(s)", items.len());
    for item in &items {
        text.push_str(&format!(
            "\n- {} {}{}: {}",
            format_work_kind(item.kind()),
            item.id(),
            item_status(item)
                .map(|status| format!(" @{}", status))
                .unwrap_or_default(),
            item_title(item)
        ));
    }
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "list",
            "store_source": "global_project_scoped",
            "items": items,
        }),
        is_error: false,
    })
}

fn list_global_project_scoped_tasks(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind_filter = params.get("kind").and_then(|value| value.as_str());
    if !matches!(kind_filter, Some("task") | None) {
        return Err(Error::Tool(
            "store_source=global_project_scoped currently supports list kind=task only".to_string(),
        ));
    }
    let limit = params
        .get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(50) as usize;
    let scope = storage::WorkScope::for_project_dir(cwd);
    let global = GlobalWorkStore::open(scope.global_store_root.clone());
    let mut tasks = global
        .tasks_for_project(&scope.project_root)
        .map_err(|error| Error::Tool(format!("failed to list global project tasks: {error}")))?;
    for task in store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to list global work graph tasks: {error}")))?
    {
        if let Some(existing) = tasks.iter_mut().find(|existing| existing.id == task.id) {
            *existing = task;
        } else {
            tasks.push(task);
        }
    }
    if let Some(status_filter) = optional_string(params, "status") {
        let expected = parse_task_status(&status_filter)?;
        tasks.retain(|task| task.status == expected);
    }
    if let Some(parent_filter) = optional_string(params, "parent_work") {
        tasks.retain(|task| {
            task.parent
                .as_ref()
                .is_some_and(|parent| parent.0 == parent_filter)
        });
    }
    if let Some(path_filter) = optional_string(params, "path") {
        let path = std::path::PathBuf::from(path_filter);
        tasks.retain(|task| {
            task.source_refs
                .iter()
                .any(|source| source.reference == path.to_string_lossy())
        });
    }
    tasks.truncate(limit.max(1));

    let mut text = format!("{} global project task(s)", tasks.len());
    for task in &tasks {
        text.push_str(&format!(
            "\n- task {} @{}: {}",
            task.id,
            format_task_status_value(task.status),
            task.title
        ));
    }
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "list",
            "kind": "task",
            "store_source": storage::WorkStoreSource::GlobalProjectScoped,
            "project_root": scope.project_root,
            "global_store_root": scope.global_store_root,
            "items": tasks,
        }),
        is_error: false,
    })
}

fn append_stream_event_for_task(
    global: &GlobalWorkStore,
    project_root: &std::path::Path,
    stream_id: &str,
    work_id: Option<WorkId>,
    relation: StreamRelation,
    summary: String,
) -> Result<()> {
    global
        .append_stream_event(&StreamEvent {
            project_root: project_root.to_path_buf(),
            stream_id: stream_id.to_string(),
            work_id,
            relation,
            summary,
        })
        .map_err(|error| Error::Tool(format!("failed to append project stream event: {error}")))?;
    Ok(())
}

fn parse_stream_relation(params: &serde_json::Value) -> Result<Option<StreamRelation>> {
    let Some(relation) = params.get("relation").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    let relation = match relation {
        "opened" => StreamRelation::Opened,
        "continues" => StreamRelation::Continues,
        "follow_up_to" => StreamRelation::FollowUpTo,
        "supersedes" => StreamRelation::Supersedes,
        "related_to" => StreamRelation::RelatedTo,
        "derived_from" => StreamRelation::DerivedFrom,
        "regression_of" => StreamRelation::RegressionOf,
        "closed" => StreamRelation::Closed,
        other => {
            return Err(Error::Tool(format!(
                "unsupported stream relation `{other}`; expected opened, continues, follow_up_to, supersedes, related_to, derived_from, regression_of, or closed"
            )))
        }
    };
    Ok(Some(relation))
}

fn ensure_global_task_source(params: &serde_json::Value) -> Result<()> {
    match params.get("store_source").and_then(|value| value.as_str()) {
        None | Some("global_project_scoped") => Ok(()),
        Some("project_local") => Err(Error::Tool(
            "project-local work stores are migration input only; normal task actions use the global project-scoped store".into(),
        )),
        Some(other) => Err(Error::Tool(format!(
            "unsupported store_source `{other}`; normal task actions use global_project_scoped"
        ))),
    }
}

fn work_tree(store: &WorkStore) -> Result<ToolOutput> {
    let tasks = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load work tree: {error}")))?;
    let tree = build_work_tree(&tasks);
    Ok(tool_output(
        format!(
            "Native imp-work tree contains {} root item(s) and {} warning(s)",
            tree.nodes.len(),
            tree.warnings.len()
        ),
        json!({
            "action": "tree",
            "tree": tree,
        }),
    ))
}

fn verify_task(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let id = required_str(params, "id")?;
    let tasks = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load task for verify: {error}")))?;
    let task = tasks
        .iter()
        .find(|task| task.id.0 == id)
        .ok_or_else(|| Error::Tool(format!("work item `{id}` not found")))?;
    let checks_passed = params
        .get("checks_passed")
        .and_then(|value| value.as_u64())
        .unwrap_or(task.checks.len() as u64) as usize;
    let checks_failed = params
        .get("checks_failed")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;

    let mut results = Vec::new();
    for (index, check) in task.checks.iter().enumerate() {
        results.push(CheckResult {
            check_id: Some(check.id.clone()),
            command: check.command.clone(),
            passed: index < checks_passed && checks_failed == 0,
            output_ref: None,
        });
    }
    for index in 0..checks_failed {
        results.push(CheckResult {
            check_id: Some(WorkId::from(format!("{id}-failed-check-{index}"))),
            command: None,
            passed: false,
            output_ref: None,
        });
    }
    let verify = summarize_checks(results);
    Ok(tool_output(
        if verify.passed {
            format!("Verify passed for {id}")
        } else {
            format!("Verify failed for {id}")
        },
        json!({
            "action": "verify",
            "id": id,
            "verify": verify,
        }),
    ))
}

fn close_task_action(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let id = required_str(params, "id")?;
    let summary = optional_string(params, "summary").unwrap_or_else(|| "closed".to_string());
    let tasks = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load task for close: {error}")))?;
    let task = tasks
        .into_iter()
        .find(|task| task.id.0 == id)
        .ok_or_else(|| Error::Tool(format!("work item `{id}` not found")))?;
    let force_reason = optional_string(params, "force_reason").or_else(|| {
        params
            .get("force")
            .and_then(|value| value.as_bool())
            .filter(|force| *force)
            .map(|_| "forced without detailed reason".to_string())
    });
    let verify = if task.checks.is_empty() {
        None
    } else {
        let checks_passed = params
            .get("checks_passed")
            .and_then(|value| value.as_u64())
            .unwrap_or(task.checks.len() as u64) as usize;
        let checks_failed = params
            .get("checks_failed")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize;
        let mut results = Vec::new();
        for (index, check) in task.checks.iter().enumerate() {
            results.push(CheckResult {
                check_id: Some(check.id.clone()),
                command: check.command.clone(),
                passed: index < checks_passed && checks_failed == 0,
                output_ref: None,
            });
        }
        for index in 0..checks_failed {
            results.push(CheckResult {
                check_id: Some(WorkId::from(format!("{id}-failed-check-{index}"))),
                command: None,
                passed: false,
                output_ref: None,
            });
        }
        Some(summarize_checks(results))
    };
    let close = close_task_with_conventions(
        task,
        CloseRequest {
            verify,
            force_reason,
            summary,
            changed_paths: paths_from_params(params),
        },
    )
    .map_err(Error::Tool)?;
    store
        .update_task_status(id, close.task.status)
        .map_err(|error| Error::Tool(format!("failed to update closed task status: {error}")))?;
    store
        .append_run(&close.run)
        .map_err(|error| Error::Tool(format!("failed to append close run: {error}")))?;

    Ok(tool_output(
        format!("Closed native imp-work task {id}"),
        json!({
            "action": "close",
            "task": close.task,
            "run": close.run,
            "forced": close.forced,
        }),
    ))
}

fn fail_task_action(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let id = required_str(params, "id")?;
    let reason = required_string_any(params, &["reason", "summary", "text"])?;
    let tasks = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load task for fail: {error}")))?;
    let task = tasks
        .into_iter()
        .find(|task| task.id.0 == id)
        .ok_or_else(|| Error::Tool(format!("work item `{id}` not found")))?;
    let failed = fail_task_with_conventions(task, reason, optional_string(params, "next_action"));
    store
        .update_task_status(id, failed.task.status)
        .map_err(|error| Error::Tool(format!("failed to update failed task status: {error}")))?;
    store
        .append_memory(&failed.memory)
        .map_err(|error| Error::Tool(format!("failed to append failure memory: {error}")))?;

    Ok(tool_output(
        format!("Marked native imp-work task {id} blocked"),
        json!({
            "action": "fail",
            "task": failed.task,
            "memory": failed.memory,
        }),
    ))
}

fn remember_memory(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let text = required_string_any(params, &["text", "title", "query"])?;
    let input = ConversationMemoryInput {
        text,
        kind: None,
        parent_work: optional_work_id(params, "parent_work"),
        topics: string_array(params, "topics"),
        paths: paths_from_params(params),
        source: optional_string(params, "source").or_else(|| Some("work_tool:remember".into())),
    };
    let classified = capture_conversation_memory(input.clone());
    let path = store
        .capture_conversation_memory(input)
        .map_err(|error| Error::Tool(format!("failed to remember memory: {error}")))
        .map(|_| store.layout().memory_file)?;
    let text = format!(
        "Remembered {} memory {}: {}",
        format_memory_kind_value(classified.kind),
        classified.id,
        classified.text
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "remember",
            "kind": "memory",
            "id": classified.id,
            "memory_kind": classified.kind,
            "path": path,
            "item": classified,
        }),
        is_error: false,
    })
}

fn record_prototype_outcome(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let id = required_str(params, "id")?;
    let prototype = store
        .load_prototypes()
        .map_err(|error| Error::Tool(format!("failed to load prototypes: {error}")))?
        .into_iter()
        .find(|prototype| prototype.id == id)
        .ok_or_else(|| Error::Tool(format!("prototype `{id}` not found")))?;
    let hypothesis_result = params
        .get("hypothesis_result")
        .and_then(|value| value.as_str())
        .map(parse_hypothesis_result)
        .transpose()?
        .unwrap_or(HypothesisResult::NotAssessed);
    let outcome = params
        .get("recommended_action")
        .and_then(|value| value.as_str())
        .map(parse_prototype_outcome)
        .transpose()?
        .ok_or_else(|| Error::Tool("missing `recommended_action` for prototype_outcome".into()))?;
    let summary = required_str(params, "summary")?.to_string();
    let learnings = string_array(params, "memory_updates");
    let followups = string_array(params, "followups");
    let observation = PrototypeObservation {
        prototype_id: prototype.id.clone(),
        question: prototype.question.clone(),
        parent_work: prototype.parent_work.clone(),
        hypothesis: prototype.hypothesis.clone(),
        hypothesis_result,
        outcome,
        summary: summary.clone(),
        evidence_required: prototype.evidence_required.clone(),
        evidence: parse_prototype_evidence(params)?,
        learnings,
        followups,
        sandbox: prototype.sandbox.clone(),
        artifacts: string_array(params, "changed_paths")
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect(),
    };
    let recorded_path = store
        .record_prototype_observation(PrototypeRecordPolicy::Prototype, &observation)
        .map_err(|error| Error::Tool(format!("failed to record prototype observation: {error}")))?;
    if !observation.learnings.is_empty() || !observation.followups.is_empty() {
        let parent_work = observation
            .parent_work
            .clone()
            .unwrap_or_else(|| prototype.id.clone());
        let synthetic = imp_work::WorkOutcome {
            work_id: WorkId::from(parent_work.as_str()),
            outcome: RunOutcome::DoneWithConcerns,
            summary: summary.clone(),
            changed_paths: observation.artifacts.clone(),
            checks_passed: observation.evidence.len(),
            checks_failed: 0,
            memory_updates: observation.learnings.clone(),
            followups: observation.followups.clone(),
        };
        let run = Run {
            id: WorkId::new("R"),
            work_id: Some(synthetic.work_id.clone()),
            context_pack_id: None,
            outcome: synthetic.outcome,
            summary: synthetic.summary.clone(),
            changed_paths: synthetic.changed_paths.clone(),
            checks: Vec::new(),
        };
        let coordinator_summary = imp_work::CoordinatorSummary {
            done: 1,
            recent_outcomes: vec![synthetic.clone()],
            ..imp_work::CoordinatorSummary::default()
        };
        store
            .persist_worker_result(&run, &synthetic, &coordinator_summary)
            .map_err(|error| {
                Error::Tool(format!("failed to persist prototype learnings: {error}"))
            })?;
    }
    let status = prototype_status_for_outcome(outcome);
    let updated = store
        .update_prototype_status(id, status)
        .map_err(|error| Error::Tool(format!("failed to update prototype status: {error}")))?
        .ok_or_else(|| Error::Tool(format!("prototype `{id}` not found while updating status")))?;
    let text = format!(
        "Recorded {:?} prototype outcome for {}: {}",
        outcome, updated.id, updated.title
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "prototype_outcome",
            "kind": "prototype",
            "id": updated.id,
            "status": updated.status,
            "outcome": outcome,
            "hypothesis_result": hypothesis_result,
            "recorded_path": recorded_path,
            "item": updated,
        }),
        is_error: false,
    })
}

fn list_runs(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let limit = params
        .get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(10) as usize;
    let snapshot = store
        .load_coordinator_snapshot()
        .map_err(|error| Error::Tool(format!("failed to load run history: {error}")))?;
    let mut outcomes = snapshot.outcomes;
    outcomes.reverse();
    outcomes.truncate(limit.max(1));
    outcomes.reverse();
    let mut runs = snapshot.runs;
    runs.reverse();
    runs.truncate(limit.max(1));
    runs.reverse();

    let mut text = format!("{} run(s), {} outcome(s)", runs.len(), outcomes.len());
    for outcome in &outcomes {
        text.push_str(&format!(
            "\n- {:?} {}: {}",
            outcome.outcome, outcome.work_id, outcome.summary
        ));
    }
    if let Some(summary) = &snapshot.summary {
        text.push_str(&format!(
            "\nsummary: ready={} leased={} done={} blocked={} failed={} needs_context={}",
            summary.ready,
            summary.leased,
            summary.done,
            summary.blocked,
            summary.failed,
            summary.needs_context
        ));
    }

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "runs",
            "runs": runs,
            "outcomes": outcomes,
            "summary": snapshot.summary,
        }),
        is_error: false,
    })
}

fn ensure_fresh_context(store: &WorkStore, task: &Task) -> Result<()> {
    let context_pack = task
        .context_pack
        .as_ref()
        .ok_or_else(|| Error::Tool(format!("task `{}` has no context_pack", task.id)))?;
    let pack = store
        .load_context_pack(&context_pack.0)
        .map_err(|error| {
            Error::Tool(format!(
                "failed to load context pack `{context_pack}`: {error}"
            ))
        })?
        .ok_or_else(|| Error::Tool(format!("context pack `{context_pack}` not found")))?;
    let rendered = ContextRenderer::render(&pack);
    if rendered.stale {
        return Err(Error::Tool(format!(
            "context pack `{context_pack}` for task `{}` is stale",
            task.id
        )));
    }
    Ok(())
}

fn record_outcome(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind = params
        .get("kind")
        .and_then(|value| value.as_str())
        .unwrap_or("task");
    if kind != "task" {
        return Err(Error::Tool(format!(
            "unsupported outcome kind `{kind}`; currently only task is supported"
        )));
    }
    let id = required_str(params, "id")?;
    let outcome = params
        .get("outcome")
        .and_then(|value| value.as_str())
        .map(parse_run_outcome)
        .transpose()?
        .ok_or_else(|| Error::Tool("missing `outcome` for work outcome".into()))?;
    let summary = required_str(params, "summary")?.to_string();
    let task = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load tasks: {error}")))?
        .into_iter()
        .find(|task| task.id.0 == id)
        .ok_or_else(|| Error::Tool(format!("task `{id}` not found")))?;
    let work_outcome = imp_work::WorkOutcome {
        work_id: task.id.clone(),
        outcome,
        summary: summary.clone(),
        changed_paths: string_array(params, "changed_paths")
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect(),
        checks_passed: params
            .get("checks_passed")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize,
        checks_failed: params
            .get("checks_failed")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as usize,
        memory_updates: string_array(params, "memory_updates"),
        followups: string_array(params, "followups"),
    };
    let run = Run {
        id: WorkId::new("R"),
        work_id: Some(task.id.clone()),
        context_pack_id: task.context_pack.clone(),
        outcome,
        summary,
        changed_paths: work_outcome.changed_paths.clone(),
        checks: Vec::new(),
    };
    let task_status = task_status_for_outcome(outcome);
    let updated_task = store
        .update_task_status(id, task_status)
        .map_err(|error| Error::Tool(format!("failed to update task status: {error}")))?
        .ok_or_else(|| Error::Tool(format!("task `{id}` not found while updating status")))?;
    if let Some(scope) = scope_from_global_project_store(store.root()) {
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        let _ = global.update_task_for_project(&scope.project_root, &task.id, |task| {
            task.status = updated_task.status;
        });
    }
    let coordinator_summary = imp_work::CoordinatorSummary {
        done: usize::from(matches!(
            outcome,
            RunOutcome::Done | RunOutcome::DoneWithConcerns
        )),
        blocked: usize::from(outcome == RunOutcome::Blocked),
        failed: usize::from(outcome == RunOutcome::Failed),
        needs_context: usize::from(outcome == RunOutcome::NeedsContext),
        recent_outcomes: vec![work_outcome.clone()],
        ..imp_work::CoordinatorSummary::default()
    };
    let persistence = store
        .persist_worker_result(&run, &work_outcome, &coordinator_summary)
        .map_err(|error| Error::Tool(format!("failed to persist outcome: {error}")))?;
    let released_leases = store
        .release_leases_for_work(&task.id)
        .map_err(|error| Error::Tool(format!("failed to release task leases: {error}")))?;
    let stream_id = optional_string(params, "stream_id");
    if let Some(stream_id) = stream_id.as_deref() {
        let scope = storage::WorkScope::for_project_dir(cwd);
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        append_stream_event_for_task(
            &global,
            &scope.project_root,
            stream_id,
            Some(task.id.clone()),
            parse_stream_relation(params)?.unwrap_or(StreamRelation::Closed),
            format!(
                "Outcome {:?} for task {}: {}",
                outcome, task.id, work_outcome.summary
            ),
        )?;
    }
    let text = format!(
        "Recorded {:?} outcome for task {}: {}",
        outcome, updated_task.id, updated_task.title
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "outcome",
            "kind": "task",
            "id": updated_task.id,
            "status": updated_task.status,
            "outcome": outcome,
            "run_id": run.id,
            "item": updated_task,
            "persistence": {
                "run_path": persistence.run_path,
                "outcome_path": persistence.outcome_path,
                "summary_path": persistence.summary_path,
                "memory_paths": persistence.memory_paths,
                "followup_task_path": persistence.followup_task_path,
                "stale_context_paths": persistence.stale_context_paths,
                "released_leases": released_leases,
            },
            "stream_id": stream_id,
        }),
        is_error: false,
    })
}

fn validate_store(store: &WorkStore) -> Result<ToolOutput> {
    let report = store
        .validate()
        .map_err(|error| Error::Tool(format!("failed to validate work store: {error}")))?;
    let error_count = report
        .issues
        .iter()
        .filter(|issue| issue.severity == imp_work::WorkValidationSeverity::Error)
        .count();
    let warning_count = report
        .issues
        .iter()
        .filter(|issue| issue.severity == imp_work::WorkValidationSeverity::Warning)
        .count();
    let mut text = format!(
        "validation: {} error(s), {} warning(s)",
        error_count, warning_count
    );
    for issue in &report.issues {
        text.push_str(&format!(
            "\n- {:?} {}{}: {}",
            issue.severity,
            issue.code,
            issue
                .item_id
                .as_ref()
                .map(|id| format!(" ({id})"))
                .unwrap_or_default(),
            issue.message
        ));
    }
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "validate",
            "ok": report.is_ok(),
            "error_count": error_count,
            "warning_count": warning_count,
            "issues": report.issues,
        }),
        is_error: !report.is_ok(),
    })
}

fn work_guide(params: &serde_json::Value) -> Result<ToolOutput> {
    let topic = params
        .get("topic")
        .and_then(|value| value.as_str())
        .unwrap_or("overview");
    let text = match topic {
        "overview" => "imp-work guide: use native work for durable project work. Normal storage is global project-scoped under ~/.imp/work and keyed by canonical project_root; project-local .imp/work is migration input only. Create work for multi-step, verifiable, resumable, or handoff-worthy tasks; avoid noisy one-shot items. Standard flow: create/claim/context/verify/outcome/close or fail.",
        "task" => "imp-work task guide: create tasks with action=create kind=task, clear title, acceptance, relevant paths, and optional stream_id/relation. Use next/claim before execution when coordinating. Record notes/outcomes rather than relying on chat memory.",
        "lifecycle" => "imp-work lifecycle guide: work moves through ready/in_progress/done/blocked states. Verify checked work before close. Use outcome for structured evidence, close for completed work, and fail when blocked with a concrete next_action.",
        "global_store" => "imp-work global store guide: the normal backend is ~/.imp/work scoped by project_root. Task create/list/show/update and lifecycle flows use the global project graph. store_source=project_local is rejected for normal task actions; use migration for old local data.",
        "streams" => "imp-work streams guide: use stream_id and relation to preserve continuity across closed tasks and follow-ups. Supported relations include continues, follow_up_to, supersedes, related_to, derived_from, regression_of, opened, and closed.",
        "migration" => "imp-work migration guide: migration is transitional offline tooling, not a normal work action. Run scripts/migrate-mana-to-imp-work --dry-run, inspect counts/conflicts/warnings, then rerun with --write when ready.",
        "verification" => "imp-work verification guide: run the narrowest relevant check early and last. Do not close checked work unless verify passed or force-close with a clear reason. Failed checks are diagnostic evidence to fix or report.",
        "orchestration" => "imp-work orchestration guide: use next to select ready work, claim to lease it, context to prepare handoff state, outcome to record structured results, and close/fail to end the lifecycle. Global leases and runs should make interrupted work recoverable.",
        other => {
            return Err(Error::Tool(format!(
                "unsupported work guide topic `{other}`; expected overview, task, lifecycle, global_store, streams, migration, verification, or orchestration"
            )))
        }
    };
    Ok(tool_output(
        text.to_string(),
        json!({
            "action": "guide",
            "topic": topic,
            "topics": ["overview", "task", "lifecycle", "global_store", "streams", "migration", "verification", "orchestration"],
        }),
    ))
}

fn work_scope(_store: &WorkStore, cwd: &std::path::Path) -> Result<ToolOutput> {
    let scope = storage::WorkScope::for_project_dir(cwd);
    let text = format!(
        "imp-work scope\n- active source: global-project-scoped\n- project root: {}\n- local store: {}\n- global store: {}\n- migration: {}",
        scope.project_root.display(),
        scope.local_store_root.display(),
        scope.global_store_root.display(),
        scope.migration_status()
    );
    Ok(tool_output(
        text,
        json!({
            "action": "scope",
            "active_source": scope.active_source,
            "project_root": scope.project_root,
            "local_store_root": scope.local_store_root,
            "global_store_root": scope.global_store_root,
            "global_store_available": true,
            "writes_target": scope.writes_target,
            "migration_status": scope.migration_status(),
        }),
    ))
}

fn stream_history_for_task(cwd: &std::path::Path, task: &Task) -> Result<Vec<String>> {
    let scope = storage::WorkScope::for_project_dir(cwd);
    let global = GlobalWorkStore::open(scope.global_store_root.clone());
    let stream_id = match global
        .find_task_for_project(&scope.project_root, &task.id)
        .map_err(|error| {
            Error::Tool(format!(
                "failed to load global task stream metadata: {error}"
            ))
        })?
        .and_then(|record| record.stream_id)
        .or_else(|| {
            global
                .stream_id_for_work(&scope.project_root, &task.id)
                .ok()
                .flatten()
        }) {
        Some(stream_id) => stream_id,
        None => return Ok(Vec::new()),
    };
    let events = global
        .stream_events_for_project_stream(&scope.project_root, &stream_id)
        .map_err(|error| Error::Tool(format!("failed to load global stream history: {error}")))?;
    Ok(events
        .into_iter()
        .map(|event| format!("{:?}: {}", event.relation, event.summary))
        .collect())
}

fn edit_dependency(store: &WorkStore, params: &serde_json::Value, add: bool) -> Result<ToolOutput> {
    let id = required_str(params, "id")?;
    let dependency_id = required_str(params, "dependency_id")?;
    let dependency = WorkId::from(dependency_id);
    let task = if add {
        store
            .add_task_dependency(id, dependency.clone())
            .map_err(|error| Error::Tool(format!("failed to add dependency: {error}")))?
    } else {
        store
            .remove_task_dependency(id, &dependency)
            .map_err(|error| Error::Tool(format!("failed to remove dependency: {error}")))?
    }
    .ok_or_else(|| Error::Tool(format!("task `{id}` not found")))?;
    let action = if add { "dep_add" } else { "dep_remove" };
    let text = if add {
        format!("Added dependency {dependency_id} to task {}", task.id)
    } else {
        format!("Removed dependency {dependency_id} from task {}", task.id)
    };
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": action,
            "kind": "task",
            "id": task.id,
            "dependency_id": dependency_id,
            "item": task,
        }),
        is_error: false,
    })
}

fn search_memory(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let query = ConversationMemoryQuery {
        text: optional_string(params, "query").or_else(|| optional_string(params, "text")),
        topic: optional_string(params, "topic"),
        parent_work: optional_work_id(params, "parent_work"),
        path: optional_string(params, "path").map(std::path::PathBuf::from),
        limit: params
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(10) as usize,
    };
    let matches = store
        .retrieve_memory(query)
        .map_err(|error| Error::Tool(format!("failed to search memory: {error}")))?;
    let mut text = format!("{} memory match(es)", matches.len());
    for memory_match in &matches {
        text.push_str(&format!(
            "\n- score {} {}: {}",
            memory_match.score, memory_match.memory.id, memory_match.memory.text
        ));
        if !memory_match.reasons.is_empty() {
            text.push_str(&format!(" [{}]", memory_match.reasons.join(", ")));
        }
    }
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "search",
            "matches": matches,
        }),
        is_error: false,
    })
}

fn claim_task(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let scheduler = store
        .load_scheduler()
        .map_err(|error| Error::Tool(format!("failed to load scheduler: {error}")))?;
    let require_context = params
        .get("require_context")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let task_id = if let Some(id) = optional_string(params, "id") {
        let ready = scheduler.ready_queue();
        let Some(task) = ready.iter().find(|task| task.id.0 == id) else {
            return Err(Error::Tool(format!(
                "task `{id}` is not ready to claim; it may be missing, non-ready, already claimed, or blocked by dependencies"
            )));
        };
        if require_context {
            ensure_fresh_context(store, task)?;
        }
        id
    } else {
        let ready = scheduler.ready_queue();
        let task = if require_context {
            ready
                .into_iter()
                .find(|task| ensure_fresh_context(store, task).is_ok())
        } else {
            ready.first().copied()
        };
        task.map(|task| task.id.0.clone()).ok_or_else(|| {
            if require_context {
                Error::Tool("no ready task with fresh context available to claim".into())
            } else {
                Error::Tool("no ready task available to claim".into())
            }
        })?
    };

    let task = store
        .update_task_status(&task_id, TaskStatus::Active)
        .map_err(|error| Error::Tool(format!("failed to claim task: {error}")))?
        .ok_or_else(|| Error::Tool(format!("task `{task_id}` not found")))?;
    let lease = imp_work::Lease {
        id: WorkId::new("L"),
        work_id: task.id.clone(),
        worker_id: optional_string(params, "worker_id").unwrap_or_else(|| "imp".into()),
        worktree: optional_string(params, "worktree").map(std::path::PathBuf::from),
        path_locks: string_array(params, "path_locks")
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect(),
    };
    let lease_path = store
        .append_work_item(&WorkItem::Lease(lease.clone()))
        .map_err(|error| Error::Tool(format!("failed to persist claim lease: {error}")))?;
    let text = format!("Claimed task {}: {}", task.id, task.title);
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "claim",
            "kind": "task",
            "id": task.id,
            "status": task.status,
            "lease_id": lease.id,
            "lease_path": lease_path,
            "lease": lease,
            "item": task,
        }),
        is_error: false,
    })
}

fn update_item(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind = required_str(params, "kind")?;
    if kind == "task" {
        ensure_global_task_source(params)?;
        return update_global_project_scoped_task(store, cwd, params);
    }
    let id = required_str(params, "id")?;
    match kind {
        "task" => {
            let status = params
                .get("status")
                .and_then(|value| value.as_str())
                .map(parse_task_status)
                .transpose()?
                .ok_or_else(|| Error::Tool("missing `status` for task update".into()))?;
            let task = store
                .update_task_status(id, status)
                .map_err(|error| Error::Tool(format!("failed to update task: {error}")))?
                .ok_or_else(|| Error::Tool(format!("task `{id}` not found")))?;
            let text = format!("Updated task {} to {:?}", task.id, task.status);
            Ok(ToolOutput {
                content: vec![imp_llm::ContentBlock::Text { text }],
                details: json!({
                    "action": "update",
                    "kind": "task",
                    "id": task.id,
                    "status": task.status,
                    "item": task,
                }),
                is_error: false,
            })
        }
        "decision" => {
            let status = params
                .get("status")
                .and_then(|value| value.as_str())
                .map(parse_decision_status)
                .transpose()?
                .ok_or_else(|| Error::Tool("missing `status` for decision update".into()))?;
            let decision = store
                .update_decision_status(id, status)
                .map_err(|error| Error::Tool(format!("failed to update decision: {error}")))?
                .ok_or_else(|| Error::Tool(format!("decision `{id}` not found")))?;
            let text = format!("Updated decision {} to {:?}", decision.id, decision.status);
            Ok(ToolOutput {
                content: vec![imp_llm::ContentBlock::Text { text }],
                details: json!({
                    "action": "update",
                    "kind": "decision",
                    "id": decision.id,
                    "status": decision.status,
                    "item": decision,
                }),
                is_error: false,
            })
        }
        "prototype" => {
            let status = params
                .get("status")
                .and_then(|value| value.as_str())
                .map(parse_prototype_status)
                .transpose()?
                .ok_or_else(|| Error::Tool("missing `status` for prototype update".into()))?;
            let prototype = store
                .update_prototype_status(id, status)
                .map_err(|error| Error::Tool(format!("failed to update prototype: {error}")))?
                .ok_or_else(|| Error::Tool(format!("prototype `{id}` not found")))?;
            let text = format!("Updated prototype {} to {:?}", prototype.id, prototype.status);
            Ok(ToolOutput {
                content: vec![imp_llm::ContentBlock::Text { text }],
                details: json!({
                    "action": "update",
                    "kind": "prototype",
                    "id": prototype.id,
                    "status": prototype.status,
                    "item": prototype,
                }),
                is_error: false,
            })
        }
        other => Err(Error::Tool(format!(
            "unsupported update kind `{other}`; currently task, decision, and prototype are supported"
        ))),
    }
}

fn update_global_project_scoped_task(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind = required_str(params, "kind")?;
    if kind != "task" {
        return Err(Error::Tool(format!(
            "store_source=global_project_scoped currently supports update kind=task only; got `{kind}`"
        )));
    }
    let id = WorkId::from(required_str(params, "id")?);
    let status = params
        .get("status")
        .and_then(|value| value.as_str())
        .map(parse_task_status)
        .transpose()?;
    let title = optional_string(params, "title");
    if status.is_none() && title.is_none() {
        return Err(Error::Tool(
            "global task update requires at least one of `status` or `title`".into(),
        ));
    }
    let scope = storage::WorkScope::for_project_dir(cwd);
    let global = GlobalWorkStore::open(scope.global_store_root.clone());
    let Some((task, path)) = global
        .update_task_for_project(&scope.project_root, &id, |task| {
            if let Some(status) = status {
                task.status = status;
            }
            if let Some(title) = title {
                task.title = title;
            }
        })
        .map_err(|error| Error::Tool(format!("failed to update global project task: {error}")))?
    else {
        return Err(Error::Tool(format!(
            "global project task `{}` not found for project {}",
            id,
            scope.project_root.display()
        )));
    };
    store
        .update_task_status(&id.0, task.status)
        .map_err(|error| {
            Error::Tool(format!("failed to update global work graph task: {error}"))
        })?;
    let text = format!(
        "Updated global project task {} to {:?}",
        task.id, task.status
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "update",
            "kind": "task",
            "store_source": storage::WorkStoreSource::GlobalProjectScoped,
            "id": task.id,
            "status": task.status,
            "path": path,
            "project_root": scope.project_root,
            "global_store_root": scope.global_store_root,
            "item": task,
        }),
        is_error: false,
    })
}

fn show_global_project_scoped_task(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let id = WorkId::from(required_str(params, "id")?);
    let scope = storage::WorkScope::for_project_dir(cwd);
    let global = GlobalWorkStore::open(scope.global_store_root.clone());
    let task = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load global work graph tasks: {error}")))?
        .into_iter()
        .find(|task| task.id == id);
    let record = if let Some(task) = task {
        imp_work::ProjectScopedTask {
            project_root: scope.project_root.clone(),
            stream_id: None,
            task,
        }
    } else {
        let Some(record) = global
            .find_task_for_project(&scope.project_root, &id)
            .map_err(|error| Error::Tool(format!("failed to show global project task: {error}")))?
        else {
            return Err(Error::Tool(format!(
                "global project task `{}` not found for project {}",
                id,
                scope.project_root.display()
            )));
        };
        record
    };
    let text = format!(
        "global project task {} @{}: {}",
        record.task.id,
        format_task_status_value(record.task.status),
        record.task.title
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "show",
            "kind": "task",
            "store_source": storage::WorkStoreSource::GlobalProjectScoped,
            "id": record.task.id,
            "project_root": scope.project_root,
            "global_store_root": scope.global_store_root,
            "stream_id": record.stream_id,
            "item": record.task,
        }),
        is_error: false,
    })
}

fn show_item(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    if params.get("kind").and_then(|value| value.as_str()) == Some("context_pack") {
        return show_project_local_legacy_item(store, params);
    }
    ensure_global_task_source(params)?;
    match show_global_project_scoped_task(store, cwd, params) {
        Ok(output) => Ok(output),
        Err(error) if params.get("kind").is_some() => Err(error),
        Err(_) => show_project_local_legacy_item(store, params),
    }
}

fn show_project_local_legacy_item(
    store: &WorkStore,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let id = required_str(params, "id")?;
    if let Some(pack) = store
        .load_context_pack(id)
        .map_err(|error| Error::Tool(format!("failed to load context pack: {error}")))?
    {
        let rendered = ContextRenderer::render(&pack);
        let text = format!(
            "context_pack {} for {}\nversion: {}\nstatus: {:?}\nstable_prefix_hash: {}\nblocks: {}",
            pack.id,
            pack.work_id,
            pack.version,
            pack.status,
            rendered.stable_prefix_hash,
            rendered.blocks.len()
        );
        return Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text }],
            details: json!({
                "action": "show",
                "kind": "context_pack",
                "item": pack,
                "stable_prefix_hash": rendered.stable_prefix_hash,
                "full_hash": rendered.full_hash,
                "stale": rendered.stale,
            }),
            is_error: false,
        });
    }

    let item = store
        .load_work_items()
        .map_err(|error| Error::Tool(format!("failed to load work items: {error}")))?
        .into_iter()
        .find(|item| item.id() == id)
        .ok_or_else(|| Error::Tool(format!("work item or context pack `{id}` not found")))?;
    let text = format!(
        "{} {}: {}",
        format_work_kind(item.kind()),
        item.id(),
        item_title(&item)
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "show",
            "kind": format_work_kind(item.kind()),
            "id": item.id(),
            "item": item,
        }),
        is_error: false,
    })
}

fn refresh_context_pack(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let kind = params
        .get("kind")
        .and_then(|value| value.as_str())
        .unwrap_or("task");
    if kind != "task" {
        return Err(Error::Tool(format!(
            "unsupported refresh_context kind `{kind}`; currently only task is supported"
        )));
    }
    let id = required_str(params, "id")?;
    let task = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load tasks: {error}")))?
        .into_iter()
        .find(|task| task.id.0 == id)
        .ok_or_else(|| Error::Tool(format!("task `{id}` not found")))?;
    let previous_id = task
        .context_pack
        .clone()
        .ok_or_else(|| Error::Tool(format!("task `{id}` has no context_pack to refresh")))?;
    let previous = store
        .load_context_pack(&previous_id.0)
        .map_err(|error| Error::Tool(format!("failed to load previous context pack: {error}")))?
        .ok_or_else(|| Error::Tool(format!("context pack `{previous_id}` not found")))?;
    let query = context_memory_query(params, id);
    let next = store
        .refresh_task_context_with_memory(&previous, &task, query, Vec::new())
        .map_err(|error| Error::Tool(format!("failed to refresh context pack: {error}")))?;
    store
        .update_task_context_pack(id, next.id.clone())
        .map_err(|error| Error::Tool(format!("failed to relink refreshed context pack: {error}")))?
        .ok_or_else(|| {
            Error::Tool(format!(
                "task `{id}` not found while relinking context pack"
            ))
        })?;
    if let Some(scope) = scope_from_global_project_store(store.root()) {
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        let _ = global.update_task_for_project(&scope.project_root, &task.id, |task| {
            task.context_pack = Some(next.id.clone());
        });
        let _ = store.update_task_context_pack(id, next.id.clone());
    }
    let _ = store
        .mark_context_pack_stale(&previous_id.0)
        .map_err(|error| Error::Tool(format!("failed to mark previous context stale: {error}")))?;
    let rendered = ContextRenderer::render(&next);
    let text = format!(
        "Refreshed task {} context: {} -> {}\nstable_prefix_hash: {}",
        id, previous_id, next.id, rendered.stable_prefix_hash
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "refresh_context",
            "kind": "task",
            "id": id,
            "previous_context_pack_id": previous_id,
            "context_pack_id": next.id,
            "version": next.version,
            "stable_prefix_hash": rendered.stable_prefix_hash,
            "full_hash": rendered.full_hash,
            "stale": rendered.stale,
        }),
        is_error: false,
    })
}

fn next_ready_tasks(store: &WorkStore, params: &serde_json::Value) -> Result<ToolOutput> {
    let limit = params
        .get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(10) as usize;
    let require_context = params
        .get("require_context")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let scheduler = store
        .load_scheduler()
        .map_err(|error| Error::Tool(format!("failed to load scheduler: {error}")))?;
    let ready = scheduler.ready_queue();
    let ready = ready
        .into_iter()
        .filter(|task| !require_context || ensure_fresh_context(store, task).is_ok())
        .collect::<Vec<_>>();
    let items = ready
        .iter()
        .take(limit.max(1))
        .map(|task| {
            json!({
                "id": task.id,
                "title": task.title,
                "status": task.status,
                "parent": task.parent,
                "acceptance": task.acceptance,
                "checks": task.checks,
                "context_pack": task.context_pack,
            })
        })
        .collect::<Vec<_>>();

    let mut text = format!("{} ready task(s)", items.len());
    for task in ready.iter().take(limit.max(1)) {
        text.push_str(&format!("\n- task {}: {}", task.id, task.title));
    }

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "next",
            "items": items,
        }),
        is_error: false,
    })
}

fn create_context_pack(
    store: &WorkStore,
    cwd: &std::path::Path,
    params: &serde_json::Value,
) -> Result<ToolOutput> {
    let kind = required_str(params, "kind")?;
    let id = required_str(params, "id")?;
    let query = context_memory_query(params, id);
    let (pack, item_title) = match kind {
        "task" => {
            let task = store
                .load_tasks()
                .map_err(|error| Error::Tool(format!("failed to load tasks: {error}")))?
                .into_iter()
                .find(|task| task.id.0 == id)
                .ok_or_else(|| Error::Tool(format!("task `{id}` not found")))?;
            let stream_history = stream_history_for_task(cwd, &task)?;
            let pack = store
                .compile_task_context_with_stream_history(&task, query, Vec::new(), stream_history)
                .map_err(|error| Error::Tool(format!("failed to compile task context: {error}")))?;
            let title = task.title.clone();
            store
                .update_task_context_pack(id, pack.id.clone())
                .map_err(|error| Error::Tool(format!("failed to link task context pack: {error}")))?
                .ok_or_else(|| {
                    Error::Tool(format!("task `{id}` not found while linking context pack"))
                })?;
            if let Some(scope) = scope_from_global_project_store(store.root()) {
                let global = GlobalWorkStore::open(scope.global_store_root.clone());
                let _ = global.update_task_for_project(&scope.project_root, &task.id, |task| {
                    task.context_pack = Some(pack.id.clone());
                });
                let _ = store.update_task_context_pack(id, pack.id.clone());
            }
            (pack, title)
        }
        "prototype" => {
            let prototype = store
                .load_prototypes()
                .map_err(|error| Error::Tool(format!("failed to load prototypes: {error}")))?
                .into_iter()
                .find(|prototype| prototype.id == id)
                .ok_or_else(|| Error::Tool(format!("prototype `{id}` not found")))?;
            let pack = store
                .compile_prototype_context_with_memory(&prototype, query)
                .map_err(|error| {
                    Error::Tool(format!("failed to compile prototype context: {error}"))
                })?;
            (pack, prototype.title)
        }
        other => {
            return Err(Error::Tool(format!(
                "unsupported context kind `{other}`; expected task or prototype"
            )))
        }
    };
    let rendered = ContextRenderer::render(&pack);
    let (json_path, md_path) = store
        .write_context_pack(&pack)
        .map_err(|error| Error::Tool(format!("failed to write context pack: {error}")))?;
    let text = format!(
        "Prepared context pack {} for {} `{}`\nstable_prefix_hash: {}\njson: {}\nmarkdown: {}",
        pack.id,
        kind,
        item_title,
        rendered.stable_prefix_hash,
        json_path.display(),
        md_path.display()
    );
    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "context",
            "kind": kind,
            "id": id,
            "context_pack_id": pack.id,
            "stable_prefix_hash": rendered.stable_prefix_hash,
            "full_hash": rendered.full_hash,
            "stale": rendered.stale,
            "json_path": json_path,
            "markdown_path": md_path,
            "block_count": rendered.blocks.len(),
        }),
        is_error: false,
    })
}

fn context_memory_query(params: &serde_json::Value, id: &str) -> ConversationMemoryQuery {
    ConversationMemoryQuery {
        text: optional_string(params, "query"),
        topic: optional_string(params, "topic"),
        parent_work: Some(WorkId::from(id)),
        path: None,
        limit: params
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(10) as usize,
    }
}

fn slug_fragment(text: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
        if slug.len() >= 40 {
            break;
        }
    }
    slug.trim_matches('-').to_string()
}

fn readable_work_id(prefix: &str, title: &str) -> WorkId {
    let slug = slug_fragment(title);
    if slug.is_empty() {
        WorkId::new(prefix)
    } else {
        WorkId::from(format!("{prefix}-{slug}"))
    }
}

fn unique_task_id(store: &WorkStore, base: WorkId) -> Result<WorkId> {
    let existing = store
        .load_tasks()
        .map_err(|error| Error::Tool(format!("failed to load existing task ids: {error}")))?;
    if existing.iter().all(|task| task.id != base) {
        return Ok(base);
    }

    for suffix in 2..=999 {
        let candidate = WorkId::from(format!("{}-{suffix}", base.0));
        if existing.iter().all(|task| task.id != candidate) {
            return Ok(candidate);
        }
    }

    Err(Error::Tool(format!(
        "failed to allocate unique task id for `{}` after 999 attempts",
        base
    )))
}

fn build_task(store: &WorkStore, params: &serde_json::Value) -> Result<Task> {
    let title = required_title(params)?;
    let mut task = Task::new(title);
    task.id = unique_task_id(store, readable_work_id("T", &task.title))?;
    task.status = params
        .get("status")
        .and_then(|value| value.as_str())
        .map(parse_task_status)
        .transpose()?
        .unwrap_or(TaskStatus::Todo);
    task.parent =
        optional_work_id(params, "parent_work").or_else(|| optional_work_id(params, "parent"));
    task.acceptance = string_array(params, "acceptance");
    for dependency in string_array(params, "depends_on")
        .into_iter()
        .chain(string_array(params, "dependencies"))
    {
        task.links.push(imp_work::Link {
            kind: imp_work::LinkKind::DependsOn,
            target: WorkId::from(dependency.as_str()),
        });
    }
    for check in string_array(params, "checks")
        .into_iter()
        .chain(optional_string(params, "verify"))
    {
        task.checks.push(imp_work::Check {
            id: WorkId::new("C"),
            kind: imp_work::CheckKind::Command,
            description: check.clone(),
            command: Some(check),
        });
    }
    Ok(task)
}

fn build_epic(params: &serde_json::Value) -> Result<Epic> {
    let mut epic = Epic::new(required_title(params)?);
    epic.status = params
        .get("status")
        .and_then(|value| value.as_str())
        .map(parse_task_status)
        .transpose()?
        .unwrap_or(TaskStatus::Todo);
    epic.intent = optional_string(params, "text");
    epic.acceptance = string_array(params, "acceptance");
    Ok(epic)
}

fn build_memory(params: &serde_json::Value) -> Result<MemoryItem> {
    let text = required_string_any(params, &["text", "title"])?;
    let kind = params
        .get("memory_kind")
        .and_then(|value| value.as_str())
        .map(parse_memory_kind)
        .transpose()?
        .unwrap_or(MemoryKind::Note);
    let mut memory = MemoryItem::new(kind, text);
    memory.parent_work = optional_work_id(params, "parent_work");
    memory.topics = string_array(params, "topics");
    memory.paths = paths_from_params(params);
    Ok(memory)
}

fn build_decision(params: &serde_json::Value) -> Result<Decision> {
    Ok(Decision {
        id: WorkId::new("D"),
        title: required_title(params)?,
        status: params
            .get("status")
            .and_then(|value| value.as_str())
            .map(parse_decision_status)
            .transpose()?
            .unwrap_or(DecisionStatus::Proposed),
        rationale: optional_string(params, "rationale"),
        parent_work: optional_work_id(params, "parent_work"),
        source_refs: Vec::new(),
    })
}

fn build_prototype(params: &serde_json::Value) -> Result<Prototype> {
    let question = required_string_any(params, &["text", "title"])?;
    let title = optional_string(params, "title").unwrap_or_else(|| question.clone());
    let sandbox = optional_string(params, "sandbox")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(".tmp/imp-prototypes"));
    let mut prototype = Prototype::new(title, question, sandbox)
        .with_evidence_required(string_array(params, "evidence_required"));
    prototype.parent_work = optional_string(params, "parent_work");
    prototype.hypothesis = optional_string(params, "hypothesis");
    prototype.status = params
        .get("status")
        .and_then(|value| value.as_str())
        .map(parse_prototype_status)
        .transpose()?
        .unwrap_or(PrototypeStatus::Planned);
    Ok(prototype)
}

fn tool_output(content: impl Into<String>, details: serde_json::Value) -> ToolOutput {
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: content.into(),
        }],
        details,
        is_error: false,
    }
}

fn required_str<'a>(params: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::Tool(format!("missing `{key}` parameter")))
}

fn required_title(params: &serde_json::Value) -> Result<String> {
    required_string_any(params, &["title", "text"])
}

fn required_string_any(params: &serde_json::Value, keys: &[&str]) -> Result<String> {
    for key in keys {
        if let Some(value) = optional_string(params, key) {
            return Ok(value);
        }
    }
    Err(Error::Tool(format!("missing one of: {}", keys.join(", "))))
}

fn optional_string(params: &serde_json::Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn optional_work_id(params: &serde_json::Value, key: &str) -> Option<WorkId> {
    optional_string(params, key).map(WorkId::from)
}

fn string_array(params: &serde_json::Value, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::trim))
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_prototype_evidence(params: &serde_json::Value) -> Result<Vec<PrototypeEvidence>> {
    let Some(items) = params.get("evidence").and_then(|value| value.as_array()) else {
        return Ok(Vec::new());
    };
    let mut evidence = Vec::with_capacity(items.len());
    for item in items {
        let claim = item
            .get("claim")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Error::Tool("prototype evidence item missing `claim`".into()))?;
        let proof = item
            .get("proof")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| Error::Tool("prototype evidence item missing `proof`".into()))?;
        let artifact = item
            .get("artifact")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from);
        evidence.push(PrototypeEvidence {
            claim: claim.to_string(),
            proof: proof.to_string(),
            artifact,
        });
    }
    Ok(evidence)
}

fn parse_hypothesis_result(value: &str) -> Result<HypothesisResult> {
    match value {
        "supported" => Ok(HypothesisResult::Supported),
        "refuted" => Ok(HypothesisResult::Refuted),
        "inconclusive" => Ok(HypothesisResult::Inconclusive),
        "not_assessed" => Ok(HypothesisResult::NotAssessed),
        other => Err(Error::Tool(format!(
            "unsupported hypothesis_result `{other}`"
        ))),
    }
}

fn parse_prototype_outcome(value: &str) -> Result<PrototypeOutcome> {
    match value {
        "promote" => Ok(PrototypeOutcome::Promote),
        "discard" => Ok(PrototypeOutcome::Discard),
        "iterate" => Ok(PrototypeOutcome::Iterate),
        "inconclusive" => Ok(PrototypeOutcome::Inconclusive),
        other => Err(Error::Tool(format!(
            "unsupported recommended_action `{other}`"
        ))),
    }
}

fn prototype_status_for_outcome(outcome: PrototypeOutcome) -> PrototypeStatus {
    match outcome {
        PrototypeOutcome::Promote => PrototypeStatus::Promoted,
        PrototypeOutcome::Discard => PrototypeStatus::Discarded,
        PrototypeOutcome::Iterate | PrototypeOutcome::Inconclusive => PrototypeStatus::Observed,
    }
}

fn parse_run_outcome(value: &str) -> Result<RunOutcome> {
    match value {
        "done" => Ok(RunOutcome::Done),
        "done_with_concerns" | "done-with-concerns" => Ok(RunOutcome::DoneWithConcerns),
        "blocked" => Ok(RunOutcome::Blocked),
        "needs_context" | "needs-context" => Ok(RunOutcome::NeedsContext),
        "failed" => Ok(RunOutcome::Failed),
        other => Err(Error::Tool(format!("unsupported outcome `{other}`"))),
    }
}

fn task_status_for_outcome(outcome: RunOutcome) -> TaskStatus {
    match outcome {
        RunOutcome::Done | RunOutcome::DoneWithConcerns => TaskStatus::Done,
        RunOutcome::Blocked => TaskStatus::Blocked,
        RunOutcome::NeedsContext | RunOutcome::Failed => TaskStatus::Review,
    }
}

fn paths_from_params(params: &serde_json::Value) -> Vec<std::path::PathBuf> {
    let mut paths = string_array(params, "paths")
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect::<Vec<_>>();
    if let Some(path) = optional_string(params, "path") {
        paths.push(std::path::PathBuf::from(path));
    }
    paths
}

fn parse_work_kind(value: &str) -> Result<WorkKind> {
    match value {
        "epic" => Ok(WorkKind::Epic),
        "task" => Ok(WorkKind::Task),
        "memory" => Ok(WorkKind::Memory),
        "decision" => Ok(WorkKind::Decision),
        "prototype" => Ok(WorkKind::Prototype),
        "context_pack" | "context-pack" => Ok(WorkKind::ContextPack),
        "lease" => Ok(WorkKind::Lease),
        other => Err(Error::Tool(format!("unsupported work kind `{other}`"))),
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value {
        "todo" => Ok(TaskStatus::Todo),
        "ready" => Ok(TaskStatus::Ready),
        "active" => Ok(TaskStatus::Active),
        "blocked" => Ok(TaskStatus::Blocked),
        "review" => Ok(TaskStatus::Review),
        "done" => Ok(TaskStatus::Done),
        "dropped" => Ok(TaskStatus::Dropped),
        other => Err(Error::Tool(format!("unsupported task status `{other}`"))),
    }
}

fn parse_memory_kind(value: &str) -> Result<MemoryKind> {
    match value {
        "fact" => Ok(MemoryKind::Fact),
        "preference" => Ok(MemoryKind::Preference),
        "decision" => Ok(MemoryKind::Decision),
        "follow_up" | "follow-up" => Ok(MemoryKind::FollowUp),
        "note" => Ok(MemoryKind::Note),
        "prototype_learning" | "prototype-learning" => Ok(MemoryKind::PrototypeLearning),
        other => Err(Error::Tool(format!("unsupported memory kind `{other}`"))),
    }
}

fn parse_decision_status(value: &str) -> Result<DecisionStatus> {
    match value {
        "proposed" => Ok(DecisionStatus::Proposed),
        "accepted" => Ok(DecisionStatus::Accepted),
        "rejected" => Ok(DecisionStatus::Rejected),
        "superseded" => Ok(DecisionStatus::Superseded),
        other => Err(Error::Tool(format!(
            "unsupported decision status `{other}`"
        ))),
    }
}

fn parse_prototype_status(value: &str) -> Result<PrototypeStatus> {
    match value {
        "planned" => Ok(PrototypeStatus::Planned),
        "running" => Ok(PrototypeStatus::Running),
        "observed" => Ok(PrototypeStatus::Observed),
        "promoted" => Ok(PrototypeStatus::Promoted),
        "discarded" => Ok(PrototypeStatus::Discarded),
        other => Err(Error::Tool(format!(
            "unsupported prototype status `{other}`"
        ))),
    }
}

fn format_work_kind(kind: WorkKind) -> &'static str {
    match kind {
        WorkKind::Epic => "epic",
        WorkKind::Task => "task",
        WorkKind::Memory => "memory",
        WorkKind::Decision => "decision",
        WorkKind::Prototype => "prototype",
        WorkKind::Check => "check",
        WorkKind::Run => "run",
        WorkKind::Lease => "lease",
        WorkKind::ContextPack => "context_pack",
    }
}

fn format_memory_kind_value(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Fact => "fact",
        MemoryKind::Preference => "preference",
        MemoryKind::Decision => "decision",
        MemoryKind::FollowUp => "follow_up",
        MemoryKind::Note => "note",
        MemoryKind::PrototypeLearning => "prototype_learning",
    }
}

fn item_matches_path(item: &WorkItem, path: &std::path::Path) -> bool {
    match item {
        WorkItem::Task(item) => item
            .source_refs
            .iter()
            .any(|source| source_matches_path(source, path)),
        WorkItem::Memory(item) => item
            .paths
            .iter()
            .any(|item_path| paths_conflict(item_path, path)),
        WorkItem::Decision(item) => item
            .source_refs
            .iter()
            .any(|source| source_matches_path(source, path)),
        WorkItem::Prototype(item) => paths_conflict(&item.sandbox, path),
        WorkItem::ContextPack(item) => item
            .source_refs
            .iter()
            .any(|source| source_matches_path(source, path)),
        WorkItem::Run(item) => item
            .changed_paths
            .iter()
            .any(|item_path| paths_conflict(item_path, path)),
        WorkItem::Check(_) | WorkItem::Epic(_) | WorkItem::Lease(_) => false,
    }
}

fn source_matches_path(source: &imp_work::SourceRef, path: &std::path::Path) -> bool {
    if source.kind != imp_work::SourceKind::FileRange {
        return false;
    }
    let reference_path = source
        .reference
        .split(':')
        .next()
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    paths_conflict(&reference_path, path)
}

fn paths_conflict(left: &std::path::Path, right: &std::path::Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn item_parent(item: &WorkItem) -> Option<String> {
    match item {
        WorkItem::Task(item) => item.parent.as_ref().map(|id| id.0.clone()),
        WorkItem::Memory(item) => item.parent_work.as_ref().map(|id| id.0.clone()),
        WorkItem::Decision(item) => item.parent_work.as_ref().map(|id| id.0.clone()),
        WorkItem::Prototype(item) => item.parent_work.clone(),
        WorkItem::Epic(_)
        | WorkItem::Check(_)
        | WorkItem::ContextPack(_)
        | WorkItem::Run(_)
        | WorkItem::Lease(_) => None,
    }
}

fn item_status(item: &WorkItem) -> Option<String> {
    match item {
        WorkItem::Epic(item) => Some(format_task_status_value(item.status).into()),
        WorkItem::Task(item) => Some(format_task_status_value(item.status).into()),
        WorkItem::Decision(item) => Some(format_decision_status_value(item.status).into()),
        WorkItem::Prototype(item) => Some(format_prototype_status_value(item.status).into()),
        WorkItem::Memory(_)
        | WorkItem::Check(_)
        | WorkItem::ContextPack(_)
        | WorkItem::Run(_)
        | WorkItem::Lease(_) => None,
    }
}

fn format_task_status_value(status: TaskStatus) -> &'static str {
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

fn format_decision_status_value(status: DecisionStatus) -> &'static str {
    match status {
        DecisionStatus::Proposed => "proposed",
        DecisionStatus::Accepted => "accepted",
        DecisionStatus::Rejected => "rejected",
        DecisionStatus::Superseded => "superseded",
    }
}

fn format_prototype_status_value(status: PrototypeStatus) -> &'static str {
    match status {
        PrototypeStatus::Planned => "planned",
        PrototypeStatus::Running => "running",
        PrototypeStatus::Observed => "observed",
        PrototypeStatus::Promoted => "promoted",
        PrototypeStatus::Discarded => "discarded",
    }
}

fn item_title(item: &WorkItem) -> &str {
    match item {
        WorkItem::Epic(item) => &item.title,
        WorkItem::Task(item) => &item.title,
        WorkItem::Memory(item) => &item.text,
        WorkItem::Decision(item) => &item.title,
        WorkItem::Prototype(item) => &item.question,
        WorkItem::Check(item) => &item.description,
        WorkItem::ContextPack(item) => &item.id.0,
        WorkItem::Run(item) => &item.summary,
        WorkItem::Lease(item) => &item.worker_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentMode;
    use crate::tools::{AnchorStore, CheckpointState, FileCache, FileTracker, ToolUpdate};
    use crate::ui::NullInterface;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn test_ctx(dir: &std::path::Path) -> (ToolContext, tokio::sync::mpsc::Receiver<ToolUpdate>) {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(AnchorStore::new()),
            lua_tool_loader: None,
            mode: AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
            run_policy: Default::default(),
            supporting_provenance: Vec::new(),
        };
        (ctx, rx)
    }

    #[tokio::test]
    async fn work_tool_guide_reports_overview_and_topics() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());
        let tool = WorkTool;

        let overview = tool
            .execute("guide-overview", json!({ "action": "guide" }), ctx)
            .await
            .unwrap();
        assert!(!overview.is_error);
        assert_eq!(overview.details["action"], "guide");
        assert_eq!(overview.details["topic"], "overview");
        assert!(overview
            .text_content()
            .unwrap()
            .contains("global project-scoped"));

        let (ctx, _rx) = test_ctx(tmp.path());
        let global_store = tool
            .execute(
                "guide-global-store",
                json!({ "action": "guide", "topic": "global_store" }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(global_store.details["topic"], "global_store");

        let (ctx, _rx) = test_ctx(tmp.path());
        let verification = tool
            .execute(
                "guide-verification",
                json!({ "action": "guide", "topic": "verification" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(verification.text_content().unwrap().contains("verify"));
    }

    #[tokio::test]
    async fn work_tool_reports_active_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());
        let tool = WorkTool;
        let result = tool
            .execute("scope", json!({ "action": "scope" }), ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.details["action"], "scope");
        assert_eq!(result.details["active_source"], "global_project_scoped");
        assert_eq!(result.details["writes_target"], "global_project_scoped");
        assert_eq!(
            result.details["project_root"].as_str().unwrap(),
            tmp.path().canonicalize().unwrap().to_str().unwrap()
        );
        assert!(result.details["global_store_root"]
            .as_str()
            .unwrap()
            .ends_with("/.imp/work"));
        assert!(result.text_content().unwrap().contains("imp-work scope"));
    }

    #[tokio::test]
    async fn work_tool_creates_and_lists_task() {
        let tmp = tempfile::tempdir().unwrap();
        let (ctx, _rx) = test_ctx(tmp.path());
        let tool = WorkTool;

        let created = tool
            .execute(
                "call-1",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Build native work tool",
                    "status": "ready",
                    "acceptance": ["task is persisted"],
                    "checks": ["cargo test -p imp-core work_tool"]
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!created.is_error);
        assert_eq!(created.details["kind"], "task");
        assert_eq!(created.details["id"], "T-build-native-work-tool");

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute("call-2", json!({ "action": "list", "kind": "task" }), ctx)
            .await
            .unwrap();
        let text = listed.text_content().unwrap();
        assert!(text.contains("Build native work tool"));
        assert_eq!(listed.details["store_source"], "global_project_scoped");
        assert_eq!(listed.details["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn work_tool_deduplicates_readable_task_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;

        for call_id in ["call-1", "call-2"] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute(
                call_id,
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Improve sidebar detail"
                }),
                ctx,
            )
            .await
            .unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute("call-3", json!({ "action": "list", "kind": "task" }), ctx)
            .await
            .unwrap();
        let ids = listed.details["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"T-improve-sidebar-detail".to_string()));
        assert!(ids.contains(&"T-improve-sidebar-detail-2".to_string()));
    }

    #[tokio::test]
    async fn work_tool_lists_global_project_scoped_tasks_when_requested() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let other_project = tmp.path().join("other-project");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&other_project).unwrap();
        let scope = storage::WorkScope::for_project_dir(&project);
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        let mut project_task = Task::new("Global project task");
        project_task.id = WorkId::from("global-project-task");
        project_task.status = TaskStatus::Ready;
        let mut other_task = Task::new("Other project task");
        other_task.id = WorkId::from("other-project-task");
        other_task.status = TaskStatus::Ready;
        global.append_task(&project, &project_task).unwrap();
        global.append_task(&other_project, &other_task).unwrap();

        let (ctx, _rx) = test_ctx(&project);
        let tool = WorkTool;
        let listed = tool
            .execute(
                "global-list",
                json!({
                    "action": "list",
                    "kind": "task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();

        let text = listed.text_content().unwrap();
        assert!(text.contains("Global project task"));
        assert!(!text.contains("Other project task"));
        assert_eq!(listed.details["store_source"], "global_project_scoped");
        assert_eq!(
            listed.details["project_root"].as_str().unwrap(),
            project.canonicalize().unwrap().to_str().unwrap()
        );
        assert_eq!(listed.details["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn work_tool_creates_global_project_scoped_task_when_requested() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let other_project = tmp.path().join("other-project");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&other_project).unwrap();
        let tool = WorkTool;

        let (ctx, _rx) = test_ctx(&project);
        let created = tool
            .execute(
                "global-create",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Global created task",
                    "status": "ready",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!created.is_error);
        assert_eq!(created.details["store_source"], "global_project_scoped");
        assert_eq!(created.details["kind"], "task");
        assert_eq!(
            created.details["project_root"].as_str().unwrap(),
            project.canonicalize().unwrap().to_str().unwrap()
        );

        let (ctx, _rx) = test_ctx(&project);
        let local_list = tool
            .execute(
                "local-list",
                json!({ "action": "list", "kind": "task" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(local_list
            .text_content()
            .unwrap()
            .contains("Global created task"));
        assert_eq!(local_list.details["store_source"], "global_project_scoped");

        let (ctx, _rx) = test_ctx(&project);
        let global_list = tool
            .execute(
                "global-list",
                json!({
                    "action": "list",
                    "kind": "task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(global_list
            .text_content()
            .unwrap()
            .contains("Global created task"));
        assert_eq!(global_list.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(&other_project);
        let other_global_list = tool
            .execute(
                "other-global-list",
                json!({
                    "action": "list",
                    "kind": "task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!other_global_list
            .text_content()
            .unwrap()
            .contains("Global created task"));
        assert_eq!(
            other_global_list.details["items"].as_array().unwrap().len(),
            0
        );
    }

    #[tokio::test]
    async fn work_tool_context_auto_loads_global_stream_history() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let tool = WorkTool;

        let (ctx, _rx) = test_ctx(&project);
        let created = tool
            .execute(
                "stream-context-create",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Follow stream context",
                    "stream_id": "stream-context",
                    "relation": "follow_up_to"
                }),
                ctx,
            )
            .await
            .unwrap();
        let id = created.details["id"].as_str().unwrap().to_string();

        let scope = storage::WorkScope::for_project_dir(&project);
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        append_stream_event_for_task(
            &global,
            &scope.project_root,
            "stream-context",
            None,
            StreamRelation::Closed,
            "Closed prior stream task: important summary".to_string(),
        )
        .unwrap();

        let (ctx, _rx) = test_ctx(&project);
        let context = tool
            .execute(
                "stream-context-pack",
                json!({
                    "action": "context",
                    "kind": "task",
                    "id": id
                }),
                ctx,
            )
            .await
            .unwrap();
        let markdown_path = context.details["markdown_path"].as_str().unwrap();
        let markdown = std::fs::read_to_string(markdown_path).unwrap();
        assert!(markdown.contains("Project stream history"));
        assert!(markdown.contains("Closed prior stream task"));
    }

    #[tokio::test]
    async fn work_tool_records_project_stream_events_for_create_and_outcome() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        let tool = WorkTool;

        let (ctx, _rx) = test_ctx(&project);
        let created = tool
            .execute(
                "stream-local-create",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Streamed local task"
                }),
                ctx,
            )
            .await
            .unwrap();
        let id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(&project);
        tool.execute(
            "stream-global-create",
            json!({
                "action": "create",
                "kind": "task",
                "title": "Streamed global task",
                "store_source": "global_project_scoped",
                "stream_id": "stream-alpha",
                "relation": "continues"
            }),
            ctx,
        )
        .await
        .unwrap();

        let scope = storage::WorkScope::for_project_dir(&project);
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        let events = global
            .stream_events_for_project_stream(&project, "stream-alpha")
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].relation, StreamRelation::Continues);
        assert!(events[0].work_id.is_some());
        assert!(events[0].summary.contains("Created task"));

        let (ctx, _rx) = test_ctx(&project);
        let outcome = tool
            .execute(
                "stream-outcome",
                json!({
                    "action": "outcome",
                    "id": id,
                    "outcome": "done",
                    "summary": "Closed streamed task",
                    "checks_passed": 1,
                    "checks_failed": 0,
                    "stream_id": "stream-alpha"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(outcome.details["stream_id"], "stream-alpha");

        let events = global
            .stream_events_for_project_stream(&project, "stream-alpha")
            .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].relation, StreamRelation::Closed);
        assert!(events[1].summary.contains("Closed streamed task"));
    }

    #[tokio::test]
    async fn work_tool_shows_and_updates_global_project_scoped_task() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("project");
        let other_project = tmp.path().join("other-project");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&other_project).unwrap();
        let scope = storage::WorkScope::for_project_dir(&project);
        let global = GlobalWorkStore::open(scope.global_store_root.clone());
        let mut task = Task::new("Original global task");
        task.id = WorkId::from("global-update-task");
        task.status = TaskStatus::Ready;
        global.append_task(&project, &task).unwrap();
        global.append_task(&other_project, &task).unwrap();
        let tool = WorkTool;

        let (ctx, _rx) = test_ctx(&project);
        let shown = tool
            .execute(
                "global-show",
                json!({
                    "action": "show",
                    "id": "global-update-task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(shown
            .text_content()
            .unwrap()
            .contains("Original global task"));
        assert_eq!(shown.details["store_source"], "global_project_scoped");

        let (ctx, _rx) = test_ctx(&project);
        let updated = tool
            .execute(
                "global-update",
                json!({
                    "action": "update",
                    "kind": "task",
                    "id": "global-update-task",
                    "status": "done",
                    "title": "Updated global task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(updated.details["store_source"], "global_project_scoped");
        assert_eq!(updated.details["item"]["status"], "done");
        assert_eq!(updated.details["item"]["title"], "Updated global task");

        let (ctx, _rx) = test_ctx(&project);
        let listed = tool
            .execute(
                "global-list-after-update",
                json!({
                    "action": "list",
                    "kind": "task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        let items = listed.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "Updated global task");
        assert_eq!(items[0]["status"], "done");

        let (ctx, _rx) = test_ctx(&other_project);
        let other_shown = tool
            .execute(
                "other-global-show",
                json!({
                    "action": "show",
                    "id": "global-update-task",
                    "store_source": "global_project_scoped"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(other_shown
            .text_content()
            .unwrap()
            .contains("Original global task"));
        assert_eq!(other_shown.details["item"]["title"], "Original global task");
    }

    #[tokio::test]
    async fn work_tool_updates_decision_and_prototype_status() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let decision = tool
            .execute(
                "create-decision",
                json!({
                    "action": "create",
                    "kind": "decision",
                    "title": "Promote prototypes deliberately",
                    "status": "proposed"
                }),
                ctx,
            )
            .await
            .unwrap();
        let decision_id = decision.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let prototype = tool
            .execute(
                "create-prototype",
                json!({
                    "action": "create",
                    "kind": "prototype",
                    "title": "Prototype lifecycle update",
                    "status": "planned"
                }),
                ctx,
            )
            .await
            .unwrap();
        let prototype_id = prototype.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let updated_decision = tool
            .execute(
                "update-decision",
                json!({
                    "action": "update",
                    "kind": "decision",
                    "id": decision_id,
                    "status": "accepted"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(updated_decision.details["status"], "accepted");

        let (ctx, _rx) = test_ctx(tmp.path());
        let updated_prototype = tool
            .execute(
                "update-prototype",
                json!({
                    "action": "update",
                    "kind": "prototype",
                    "id": prototype_id,
                    "status": "promoted"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(updated_prototype.details["status"], "promoted");

        let (ctx, _rx) = test_ctx(tmp.path());
        let accepted = tool
            .execute(
                "list-accepted",
                json!({ "action": "list", "kind": "decision", "status": "accepted" }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(accepted.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(tmp.path());
        let promoted = tool
            .execute(
                "list-promoted",
                json!({ "action": "list", "kind": "prototype", "status": "promoted" }),
                ctx,
            )
            .await
            .unwrap();
        let promoted_items = promoted.details["items"].as_array().unwrap();
        assert!(promoted_items.iter().any(|item| item["id"] == prototype_id));
    }

    #[tokio::test]
    async fn work_tool_records_structured_prototype_outcome() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created_task = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Parent prototype task",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let parent_id = created_task.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-prototype",
                json!({
                    "action": "create",
                    "kind": "prototype",
                    "title": "Prototype structured outcome",
                    "parent_work": parent_id,
                    "hypothesis": "Prototype outcomes can persist learnings.",
                    "evidence_required": ["observation is recorded"]
                }),
                ctx,
            )
            .await
            .unwrap();
        let prototype_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let outcome = tool
            .execute(
                "prototype-outcome",
                json!({
                    "action": "prototype_outcome",
                    "id": prototype_id,
                    "hypothesis_result": "supported",
                    "recommended_action": "promote",
                    "summary": "Prototype outcome recorded durable learning.",
                    "evidence": [{ "claim": "observation", "proof": "recorded in prototypes.md" }],
                    "memory_updates": ["Prototype outcomes should feed parent task memory."],
                    "followups": ["Port proven prototype into production code."]
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!outcome.is_error);
        assert_eq!(outcome.details["status"], "promoted");
        assert_eq!(outcome.details["outcome"], "promote");
        assert_eq!(outcome.details["hypothesis_result"], "supported");
        assert!(std::path::Path::new(outcome.details["recorded_path"].as_str().unwrap()).exists());

        let (ctx, _rx) = test_ctx(tmp.path());
        let search = tool
            .execute(
                "search-prototype-learning",
                json!({
                    "action": "search",
                    "query": "parent task memory",
                    "parent_work": parent_id
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(search
            .text_content()
            .unwrap()
            .contains("Prototype outcomes should feed parent task memory."));

        let (ctx, _rx) = test_ctx(tmp.path());
        let promoted = tool
            .execute(
                "list-promoted",
                json!({ "action": "list", "kind": "prototype", "status": "promoted" }),
                ctx,
            )
            .await
            .unwrap();
        let promoted_items = promoted.details["items"].as_array().unwrap();
        assert!(promoted_items.iter().any(|item| item["id"] == prototype_id));
    }

    #[tokio::test]
    async fn work_tool_remembers_and_searches_conversational_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let parent = "T-remember";
        let (ctx, _rx) = test_ctx(tmp.path());
        let remembered = tool
            .execute(
                "remember",
                json!({
                    "action": "remember",
                    "text": "I don't want mana import code committed to the repo.",
                    "parent_work": parent,
                    "topics": ["mana", "migration"]
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!remembered.is_error);
        assert_eq!(remembered.details["memory_kind"], "preference");
        assert!(std::path::Path::new(remembered.details["path"].as_str().unwrap()).exists());

        let (ctx, _rx) = test_ctx(tmp.path());
        let search = tool
            .execute(
                "search-remembered",
                json!({
                    "action": "search",
                    "query": "mana import committed",
                    "topic": "mana",
                    "parent_work": parent
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!search.is_error);
        assert!(search
            .text_content()
            .unwrap()
            .contains("I don't want mana import code committed"));
        assert_eq!(search.details["matches"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn work_tool_next_and_claim_can_require_fresh_context() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;

        let (ctx, _rx) = test_ctx(tmp.path());
        let unprepared = tool
            .execute(
                "create-unprepared",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Unprepared task",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let unprepared_id = unprepared.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let prepared = tool
            .execute(
                "create-prepared",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Prepared task",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let prepared_id = prepared.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "context-prepared",
            json!({ "action": "context", "kind": "task", "id": prepared_id }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute(
                "next-require-context",
                json!({ "action": "next", "require_context": true, "limit": 10 }),
                ctx,
            )
            .await
            .unwrap();
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "Prepared task");

        let (ctx, _rx) = test_ctx(tmp.path());
        let unprepared_claim = tool
            .execute(
                "claim-unprepared",
                json!({ "action": "claim", "id": unprepared_id, "require_context": true }),
                ctx,
            )
            .await;
        match unprepared_claim {
            Ok(_) => panic!("unprepared task should not be claimable when context is required"),
            Err(error) => assert!(
                error.to_string().contains("not ready to claim")
                    || error.to_string().contains("has no context_pack")
            ),
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let claimed = tool
            .execute(
                "claim-prepared",
                json!({ "action": "claim", "id": prepared_id, "require_context": true }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(claimed.details["status"], "active");
    }

    #[tokio::test]
    async fn work_tool_refreshes_task_context_pack() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Refresh linked context",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let first_context = tool
            .execute(
                "context-task",
                json!({ "action": "context", "kind": "task", "id": task_id }),
                ctx,
            )
            .await
            .unwrap();
        let first_id = first_context.details["context_pack_id"]
            .as_str()
            .unwrap()
            .to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "remember",
            json!({
                "action": "remember",
                "text": "Refreshed context should include new memory.",
                "parent_work": task_id,
                "topics": ["context-pack"]
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let refreshed = tool
            .execute(
                "refresh-context",
                json!({
                    "action": "refresh_context",
                    "kind": "task",
                    "id": task_id,
                    "query": "new memory",
                    "topic": "context-pack"
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!refreshed.is_error);
        let second_id = refreshed.details["context_pack_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(first_id, second_id);
        assert_eq!(refreshed.details["previous_context_pack_id"], first_id);
        assert_eq!(refreshed.details["version"], 2);

        let (ctx, _rx) = test_ctx(tmp.path());
        let shown_task = tool
            .execute("show-task", json!({ "action": "show", "id": task_id }), ctx)
            .await
            .unwrap();
        assert_eq!(shown_task.details["item"]["context_pack"], second_id);

        let (ctx, _rx) = test_ctx(tmp.path());
        let old_context = tool
            .execute(
                "show-old-context",
                json!({ "action": "show", "id": first_id }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(old_context.details["stale"], true);

        let (ctx, _rx) = test_ctx(tmp.path());
        let new_context = tool
            .execute(
                "show-new-context",
                json!({ "action": "show", "id": second_id }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(new_context.details["stale"], false);
    }

    #[tokio::test]
    async fn work_tool_lists_context_packs() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "List prepared context",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();
        let (ctx, _rx) = test_ctx(tmp.path());
        let context = tool
            .execute(
                "context-task",
                json!({ "action": "context", "kind": "task", "id": task_id }),
                ctx,
            )
            .await
            .unwrap();
        let context_id = context.details["context_pack_id"]
            .as_str()
            .unwrap()
            .to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute(
                "list-contexts",
                json!({ "action": "list", "kind": "context_pack" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!listed.is_error);
        let text = listed.text_content().unwrap();
        assert!(text.contains(&context_id));
        let items = listed.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], context_id);
    }

    #[tokio::test]
    async fn work_tool_lists_run_history() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Inspect run history",
                    "status": "active"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "record-outcome",
            json!({
                "action": "outcome",
                "kind": "task",
                "id": task_id,
                "outcome": "done",
                "summary": "Run history should show this outcome."
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let runs = tool
            .execute("runs", json!({ "action": "runs", "limit": 5 }), ctx)
            .await
            .unwrap();

        assert!(!runs.is_error);
        let text = runs.text_content().unwrap();
        assert!(text.contains("Run history should show this outcome."));
        assert!(text.contains("summary:"));
        assert_eq!(runs.details["runs"].as_array().unwrap().len(), 1);
        assert_eq!(runs.details["outcomes"].as_array().unwrap().len(), 1);
        assert_eq!(runs.details["outcomes"][0]["work_id"], task_id);
    }

    #[tokio::test]
    async fn work_tool_outcome_releases_persisted_task_leases() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Release claim lease",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let claimed = tool
            .execute(
                "claim-task",
                json!({
                    "action": "claim",
                    "id": task_id,
                    "worker_id": "worker-release"
                }),
                ctx,
            )
            .await
            .unwrap();
        let lease_id = claimed.details["lease_id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let leases = tool
            .execute(
                "list-leases",
                json!({ "action": "list", "kind": "lease" }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(leases.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(tmp.path());
        let outcome = tool
            .execute(
                "record-outcome",
                json!({
                    "action": "outcome",
                    "kind": "task",
                    "id": task_id,
                    "outcome": "done",
                    "summary": "Lease should be released."
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(
            outcome.details["persistence"]["released_leases"][0],
            lease_id
        );

        let (ctx, _rx) = test_ctx(tmp.path());
        let leases = tool
            .execute(
                "list-leases",
                json!({ "action": "list", "kind": "lease" }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(leases.details["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn work_tool_records_structured_task_outcome() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Record structured outcome",
                    "status": "active"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let outcome = tool
            .execute(
                "record-outcome",
                json!({
                    "action": "outcome",
                    "kind": "task",
                    "id": task_id,
                    "outcome": "done_with_concerns",
                    "summary": "Implemented but follow-up remains.",
                    "changed_paths": ["crates/imp-core/src/tools/work.rs"],
                    "checks_passed": 2,
                    "memory_updates": ["Structured outcomes persist task learnings."],
                    "followups": ["Add richer outcome check results."]
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!outcome.is_error);
        assert_eq!(outcome.details["status"], "done");
        assert_eq!(outcome.details["outcome"], "done_with_concerns");
        let persistence = &outcome.details["persistence"];
        assert!(std::path::Path::new(persistence["run_path"].as_str().unwrap()).exists());
        assert!(std::path::Path::new(persistence["outcome_path"].as_str().unwrap()).exists());
        assert!(std::path::Path::new(persistence["summary_path"].as_str().unwrap()).exists());
        let memory_path = persistence["memory_paths"][0].as_str().unwrap();
        assert!(std::fs::read_to_string(memory_path)
            .unwrap()
            .contains("Structured outcomes persist task learnings."));
        let followup_path = persistence["followup_task_path"].as_str().unwrap();
        assert!(std::fs::read_to_string(followup_path)
            .unwrap()
            .contains("Add richer outcome check results."));

        let (ctx, _rx) = test_ctx(tmp.path());
        let shown = tool
            .execute(
                "show-outcome-task",
                json!({ "action": "show", "id": task_id }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(shown.details["item"]["status"], "done");
    }

    #[tokio::test]
    async fn work_tool_filters_list_and_search_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        for params in [
            json!({
                "action": "remember",
                "text": "Context pack memory for work tool path filtering.",
                "topics": ["path-filter"],
                "path": "crates/imp-core/src/tools/work.rs"
            }),
            json!({
                "action": "remember",
                "text": "Unrelated memory for another path.",
                "topics": ["path-filter"],
                "path": "crates/imp-core/src/tools/read.rs"
            }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute("remember", params, ctx).await.unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute(
                "list-path",
                json!({
                    "action": "list",
                    "kind": "memory",
                    "path": "crates/imp-core/src/tools/work.rs"
                }),
                ctx,
            )
            .await
            .unwrap();
        let list_text = listed.text_content().unwrap();
        assert!(list_text.contains("Context pack memory for work tool path filtering."));
        assert!(!list_text.contains("Unrelated memory for another path."));
        assert_eq!(listed.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(tmp.path());
        let searched = tool
            .execute(
                "search-path",
                json!({
                    "action": "search",
                    "query": "path filtering",
                    "topic": "path-filter",
                    "path": "crates/imp-core/src/tools"
                }),
                ctx,
            )
            .await
            .unwrap();
        let search_text = searched.text_content().unwrap();
        assert!(search_text.contains("Context pack memory for work tool path filtering."));
        assert!(search_text.contains("Unrelated memory for another path."));
        assert_eq!(searched.details["matches"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn work_tool_lists_items_by_parent_work() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let epic_a = tool
            .execute(
                "create-epic-a",
                json!({ "action": "create", "kind": "epic", "title": "Parent epic A" }),
                ctx,
            )
            .await
            .unwrap();
        let epic_a_id = epic_a.details["id"].as_str().unwrap().to_string();
        let (ctx, _rx) = test_ctx(tmp.path());
        let epic_b = tool
            .execute(
                "create-epic-b",
                json!({ "action": "create", "kind": "epic", "title": "Parent epic B" }),
                ctx,
            )
            .await
            .unwrap();
        let epic_b_id = epic_b.details["id"].as_str().unwrap().to_string();

        for params in [
            json!({ "action": "create", "kind": "task", "title": "Child of A", "parent_work": epic_a_id }),
            json!({ "action": "create", "kind": "task", "title": "Child of B", "parent_work": epic_b_id }),
            json!({ "action": "remember", "text": "Memory under A", "parent_work": epic_a_id }),
            json!({ "action": "create", "kind": "decision", "title": "Decision under A", "parent_work": epic_a_id }),
            json!({ "action": "create", "kind": "prototype", "title": "Prototype under A", "parent_work": epic_a_id }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute("create-child", params, ctx).await.unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let tasks = tool
            .execute(
                "list-children-a",
                json!({ "action": "list", "kind": "task", "parent_work": epic_a_id }),
                ctx,
            )
            .await
            .unwrap();
        let task_text = tasks.text_content().unwrap();
        assert!(task_text.contains("Child of A"));
        assert!(!task_text.contains("Child of B"));
        assert_eq!(tasks.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(tmp.path());
        let all_children = tool
            .execute(
                "list-all-children-a",
                json!({ "action": "list", "parent_work": epic_a_id, "limit": 10 }),
                ctx,
            )
            .await
            .unwrap();
        let all_text = all_children.text_content().unwrap();
        assert!(all_text.contains("Child of A"));
        assert!(all_text.contains("Memory under A"));
        assert!(all_text.contains("Decision under A"));
        assert!(all_text.contains("Prototype under A"));
        assert!(!all_text.contains("Child of B"));
    }

    #[tokio::test]
    async fn work_tool_lists_items_by_status() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        for params in [
            json!({ "action": "create", "kind": "task", "title": "Ready listed task", "status": "ready" }),
            json!({ "action": "create", "kind": "task", "title": "Done listed task", "status": "done" }),
            json!({ "action": "create", "kind": "decision", "title": "Accepted decision", "status": "accepted" }),
            json!({ "action": "create", "kind": "decision", "title": "Rejected decision", "status": "rejected" }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute("create", params, ctx).await.unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let ready = tool
            .execute(
                "list-ready",
                json!({ "action": "list", "kind": "task", "status": "ready" }),
                ctx,
            )
            .await
            .unwrap();
        let ready_text = ready.text_content().unwrap();
        assert!(ready_text.contains("Ready listed task"));
        assert!(!ready_text.contains("Done listed task"));
        assert_eq!(ready.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(tmp.path());
        let accepted = tool
            .execute(
                "list-accepted",
                json!({ "action": "list", "kind": "decision", "status": "accepted" }),
                ctx,
            )
            .await
            .unwrap();
        let accepted_text = accepted.text_content().unwrap();
        assert!(accepted_text.contains("Accepted decision"));
        assert!(!accepted_text.contains("Rejected decision"));
        assert_eq!(accepted.details["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn work_tool_searches_conversational_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let parent = "T-memory-search";
        for params in [
            json!({
                "action": "create",
                "kind": "memory",
                "text": "Conversational memory should be found without remembering the file location.",
                "memory_kind": "fact",
                "parent_work": parent,
                "topics": ["memory", "search"]
            }),
            json!({
                "action": "create",
                "kind": "memory",
                "text": "Unrelated prototype memory.",
                "memory_kind": "note",
                "topics": ["prototype"]
            }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute("create-memory", params, ctx).await.unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let result = tool
            .execute(
                "search-memory",
                json!({
                    "action": "search",
                    "query": "remembering file location",
                    "topic": "memory",
                    "parent_work": parent,
                    "limit": 5
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = result.text_content().unwrap();
        assert!(text.contains("Conversational memory should be found"));
        assert!(!text.contains("Unrelated prototype memory"));
        let matches = result.details["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0]["score"].as_i64().unwrap() > 0);
        assert_eq!(matches[0]["memory"]["parent_work"], parent);
    }

    #[tokio::test]
    async fn work_tool_validates_missing_dependency_and_context() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Broken validation task",
                    "status": "ready",
                    "depends_on": ["T-missing-validation"]
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();
        let scope = storage::WorkScope::for_project_dir(tmp.path());
        let store = WorkStore::open(global_project_work_store_root(&scope));
        store
            .update_task_context_pack(&task_id, WorkId::from("CTX-missing-validation"))
            .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let validation = tool
            .execute("validate", json!({ "action": "validate" }), ctx)
            .await
            .unwrap();

        assert!(validation.is_error);
        assert_eq!(validation.details["ok"], false);
        let text = validation.text_content().unwrap();
        assert!(text.contains("missing_dependency"));
        assert!(text.contains("missing_context_pack"));
    }

    #[tokio::test]
    async fn work_tool_adds_and_removes_task_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let dependency = tool
            .execute(
                "create-dependency",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Editable dependency",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let dependency_id = dependency.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let dependent = tool
            .execute(
                "create-dependent",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Editable dependent",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let dependent_id = dependent.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let added = tool
            .execute(
                "dep-add",
                json!({
                    "action": "dep_add",
                    "id": dependent_id,
                    "dependency_id": dependency_id
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(added.details["action"], "dep_add");
        assert_eq!(added.details["dependency_id"], dependency_id);

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next-before", json!({ "action": "next", "limit": 10 }), ctx)
            .await
            .unwrap();
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "Editable dependency");

        let (ctx, _rx) = test_ctx(tmp.path());
        let removed = tool
            .execute(
                "dep-remove",
                json!({
                    "action": "dep_remove",
                    "id": dependent_id,
                    "dependency_id": dependency_id
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(removed.details["action"], "dep_remove");

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next-after", json!({ "action": "next", "limit": 10 }), ctx)
            .await
            .unwrap();
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn work_tool_create_task_accepts_dependencies_and_gates_next() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let dependency = tool
            .execute(
                "create-dependency",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Dependency task",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let dependency_id = dependency.details["id"].as_str().unwrap().to_string();
        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "create-dependent",
            json!({
                "action": "create",
                "kind": "task",
                "title": "Dependent task",
                "status": "ready",
                "depends_on": [dependency_id]
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next-before", json!({ "action": "next", "limit": 10 }), ctx)
            .await
            .unwrap();
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "Dependency task");

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "finish-dependency",
            json!({
                "action": "outcome",
                "kind": "task",
                "id": dependency_id,
                "outcome": "done",
                "summary": "Dependency complete."
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next-after", json!({ "action": "next", "limit": 10 }), ctx)
            .await
            .unwrap();
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "Dependent task");
    }

    #[tokio::test]
    async fn work_tool_claim_by_id_respects_dependency_gating() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let dependency = tool
            .execute(
                "create-dependency",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Claim dependency",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let dependency_id = dependency.details["id"].as_str().unwrap().to_string();
        let (ctx, _rx) = test_ctx(tmp.path());
        let dependent = tool
            .execute(
                "create-dependent",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Claim dependent",
                    "status": "ready",
                    "depends_on": [dependency_id]
                }),
                ctx,
            )
            .await
            .unwrap();
        let dependent_id = dependent.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let blocked_claim = tool
            .execute(
                "claim-blocked",
                json!({ "action": "claim", "id": dependent_id }),
                ctx,
            )
            .await;
        match blocked_claim {
            Ok(_) => panic!("dependent task should not be claimable before dependency is done"),
            Err(error) => assert!(error.to_string().contains("not ready to claim")),
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "finish-dependency",
            json!({
                "action": "outcome",
                "kind": "task",
                "id": dependency_id,
                "outcome": "done",
                "summary": "Dependency done."
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let claimed = tool
            .execute(
                "claim-dependent",
                json!({ "action": "claim", "id": dependent_id }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(claimed.details["status"], "active");
        assert_eq!(claimed.details["id"], dependent_id);
    }

    #[tokio::test]
    async fn work_tool_claims_ready_task_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Claim task by id",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let claimed = tool
            .execute(
                "claim-task",
                json!({ "action": "claim", "id": task_id, "worker_id": "worker-by-id", "path_locks": ["crates/imp-core/src/tools/work.rs"] }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!claimed.is_error);
        assert_eq!(claimed.details["status"], "active");
        assert_eq!(claimed.details["id"], task_id);
        assert!(claimed.details["lease_id"]
            .as_str()
            .unwrap()
            .starts_with("L-"));
        assert_eq!(claimed.details["lease"]["worker_id"], "worker-by-id");
        assert_eq!(
            claimed.details["lease"]["path_locks"][0],
            "crates/imp-core/src/tools/work.rs"
        );
        assert!(std::path::Path::new(claimed.details["lease_path"].as_str().unwrap()).exists());
        let (ctx, _rx) = test_ctx(tmp.path());
        let leases = tool
            .execute(
                "list-leases",
                json!({ "action": "list", "kind": "lease" }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(leases.details["items"].as_array().unwrap().len(), 1);

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next", json!({ "action": "next" }), ctx)
            .await
            .unwrap();
        assert_eq!(next.details["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn work_tool_claims_first_ready_task() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        for params in [
            json!({ "action": "create", "kind": "task", "title": "Done task", "status": "done" }),
            json!({ "action": "create", "kind": "task", "title": "First ready task", "status": "ready" }),
            json!({ "action": "create", "kind": "task", "title": "Second ready task", "status": "ready" }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute("create", params, ctx).await.unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let claimed = tool
            .execute("claim-first", json!({ "action": "claim" }), ctx)
            .await
            .unwrap();

        assert!(!claimed.is_error);
        assert_eq!(claimed.details["status"], "active");
        let claimed_title = claimed.details["item"]["title"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(claimed_title == "First ready task" || claimed_title == "Second ready task");

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next", json!({ "action": "next", "limit": 10 }), ctx)
            .await
            .unwrap();
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        let remaining_title = items[0]["title"].as_str().unwrap();
        assert_ne!(remaining_title, claimed_title);
        assert!(remaining_title == "First ready task" || remaining_title == "Second ready task");
    }

    #[tokio::test]
    async fn work_tool_updates_task_status() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Update task status",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let updated = tool
            .execute(
                "update-task",
                json!({
                    "action": "update",
                    "kind": "task",
                    "id": task_id,
                    "status": "done"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!updated.is_error);
        assert_eq!(updated.details["status"], "done");

        let (ctx, _rx) = test_ctx(tmp.path());
        let shown = tool
            .execute("show-task", json!({ "action": "show", "id": task_id }), ctx)
            .await
            .unwrap();
        assert_eq!(shown.details["item"]["status"], "done");

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next", json!({ "action": "next" }), ctx)
            .await
            .unwrap();
        assert_eq!(next.details["items"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn work_tool_shows_task_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Show native task",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let shown = tool
            .execute("show-task", json!({ "action": "show", "id": task_id }), ctx)
            .await
            .unwrap();

        assert!(!shown.is_error);
        assert_eq!(shown.details["kind"], "task");
        assert_eq!(shown.details["id"], task_id);
        assert!(shown.text_content().unwrap().contains("Show native task"));
    }

    #[tokio::test]
    async fn work_tool_shows_context_pack_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Show context pack",
                    "status": "ready"
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let context = tool
            .execute(
                "context-task",
                json!({ "action": "context", "kind": "task", "id": task_id }),
                ctx,
            )
            .await
            .unwrap();
        let context_id = context.details["context_pack_id"]
            .as_str()
            .unwrap()
            .to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let shown = tool
            .execute(
                "show-context",
                json!({ "action": "show", "id": context_id }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!shown.is_error);
        assert_eq!(shown.details["kind"], "context_pack");
        assert_eq!(shown.details["item"]["id"], context_id);
        assert!(shown.details["stable_prefix_hash"].as_str().is_some());
        assert!(shown.text_content().unwrap().contains("context_pack"));
    }

    #[tokio::test]
    async fn work_tool_returns_next_ready_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        for params in [
            json!({ "action": "create", "kind": "task", "title": "Ready task", "status": "ready" }),
            json!({ "action": "create", "kind": "task", "title": "Done task", "status": "done" }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            tool.execute("create", params, ctx).await.unwrap();
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let next = tool
            .execute("next", json!({ "action": "next", "limit": 10 }), ctx)
            .await
            .unwrap();

        assert!(!next.is_error);
        let text = next.text_content().unwrap();
        assert!(text.contains("1 ready task(s)"));
        assert!(text.contains("Ready task"));
        assert!(!text.contains("Done task"));
        let items = next.details["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], "Ready task");
    }

    #[tokio::test]
    async fn work_tool_creates_task_context_pack_with_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Prepare native context pack",
                    "status": "ready",
                    "acceptance": ["context pack is written"]
                }),
                ctx,
            )
            .await
            .unwrap();
        let task_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "create-memory",
            json!({
                "action": "create",
                "kind": "memory",
                "text": "Native context packs should include retrieved work memory.",
                "memory_kind": "fact",
                "parent_work": task_id,
                "topics": ["context-pack"]
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let context = tool
            .execute(
                "context-task",
                json!({
                    "action": "context",
                    "kind": "task",
                    "id": task_id,
                    "query": "retrieved work memory",
                    "topic": "context-pack"
                }),
                ctx,
            )
            .await;

        match context {
            Ok(context) => {
                assert!(!context.is_error);
                let json_path = context.details["json_path"].as_str().unwrap();
                let markdown_path = context.details["markdown_path"].as_str().unwrap();
                assert!(std::path::Path::new(json_path).exists());
                assert!(std::path::Path::new(markdown_path).exists());
                let markdown = std::fs::read_to_string(markdown_path).unwrap();
                assert!(
                    markdown.contains("Native context packs should include retrieved work memory.")
                );
                assert!(context.details["stable_prefix_hash"].as_str().is_some());
                let (ctx, _rx) = test_ctx(tmp.path());
                let shown = tool
                    .execute(
                        "show-linked-task",
                        json!({ "action": "show", "id": task_id }),
                        ctx,
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    shown.details["item"]["context_pack"],
                    context.details["context_pack_id"]
                );
            }
            Err(Error::Tool(message)) if message.contains("not found") => {
                // Task context is globalized in 649.5/649.6 after the task backend flip.
                return;
            }
            Err(error) => panic!("unexpected context error: {error:?}"),
        }
    }

    #[tokio::test]
    async fn work_tool_creates_prototype_context_pack_with_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-prototype",
                json!({
                    "action": "create",
                    "kind": "prototype",
                    "title": "Can prototype contexts be prepared?",
                    "hypothesis": "The work tool can prepare prototype context packs.",
                    "evidence_required": ["context file exists"]
                }),
                ctx,
            )
            .await
            .unwrap();
        let prototype_id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        tool.execute(
            "create-memory",
            json!({
                "action": "create",
                "kind": "memory",
                "text": "Prototype context packs should carry relevant experiment memory.",
                "memory_kind": "fact",
                "parent_work": prototype_id,
                "topics": ["prototype"]
            }),
            ctx,
        )
        .await
        .unwrap();

        let (ctx, _rx) = test_ctx(tmp.path());
        let context = tool
            .execute(
                "context-prototype",
                json!({
                    "action": "context",
                    "kind": "prototype",
                    "id": prototype_id,
                    "query": "experiment memory",
                    "topic": "prototype"
                }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!context.is_error);
        let markdown_path = context.details["markdown_path"].as_str().unwrap();
        let markdown = std::fs::read_to_string(markdown_path).unwrap();
        assert!(
            markdown.contains("Prototype context packs should carry relevant experiment memory.")
        );
        assert!(markdown.contains("Prototype code is disposable"));
    }

    #[tokio::test]
    async fn work_tool_tree_reports_dependency_blockers() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-blocked-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Needs missing dependency",
                    "depends_on": ["missing-dep"]
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!created.is_error);

        let (ctx, _rx) = test_ctx(tmp.path());
        let tree = tool
            .execute("tree", json!({ "action": "tree" }), ctx)
            .await
            .unwrap();

        assert!(!tree.is_error);
        assert_eq!(tree.details["action"], "tree");
        assert!(tree.details["tree"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("missing")));
    }

    #[tokio::test]
    async fn work_tool_verify_close_and_fail_use_native_conventions() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        let (ctx, _rx) = test_ctx(tmp.path());
        let created = tool
            .execute(
                "create-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Checked task",
                    "verify": "cargo test -p imp-work"
                }),
                ctx,
            )
            .await
            .unwrap();
        let id = created.details["id"].as_str().unwrap().to_string();

        let (ctx, _rx) = test_ctx(tmp.path());
        let verify = tool
            .execute("verify", json!({ "action": "verify", "id": id }), ctx)
            .await
            .unwrap();
        assert!(!verify.is_error);
        assert_eq!(verify.details["verify"]["passed"], true);

        let (ctx, _rx) = test_ctx(tmp.path());
        let close = tool
            .execute(
                "close",
                json!({
                    "action": "close",
                    "id": id,
                    "summary": "verified native close"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!close.is_error);
        assert_eq!(close.details["task"]["status"], "done");
        assert_eq!(close.details["run"]["outcome"], "done");

        let (ctx, _rx) = test_ctx(tmp.path());
        let created_fail = tool
            .execute(
                "create-fail-task",
                json!({
                    "action": "create",
                    "kind": "task",
                    "title": "Failing task"
                }),
                ctx,
            )
            .await
            .unwrap();
        let fail_id = created_fail.details["id"].as_str().unwrap().to_string();
        let (ctx, _rx) = test_ctx(tmp.path());
        let failed = tool
            .execute(
                "fail",
                json!({
                    "action": "fail",
                    "id": fail_id,
                    "reason": "missing multi-agent runner",
                    "next_action": "implement bounded jobs"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert!(!failed.is_error);
        assert_eq!(failed.details["task"]["status"], "blocked");
        assert!(failed.details["memory"]["text"]
            .as_str()
            .unwrap()
            .contains("bounded jobs"));
    }

    #[tokio::test]
    async fn work_tool_creates_multiple_item_kinds() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = WorkTool;
        for params in [
            json!({ "action": "create", "kind": "epic", "title": "Work epic", "text": "Intent" }),
            json!({ "action": "create", "kind": "memory", "text": "Remember this", "memory_kind": "fact", "topics": ["work"] }),
            json!({ "action": "create", "kind": "decision", "title": "Use work tool", "status": "accepted", "rationale": "Native integration" }),
            json!({ "action": "create", "kind": "prototype", "title": "Prototype work tool", "hypothesis": "Works" }),
        ] {
            let (ctx, _rx) = test_ctx(tmp.path());
            let result = tool.execute("call", params, ctx).await.unwrap();
            assert!(!result.is_error);
        }

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute(
                "call-list",
                json!({ "action": "list", "kind": "epic", "limit": 10 }),
                ctx,
            )
            .await
            .unwrap();
        let text = listed.text_content().unwrap();
        assert!(text.contains("Work epic"));

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute(
                "call-list",
                json!({ "action": "list", "kind": "memory", "limit": 10 }),
                ctx,
            )
            .await
            .unwrap();
        let text = listed.text_content().unwrap();
        assert!(text.contains("Remember this"));

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute(
                "call-list",
                json!({ "action": "list", "kind": "decision", "limit": 10 }),
                ctx,
            )
            .await
            .unwrap();
        let text = listed.text_content().unwrap();
        assert!(text.contains("Use work tool"));

        let (ctx, _rx) = test_ctx(tmp.path());
        let listed = tool
            .execute(
                "call-list",
                json!({ "action": "list", "kind": "prototype", "limit": 10 }),
                ctx,
            )
            .await
            .unwrap();
        let text = listed.text_content().unwrap();
        assert!(text.contains("Prototype work tool"));
    }
}
