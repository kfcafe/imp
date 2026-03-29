use std::path::Path;

use async_trait::async_trait;
use mana::commands::agents::load_agents;
use mana::commands::logs::find_all_logs;
use mana::commands::next::ScoredUnit;
use mana::commands::run::{RunArgs, RunView};
use mana_core::ops::claim::ClaimParams;
use serde_json::json;

use super::{truncate_head, Tool, ToolContext, ToolOutput, ToolUpdate};
use crate::error::Result;
use crate::ui::{NotifyLevel, WidgetContent};
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;

fn find_mana_dir(cwd: &Path) -> std::result::Result<std::path::PathBuf, String> {
    mana_core::discovery::find_mana_dir(cwd).map_err(|e| e.to_string())
}

fn json_output(value: &impl serde::Serialize) -> ToolOutput {
    match serde_json::to_string_pretty(value) {
        Ok(json) => ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text: json }],
            details: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
            is_error: false,
        },
        Err(e) => ToolOutput::error(format!("Failed to serialize: {e}")),
    }
}

fn send_update(ctx: &ToolContext, text: impl Into<String>, details: serde_json::Value) {
    let _ = ctx.update_tx.try_send(ToolUpdate {
        content: vec![imp_llm::ContentBlock::Text { text: text.into() }],
        details,
    });
}

fn run_summary_lines(view: &RunView) -> Vec<String> {
    let mut lines = vec![format!(
        "Mana run: {} total · {} done · {} failed · {} awaiting verify · {} skipped",
        view.summary.total_units,
        view.summary.total_closed,
        view.summary.total_failed,
        view.summary.total_awaiting_verify,
        view.summary.total_skipped
    )];

    for unit in &view.units {
        let marker = match unit.status.as_str() {
            "running" => "▶",
            "done" => "✓",
            "failed" => "✗",
            "blocked" => "!",
            _ => "…",
        };
        let mut extras = Vec::new();
        if let Some(round) = unit.round {
            extras.push(format!("wave {round}"));
        }
        if let Some(agent) = &unit.agent {
            extras.push(agent.clone());
        }
        if let Some(duration) = unit.duration_secs {
            extras.push(format!("{}s", duration));
        }
        let extra_suffix = if extras.is_empty() {
            String::new()
        } else {
            format!("  {}", extras.join(" · "))
        };
        lines.push(format!(
            "{marker} {}  {}  {}{}",
            unit.id, unit.title, unit.status, extra_suffix
        ));
    }

    lines
}

fn mana_widget_lines(summary: impl Into<String>, detail: Option<String>) -> WidgetContent {
    let mut lines = vec![summary.into()];
    if let Some(detail) = detail {
        lines.push(detail);
    }
    WidgetContent::Lines(lines)
}

fn background_run_started_output(scope: &str, run_args: &RunArgs) -> ToolOutput {
    let text = format!(
        "Started mana run in background for {scope}. Use mana(action=\"agents\") to inspect active workers, mana(action=\"logs\", id=...) for output, and mana(action=\"status\") or mana(action=\"next\") to inspect project state."
    );
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "background": true,
            "scope": scope,
            "jobs": run_args.jobs,
            "loop": run_args.loop_mode,
            "dry_run": run_args.dry_run,
            "review": run_args.review,
        }),
        is_error: false,
    }
}

fn spawn_background_run(mana_dir: std::path::PathBuf, run_args: RunArgs, ctx: ToolContext) {
    let ui = ctx.ui.clone();
    let scope = run_args
        .id
        .as_deref()
        .map(|id| format!("unit {id}"))
        .unwrap_or_else(|| "all ready units".to_string());

    tokio::spawn(async move {
        ui.set_status("mana", Some(&format!("mana: running {scope}")))
            .await;
        ui.set_widget(
            "mana",
            Some(mana_widget_lines(
                format!("running {scope}"),
                Some("inspect with mana agents / logs / status".to_string()),
            )),
        )
        .await;

        let result = tokio::task::spawn_blocking(move || {
            mana::commands::run::run_with_stream_capture_and_sink(&mana_dir, run_args, None)
        })
        .await;

        match result {
            Ok(Ok(view)) => {
                let summary = format!(
                    "mana: {scope} finished · {} done · {} failed",
                    view.summary.total_closed, view.summary.total_failed
                );
                ui.set_status("mana", Some(&summary)).await;
                ui.set_widget("mana", Some(mana_widget_lines(summary.clone(), None)))
                    .await;
                ui.notify(&summary, NotifyLevel::Info).await;
                let ui_clear = ui.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(12)).await;
                    ui_clear.set_widget("mana", None).await;
                    ui_clear.set_status("mana", None).await;
                });
            }
            Ok(Err(err)) => {
                let message = format!("mana: {scope} failed: {err}");
                ui.set_status("mana", Some(&message)).await;
                ui.set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
                    .await;
                ui.notify(&message, NotifyLevel::Error).await;
            }
            Err(join_err) => {
                let message = format!("mana: {scope} task failed: {join_err}");
                ui.set_status("mana", Some(&message)).await;
                ui.set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
                    .await;
                ui.notify(&message, NotifyLevel::Error).await;
            }
        }
    });
}

fn text_output(text: String, details: serde_json::Value) -> ToolOutput {
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details,
        is_error: false,
    }
}

fn claim_output(result: &mana_core::ops::claim::ClaimResult) -> ToolOutput {
    let text = format!(
        "Claimed unit {} ({}) by {}",
        result.unit.id, result.unit.title, result.claimer
    );
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "unit": {
                "id": result.unit.id,
                "title": result.unit.title,
                "status": result.unit.status,
                "claimed_by": result.unit.claimed_by,
            },
            "claimer": result.claimer,
            "is_goal": result.is_goal,
            "path": result.path,
        }),
        is_error: false,
    }
}

fn release_output(result: &mana_core::ops::claim::ReleaseResult) -> ToolOutput {
    let text = format!(
        "Released unit {} ({}) back to {}",
        result.unit.id, result.unit.title, result.unit.status
    );
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "unit": {
                "id": result.unit.id,
                "title": result.unit.title,
                "status": result.unit.status,
                "claimed_by": result.unit.claimed_by,
            },
            "path": result.path,
        }),
        is_error: false,
    }
}

fn truncate_with_note(text: &str) -> String {
    let result = truncate_head(text, MAX_OUTPUT_LINES, MAX_OUTPUT_BYTES);
    if !result.truncated {
        return result.content;
    }

    let mut output = result.content;
    output.push_str(&format!(
        "\n[Output truncated: showing first {} of {} lines{}]",
        result.output_lines,
        result.total_lines,
        result
            .temp_file
            .as_ref()
            .map(|p| format!(". Full output saved to {}", p.display()))
            .unwrap_or_default()
    ));
    output
}

fn scored_units_to_text(units: &[ScoredUnit]) -> String {
    if units.is_empty() {
        return "No ready units. Create one with: mana create \"task\" --verify \"cmd\""
            .to_string();
    }

    let mut lines = Vec::new();
    for unit in units {
        lines.push(format!(
            "P{}  {:.1}  {}",
            unit.priority, unit.score, unit.title
        ));
        if !unit.unblocks.is_empty() {
            lines.push(format!("      Unblocks: {}", unit.unblocks.join(", ")));
        }
        let attempts = if unit.attempts > 0 {
            format!(" | Attempts: {}", unit.attempts)
        } else {
            String::new()
        };
        lines.push(format!(
            "      ID: {} | Age: {} days{}",
            unit.id, unit.age_days, attempts
        ));
        lines.push(String::new());
    }
    lines.join("\n")
}

fn tree_lines(node: &mana_core::api::TreeNode, indent: usize, out: &mut Vec<String>) {
    let prefix = "  ".repeat(indent);
    let verify = if node.has_verify { "spec" } else { "goal" };
    out.push(format!(
        "{}{} {} [{} P{} · {}]",
        prefix, node.id, node.title, node.status, node.priority, verify
    ));
    for child in &node.children {
        tree_lines(child, indent + 1, out);
    }
}

pub struct ManaTool;

#[async_trait]
impl Tool for ManaTool {
    fn name(&self) -> &str {
        "mana"
    }
    fn label(&self) -> &str {
        "Mana"
    }
    fn description(&self) -> &str {
        "Work coordination substrate. Prefer this over bash for mana operations: inspect units, create/update/claim/release/close work, inspect logs/agents, and run orchestration natively. Use for complex tasks or delegation."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["status", "list", "show", "create", "close", "update", "run", "claim", "release", "logs", "agents", "next", "tree"] },
                "id": { "type": "string" },
                "title": { "type": "string" },
                "verify": { "type": "string", "description": "Shell command, must exit 0" },
                "description": { "type": "string" },
                "parent": { "type": "string" },
                "deps": { "type": "string", "description": "Comma-separated IDs" },
                "status": { "type": "string" },
                "notes": { "type": "string", "description": "Progress log (update)" },
                "priority": { "type": "integer" },
                "labels": { "type": "string" },
                "force": { "type": "boolean" },
                "reason": { "type": "string" },
                "all": { "type": "boolean" },
                "by": { "type": "string", "description": "Who is claiming the unit" },
                "count": { "type": "integer", "description": "Number of next recommendations to return" },
                "background": { "type": "boolean", "description": "Run mana orchestration in the background and return immediately (default true unless dry_run=true)" }
            },
            "required": ["action"],
        })
    }
    fn is_readonly(&self) -> bool {
        false
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let action = params["action"]
            .as_str()
            .ok_or_else(|| crate::error::Error::Tool("missing 'action' parameter".into()))?;

        let mode = ctx.mode;

        if !mode.allows_mana_action(action) {
            let mode_name = format!("{mode:?}").to_lowercase();
            return Ok(ToolOutput::error(format!(
                "Mana action '{action}' is not available in {mode_name} mode"
            )));
        }

        let mana_dir = find_mana_dir(&ctx.cwd).map_err(crate::error::Error::Tool)?;

        match action {
            "status" => match mana_core::api::get_status(&mana_dir) {
                Ok(status) => Ok(json_output(&status)),
                Err(e) => Ok(ToolOutput::error(e.to_string())),
            },
            "list" => {
                let list_params = mana_core::ops::list::ListParams {
                    status: params["status"].as_str().map(|s| s.to_string()),
                    priority: params["priority"].as_u64().map(|p| p as u8),
                    parent: params["parent"].as_str().map(|s| s.to_string()),
                    label: params["label"].as_str().map(|s| s.to_string()),
                    assignee: None,
                    current_user: None,
                    include_closed: params["all"].as_bool().unwrap_or(false),
                };
                match mana_core::api::list_units(&mana_dir, &list_params) {
                    Ok(entries) => Ok(json_output(&entries)),
                    Err(e) => {
                        let message = format!("mana run failed: {e}");
                        ctx.ui
                            .set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
                            .await;
                        ctx.ui.set_status("mana", Some(&message)).await;
                        Ok(ToolOutput::error(e.to_string()))
                    },
                }
            }
            "show" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("show requires 'id'".into()))?;
                match mana_core::api::get_unit(&mana_dir, id) {
                    Ok(unit) => Ok(json_output(&unit)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "create" => {
                let title = params["title"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("create requires 'title'".into()))?;
                let deps: Vec<String> = params["deps"]
                    .as_str()
                    .map(|s| {
                        s.split(',')
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                let labels: Vec<String> = params["labels"]
                    .as_str()
                    .map(|s| {
                        s.split(',')
                            .map(|l| l.trim().to_string())
                            .filter(|l| !l.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                let create_params = mana_core::ops::create::CreateParams {
                    title: title.to_string(),
                    description: params["description"].as_str().map(|s| s.to_string()),
                    acceptance: None,
                    notes: None,
                    design: None,
                    verify: params["verify"].as_str().map(|s| s.to_string()),
                    priority: params["priority"].as_u64().map(|p| p as u8),
                    labels,
                    assignee: None,
                    dependencies: deps,
                    parent: params["parent"].as_str().map(|s| s.to_string()),
                    produces: Vec::new(),
                    requires: Vec::new(),
                    paths: Vec::new(),
                    on_fail: None,
                    fail_first: false,
                    feature: false,
                    verify_timeout: None,
                    decisions: Vec::new(),
                    force: true,
                };
                match mana_core::api::create_unit(&mana_dir, create_params) {
                    Ok(result) => Ok(json_output(&result)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "claim" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("claim requires 'id'".into()))?;
                let claim_params = ClaimParams {
                    by: params["by"].as_str().map(|s| s.to_string()),
                    force: params["force"].as_bool().unwrap_or(true),
                };
                match mana_core::api::claim_unit(&mana_dir, id, claim_params) {
                    Ok(result) => Ok(claim_output(&result)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "release" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("release requires 'id'".into()))?;
                match mana_core::api::release_unit(&mana_dir, id) {
                    Ok(result) => Ok(release_output(&result)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "close" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("close requires 'id'".into()))?;
                let opts = mana_core::ops::close::CloseOpts {
                    reason: params["reason"].as_str().map(|s| s.to_string()),
                    force: params["force"].as_bool().unwrap_or(false),
                    defer_verify: false,
                };
                match mana_core::api::close_unit(&mana_dir, id, opts) {
                    Ok(outcome) => Ok(json_output(&outcome)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "update" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("update requires 'id'".into()))?;
                let update_params = mana_core::ops::update::UpdateParams {
                    title: params["title"].as_str().map(|s| s.to_string()),
                    description: params["description"].as_str().map(|s| s.to_string()),
                    acceptance: None,
                    notes: params["notes"].as_str().map(|s| s.to_string()),
                    design: None,
                    status: params["status"].as_str().map(|s| s.to_string()),
                    priority: params["priority"].as_u64().map(|p| p as u8),
                    assignee: None,
                    add_label: None,
                    remove_label: None,
                    decisions: Vec::new(),
                    resolve_decisions: Vec::new(),
                };
                match mana_core::api::update_unit(&mana_dir, id, update_params) {
                    Ok(result) => Ok(json_output(&result)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "logs" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("logs requires 'id'".into()))?;
                match find_all_logs(id) {
                    Ok(paths) if paths.is_empty() => Ok(ToolOutput::text(format!(
                        "No logs for unit {id}. Has it been dispatched with mana run?"
                    ))),
                    Ok(paths) => {
                        let mut sections = Vec::new();
                        for path in &paths {
                            let filename = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown");
                            let body = std::fs::read_to_string(path)
                                .unwrap_or_else(|e| format!("(error reading {}: {e})", path.display()));
                            sections.push(format!("═══ {filename} ═══\n\n{body}"));
                        }
                        let text = truncate_with_note(&sections.join("\n\n"));
                        Ok(text_output(text, json!({ "unit_id": id, "logs": paths })))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "agents" => match load_agents() {
                Ok(agents) => Ok(json_output(&agents)),
                Err(e) => Ok(ToolOutput::error(e.to_string())),
            },
            "next" => {
                let count = params["count"].as_u64().unwrap_or(1).max(1) as usize;
                match mana_core::api::load_index(&mana_dir) {
                    Ok(index) => {
                        let ready: Vec<&mana_core::index::IndexEntry> = index
                            .units
                            .iter()
                            .filter(|e| {
                                e.status == mana_core::unit::Status::Open
                                    && e.has_verify
                                    && !e.feature
                                    && mana_core::blocking::check_blocked(e, &index).is_none()
                            })
                            .collect();

                        let mut reverse_deps: std::collections::HashMap<String, Vec<String>> =
                            std::collections::HashMap::new();
                        for entry in &index.units {
                            for dep_id in &entry.dependencies {
                                reverse_deps
                                    .entry(dep_id.clone())
                                    .or_default()
                                    .push(entry.id.clone());
                            }
                        }

                        fn count_transitive_unblocks(
                            unit_id: &str,
                            reverse_deps: &std::collections::HashMap<String, Vec<String>>,
                        ) -> usize {
                            let mut visited = std::collections::HashSet::new();
                            let mut stack = vec![unit_id.to_string()];
                            while let Some(current) = stack.pop() {
                                if let Some(dependents) = reverse_deps.get(&current) {
                                    for dep in dependents {
                                        if visited.insert(dep.clone()) {
                                            stack.push(dep.clone());
                                        }
                                    }
                                }
                            }
                            visited.len()
                        }

                        fn score_unit(entry: &mana_core::index::IndexEntry, unblock_count: usize) -> f64 {
                            let priority_score =
                                (5u8.saturating_sub(entry.priority)) as f64 * 10.0;
                            let unblock_score = (unblock_count as f64 * 5.0).min(50.0);
                            let age_days = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                                / 86_400;
                            let created_days = entry.created_at.timestamp().max(0) as u64 / 86_400;
                            let age_days = age_days.saturating_sub(created_days) as f64;
                            let age_score = age_days.min(30.0);
                            let attempt_penalty = (entry.attempts as f64 * 3.0).min(15.0);
                            priority_score + unblock_score + age_score - attempt_penalty
                        }

                        let mut scored: Vec<ScoredUnit> = ready
                            .iter()
                            .map(|entry| {
                                let transitive_count =
                                    count_transitive_unblocks(&entry.id, &reverse_deps);
                                let unblocks = reverse_deps
                                    .get(&entry.id)
                                    .cloned()
                                    .unwrap_or_default();
                                let score = score_unit(entry, transitive_count);
                                let now_days = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs()
                                    / 86_400;
                                let created_days = entry.created_at.timestamp().max(0) as u64 / 86_400;
                                let age_days = now_days.saturating_sub(created_days);
                                ScoredUnit {
                                    id: entry.id.clone(),
                                    title: entry.title.clone(),
                                    priority: entry.priority,
                                    score,
                                    unblocks,
                                    age_days,
                                    attempts: entry.attempts,
                                }
                            })
                            .collect();

                        scored.sort_by(|a, b| {
                            b.score
                                .partial_cmp(&a.score)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        scored.truncate(count);
                        Ok(text_output(
                            scored_units_to_text(&scored),
                            serde_json::to_value(&scored)
                                .unwrap_or(serde_json::Value::Null),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "tree" => {
                let id = params["id"].as_str();
                let lines = if let Some(root_id) = id {
                    match mana_core::api::get_tree(&mana_dir, root_id) {
                        Ok(tree) => {
                            let mut lines = Vec::new();
                            tree_lines(&tree, 0, &mut lines);
                            lines
                        }
                        Err(e) => return Ok(ToolOutput::error(e.to_string())),
                    }
                } else {
                    match mana_core::api::load_index(&mana_dir) {
                        Ok(index) => {
                            let roots: Vec<_> = index
                                .units
                                .iter()
                                .filter(|entry| entry.parent.is_none())
                                .map(|entry| entry.id.clone())
                                .collect();
                            let mut lines = Vec::new();
                            for (idx, root_id) in roots.iter().enumerate() {
                                match mana_core::api::get_tree(&mana_dir, root_id) {
                                    Ok(tree) => {
                                        if idx > 0 {
                                            lines.push(String::new());
                                        }
                                        tree_lines(&tree, 0, &mut lines);
                                    }
                                    Err(e) => return Ok(ToolOutput::error(e.to_string())),
                                }
                            }
                            lines
                        }
                        Err(e) => return Ok(ToolOutput::error(e.to_string())),
                    }
                };
                let text = if lines.is_empty() {
                    "(no units)".to_string()
                } else {
                    truncate_with_note(&lines.join("\n"))
                };
                Ok(text_output(text, json!({ "root": id })))
            }
            "run" => {
                let run_args = RunArgs {
                    id: params["id"].as_str().map(|s| s.to_string()),
                    jobs: params["jobs"].as_u64().unwrap_or(4) as u32,
                    dry_run: params["dry_run"].as_bool().unwrap_or(false),
                    loop_mode: params["loop"].as_bool().unwrap_or(false),
                    keep_going: params["keep_going"].as_bool().unwrap_or(false),
                    timeout: params["timeout"].as_u64().unwrap_or(30) as u32,
                    idle_timeout: params["idle_timeout"].as_u64().unwrap_or(5) as u32,
                    json_stream: true,
                    review: params["review"].as_bool().unwrap_or(false),
                };
                let background = params["background"].as_bool().unwrap_or(!run_args.dry_run);

                if background {
                    let scope = run_args
                        .id
                        .as_deref()
                        .map(|id| format!("unit {id}"))
                        .unwrap_or_else(|| "all ready units".to_string());
                    let started = background_run_started_output(&scope, &run_args);
                    spawn_background_run(mana_dir.clone(), run_args, ctx);
                    return Ok(started);
                }

                send_update(
                    &ctx,
                    "Starting mana run...",
                    json!({"kind": "mana_run_status", "status": "starting"}),
                );
                ctx.ui
                    .set_widget(
                        "mana",
                        Some(mana_widget_lines(
                            "running mana".to_string(),
                            Some("native foreground orchestration".to_string()),
                        )),
                    )
                    .await;
                ctx.ui.set_status("mana", Some("mana: running")).await;

                match mana::commands::run::run_with_stream_capture_and_sink(
                    &mana_dir,
                    run_args,
                    Some(std::sync::Arc::new({
                        let update_tx = ctx.update_tx.clone();
                        move |event| {
                            let line = match &event {
                                mana::stream::StreamEvent::RunStart {
                                    total_units,
                                    total_rounds,
                                    ..
                                } => format!(
                                    "Mana run started: {total_units} jobs across {total_rounds} waves"
                                ),
                                mana::stream::StreamEvent::UnitStart { id, title, round, .. } => {
                                    format!("▶ {id}  {title}  wave {round}")
                                }
                                mana::stream::StreamEvent::UnitDone {
                                    id,
                                    success,
                                    duration_secs,
                                    error,
                                    ..
                                } => {
                                    if *success {
                                        format!("✓ {id}  done  {}s", duration_secs)
                                    } else {
                                        format!(
                                            "✗ {id}  failed  {}",
                                            error.clone().unwrap_or_else(|| "error".to_string())
                                        )
                                    }
                                }
                                mana::stream::StreamEvent::RunEnd {
                                    total_closed,
                                    total_failed,
                                    duration_secs,
                                    ..
                                } => format!(
                                    "Mana run finished: {total_closed} done · {total_failed} failed · {}s",
                                    duration_secs
                                ),
                                _ => return,
                            };
                            let _ = update_tx.try_send(ToolUpdate {
                                content: vec![imp_llm::ContentBlock::Text { text: line }],
                                details: serde_json::to_value(&event)
                                    .unwrap_or(serde_json::Value::Null),
                            });
                        }
                    })),
                ) {
                    Ok(view) => {
                        for line in run_summary_lines(&view) {
                            send_update(&ctx, line, json!({"kind": "mana_run_view", "view": view}));
                        }
                        let summary = format!(
                            "mana finished · {} done · {} failed",
                            view.summary.total_closed, view.summary.total_failed
                        );
                        ctx.ui
                            .set_widget("mana", Some(mana_widget_lines(summary.clone(), None)))
                            .await;
                        ctx.ui.set_status("mana", Some(&summary)).await;
                        Ok(ToolOutput {
                            content: run_summary_lines(&view)
                                .into_iter()
                                .map(|text| imp_llm::ContentBlock::Text { text })
                                .collect(),
                            details: serde_json::to_value(&view).unwrap_or(serde_json::Value::Null),
                            is_error: false,
                        })
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            other => Ok(ToolOutput::error(format!(
                "Unknown action: {other}. Use: status, list, show, create, close, update, run, claim, release, logs, agents, next, tree"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::mpsc;

    use super::ManaTool;
    use crate::tools::{FileCache, FileTracker, Tool, ToolContext, ToolUpdate};
    use crate::ui::NullInterface;

    enum ManaResult {
        ModeBlocked(String),
        Attempted(crate::tools::ToolOutput),
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    async fn run_with_mode(mode_name: &str, action: &str) -> ManaResult {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("IMP_MODE").ok();
        std::env::set_var("IMP_MODE", mode_name);

        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            mode: crate::config::AgentMode::from_name(mode_name)
                .unwrap_or(crate::config::AgentMode::Full),
            read_max_lines: 500,
        };

        let tool = ManaTool;
        let outcome = tool
            .execute("call_1", json!({ "action": action, "id": "1" }), ctx)
            .await;

        match prev {
            Some(v) => std::env::set_var("IMP_MODE", v),
            None => std::env::remove_var("IMP_MODE"),
        }

        match outcome {
            Err(crate::error::Error::Tool(msg)) => {
                ManaResult::Attempted(crate::tools::ToolOutput::error(msg))
            }
            Err(e) => ManaResult::Attempted(crate::tools::ToolOutput::error(e.to_string())),
            Ok(output) => {
                if output.is_error {
                    if let Some(text) = output.text_content() {
                        if text.contains("mode") && text.contains(action) {
                            return ManaResult::ModeBlocked(text.to_string());
                        }
                    }
                }
                ManaResult::Attempted(output)
            }
        }
    }

    fn ctx_with_mode(
        dir: &std::path::Path,
        mode: crate::config::AgentMode,
    ) -> (ToolContext, tempfile::TempDir) {
        let mana_dir = dir.join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let ctx = ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            mode,
            read_max_lines: 500,
        };
        (ctx, tempfile::tempdir().unwrap())
    }

    async fn run_with_ctx_mode(mode: crate::config::AgentMode, action: &str) -> ManaResult {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), mode);
        let tool = ManaTool;
        let outcome = tool
            .execute("call_ctx", json!({ "action": action, "id": "1" }), ctx)
            .await;
        match outcome {
            Err(crate::error::Error::Tool(msg)) => {
                ManaResult::Attempted(crate::tools::ToolOutput::error(msg))
            }
            Err(e) => ManaResult::Attempted(crate::tools::ToolOutput::error(e.to_string())),
            Ok(output) => {
                if output.is_error {
                    if let Some(text) = output.text_content() {
                        if text.contains("mode") && text.contains(action) {
                            return ManaResult::ModeBlocked(text.to_string());
                        }
                    }
                }
                ManaResult::Attempted(output)
            }
        }
    }

    #[tokio::test]
    async fn worker_blocks_create() {
        match run_with_mode("worker", "create").await {
            ManaResult::ModeBlocked(_) => {}
            ManaResult::Attempted(out) => {
                panic!(
                    "worker should block 'create', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn planner_allows_create() {
        match run_with_mode("planner", "create").await {
            ManaResult::Attempted(_) => {}
            ManaResult::ModeBlocked(msg) => {
                panic!("planner should allow 'create' but was blocked: {msg}")
            }
        }
    }

    #[tokio::test]
    async fn planner_blocks_close() {
        match run_with_mode("planner", "close").await {
            ManaResult::ModeBlocked(_) => {}
            ManaResult::Attempted(out) => {
                panic!(
                    "planner should block 'close', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn auditor_allows_show() {
        match run_with_mode("auditor", "show").await {
            ManaResult::Attempted(_) => {}
            ManaResult::ModeBlocked(msg) => {
                panic!("auditor should allow 'show' but was blocked: {msg}")
            }
        }
    }

    #[tokio::test]
    async fn auditor_blocks_update() {
        match run_with_mode("auditor", "update").await {
            ManaResult::ModeBlocked(_) => {}
            ManaResult::Attempted(out) => {
                panic!(
                    "auditor should block 'update', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn worker_allows_logs() {
        match run_with_mode("worker", "logs").await {
            ManaResult::Attempted(_) => {}
            ManaResult::ModeBlocked(msg) => {
                panic!("worker should allow 'logs' but was blocked: {msg}")
            }
        }
    }

    #[tokio::test]
    async fn orchestrator_allows_extended_actions() {
        for action in &[
            "status", "list", "show", "create", "close", "update", "run", "claim", "release",
            "logs", "agents", "next",
        ] {
            match run_with_mode("orchestrator", action).await {
                ManaResult::Attempted(_) => {}
                ManaResult::ModeBlocked(msg) => {
                    panic!("orchestrator should allow '{action}' but was blocked: {msg}")
                }
            }
        }
    }

    #[tokio::test]
    async fn ctx_mode_wins_over_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("IMP_MODE").ok();
        std::env::set_var("IMP_MODE", "full");

        let result = run_with_ctx_mode(crate::config::AgentMode::Worker, "create").await;

        match prev {
            Some(v) => std::env::set_var("IMP_MODE", v),
            None => std::env::remove_var("IMP_MODE"),
        }

        match result {
            ManaResult::ModeBlocked(_) => {}
            ManaResult::Attempted(out) => {
                panic!(
                    "ctx.mode=Worker should block 'create' even when IMP_MODE=full, got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn ctx_worker_blocks_create() {
        match run_with_ctx_mode(crate::config::AgentMode::Worker, "create").await {
            ManaResult::ModeBlocked(_) => {}
            ManaResult::Attempted(out) => {
                panic!(
                    "ctx Worker mode should block 'create', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn ctx_full_allows_extended_actions() {
        for action in &[
            "status", "list", "show", "create", "close", "update", "run", "claim", "release",
            "logs", "agents", "next", "tree",
        ] {
            match run_with_ctx_mode(crate::config::AgentMode::Full, action).await {
                ManaResult::Attempted(_) => {}
                ManaResult::ModeBlocked(msg) => {
                    panic!("ctx Full mode should allow '{action}' but was blocked: {msg}")
                }
            }
        }
    }

    #[tokio::test]
    async fn next_returns_ranked_text() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool;
        let result = tool
            .execute("call_next", json!({ "action": "next", "count": 1 }), ctx)
            .await
            .unwrap();
        let text = result.text_content().unwrap_or("");
        assert!(text.contains("Test unit") || text.contains("No ready units"));
    }

    #[tokio::test]
    async fn background_run_returns_promptly() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool;
        let result = tool
            .execute(
                "call_bg",
                json!({ "action": "run", "background": true, "dry_run": true }),
                ctx,
            )
            .await
            .unwrap();
        let text = result.text_content().unwrap_or("");
        assert!(text.contains("Started mana run in background"));
        assert_eq!(result.details["background"], true);
    }
}
