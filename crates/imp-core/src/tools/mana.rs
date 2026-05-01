use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use mana::commands::agents::{agents_file_path, load_agents};
use mana::commands::logs::find_all_logs;
use mana::commands::next::ScoredUnit;
use mana::commands::run::{NativeRunParams, RunSummary, RunTarget, RunUnitStatus, RunView};
use mana::stream::StreamEvent;
use mana_core::ops::claim::ClaimParams;
use mana_core::unit::{OnFailAction, UnitType};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput, ToolUpdate, truncate_head};
use crate::error::Result;
use crate::ui::{NotifyLevel, WidgetContent};
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_STORED_RUN_EVENTS: usize = 64;
const MAX_PERSISTED_RUN_LOG_LINES: usize = 50;
const FINISHED_RUN_TTL_MS: u128 = 60 * 60 * 1000;
const INTERRUPTED_RUN_STALE_MS: u128 = 6 * 60 * 60 * 1000;

fn find_mana_dir(cwd: &Path) -> std::result::Result<std::path::PathBuf, String> {
    mana_core::discovery::find_mana_dir(cwd).map_err(|e| e.to_string())
}

fn resolve_mana_dir(
    cwd: &Path,
    params: &serde_json::Value,
) -> std::result::Result<std::path::PathBuf, String> {
    // Transitional compatibility: runtime still accepts legacy alias fields even though
    // the model-facing schema advertises only canonical `scope` and `path`.
    let scope = params
        .get("scope")
        .and_then(|v| v.as_str())
        .or_else(|| params.get("mana_scope").and_then(|v| v.as_str()))
        .unwrap_or("auto");

    if let Some(explicit) = params
        .get("path")
        .and_then(|v| v.as_str())
        .or_else(|| params.get("mana_dir").and_then(|v| v.as_str()))
    {
        let path = Path::new(explicit);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        return Ok(
            if resolved.file_name().and_then(|name| name.to_str()) == Some(".mana") {
                resolved
            } else {
                resolved.join(".mana")
            },
        );
    }

    match scope {
        "auto" | "project" => find_mana_dir(cwd),
        "root" => mana_core::discovery::find_outermost_mana_dir(cwd).map_err(|e| e.to_string()),
        other => Err(format!(
            "Unknown mana scope '{other}'. Use auto, project, or root."
        )),
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeRunParamsView {
    target: serde_json::Value,
    jobs: u32,
    dry_run: bool,
    loop_mode: bool,
    keep_going: bool,
    timeout: u32,
    idle_timeout: u32,
    review: bool,
}

impl From<&NativeRunParams> for NativeRunParamsView {
    fn from(args: &NativeRunParams) -> Self {
        let target = match &args.target {
            RunTarget::AllReady => json!({"kind": "all_ready"}),
            RunTarget::Unit(id) => json!({"kind": "unit", "id": id}),
            RunTarget::Explicit(ids) => json!({"kind": "explicit", "ids": ids}),
        };
        Self {
            target,
            jobs: args.jobs,
            dry_run: args.dry_run,
            loop_mode: args.loop_mode,
            keep_going: args.keep_going,
            timeout: args.timeout,
            idle_timeout: args.idle_timeout,
            review: args.review,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NativeRunState {
    run_id: String,
    scope: String,
    background: bool,
    status: String,
    error: Option<String>,
    started_at_ms: u128,
    finished_at_ms: Option<u128>,
    #[serde(default)]
    last_event_at_ms: u128,
    args: NativeRunParamsView,
    runtime: Option<serde_json::Value>,
    summary: RunSummary,
    units: Vec<RunUnitStatus>,
    log_lines: Vec<String>,
    event_count: usize,
}

impl NativeRunState {
    fn new(run_id: String, scope: String, background: bool, args: &NativeRunParams) -> Self {
        Self {
            run_id,
            scope,
            background,
            status: "starting".to_string(),
            error: None,
            started_at_ms: unix_time_ms(),
            finished_at_ms: None,
            last_event_at_ms: unix_time_ms(),
            args: NativeRunParamsView::from(args),
            runtime: None,
            summary: RunSummary {
                total_units: 0,
                total_rounds: 0,
                total_closed: 0,
                total_failed: 0,
                total_abandoned: 0,
                total_awaiting_verify: 0,
                total_skipped: 0,
                duration_secs: 0,
            },
            units: Vec::new(),
            log_lines: Vec::new(),
            event_count: 0,
        }
    }

    fn apply_event(&mut self, event: &StreamEvent) {
        self.event_count += 1;
        self.last_event_at_ms = unix_time_ms();
        if let Some(line) = stream_event_line(event) {
            self.log_lines.push(line);
            trim_log_lines(&mut self.log_lines, MAX_STORED_RUN_EVENTS);
        }

        match event {
            StreamEvent::RunStart {
                total_units,
                total_rounds,
                units,
                runtime,
                ..
            } => {
                self.status = "running".to_string();
                self.summary.total_units = *total_units;
                self.summary.total_rounds = *total_rounds;
                self.runtime = runtime
                    .as_ref()
                    .and_then(|value| serde_json::to_value(value).ok());
                self.units = units
                    .iter()
                    .map(|info| RunUnitStatus {
                        id: info.id.clone(),
                        title: info.title.clone(),
                        status: "queued".to_string(),
                        round: Some(info.round),
                        agent: None,
                        model: None,
                        duration_secs: None,
                        tool_count: None,
                        turns: None,
                        failure_summary: None,
                        error: None,
                    })
                    .collect();
                self.units.sort_by(|a, b| a.id.cmp(&b.id));
            }
            StreamEvent::RunPlan {
                total_units,
                runtime,
                ..
            } => {
                self.status = "running".to_string();
                self.summary.total_units = (*total_units).max(self.summary.total_units);
                if runtime.is_some() {
                    self.runtime = runtime
                        .as_ref()
                        .and_then(|value| serde_json::to_value(value).ok());
                }
            }
            StreamEvent::RoundStart { total_rounds, .. } => {
                self.status = "running".to_string();
                self.summary.total_rounds = (*total_rounds).max(self.summary.total_rounds);
            }
            StreamEvent::UnitReady { id, title, .. } => {
                let unit = ensure_unit_status(&mut self.units, id, title);
                unit.status = "queued".to_string();
            }
            StreamEvent::UnitStart {
                id, title, round, ..
            } => {
                self.status = "running".to_string();
                let unit = ensure_unit_status(&mut self.units, id, title);
                unit.title = title.clone();
                unit.round = Some(*round);
                unit.status = "running".to_string();
            }
            StreamEvent::UnitDone {
                id,
                success,
                duration_secs,
                error,
                tool_count,
                turns,
                failure_summary,
                ..
            } => {
                let unit = ensure_unit_status(&mut self.units, id, id);
                unit.status = if *success { "done" } else { "failed" }.to_string();
                unit.duration_secs = Some(*duration_secs);
                unit.tool_count = *tool_count;
                unit.turns = *turns;
                unit.failure_summary = failure_summary.clone();
                unit.error = error.clone();
            }
            StreamEvent::BatchVerify { passed, failed, .. } => {
                for id in passed {
                    let unit = ensure_unit_status(&mut self.units, id, id);
                    unit.status = "done".to_string();
                }
                for id in failed {
                    let unit = ensure_unit_status(&mut self.units, id, id);
                    unit.status = "failed".to_string();
                }
            }
            StreamEvent::RunEnd {
                total_closed,
                total_failed,
                total_abandoned,
                total_awaiting_verify,
                total_skipped,
                duration_secs,
                ..
            } => {
                self.summary.total_closed = *total_closed;
                self.summary.total_failed = *total_failed;
                self.summary.total_abandoned = *total_abandoned;
                self.summary.total_awaiting_verify = *total_awaiting_verify;
                self.summary.total_skipped = *total_skipped;
                self.summary.duration_secs = *duration_secs;
                self.status = "finished".to_string();
                self.finished_at_ms = Some(unix_time_ms());
            }
            StreamEvent::DryRun { runtime, .. } => {
                self.status = "finished".to_string();
                if runtime.is_some() {
                    self.runtime = runtime
                        .as_ref()
                        .and_then(|value| serde_json::to_value(value).ok());
                }
                self.finished_at_ms = Some(unix_time_ms());
            }
            StreamEvent::Error { message } => {
                self.status = "failed".to_string();
                self.error = Some(message.clone());
                self.finished_at_ms = Some(unix_time_ms());
            }
            _ => {}
        }
    }

    fn finish_with_view(&mut self, view: &RunView) {
        let now = unix_time_ms();
        self.summary = view.summary.clone();
        self.units = view.units.clone();
        self.runtime = view
            .runtime
            .as_ref()
            .and_then(|value| serde_json::to_value(value).ok());
        self.status = "finished".to_string();
        self.error = None;
        self.finished_at_ms = Some(now);
        self.last_event_at_ms = now;
    }

    fn fail(&mut self, error: String) {
        let now = unix_time_ms();
        self.status = "failed".to_string();
        self.error = Some(error.clone());
        self.finished_at_ms = Some(now);
        self.last_event_at_ms = now;
        self.log_lines.push(error);
        trim_log_lines(&mut self.log_lines, MAX_STORED_RUN_EVENTS);
    }

    fn persisted(&self) -> Self {
        let mut state = self.clone();
        trim_log_lines(&mut state.log_lines, MAX_PERSISTED_RUN_LOG_LINES);
        state
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ManaRunStore {
    next_id: u64,
    runs: Vec<NativeRunState>,
}

impl ManaRunStore {
    fn start_run(&mut self, scope: String, background: bool, args: &NativeRunParams) -> String {
        self.next_id += 1;
        let run_id = format!("run-{}", self.next_id);
        self.runs
            .push(NativeRunState::new(run_id.clone(), scope, background, args));
        self.trim_history();
        run_id
    }

    fn persist(&self) {
        let path = run_state_file();
        let persisted = Self {
            next_id: self.next_id,
            runs: self.runs.iter().map(NativeRunState::persisted).collect(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&persisted) {
            let _ = std::fs::write(path, json);
        }
    }

    fn load_persisted() -> Self {
        let path = run_state_file();
        if !path.exists() {
            return Self::default();
        }

        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        if contents.trim().is_empty() {
            return Self::default();
        }

        let Ok(mut store) = serde_json::from_str::<Self>(&contents) else {
            return Self::default();
        };

        store.discard_expired_finished_runs();
        store.classify_stale_unfinished_runs();
        store.trim_history();
        store
    }

    fn discard_expired_finished_runs(&mut self) {
        let cutoff = unix_time_ms().saturating_sub(FINISHED_RUN_TTL_MS);
        self.runs.retain(|run| match run.finished_at_ms {
            Some(finished_at_ms) => finished_at_ms >= cutoff,
            None => true,
        });
    }

    fn classify_stale_unfinished_runs(&mut self) {
        let cutoff = unix_time_ms().saturating_sub(INTERRUPTED_RUN_STALE_MS);
        for run in &mut self.runs {
            if (run.status == "starting" || run.status == "running")
                && run.finished_at_ms.is_none()
                && run.last_event_at_ms > 0
                && run.last_event_at_ms < cutoff
            {
                run.status = "interrupted".to_string();
                run.error = Some(
                    "Run state is stale after process restart or lost background worker; inspect logs before rerun".to_string(),
                );
                run.finished_at_ms = Some(run.last_event_at_ms);
                run.log_lines.push(
                    "Run marked interrupted: stale persisted running state after reload"
                        .to_string(),
                );
                trim_log_lines(&mut run.log_lines, MAX_STORED_RUN_EVENTS);
            }
        }
    }

    fn update_with_event(&mut self, run_id: &str, event: &StreamEvent) {
        if let Some(run) = self.runs.iter_mut().find(|run| run.run_id == run_id) {
            run.apply_event(event);
        }
    }

    fn finish_run(&mut self, run_id: &str, view: &RunView) {
        if let Some(run) = self.runs.iter_mut().find(|run| run.run_id == run_id) {
            run.finish_with_view(view);
        }
        self.trim_history();
    }

    fn fail_run(&mut self, run_id: &str, error: String) {
        if let Some(run) = self.runs.iter_mut().find(|run| run.run_id == run_id) {
            run.fail(error);
        }
        self.trim_history();
    }

    fn snapshot(&self, run_id: Option<&str>) -> Option<NativeRunState> {
        if let Some(run_id) = run_id {
            return self.runs.iter().find(|run| run.run_id == run_id).cloned();
        }

        self.runs
            .iter()
            .rev()
            .find(|run| run.status == "starting" || run.status == "running")
            .cloned()
            .or_else(|| self.runs.last().cloned())
    }

    fn trim_history(&mut self) {
        while self.runs.len() > 8 {
            let newest_index = self.runs.len().saturating_sub(1);
            if let Some(index) =
                self.runs
                    .iter()
                    .enumerate()
                    .take(newest_index)
                    .find_map(|(index, run)| {
                        (run.status != "starting" && run.status != "running").then_some(index)
                    })
            {
                self.runs.remove(index);
            } else if !self.runs.is_empty() {
                self.runs.remove(0);
            } else {
                break;
            }
        }
    }
}

fn trim_log_lines(log_lines: &mut Vec<String>, max_lines: usize) {
    if log_lines.len() > max_lines {
        let overflow = log_lines.len() - max_lines;
        log_lines.drain(0..overflow);
    }
}

fn run_state_file() -> std::path::PathBuf {
    if let Ok(path) = agents_file_path() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
            return dir.join("run_state.json");
        }
    }

    let dir = std::env::var("HOME")
        .map(|h| {
            std::path::PathBuf::from(h)
                .join(".local")
                .join("share")
                .join("units")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp").join("mana"));
    std::fs::create_dir_all(&dir).ok();
    dir.join("run_state.json")
}

fn unix_time_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn ensure_unit_status<'a>(
    units: &'a mut Vec<RunUnitStatus>,
    id: &str,
    title: &str,
) -> &'a mut RunUnitStatus {
    if let Some(index) = units.iter().position(|unit| unit.id == id) {
        return &mut units[index];
    }

    units.push(RunUnitStatus {
        id: id.to_string(),
        title: title.to_string(),
        status: "queued".to_string(),
        round: None,
        agent: None,
        model: None,
        duration_secs: None,
        tool_count: None,
        turns: None,
        failure_summary: None,
        error: None,
    });
    let index = units.len() - 1;
    &mut units[index]
}

fn stream_event_line(event: &StreamEvent) -> Option<String> {
    match event {
        StreamEvent::RunStart {
            total_units,
            total_rounds,
            ..
        } => Some(format!(
            "Mana run started: {total_units} jobs across {total_rounds} waves"
        )),
        StreamEvent::RunPlan {
            waves,
            file_overlaps,
            ..
        } => Some(format!(
            "Plan ready: {} waves · {} overlapping file groups",
            waves.len(),
            file_overlaps.len()
        )),
        StreamEvent::RoundStart {
            round,
            total_rounds,
            unit_count,
        } => Some(format!(
            "Round {round}/{total_rounds}: {unit_count} unit(s)"
        )),
        StreamEvent::UnitReady {
            id,
            title,
            unblocked_by,
        } => Some(format!("Ready: {id} {title} (unblocked by {unblocked_by})")),
        StreamEvent::UnitStart {
            id, title, round, ..
        } => Some(format!("▶ {id}  {title}  wave {round}")),
        StreamEvent::UnitThinking { id, text } => {
            Some(format!("… {id}  {}", truncate_line_for_log(text)))
        }
        StreamEvent::UnitTool {
            id,
            tool_name,
            tool_count,
            file_path,
        } => Some(match file_path {
            Some(path) => format!("⚙ {id}  #{tool_count} {tool_name}  {path}"),
            None => format!("⚙ {id}  #{tool_count} {tool_name}"),
        }),
        StreamEvent::UnitTokens {
            id,
            input_tokens,
            output_tokens,
            cost,
            ..
        } => Some(format!(
            "$ {id}  in {input_tokens} · out {output_tokens} · ${cost:.4}"
        )),
        StreamEvent::UnitDone {
            id,
            success,
            duration_secs,
            error,
            ..
        } => Some(if *success {
            format!("✓ {id}  done  {duration_secs}s")
        } else {
            format!(
                "✗ {id}  failed  {}",
                error.clone().unwrap_or_else(|| "error".to_string())
            )
        }),
        StreamEvent::RoundEnd {
            round,
            success_count,
            failed_count,
        } => Some(format!(
            "Round {round} complete: {success_count} done · {failed_count} failed"
        )),
        StreamEvent::RunEnd {
            total_closed,
            total_failed,
            duration_secs,
            ..
        } => Some(format!(
            "Mana run finished: {total_closed} done · {total_failed} failed · {duration_secs}s"
        )),
        StreamEvent::BatchVerify {
            commands_run,
            passed,
            failed,
        } => Some(format!(
            "Batch verify: {commands_run} command(s) · {} passed · {} failed",
            passed.len(),
            failed.len()
        )),
        StreamEvent::VerifyGroupRun {
            command,
            unit_ids,
            success,
        } => Some(format!(
            "Verify command: {} · {} unit(s) · {}",
            truncate_line_for_log(command),
            unit_ids.len(),
            if *success { "passed" } else { "failed" }
        )),
        StreamEvent::DryRun { rounds, .. } => {
            Some(format!("Dry run: {} planned wave(s)", rounds.len()))
        }
        StreamEvent::Error { message } => Some(format!("Run error: {message}")),
    }
}

fn truncate_line_for_log(text: &str) -> String {
    const MAX_CHARS: usize = 160;
    let mut out = String::new();
    let mut chars = text.chars();
    for _ in 0..MAX_CHARS {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            return out;
        }
    }
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

fn update_run_store_with_event(
    store: &std::sync::Mutex<ManaRunStore>,
    run_id: &str,
    event: &StreamEvent,
) {
    if let Ok(mut store) = store.lock() {
        store.update_with_event(run_id, event);
        store.persist();
    }
}

fn finish_run_in_store(store: &std::sync::Mutex<ManaRunStore>, run_id: &str, view: &RunView) {
    if let Ok(mut store) = store.lock() {
        store.finish_run(run_id, view);
        store.persist();
    }
}

fn fail_run_in_store(store: &std::sync::Mutex<ManaRunStore>, run_id: &str, error: String) {
    if let Ok(mut store) = store.lock() {
        store.fail_run(run_id, error);
        store.persist();
    }
}

fn run_summary_lines(view: &RunView) -> Vec<String> {
    let mut lines = vec![format!(
        "Mana run: {} total · {} done · {} failed · {} candidate complete / awaiting verify · {} skipped",
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

async fn set_mana_delta_widget(
    ctx: &ToolContext,
    summary: impl Into<String>,
    detail: Option<String>,
) {
    ctx.ui
        .set_widget("mana", Some(mana_widget_lines(summary, detail)))
        .await;
}

fn unit_delta_label(unit: &serde_json::Value) -> Option<String> {
    let id = unit.get("id").and_then(|v| v.as_str())?;
    let title = unit
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(untitled)");
    Some(format!("{id} · {title}"))
}

fn target_from_params(params: &serde_json::Value) -> Result<RunTarget> {
    if let Some(values) = params["targets"].as_array() {
        let ids: Vec<String> = values
            .iter()
            .filter_map(|value| value.as_str().map(|s| s.to_string()))
            .collect();
        if ids.is_empty() {
            return Err(crate::error::Error::Tool(
                "mana run targets must contain at least one string id".into(),
            ));
        }
        return Ok(RunTarget::Explicit(ids));
    }

    if let Some(id) = params["id"].as_str() {
        return Ok(RunTarget::Unit(id.to_string()));
    }

    Ok(RunTarget::AllReady)
}

fn target_ids_from_run_target(target: &RunTarget) -> Vec<String> {
    match target {
        RunTarget::Unit(id) => vec![id.clone()],
        RunTarget::Explicit(ids) => ids.clone(),
        RunTarget::AllReady => Vec::new(),
    }
}

fn scope_from_target(target: &RunTarget) -> String {
    match target {
        RunTarget::AllReady => "all ready units".to_string(),
        RunTarget::Unit(id) => format!("unit {id}"),
        RunTarget::Explicit(ids) => format!("targets {}", ids.join(", ")),
    }
}

fn make_follow_up_summary(scope: &str, view: &RunView) -> String {
    let mut summary = if view.summary.total_failed > 0 {
        format!(
            "Native mana orchestration finished for {scope}: {} done, {} failed, {} candidate complete / awaiting verify.",
            view.summary.total_closed,
            view.summary.total_failed,
            view.summary.total_awaiting_verify
        )
    } else if view.summary.total_awaiting_verify > 0 {
        format!(
            "Native mana orchestration finished for {scope}: {} done, {} candidate complete / awaiting verify.",
            view.summary.total_closed, view.summary.total_awaiting_verify
        )
    } else {
        format!(
            "Native mana orchestration finished for {scope}: {} done, 0 failed.",
            view.summary.total_closed
        )
    };

    if let Some(runtime) = &view.runtime {
        let agent = runtime.direct_agent.as_deref().unwrap_or("imp-worker");
        let model = runtime.model.as_deref().unwrap_or("default-model");
        summary.push_str(&format!(
            " Orchestration ran through mana; worker runtime: {agent} · {model}."
        ));
    }

    summary.push_str(" Inspect with mana(action=\"run_state\") or mana(action=\"evaluate\").");
    summary
}

fn parse_csv_strings(value: &serde_json::Value, field_name: &str) -> Result<Vec<String>> {
    if let Some(values) = value.as_array() {
        let parsed = values
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(parsed);
    }

    if let Some(raw) = value.as_str() {
        return Ok(raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect());
    }

    if value.is_null() {
        return Ok(Vec::new());
    }

    Err(crate::error::Error::Tool(format!(
        "{field_name} must be a comma-separated string or array of strings"
    )))
}

fn parse_optional_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn parse_on_fail(value: &serde_json::Value) -> Result<Option<OnFailAction>> {
    if value.is_null() {
        return Ok(None);
    }

    if let Some(raw) = value.as_str() {
        return mana_core::ops::create::parse_on_fail(raw)
            .map(Some)
            .map_err(|e| crate::error::Error::Tool(e.to_string()));
    }

    let Some(obj) = value.as_object() else {
        return Err(crate::error::Error::Tool(
            "on_fail must be a string like 'retry:3'/'escalate:P1' or an object".into(),
        ));
    };

    let action = obj
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::Error::Tool("on_fail object requires 'action'".into()))?;

    match action {
        "retry" => Ok(Some(OnFailAction::Retry {
            max: obj.get("max").and_then(|v| v.as_u64()).map(|v| v as u32),
            delay_secs: obj.get("delay_secs").and_then(|v| v.as_u64()),
        })),
        "escalate" => Ok(Some(OnFailAction::Escalate {
            priority: obj
                .get("priority")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8),
            message: obj
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        })),
        other => Err(crate::error::Error::Tool(format!(
            "unsupported on_fail action: {other}"
        ))),
    }
}

fn parent_placement_details(
    parent: Option<&str>,
    parent_reason: Option<&str>,
) -> serde_json::Value {
    match (parent, parent_reason) {
        (Some(parent), Some(reason)) if !reason.trim().is_empty() => json!({
            "parent": parent,
            "parent_reason": reason,
            "warning": null,
            "hint": "Parent placement was explained explicitly.",
        }),
        (Some(parent), _) => json!({
            "parent": parent,
            "parent_reason": null,
            "warning": "parent_reason_missing",
            "hint": "Before creating follow-up work under the active epic, confirm it belongs to that product/scope. If this is a workflow, reliability, or cross-cutting issue, attach it to a matching epic or create one instead.",
        }),
        (None, _) => json!({
            "parent": null,
            "parent_reason": null,
            "warning": null,
            "hint": "No parent selected. For durable multi-step work, choose or create the matching epic before adding child tasks.",
        }),
    }
}

fn mana_close_force_reason_error(id: &str) -> ToolOutput {
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: format!(
                "mana close {id} with force=true requires reason with equivalent verify evidence"
            ),
        }],
        details: json!({
            "action": "close",
            "id": id,
            "ok": false,
            "force": true,
            "missing": ["reason"],
            "hint": "When stored verify is stale or invalid, rerun equivalent checks and close with force=true plus a reason that names the passing commands/evidence.",
            "example": {
                "action": "close",
                "id": id,
                "force": true,
                "reason": "Equivalent verify passed: cargo test -p imp-core mana -- --nocapture; commit abc123"
            }
        }),
        is_error: true,
    }
}

fn mana_close_error_output(id: &str, error: String) -> ToolOutput {
    let verify_related = is_close_verify_error(&error);
    let hint = if verify_related {
        Some(
            "If the stored verify command is stale or invalid, run equivalent focused checks, then close with force=true and reason containing the passing commands/evidence.",
        )
    } else {
        None
    };
    let text = match hint {
        Some(hint) => format!("{error}\n\nRecovery: {hint}"),
        None => error.clone(),
    };
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "action": "close",
            "id": id,
            "ok": false,
            "error": error,
            "verify_related": verify_related,
            "recovery_hint": hint,
            "force_requires_reason": true,
        }),
        is_error: true,
    }
}

fn is_close_verify_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("verify")
        || lower.contains("verification")
        || lower.contains("exit")
        || lower.contains("command")
        || lower.contains("timed out")
}

fn mana_validation_error(
    action: &str,
    missing: Vec<&'static str>,
    invalid: Vec<&'static str>,
    hint: &'static str,
    canonical_fields: Vec<&'static str>,
) -> ToolOutput {
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text {
            text: format!("mana {action} validation failed: {hint}"),
        }],
        details: json!({
            "action": action,
            "ok": false,
            "missing": missing,
            "invalid": invalid,
            "hint": hint,
            "canonical_fields": canonical_fields,
        }),
        is_error: true,
    }
}

fn has_text(params: &serde_json::Value, field: &str) -> bool {
    parse_optional_string(&params[field]).is_some()
}

fn has_nonempty_csv(params: &serde_json::Value, field: &str) -> bool {
    parse_csv_strings(&params[field], field)
        .map(|values| !values.is_empty())
        .unwrap_or(false)
}

fn validate_mana_action(action: &str, params: &serde_json::Value) -> Option<ToolOutput> {
    let missing = |fields: Vec<&'static str>, hint: &'static str, canonical: Vec<&'static str>| {
        Some(mana_validation_error(
            action,
            fields,
            Vec::new(),
            hint,
            canonical,
        ))
    };

    if params.get("path").is_some() && params.get("paths").is_none() {
        match action {
            "create" | "update" | "fact_create" => {
                return Some(mana_validation_error(
                    action,
                    Vec::new(),
                    vec!["path"],
                    "Use path for project/.mana location; use paths to attach relevant files to units.",
                    vec!["path", "paths"],
                ));
            }
            _ => {}
        }
    }

    match action {
        "show" | "claim" | "release" | "close" | "reopen" | "verify" | "fail" | "delete" => {
            if !has_text(params, "id") {
                return missing(vec!["id"], "Provide the unit id.", vec!["id"]);
            }
        }
        "create" => {
            if !has_text(params, "title") {
                return missing(
                    vec!["title"],
                    "create requires title. For executable tasks, include description, acceptance, and verify.",
                    vec!["title", "description", "acceptance", "verify", "paths"],
                );
            }
        }
        "update" => {
            if !has_text(params, "id") {
                return missing(
                    vec!["id"],
                    "update requires the unit id to modify.",
                    vec!["id"],
                );
            }
        }
        "notes_append" => {
            let mut fields = Vec::new();
            if !has_text(params, "id") {
                fields.push("id");
            }
            if !has_text(params, "notes") {
                fields.push("notes");
            }
            if !fields.is_empty() {
                return missing(
                    fields,
                    "notes_append requires id and notes; use notes for durable progress/context.",
                    vec!["id", "notes"],
                );
            }
        }
        "decision_add" => {
            let mut fields = Vec::new();
            if !has_text(params, "id") {
                fields.push("id");
            }
            if !has_text(params, "description") && !has_nonempty_csv(params, "decisions") {
                fields.push("description");
            }
            if !fields.is_empty() {
                return missing(
                    fields,
                    "decision_add requires id and description/decisions for scope, architecture, or sequencing choices.",
                    vec!["id", "description", "decisions"],
                );
            }
        }
        "decision_resolve" => {
            let mut fields = Vec::new();
            if !has_text(params, "id") {
                fields.push("id");
            }
            if !has_nonempty_csv(params, "resolve_decisions") {
                fields.push("resolve_decisions");
            }
            if !fields.is_empty() {
                return missing(
                    fields,
                    "decision_resolve requires id and resolve_decisions.",
                    vec!["id", "resolve_decisions"],
                );
            }
        }
        "dep_add" | "dep_remove" => {
            let mut fields = Vec::new();
            if !has_text(params, "from_id") {
                fields.push("from_id");
            }
            if !has_text(params, "dep_id") {
                fields.push("dep_id");
            }
            if !fields.is_empty() {
                return missing(
                    fields,
                    "Dependency edits require from_id and dep_id.",
                    vec!["from_id", "dep_id"],
                );
            }
        }
        "fact_create" => {
            let mut fields = Vec::new();
            if !has_text(params, "title") {
                fields.push("title");
            }
            if !has_text(params, "verify") {
                fields.push("verify");
            }
            if !fields.is_empty() {
                return missing(
                    fields,
                    "fact_create requires title and verify so the fact is re-checkable.",
                    vec!["title", "verify", "paths"],
                );
            }
        }
        "logs" => {
            if !has_text(params, "id") && !has_text(params, "run_id") {
                return missing(
                    vec!["id"],
                    "logs requires id or run_id.",
                    vec!["id", "run_id"],
                );
            }
        }
        "reparent" => {
            if !has_text(params, "id") {
                return missing(
                    vec!["id"],
                    "reparent requires id and parent.",
                    vec!["id", "parent", "reason"],
                );
            }
            if params.get("parent").is_none() {
                return missing(
                    vec!["parent"],
                    "reparent requires the new parent id. Root detach can be added as a separate explicit action later.",
                    vec!["id", "parent", "reason"],
                );
            }
        }
        "run_state" | "evaluate" => {
            if !has_text(params, "run_id") {
                return missing(
                    vec!["run_id"],
                    "run_state/evaluate requires run_id from mana action=run.",
                    vec!["run_id"],
                );
            }
        }
        "run" => {
            if params.get("target").is_some() && params.get("targets").is_none() {
                return Some(mana_validation_error(
                    action,
                    Vec::new(),
                    vec!["target"],
                    "Use targets for explicit unit ids; target is an internal run concept.",
                    vec!["targets", "id"],
                ));
            }
        }
        _ => {}
    }
    None
}

fn parse_unit_kind(value: &serde_json::Value) -> Result<Option<UnitType>> {
    let Some(raw) = value.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };

    match raw {
        "epic" => Ok(Some(UnitType::Epic)),
        "task" => Ok(Some(UnitType::Task)),
        // Transitional compatibility: accept legacy `job` at runtime, but keep it out
        // of the model-facing schema so new calls converge on `task`.
        "job" => Ok(Some(UnitType::Task)),
        "fact" => Ok(Some(UnitType::Fact)),
        other => Err(crate::error::Error::Tool(format!(
            "kind must be one of: epic, task, fact (legacy runtime alias: job; got {other})"
        ))),
    }
}

fn background_run_started_output(
    scope: &str,
    run_id: &str,
    run_args: &NativeRunParams,
) -> ToolOutput {
    let text = format!(
        "Started native mana orchestration in background for {scope} as {run_id}. Mana will coordinate the run and dispatch imp workers underneath. Use mana(action=\"run_state\", run_id=\"{run_id}\") for orchestration status, mana(action=\"logs\", run_id=\"{run_id}\") for recent native events, and mana(action=\"agents\") / mana(action=\"logs\", id=...) for worker output."
    );
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "background": true,
            "run_id": run_id,
            "scope": scope,
            "target": match &run_args.target {
                RunTarget::AllReady => json!({"kind": "all_ready"}),
                RunTarget::Unit(id) => json!({"kind": "unit", "id": id}),
                RunTarget::Explicit(ids) => json!({"kind": "explicit", "ids": ids}),
            },
            "jobs": run_args.jobs,
            "loop": run_args.loop_mode,
            "dry_run": run_args.dry_run,
            "review": run_args.review,
        }),
        is_error: false,
    }
}

fn spawn_background_run(
    mana_dir: std::path::PathBuf,
    run_args: NativeRunParams,
    ctx: ToolContext,
    run_store: Arc<std::sync::Mutex<ManaRunStore>>,
    run_id: String,
) {
    let ui = ctx.ui.clone();
    let command_tx = ctx.command_tx.clone();
    let scope = scope_from_target(&run_args.target);

    tokio::spawn(async move {
        ui.set_status(
            "mana",
            Some(&format!("mana orchestration: running {scope}")),
        )
        .await;
        ui.set_widget(
            "mana",
            Some(mana_widget_lines(
                format!("orchestrating {scope}"),
                Some(format!(
                    "native mana tool → mana orchestration → imp workers · inspect with mana run_state/logs (run_id={run_id})"
                )),
            )),
        )
        .await;

        let run_store_for_sink = run_store.clone();
        let run_id_for_sink = run_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            mana::commands::run::run_with_stream_capture_and_sink(
                &mana_dir,
                run_args,
                Some(Arc::new(move |event| {
                    update_run_store_with_event(&run_store_for_sink, &run_id_for_sink, &event);
                })),
            )
        })
        .await;

        match result {
            Ok(Ok(view)) => {
                finish_run_in_store(&run_store, &run_id, &view);
                let summary = format!(
                    "mana orchestration: {scope} finished · {} done · {} failed",
                    view.summary.total_closed, view.summary.total_failed
                );
                let runtime_detail = view
                    .runtime
                    .as_ref()
                    .map(|runtime| {
                        let agent = runtime.direct_agent.as_deref().unwrap_or("imp-worker");
                        let model = runtime.model.as_deref().unwrap_or("default-model");
                        format!(
                            "native mana tool → mana orchestration → {agent} workers · {scope} · {model}"
                        )
                    })
                    .unwrap_or_else(|| scope.clone());
                ui.set_status("mana", Some(&summary)).await;
                ui.set_widget(
                    "mana",
                    Some(mana_widget_lines(summary.clone(), Some(runtime_detail))),
                )
                .await;
                ui.notify(&summary, NotifyLevel::Info).await;
                if !ui.has_ui() {
                    let _ = command_tx
                        .send(crate::agent::AgentCommand::FollowUp(
                            make_follow_up_summary(&scope, &view),
                        ))
                        .await;
                }
                let ui_clear = ui.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(12)).await;
                    ui_clear.set_widget("mana", None).await;
                    ui_clear.set_status("mana", None).await;
                });
            }
            Ok(Err(err)) => {
                let message = format!("mana orchestration: {scope} failed: {err}");
                fail_run_in_store(&run_store, &run_id, message.clone());
                ui.set_status("mana", Some(&message)).await;
                ui.set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
                    .await;
                ui.notify(&message, NotifyLevel::Error).await;
                if !ui.has_ui() {
                    let _ = command_tx
                        .send(crate::agent::AgentCommand::FollowUp(format!(
                            "Native mana orchestration failed for {scope}: {err}. Inspect with mana(action=\"run_state\") or mana(action=\"logs\", run_id=\"{run_id}\")."
                        )))
                        .await;
                }
            }
            Err(join_err) => {
                let message = format!("mana orchestration: {scope} task failed: {join_err}");
                fail_run_in_store(&run_store, &run_id, message.clone());
                ui.set_status("mana", Some(&message)).await;
                ui.set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
                    .await;
                ui.notify(&message, NotifyLevel::Error).await;
                if !ui.has_ui() {
                    let _ = command_tx
                        .send(crate::agent::AgentCommand::FollowUp(format!(
                            "Native mana orchestration background task failed for {scope}: {join_err}. Inspect with mana(action=\"run_state\") or mana(action=\"logs\", run_id=\"{run_id}\")."
                        )))
                        .await;
                }
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuideTopic {
    Overview,
    Task,
    Epic,
    Decision,
    Notes,
    Verify,
    Orchestrate,
    WorkerContext,
}

impl GuideTopic {
    fn as_str(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Task => "task",
            Self::Epic => "epic",
            Self::Decision => "decision",
            Self::Notes => "notes",
            Self::Verify => "verify",
            Self::Orchestrate => "orchestrate",
            Self::WorkerContext => "worker_context",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "overview" => Some(Self::Overview),
            "task" => Some(Self::Task),
            "epic" => Some(Self::Epic),
            "decision" => Some(Self::Decision),
            "notes" => Some(Self::Notes),
            "verify" => Some(Self::Verify),
            "orchestrate" => Some(Self::Orchestrate),
            "worker_context" => Some(Self::WorkerContext),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateKind {
    Epic,
    Task,
    Fact,
}

impl TemplateKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Epic => "epic",
            Self::Task => "task",
            Self::Fact => "fact",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "epic" => Some(Self::Epic),
            "task" => Some(Self::Task),
            "fact" => Some(Self::Fact),
            _ => None,
        }
    }
}

fn parse_guide_topic(params: &serde_json::Value) -> Result<GuideTopic> {
    let topic = parse_optional_string(&params["topic"]).unwrap_or_else(|| "overview".to_string());
    GuideTopic::parse(&topic).ok_or_else(|| {
        crate::error::Error::Tool(format!(
            "Invalid mana guide topic '{topic}'. Use overview, task, epic, decision, notes, verify, orchestrate, or worker_context."
        ))
    })
}

fn parse_optional_guide_topic(params: &serde_json::Value) -> Result<Option<GuideTopic>> {
    match parse_optional_string(&params["topic"]) {
        Some(topic) => GuideTopic::parse(&topic).map(Some).ok_or_else(|| {
            crate::error::Error::Tool(format!(
                "Invalid mana template topic '{topic}'. Use overview, task, epic, decision, notes, verify, orchestrate, or worker_context."
            ))
        }),
        None => Ok(None),
    }
}

fn parse_template_kind(params: &serde_json::Value) -> Result<TemplateKind> {
    let kind = parse_optional_string(&params["kind"]).unwrap_or_else(|| "task".to_string());
    TemplateKind::parse(&kind).ok_or_else(|| {
        crate::error::Error::Tool(format!(
            "Invalid mana template kind '{kind}'. Use epic, task, or fact."
        ))
    })
}

fn topic_guidance(topic: GuideTopic) -> (&'static str, Vec<&'static str>, Vec<&'static str>) {
    match topic {
        GuideTopic::Overview => (
            "Use mana when work needs durable scope, verification, dependencies, retries, or handoff; use direct edits for small one-pass changes.",
            vec![
                "Create epics for durable goals and tasks for executable units.",
                "Record decisions/notes when context should survive the turn.",
                "Close only after the verify command or equivalent evidence passes.",
            ],
            vec!["template kind=task", "guide topic=orchestrate"],
        ),
        GuideTopic::Task => (
            "A task is a worker-ready executable spec with clear scope, acceptance, files, and a verify gate.",
            vec![
                "Title the outcome, not the activity.",
                "Description should include current state, exact steps, edge cases, and non-goals.",
                "Acceptance and verify define done.",
            ],
            vec![
                "template kind=task",
                "create kind=task title=... verify=...",
            ],
        ),
        GuideTopic::Epic => (
            "An epic is a durable feature/spec container that decomposes into executable child tasks.",
            vec![
                "Capture goal, constraints, architecture direction, and sequencing.",
                "Keep implementation in child tasks with verify commands.",
            ],
            vec![
                "template kind=epic",
                "create kind=epic title=... feature=true",
            ],
        ),
        GuideTopic::Decision => (
            "Use decisions for scope, architecture, sequencing, and tradeoffs future workers should not relitigate.",
            vec![
                "Add decisions when a choice changes implementation direction.",
                "Resolve decisions when the blocker is answered or superseded.",
            ],
            vec![
                "decision_add id=... description=...",
                "decision_resolve id=... resolve_decisions=...",
            ],
        ),
        GuideTopic::Notes => (
            "Use notes for durable progress, diagnosis, blockers, failed attempts, and retry changes.",
            vec![
                "Append concrete evidence, commands, files, and observed errors.",
                "After failures, update notes before retrying with a changed plan.",
            ],
            vec!["notes_append id=... notes=..."],
        ),
        GuideTopic::Verify => (
            "Verification is first-class: acceptance says what must be true; verify is the command/evidence that proves it.",
            vec![
                "Use fail_first for regression tasks where the check should fail before implementation.",
                "Prefer narrow, repeatable commands over broad expensive checks.",
                "If verify is wrong, record why and use equivalent evidence explicitly.",
            ],
            vec![
                "create title=... acceptance=... verify=...",
                "verify id=...",
            ],
        ),
        GuideTopic::Orchestrate => (
            "Orchestration runs ready tasks in dependency waves and returns run_id for state/log inspection.",
            vec![
                "Create dependencies before running parallel waves.",
                "Use run_state/logs/agents to inspect active work.",
                "Update failed units with new context before retrying.",
            ],
            vec![
                "run targets=[...]",
                "run_state run_id=...",
                "logs run_id=...",
            ],
        ),
        GuideTopic::WorkerContext => (
            "Worker context is assembled from unit fields: title, description, acceptance, verify, paths, dependencies, notes, and decisions.",
            vec![
                "Write units so another agent can execute cold without guessing.",
                "Put architecture or scope choices in decisions/notes, not transient chat.",
            ],
            vec!["show id=...", "notes_append id=... notes=..."],
        ),
    }
}

fn mana_guide_output(topic: GuideTopic) -> ToolOutput {
    let (summary, guidance, next_actions) = topic_guidance(topic);
    text_output(
        format!(
            "mana guide: {}\n{}\n- {}\nnext: {}",
            topic.as_str(),
            summary,
            guidance.join("\n- "),
            next_actions.join("; ")
        ),
        json!({
            "action": "guide",
            "topic": topic.as_str(),
            "summary": summary,
            "guidance": guidance,
            "next_actions": next_actions,
        }),
    )
}

fn template_body(kind: TemplateKind, topic: Option<GuideTopic>) -> serde_json::Value {
    match kind {
        TemplateKind::Epic => json!({
            "kind": "epic",
            "title": "Outcome-oriented goal",
            "description": "Goal, users, constraints, architecture direction, decomposition plan, and non-goals.",
            "acceptance": "Child tasks cover implementation, verification, docs, and rollout risks.",
            "feature": true,
            "labels": ["feature"],
        }),
        TemplateKind::Task => json!({
            "kind": "task",
            "title": "Implement/fix concrete outcome",
            "description": "Current state, exact steps, relevant files, edge cases, and scope boundaries. Include enough context for a cold worker.",
            "acceptance": "Observable behavior or artifact that defines done.",
            "verify": "targeted command that proves the task",
            "paths": ["path/to/file"],
            "fail_first": topic == Some(GuideTopic::Verify),
        }),
        TemplateKind::Fact => json!({
            "kind": "fact",
            "title": "Verifiable project claim",
            "verify": "command that exits 0 while the claim remains true",
            "ttl_days": 30,
            "paths": ["path/to/evidence"],
        }),
    }
}

fn mana_template_output(kind: TemplateKind, topic: Option<GuideTopic>) -> ToolOutput {
    let template = template_body(kind, topic);
    let topic_text = topic.map(|topic| topic.as_str()).unwrap_or("general");
    let summary = match kind {
        TemplateKind::Epic => "Epic template for durable feature/spec containers.",
        TemplateKind::Task => "Task template for worker-ready executable specs.",
        TemplateKind::Fact => "Fact template for re-checkable project claims.",
    };
    text_output(
        format!(
            "mana template: {} ({})\n{}\n{}",
            kind.as_str(),
            topic_text,
            summary,
            serde_json::to_string_pretty(&template).unwrap_or_else(|_| template.to_string())
        ),
        json!({
            "action": "template",
            "kind": kind.as_str(),
            "topic": topic_text,
            "summary": summary,
            "template": template,
            "next_actions": ["create", "update", "notes_append"],
        }),
    )
}

fn text_output(text: String, details: serde_json::Value) -> ToolOutput {
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details,
        is_error: false,
    }
}

fn run_state_snapshot(
    run_store: &Arc<std::sync::Mutex<ManaRunStore>>,
    run_id: Option<&str>,
) -> Option<NativeRunState> {
    run_store
        .lock()
        .ok()
        .and_then(|store| store.snapshot(run_id))
}

fn retry_guardrail_for_targets(
    mana_dir: &Path,
    target_ids: &[String],
) -> Result<Option<serde_json::Value>> {
    let mut blocked_units = Vec::new();
    for id in target_ids {
        let Ok(result) = mana_core::ops::show::get(mana_dir, id) else {
            continue;
        };
        let unit = result.unit;
        let Some(attempt) = unit.attempt_log.last() else {
            continue;
        };
        if !matches!(
            attempt.outcome,
            mana_core::unit::types::AttemptOutcome::Failed
        ) {
            continue;
        }
        let Some(finished_at) = attempt.finished_at else {
            continue;
        };
        if unit.updated_at <= finished_at {
            blocked_units.push(json!({
                "id": unit.id,
                "title": unit.title,
                "failed_at": finished_at,
                "updated_at": unit.updated_at,
                "last_failure": attempt.notes,
            }));
        }
    }

    if blocked_units.is_empty() {
        return Ok(None);
    }

    Ok(Some(json!({
        "retry_requires_unit_update": true,
        "blocked_units": blocked_units,
        "next_actions": [
            "Inspect failed unit with mana action=show id=<unit>",
            "Append notes with failure evidence and a changed retry plan",
            "Retry only after updating the failed unit"
        ],
    })))
}

fn run_recovery_details(state: &NativeRunState) -> serde_json::Value {
    let failed_units: Vec<_> = state
        .units
        .iter()
        .filter(|unit| unit.status == "failed")
        .map(|unit| {
            json!({
                "id": unit.id,
                "title": unit.title,
                "failure_summary": unit.failure_summary,
                "error": unit.error,
            })
        })
        .collect();
    let running_units: Vec<_> = state
        .units
        .iter()
        .filter(|unit| unit.status == "running")
        .map(|unit| json!({ "id": unit.id, "title": unit.title, "agent": unit.agent }))
        .collect();
    let awaiting_verify_units: Vec<_> = state
        .units
        .iter()
        .filter(|unit| unit.status == "awaiting_verify")
        .map(|unit| json!({ "id": unit.id, "title": unit.title }))
        .collect();
    let interrupted = state.status == "interrupted";
    let last_event_at_ms = if state.last_event_at_ms > 0 {
        state.last_event_at_ms
    } else {
        state.finished_at_ms.unwrap_or(state.started_at_ms)
    };
    let mut next_actions = Vec::new();
    let mut retry_requires_unit_update = false;

    if interrupted {
        next_actions.push(
            "Run was marked interrupted/stale after reload; inspect logs before rerun".to_string(),
        );
        next_actions.push("Do not assume in-flight workers or tools are still running".to_string());
    }
    if !failed_units.is_empty() || state.status == "failed" || state.summary.total_failed > 0 {
        retry_requires_unit_update = true;
        next_actions.push("Inspect failed units with mana action=show id=<unit>".to_string());
        next_actions.push(
            "Append notes with the failure evidence and changed retry plan before rerun"
                .to_string(),
        );
        next_actions
            .push("Retry only after updating failed units; do not rerun unchanged".to_string());
    }
    if !awaiting_verify_units.is_empty() || state.summary.total_awaiting_verify > 0 {
        next_actions.push("Verify candidate-complete units or close with equivalent evidence if stored verify is stale".to_string());
    }
    if state.status == "running" || state.status == "starting" || !running_units.is_empty() {
        next_actions.push("Inspect logs/agents before assuming the run is stale".to_string());
    }
    if next_actions.is_empty() {
        next_actions.push("No recovery action required".to_string());
    }

    json!({
        "status": state.status,
        "failed_units": failed_units,
        "running_units": running_units,
        "awaiting_verify_units": awaiting_verify_units,
        "stale_workers": if interrupted { json!([{"run_id": state.run_id, "status": state.status}]) } else { json!([]) },
        "last_event_at_ms": last_event_at_ms,
        "next_actions": next_actions,
        "retry_requires_unit_update": retry_requires_unit_update,
    })
}

fn run_state_details(state: &NativeRunState) -> serde_json::Value {
    let mut details = serde_json::to_value(state).unwrap_or(serde_json::Value::Null);
    if let Some(object) = details.as_object_mut() {
        object.insert("recovery".to_string(), run_recovery_details(state));
    }
    details
}

fn run_state_output(state: &NativeRunState) -> ToolOutput {
    let mut lines = vec![format!(
        "Native mana orchestration {}: {} · {}",
        state.run_id, state.scope, state.status
    )];
    if let Some(runtime) = &state.runtime {
        let agent = runtime["direct_agent"].as_str().unwrap_or("imp-worker");
        let model = runtime["model"].as_str().unwrap_or("default-model");
        lines.push(format!("Worker runtime: {agent} · {model}"));
    }
    lines.push(format!(
        "{} total · {} done · {} failed · {} candidate complete / awaiting verify · {} skipped",
        state.summary.total_units,
        state.summary.total_closed,
        state.summary.total_failed,
        state.summary.total_awaiting_verify,
        state.summary.total_skipped
    ));

    if state.status == "interrupted" {
        lines.push(
            "Interrupted: persisted running state is stale; inspect logs before rerun".to_string(),
        );
    }

    if !state.units.is_empty() {
        let preview = state
            .units
            .iter()
            .take(3)
            .map(|unit| format!("{}:{}", unit.id, unit.status))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Units: {preview}"));
    }

    if let Some(last) = state.log_lines.last() {
        lines.push(format!("Latest: {last}"));
    }
    let recovery = run_recovery_details(state);
    if let Some(actions) = recovery["next_actions"].as_array() {
        if let Some(first) = actions.first().and_then(|value| value.as_str()) {
            lines.push(format!("Next: {first}"));
        }
    }
    text_output(lines.join("\n"), run_state_details(state))
}

fn evaluate_run_output(state: &NativeRunState) -> ToolOutput {
    let headline = match state.status.as_str() {
        "starting" | "running" => {
            format!(
                "Native mana orchestration run {} is still running for {}.",
                state.run_id, state.scope
            )
        }
        "failed" => format!(
            "Native mana orchestration run {} failed for {}.",
            state.run_id, state.scope
        ),
        _ if state.summary.total_failed > 0 => format!(
            "Native mana orchestration run {} finished with {} failed unit(s).",
            state.run_id, state.summary.total_failed
        ),
        _ if state.summary.total_awaiting_verify > 0 => format!(
            "Native mana orchestration run {} finished with {} unit(s) candidate complete / awaiting verify.",
            state.run_id, state.summary.total_awaiting_verify
        ),
        _ => format!(
            "Native mana orchestration run {} finished successfully: {} unit(s) done.",
            state.run_id, state.summary.total_closed
        ),
    };

    let runtime = state
        .runtime
        .as_ref()
        .map(|runtime| {
            format!(
                "Worker runtime: {} · {}",
                runtime["direct_agent"].as_str().unwrap_or("imp-worker"),
                runtime["model"].as_str().unwrap_or("default-model")
            )
        })
        .unwrap_or_else(|| "Runtime: unknown".to_string());

    let latest = state
        .log_lines
        .last()
        .map(|line| format!("Latest: {line}"))
        .unwrap_or_else(|| "Latest: (no stream events captured yet)".to_string());

    let recovery = run_recovery_details(state);
    let next = recovery["next_actions"]
        .as_array()
        .and_then(|actions| actions.first())
        .and_then(|value| value.as_str())
        .map(|action| format!("Next: {action}"))
        .unwrap_or_else(|| "Next: No recovery action required".to_string());

    text_output(
        format!("{headline}\n{runtime}\n{latest}\n{next}"),
        run_state_details(state),
    )
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

pub struct ManaTool {
    run_store: Arc<std::sync::Mutex<ManaRunStore>>,
}

impl Default for ManaTool {
    fn default() -> Self {
        Self {
            run_store: Arc::new(std::sync::Mutex::new(ManaRunStore::load_persisted())),
        }
    }
}

#[async_trait]
impl Tool for ManaTool {
    fn name(&self) -> &str {
        "mana"
    }
    fn label(&self) -> &str {
        "Mana"
    }
    fn description(&self) -> &str {
        "Native mana work coordination: inspect, update, create, and run units or orchestration state. Prefer it over bash for equivalent mana actions."
    }
    fn parameters(&self) -> serde_json::Value {
        let string_or_array = || {
            json!({
                "oneOf": [
                    { "type": "string" },
                    { "type": "array", "items": { "type": "string" } }
                ]
            })
        };

        let mut properties = serde_json::Map::new();
        properties.insert(
            "action".into(),
            json!({ "type": "string", "enum": ["status", "list", "show", "create", "close", "update", "run", "run_state", "evaluate", "claim", "release", "logs", "agents", "next", "tree", "reopen", "verify", "fail", "delete", "dep_add", "dep_remove", "fact_create", "fact_verify", "notes_append", "decision_add", "decision_resolve", "reparent", "guide", "template"] }),
        );
        properties.insert("id".into(), json!({ "type": "string" }));
        properties.insert(
            "scope".into(),
            json!({ "type": "string", "enum": ["auto", "project", "root"], "description": "Mana scope" }),
        );
        properties.insert(
            "path".into(),
            json!({ "type": "string", "description": "Project or .mana directory" }),
        );
        properties.insert(
            "from_id".into(),
            json!({ "type": "string", "description": "Source unit ID for dependency updates" }),
        );
        properties.insert(
            "dep_id".into(),
            json!({ "type": "string", "description": "Dependency unit ID to add or remove" }),
        );
        properties.insert(
            "run_id".into(),
            json!({ "type": "string", "description": "Native in-session mana run ID, returned by action=run" }),
        );
        properties.insert("title".into(), json!({ "type": "string" }));
        properties.insert(
            "topic".into(),
            json!({ "type": "string", "enum": ["overview", "task", "epic", "decision", "notes", "verify", "orchestrate", "worker_context"], "description": "Guide/template topic" }),
        );
        properties.insert(
            "verify".into(),
            json!({ "type": "string", "description": "Shell command, must exit 0" }),
        );
        properties.insert("description".into(), json!({ "type": "string" }));
        properties.insert(
            "acceptance".into(),
            json!({ "type": "string", "description": "Concrete acceptance criteria for the unit" }),
        );
        properties.insert(
            "notes".into(),
            json!({ "type": "string", "description": "Progress log or authoring notes" }),
        );
        properties.insert(
            "design".into(),
            json!({ "type": "string", "description": "Supplemental design context for the unit" }),
        );
        properties.insert(
            "assignee".into(),
            json!({ "type": "string", "description": "Assignee or owner for the unit" }),
        );
        properties.insert("parent".into(), json!({ "type": "string" }));
        properties.insert(
            "parent_reason".into(),
            json!({ "type": "string", "description": "For create: why this unit belongs under the chosen parent; helps prevent mis-scoped work graphs" }),
        );
        let mut deps = string_or_array();
        deps["description"] = json!("Dependency unit IDs as a comma-separated string or array");
        properties.insert("deps".into(), deps);
        let mut produces = string_or_array();
        produces["description"] = json!("Artifacts this unit produces");
        properties.insert("produces".into(), produces);
        let mut requires = string_or_array();
        requires["description"] = json!("Artifacts this unit requires");
        properties.insert("requires".into(), requires);
        let mut paths = string_or_array();
        paths["description"] = json!("Relevant file paths");
        properties.insert("paths".into(), paths);
        let mut decisions = string_or_array();
        decisions["description"] = json!("Blocking decisions to record on the unit");
        properties.insert("decisions".into(), decisions);
        let mut resolve_decisions = string_or_array();
        resolve_decisions["description"] =
            json!("Decision entries or indexes to resolve during update");
        properties.insert("resolve_decisions".into(), resolve_decisions);
        properties.insert("status".into(), json!({ "type": "string" }));
        properties.insert("priority".into(), json!({ "type": "integer" }));
        let mut labels = string_or_array();
        labels["description"] = json!("Labels as a comma-separated string or array");
        properties.insert("labels".into(), labels);
        properties.insert(
            "kind".into(),
            json!({ "type": "string", "enum": ["epic", "task", "fact"], "description": "Mana unit type" }),
        );
        properties.insert(
            "feature".into(),
            json!({ "type": "boolean", "description": "Whether the unit is a feature-level goal" }),
        );
        properties.insert(
            "fail_first".into(),
            json!({ "type": "boolean", "description": "Require verify to fail first at creation time" }),
        );
        properties.insert(
            "verify_timeout".into(),
            json!({ "type": "integer", "description": "Timeout in seconds for verify" }),
        );
        properties.insert(
            "on_fail".into(),
            json!({ "description": "On-fail policy like retry:3 or escalate:P1" }),
        );
        properties.insert(
            "ttl_days".into(),
            json!({ "type": "integer", "description": "TTL in days for fact_create" }),
        );
        properties.insert(
            "pass_ok".into(),
            json!({ "type": "boolean", "description": "Permit fact creation even if verify currently passes" }),
        );
        properties.insert(
            "force".into(),
            json!({ "type": "boolean", "description": "For close: bypass stored verify only with a reason containing equivalent verification evidence" }),
        );
        properties.insert("reason".into(), json!({ "type": "string" }));
        properties.insert("all".into(), json!({ "type": "boolean" }));
        properties.insert(
            "by".into(),
            json!({ "type": "string", "description": "Who is claiming the unit" }),
        );
        properties.insert(
            "count".into(),
            json!({ "type": "integer", "description": "Number of next recommendations to return" }),
        );
        properties.insert(
            "background".into(),
            json!({ "type": "boolean", "description": "Run mana orchestration in the background and return immediately (default true unless dry_run=true)" }),
        );
        properties.insert(
            "targets".into(),
            json!({ "type": "array", "items": { "type": "string" }, "description": "Explicit target unit IDs to run as a canonical target set" }),
        );
        properties.insert("jobs".into(), json!({ "type": "integer" }));
        properties.insert("dry_run".into(), json!({ "type": "boolean" }));
        properties.insert("loop".into(), json!({ "type": "boolean" }));
        properties.insert("keep_going".into(), json!({ "type": "boolean" }));
        properties.insert("timeout".into(), json!({ "type": "integer" }));
        properties.insert("idle_timeout".into(), json!({ "type": "integer" }));
        properties.insert("review".into(), json!({ "type": "boolean" }));

        serde_json::Value::Object(serde_json::Map::from_iter([
            ("type".into(), json!("object")),
            ("properties".into(), serde_json::Value::Object(properties)),
            ("required".into(), json!(["action"])),
        ]))
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

        match action {
            "guide" => return Ok(mana_guide_output(parse_guide_topic(&params)?)),
            "template" => {
                return Ok(mana_template_output(
                    parse_template_kind(&params)?,
                    parse_optional_guide_topic(&params)?,
                ));
            }
            _ => {}
        }

        if !mode.allows_mana_action(action) {
            let mode_name = format!("{mode:?}").to_lowercase();
            return Ok(ToolOutput::error(format!(
                "Mana action '{action}' is not available in {mode_name} mode"
            )));
        }

        if let Some(validation_error) = validate_mana_action(action, &params) {
            return Ok(validation_error);
        }

        let mana_dir = resolve_mana_dir(&ctx.cwd, &params).map_err(crate::error::Error::Tool)?;

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
                    }
                }
            }
            "show" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("show requires 'id'".into()))?;
                match mana_core::ops::show::get(&mana_dir, id) {
                    Ok(result) => Ok(json_output(&result.unit)),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "create" => {
                let title = params["title"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("create requires 'title'".into()))?;
                let dependencies = parse_csv_strings(&params["deps"], "deps")?;
                let labels = parse_csv_strings(&params["labels"], "labels")?;
                let produces = parse_csv_strings(&params["produces"], "produces")?;
                let requires = parse_csv_strings(&params["requires"], "requires")?;
                let paths = parse_csv_strings(&params["paths"], "paths")?;
                let decisions = parse_csv_strings(&params["decisions"], "decisions")?;
                let on_fail = parse_on_fail(&params["on_fail"])?;
                let kind = parse_unit_kind(&params["kind"])?;

                let create_params = mana_core::ops::create::CreateParams {
                    title: title.to_string(),
                    description: parse_optional_string(&params["description"]),
                    acceptance: parse_optional_string(&params["acceptance"]),
                    notes: parse_optional_string(&params["notes"]),
                    design: parse_optional_string(&params["design"]),
                    verify: parse_optional_string(&params["verify"]),
                    priority: params["priority"].as_u64().map(|p| p as u8),
                    labels,
                    assignee: parse_optional_string(&params["assignee"]),
                    dependencies,
                    parent: parse_optional_string(&params["parent"]),
                    produces,
                    requires,
                    paths,
                    on_fail,
                    fail_first: params["fail_first"].as_bool().unwrap_or(false),
                    feature: params["feature"].as_bool().unwrap_or(false),
                    kind,
                    verify_timeout: params["verify_timeout"].as_u64(),
                    decisions,
                    force: params["force"].as_bool().unwrap_or(false),
                };
                match mana_core::api::create_unit(&mana_dir, create_params) {
                    Ok(result) => {
                        let unit_value =
                            serde_json::to_value(&result.unit).unwrap_or(serde_json::Value::Null);
                        let summary = unit_delta_label(&unit_value)
                            .map(|label| format!("mana delta: created {label}"))
                            .unwrap_or_else(|| "mana delta: created unit".to_string());
                        let parent = parse_optional_string(&params["parent"]);
                        let parent_reason = parse_optional_string(&params["parent_reason"]);
                        let detail = parent
                            .as_ref()
                            .map(|parent| match parent_reason.as_deref() {
                                Some(reason) => format!("parent {parent}: {reason}"),
                                None => format!("parent {parent}; parent_reason missing"),
                            });
                        set_mana_delta_widget(&ctx, summary.clone(), detail).await;
                        Ok(text_output(
                            summary,
                            json!({
                                "action": "create",
                                "title": title,
                                "description": params["description"],
                                "verify": params["verify"],
                                "priority": params["priority"],
                                "parent": params["parent"],
                                "parent_reason": params["parent_reason"],
                                "placement": parent_placement_details(parent.as_deref(), parent_reason.as_deref()),
                                "deps": params["deps"],
                                "labels": params["labels"],
                                "unit": unit_value,
                                "path": result.path,
                            }),
                        ))
                    }
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
                let force = params["force"].as_bool().unwrap_or(false);
                let reason = params["reason"].as_str().map(|s| s.to_string());
                if force
                    && reason
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or_default()
                        .is_empty()
                {
                    return Ok(mana_close_force_reason_error(id));
                }
                let opts = mana_core::ops::close::CloseOpts {
                    reason: reason.clone(),
                    force,
                    defer_verify: false,
                };
                match mana_core::api::close_unit(&mana_dir, id, opts) {
                    Ok(outcome) => {
                        let details =
                            serde_json::to_value(&outcome).unwrap_or(serde_json::Value::Null);
                        if let Some(unit) = details.get("unit") {
                            let summary = unit_delta_label(unit)
                                .map(|label| format!("mana delta: closed {label}"))
                                .unwrap_or_else(|| format!("mana delta: closed {id}"));
                            set_mana_delta_widget(&ctx, summary, reason.clone()).await;
                        }
                        let mut details_obj = details.as_object().cloned().unwrap_or_default();
                        details_obj.insert("action".into(), json!("close"));
                        details_obj.insert("force".into(), json!(force));
                        if let Some(reason) = reason.as_deref() {
                            details_obj.insert("reason".into(), json!(reason));
                            if force {
                                details_obj
                                    .insert("equivalent_verify_evidence".into(), json!(reason));
                            }
                        }
                        Ok(text_output(
                            format!("Closed unit {id}"),
                            serde_json::Value::Object(details_obj),
                        ))
                    }
                    Err(e) => Ok(mana_close_error_output(id, e.to_string())),
                }
            }
            "update" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("update requires 'id'".into()))?;
                let decisions = parse_csv_strings(&params["decisions"], "decisions")?;
                let resolve_decisions =
                    parse_csv_strings(&params["resolve_decisions"], "resolve_decisions")?;
                let update_params = mana_core::ops::update::UpdateParams {
                    title: parse_optional_string(&params["title"]),
                    description: parse_optional_string(&params["description"]),
                    acceptance: parse_optional_string(&params["acceptance"]),
                    notes: parse_optional_string(&params["notes"]),
                    design: parse_optional_string(&params["design"]),
                    status: parse_optional_string(&params["status"]),
                    priority: params["priority"].as_u64().map(|p| p as u8),
                    assignee: parse_optional_string(&params["assignee"]),
                    add_label: parse_optional_string(&params["add_label"]),
                    remove_label: parse_optional_string(&params["remove_label"]),
                    decisions,
                    resolve_decisions,
                };
                match mana_core::api::update_unit(&mana_dir, id, update_params) {
                    Ok(result) => {
                        let unit_value =
                            serde_json::to_value(&result.unit).unwrap_or(serde_json::Value::Null);
                        let summary = unit_delta_label(&unit_value)
                            .map(|label| format!("mana delta: updated {label}"))
                            .unwrap_or_else(|| format!("mana delta: updated {id}"));
                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
                        Ok(text_output(
                            summary,
                            json!({
                                "action": "update",
                                "id": id,
                                "status": params["status"],
                                "title": params["title"],
                                "description": params["description"],
                                "priority": params["priority"],
                                "notes": params["notes"],
                                "acceptance": params["acceptance"],
                                "add_label": params["add_label"],
                                "remove_label": params["remove_label"],
                                "decisions": params["decisions"],
                                "resolve_decisions": params["resolve_decisions"],
                                "unit": unit_value,
                                "path": result.path,
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "notes_append" => {
                let id = params["id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("notes_append requires 'id'".into())
                })?;
                let note = parse_optional_string(&params["notes"]).ok_or_else(|| {
                    crate::error::Error::Tool("notes_append requires 'notes'".into())
                })?;
                let update_params = mana_core::ops::update::UpdateParams {
                    title: None,
                    description: None,
                    acceptance: None,
                    notes: Some(note),
                    design: None,
                    status: None,
                    priority: None,
                    assignee: None,
                    add_label: None,
                    remove_label: None,
                    decisions: Vec::new(),
                    resolve_decisions: Vec::new(),
                };
                match mana_core::api::update_unit(&mana_dir, id, update_params) {
                    Ok(result) => {
                        let unit_value =
                            serde_json::to_value(&result.unit).unwrap_or(serde_json::Value::Null);
                        let summary = unit_delta_label(&unit_value)
                            .map(|label| format!("mana delta: notes appended on {label}"))
                            .unwrap_or_else(|| format!("mana delta: notes appended on {id}"));
                        set_mana_delta_widget(&ctx, summary.clone(), Some("notes appended".into()))
                            .await;
                        Ok(text_output(
                            summary,
                            json!({
                                "action": "notes_append",
                                "id": id,
                                "notes": params["notes"],
                                "unit": unit_value,
                                "path": result.path,
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "decision_add" => {
                let id = params["id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("decision_add requires 'id'".into())
                })?;
                let decision = parse_optional_string(&params["description"])
                    .or_else(|| {
                        parse_csv_strings(&params["decisions"], "decisions")
                            .ok()
                            .and_then(|mut decisions| decisions.drain(..).next())
                    })
                    .or_else(|| parse_optional_string(&params["notes"]))
                    .ok_or_else(|| {
                        crate::error::Error::Tool(
                            "decision_add requires 'description' or 'decisions'".into(),
                        )
                    })?;
                let update_params = mana_core::ops::update::UpdateParams {
                    title: None,
                    description: None,
                    acceptance: None,
                    notes: None,
                    design: None,
                    status: None,
                    priority: None,
                    assignee: None,
                    add_label: None,
                    remove_label: None,
                    decisions: vec![decision],
                    resolve_decisions: Vec::new(),
                };
                match mana_core::api::update_unit(&mana_dir, id, update_params) {
                    Ok(result) => {
                        let unit_value =
                            serde_json::to_value(&result.unit).unwrap_or(serde_json::Value::Null);
                        let summary = unit_delta_label(&unit_value)
                            .map(|label| format!("mana delta: decision added on {label}"))
                            .unwrap_or_else(|| format!("mana delta: decision added on {id}"));
                        set_mana_delta_widget(&ctx, summary.clone(), Some("decision added".into()))
                            .await;
                        Ok(text_output(
                            summary,
                            json!({
                                "action": "decision_add",
                                "id": id,
                                "description": params["description"],
                                "unit": unit_value,
                                "path": result.path,
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "decision_resolve" => {
                let id = params["id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("decision_resolve requires 'id'".into())
                })?;
                let resolve_decisions =
                    parse_csv_strings(&params["resolve_decisions"], "resolve_decisions")?;
                if resolve_decisions.is_empty() {
                    return Ok(ToolOutput::error(
                        "decision_resolve requires 'resolve_decisions'",
                    ));
                }
                let update_params = mana_core::ops::update::UpdateParams {
                    title: None,
                    description: None,
                    acceptance: None,
                    notes: None,
                    design: None,
                    status: None,
                    priority: None,
                    assignee: None,
                    add_label: None,
                    remove_label: None,
                    decisions: Vec::new(),
                    resolve_decisions,
                };
                match mana_core::api::update_unit(&mana_dir, id, update_params) {
                    Ok(result) => {
                        let unit_value =
                            serde_json::to_value(&result.unit).unwrap_or(serde_json::Value::Null);
                        let summary = unit_delta_label(&unit_value)
                            .map(|label| format!("mana delta: decision resolved on {label}"))
                            .unwrap_or_else(|| format!("mana delta: decision resolved on {id}"));
                        set_mana_delta_widget(
                            &ctx,
                            summary.clone(),
                            Some("decision resolved".into()),
                        )
                        .await;
                        Ok(text_output(
                            summary,
                            json!({
                                "action": "decision_resolve",
                                "id": id,
                                "resolve_decisions": params["resolve_decisions"],
                                "unit": unit_value,
                                "path": result.path,
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "reopen" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("reopen requires 'id'".into()))?;
                match mana_core::api::reopen_unit(&mana_dir, id) {
                    Ok(result) => {
                        let summary = format!(
                            "mana delta: reopened {} ({})",
                            result.unit.id, result.unit.title
                        );
                        set_mana_delta_widget(&ctx, summary, Some("status=open".into())).await;
                        Ok(text_output(
                            format!("Reopened unit {} ({})", result.unit.id, result.unit.title),
                            json!({
                                "action": "reopen",
                                "unit": {
                                    "id": result.unit.id,
                                    "title": result.unit.title,
                                    "status": result.unit.status,
                                },
                                "path": result.path,
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "verify" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("verify requires 'id'".into()))?;
                match mana_core::api::run_verify(&mana_dir, id) {
                    Ok(Some(result)) => Ok(text_output(
                        format!(
                            "Verify {} for unit {id}{}",
                            if result.passed { "passed" } else { "failed" },
                            result
                                .exit_code
                                .map(|code| format!(" (exit {code})"))
                                .unwrap_or_default()
                        ),
                        json!({
                            "passed": result.passed,
                            "exit_code": result.exit_code,
                            "stdout": result.stdout,
                            "stderr": result.stderr,
                            "timed_out": result.timed_out,
                            "command": result.command,
                            "timeout_secs": result.timeout_secs,
                            "unit_id": id,
                        }),
                    )),
                    Ok(None) => Ok(ToolOutput::text(format!(
                        "Unit {id} has no verify command."
                    ))),
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "fail" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("fail requires 'id'".into()))?;
                match mana_core::api::fail_unit(
                    &mana_dir,
                    id,
                    parse_optional_string(&params["reason"]),
                ) {
                    Ok(unit) => {
                        let unit_value =
                            serde_json::to_value(&unit).unwrap_or(serde_json::Value::Null);
                        let summary = unit_delta_label(&unit_value)
                            .map(|label| format!("mana delta: marked failed {label}"))
                            .unwrap_or_else(|| format!("mana delta: marked failed {id}"));
                        set_mana_delta_widget(
                            &ctx,
                            summary,
                            params["reason"].as_str().map(|s| s.to_string()),
                        )
                        .await;
                        Ok(text_output(
                            format!("Marked unit {id} as failed"),
                            json!({
                                "action": "fail",
                                "id": id,
                                "reason": params["reason"],
                                "unit": unit_value,
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "delete" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("delete requires 'id'".into()))?;
                match mana_core::api::delete_unit(&mana_dir, id) {
                    Ok(result) => {
                        let summary =
                            format!("mana delta: deleted {} ({})", result.id, result.title);
                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
                        Ok(text_output(
                            format!("Deleted unit {} ({})", result.id, result.title),
                            json!({ "action": "delete", "id": result.id, "title": result.title }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "dep_add" => {
                let from_id = params["from_id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("dep_add requires 'from_id'".into())
                })?;
                let dep_id = params["dep_id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("dep_add requires 'dep_id'".into()))?;
                match mana_core::api::add_dep(&mana_dir, from_id, dep_id) {
                    Ok(result) => {
                        let summary = format!(
                            "mana delta: dependency added {} -> {}",
                            result.from_id, result.to_id
                        );
                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
                        Ok(text_output(
                            format!(
                                "Added dependency: {} depends on {}",
                                result.from_id, result.to_id
                            ),
                            json!({ "action": "dep_add", "from_id": result.from_id, "dep_id": result.to_id }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "dep_remove" => {
                let from_id = params["from_id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("dep_remove requires 'from_id'".into())
                })?;
                let dep_id = params["dep_id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("dep_remove requires 'dep_id'".into())
                })?;
                match mana_core::api::remove_dep(&mana_dir, from_id, dep_id) {
                    Ok(result) => {
                        let summary = format!(
                            "mana delta: dependency removed {} -> {}",
                            result.from_id, result.to_id
                        );
                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
                        Ok(text_output(
                            format!(
                                "Removed dependency: {} no longer depends on {}",
                                result.from_id, result.to_id
                            ),
                            json!({ "action": "dep_remove", "from_id": result.from_id, "dep_id": result.to_id }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "fact_create" => {
                let title = parse_optional_string(&params["title"])
                    .or_else(|| parse_optional_string(&params["fact_title"]))
                    .ok_or_else(|| {
                        crate::error::Error::Tool("fact_create requires 'title'".into())
                    })?;
                let verify = parse_optional_string(&params["verify"]).ok_or_else(|| {
                    crate::error::Error::Tool("fact_create requires 'verify'".into())
                })?;
                // Transitional compatibility: runtime still accepts legacy `paths_csv`, but
                // the model-facing schema advertises only canonical `paths`.
                let fact_paths = parse_optional_string(&params["paths_csv"]).or_else(|| {
                    let paths = parse_csv_strings(&params["paths"], "paths").ok()?;
                    if paths.is_empty() {
                        None
                    } else {
                        Some(paths.join(","))
                    }
                });
                let fact_params = mana_core::ops::fact::FactParams {
                    title,
                    verify,
                    description: parse_optional_string(&params["description"]),
                    paths: fact_paths,
                    ttl_days: params["ttl_days"].as_i64(),
                    pass_ok: params["pass_ok"].as_bool().unwrap_or(true),
                };
                match mana_core::api::create_fact(&mana_dir, fact_params) {
                    Ok(result) => {
                        let summary = format!(
                            "mana delta: created fact {} ({})",
                            result.unit_id, result.unit.title
                        );
                        set_mana_delta_widget(&ctx, summary.clone(), Some("fact".into())).await;
                        Ok(text_output(
                            format!("Created fact {} ({})", result.unit_id, result.unit.title),
                            json!({
                                "action": "fact_create",
                                "unit_id": result.unit_id,
                                "unit": {
                                    "id": result.unit.id,
                                    "title": result.unit.title,
                                    "unit_type": result.unit.unit_type,
                                    "verify": result.unit.verify,
                                    "paths": result.unit.paths,
                                    "stale_after": result.unit.stale_after,
                                }
                            }),
                        ))
                    }
                    Err(e) => Ok(ToolOutput::error(e.to_string())),
                }
            }
            "fact_verify" => match mana_core::api::verify_facts(&mana_dir) {
                Ok(result) => Ok(text_output(
                    format!(
                        "Verified {}/{} facts · {} stale · {} failing · {} suspect",
                        result.verified_count,
                        result.total_facts,
                        result.stale_count,
                        result.failing_count,
                        result.suspect_count
                    ),
                    json!({
                        "total_facts": result.total_facts,
                        "verified_count": result.verified_count,
                        "stale_count": result.stale_count,
                        "failing_count": result.failing_count,
                        "suspect_count": result.suspect_count,
                    }),
                )),
                Err(e) => Ok(ToolOutput::error(e.to_string())),
            },
            "logs" => {
                if let Some(run_id) = params["run_id"].as_str() {
                    if let Some(state) = run_state_snapshot(&self.run_store, Some(run_id)) {
                        let text = if state.log_lines.is_empty() {
                            format!(
                                "No native stream events captured yet for run {}.",
                                state.run_id
                            )
                        } else {
                            truncate_with_note(&state.log_lines.join("\n"))
                        };
                        return Ok(text_output(
                            text,
                            serde_json::to_value(&state).unwrap_or(serde_json::Value::Null),
                        ));
                    }
                    return Ok(ToolOutput::error(format!(
                        "Unknown native mana run_id: {run_id}"
                    )));
                }

                let id = params["id"].as_str().ok_or_else(|| {
                    crate::error::Error::Tool("logs requires 'id' or 'run_id'".into())
                })?;
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
                            let body = std::fs::read_to_string(path).unwrap_or_else(|e| {
                                format!("(error reading {}: {e})", path.display())
                            });
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
            "reparent" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("reparent requires 'id'".into()))?;
                let parent = parse_optional_string(&params["parent"]);
                let reason = parse_optional_string(&params["reason"])
                    .or_else(|| parse_optional_string(&params["parent_reason"]));
                let result = mana_core::api::reparent_unit(
                    &mana_dir,
                    id,
                    mana_core::ops::reparent::ReparentParams {
                        parent: parent.clone(),
                        reason: reason.clone(),
                    },
                )
                .map_err(|error| crate::error::Error::Tool(error.to_string()))?;
                let details = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
                let summary = format!(
                    "mana delta: reparented {} from {} to {}",
                    id,
                    result.old_parent.as_deref().unwrap_or("<root>"),
                    result.new_parent.as_deref().unwrap_or("<root>")
                );
                set_mana_delta_widget(&ctx, summary.clone(), reason.clone()).await;
                let mut details_obj = details.as_object().cloned().unwrap_or_default();
                details_obj.insert("action".into(), json!("reparent"));
                details_obj.insert("reason".into(), json!(reason));
                Ok(text_output(summary, serde_json::Value::Object(details_obj)))
            }
            "run_state" | "evaluate" => {
                let run_id = params["run_id"].as_str();
                match run_state_snapshot(&self.run_store, run_id) {
                    Some(state) => {
                        if action == "evaluate" {
                            Ok(evaluate_run_output(&state))
                        } else {
                            Ok(run_state_output(&state))
                        }
                    }
                    None => {
                        let which = run_id.unwrap_or("latest");
                        Ok(ToolOutput::error(format!(
                            "No native mana run state available for {which}. Start one with mana(action=\"run\")."
                        )))
                    }
                }
            }
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
                                    && !index.units.iter().any(|child| {
                                        child.parent.as_deref() == Some(e.id.as_str())
                                            && child.status != mana_core::unit::Status::Closed
                                    })
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

                        fn score_unit(
                            entry: &mana_core::index::IndexEntry,
                            unblock_count: usize,
                        ) -> f64 {
                            let priority_score = (5u8.saturating_sub(entry.priority)) as f64 * 10.0;
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
                                let unblocks =
                                    reverse_deps.get(&entry.id).cloned().unwrap_or_default();
                                let score = score_unit(entry, transitive_count);
                                let now_days = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs()
                                    / 86_400;
                                let created_days =
                                    entry.created_at.timestamp().max(0) as u64 / 86_400;
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
                            serde_json::to_value(&scored).unwrap_or(serde_json::Value::Null),
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
                        Err(tree_err) => match mana_core::ops::show::get(&mana_dir, root_id) {
                            Ok(result) if result.unit.is_archived => {
                                return Ok(ToolOutput::error(format!(
                                    "Archived unit {root_id} can be shown but not rendered in tree view. Tree only includes active units."
                                )));
                            }
                            Ok(_) | Err(_) => return Ok(ToolOutput::error(tree_err.to_string())),
                        },
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
                let run_params = NativeRunParams {
                    target: target_from_params(&params)?,
                    jobs: params["jobs"].as_u64().unwrap_or(4) as u32,
                    dry_run: params["dry_run"].as_bool().unwrap_or(false),
                    loop_mode: params["loop"].as_bool().unwrap_or(false),
                    keep_going: params["keep_going"].as_bool().unwrap_or(false),
                    timeout: params["timeout"].as_u64().unwrap_or(30) as u32,
                    idle_timeout: params["idle_timeout"].as_u64().unwrap_or(5) as u32,
                    json_stream: true,
                    review: params["review"].as_bool().unwrap_or(false),
                };
                let target_ids = target_ids_from_run_target(&run_params.target);
                if !target_ids.is_empty() {
                    if let Some(guardrail) = retry_guardrail_for_targets(&mana_dir, &target_ids)? {
                        return Ok(ToolOutput {
                            content: vec![imp_llm::ContentBlock::Text {
                                text: "mana run blocked: failed units must be updated before retry"
                                    .to_string(),
                            }],
                            details: guardrail,
                            is_error: true,
                        });
                    }
                }
                let background = params["background"]
                    .as_bool()
                    .unwrap_or(!run_params.dry_run);
                let scope = scope_from_target(&run_params.target);
                let run_id = {
                    let mut store = self.run_store.lock().map_err(|_| {
                        crate::error::Error::Tool("mana run state lock poisoned".into())
                    })?;
                    let run_id = store.start_run(scope.clone(), background, &run_params);
                    store.persist();
                    run_id
                };

                if background {
                    let started = background_run_started_output(&scope, &run_id, &run_params);
                    spawn_background_run(
                        mana_dir.clone(),
                        run_params,
                        ctx,
                        self.run_store.clone(),
                        run_id,
                    );
                    return Ok(started);
                }

                send_update(
                    &ctx,
                    format!("Starting mana run {run_id}..."),
                    json!({"kind": "mana_run_status", "status": "starting", "run_id": run_id, "scope": scope}),
                );
                ctx.ui
                    .set_widget(
                        "mana",
                        Some(mana_widget_lines(
                            format!("running mana ({run_id})"),
                            Some(format!("native foreground orchestration · {scope}")),
                        )),
                    )
                    .await;
                ctx.ui.set_status("mana", Some("mana: running")).await;

                let run_store = self.run_store.clone();
                let run_id_for_sink = run_id.clone();
                let update_tx = ctx.update_tx.clone();
                match mana::commands::run::run_with_stream_capture_and_sink(
                    &mana_dir,
                    run_params,
                    Some(Arc::new(move |event| {
                        update_run_store_with_event(&run_store, &run_id_for_sink, &event);
                        if let Some(line) = stream_event_line(&event) {
                            let _ = update_tx.try_send(ToolUpdate {
                                content: vec![imp_llm::ContentBlock::Text { text: line }],
                                details: serde_json::to_value(&event)
                                    .unwrap_or(serde_json::Value::Null),
                            });
                        }
                    })),
                ) {
                    Ok(view) => {
                        finish_run_in_store(&self.run_store, &run_id, &view);
                        for line in run_summary_lines(&view) {
                            send_update(
                                &ctx,
                                line,
                                json!({"kind": "mana_run_view", "run_id": run_id, "scope": scope, "view": view}),
                            );
                        }
                        let summary = format!(
                            "mana finished · {} done · {} failed",
                            view.summary.total_closed, view.summary.total_failed
                        );
                        ctx.ui
                            .set_widget(
                                "mana",
                                Some(mana_widget_lines(summary.clone(), Some(scope.clone()))),
                            )
                            .await;
                        ctx.ui.set_status("mana", Some(&summary)).await;
                        Ok(ToolOutput {
                            content: run_summary_lines(&view)
                                .into_iter()
                                .map(|text| imp_llm::ContentBlock::Text { text })
                                .collect(),
                            details: json!({
                                "run_id": run_id,
                                "scope": scope,
                                "runtime": view.runtime,
                                "view": serde_json::to_value(&view).unwrap_or(serde_json::Value::Null)
                            }),
                            is_error: false,
                        })
                    }
                    Err(e) => {
                        fail_run_in_store(&self.run_store, &run_id, e.to_string());
                        Ok(ToolOutput::error(e.to_string()))
                    }
                }
            }
            other => Ok(ToolOutput::error(format!(
                "Unknown action: {other}. Use: status, list, show, create, close, update, run, run_state, evaluate, claim, release, logs, agents, next, tree, reopen, verify, fail, delete, dep_add, dep_remove, fact_create, fact_verify, notes_append, decision_add, decision_resolve"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;
    use tokio::sync::mpsc;

    use super::{
        GuideTopic, INTERRUPTED_RUN_STALE_MS, ManaRunStore, ManaTool, NativeRunState, RunTarget,
        RunUnitStatus, TemplateKind, evaluate_run_output, mana_close_error_output,
        mana_close_force_reason_error, mana_guide_output, mana_template_output,
        parent_placement_details, parse_guide_topic, parse_template_kind,
        retry_guardrail_for_targets, run_state_output, stream_event_line,
        target_ids_from_run_target, unix_time_ms, validate_mana_action,
    };
    use crate::tools::{FileCache, FileTracker, Tool, ToolContext, ToolUpdate};
    use crate::ui::{NotifyLevel, NullInterface, WidgetContent};

    enum ManaResult {
        ModeBlocked(String),
        Attempted(crate::tools::ToolOutput),
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct TestUi {
        widgets: Arc<std::sync::Mutex<Vec<(String, Option<WidgetContent>)>>>,
    }

    #[async_trait]
    impl crate::ui::UserInterface for TestUi {
        fn has_ui(&self) -> bool {
            true
        }

        async fn notify(&self, _message: &str, _level: NotifyLevel) {}

        async fn confirm(&self, _title: &str, _message: &str) -> Option<bool> {
            None
        }

        async fn select_with_context(
            &self,
            _title: &str,
            _context: &str,
            _options: &[crate::ui::SelectOption],
        ) -> Option<usize> {
            None
        }

        async fn input_with_context(
            &self,
            _title: &str,
            _context: &str,
            _placeholder: &str,
        ) -> Option<String> {
            None
        }

        async fn set_status(&self, _key: &str, _text: Option<&str>) {}

        async fn set_widget(&self, key: &str, content: Option<WidgetContent>) {
            self.widgets
                .lock()
                .unwrap()
                .push((key.to_string(), content));
        }

        async fn custom(&self, _component: crate::ui::ComponentSpec) -> Option<serde_json::Value> {
            None
        }
    }

    async fn run_with_mode(mode_name: &str, action: &str) -> ManaResult {
        let prev = {
            let _guard = ENV_LOCK.lock().unwrap();
            let prev = std::env::var("IMP_MODE").ok();
            std::env::set_var("IMP_MODE", mode_name);
            prev
        };

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
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::from_name(mode_name)
                .unwrap_or(crate::config::AgentMode::Full),
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };

        let tool = ManaTool::default();
        let outcome = tool
            .execute("call_1", json!({ "action": action, "id": "1" }), ctx)
            .await;

        match prev {
            Some(v) => {
                let _guard = ENV_LOCK.lock().unwrap();
                std::env::set_var("IMP_MODE", v)
            }
            None => {
                let _guard = ENV_LOCK.lock().unwrap();
                std::env::remove_var("IMP_MODE")
            }
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
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        (ctx, tempfile::tempdir().unwrap())
    }

    fn ctx_with_ui(
        dir: &std::path::Path,
        mode: crate::config::AgentMode,
    ) -> (
        ToolContext,
        tempfile::TempDir,
        Arc<std::sync::Mutex<Vec<(String, Option<WidgetContent>)>>>,
    ) {
        let mana_dir = dir.join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        )
        .unwrap();
        let widgets = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(TestUi {
                widgets: widgets.clone(),
            }),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        (ctx, tempfile::tempdir().unwrap(), widgets)
    }

    async fn run_with_ctx_mode(mode: crate::config::AgentMode, action: &str) -> ManaResult {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), mode);
        let tool = ManaTool::default();
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
    async fn create_sets_mana_delta_widget_and_action_details() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep, widgets) = ctx_with_ui(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_create_widget",
                json!({ "action": "create", "title": "Widget unit", "verify": "test -n ok" }),
                ctx,
            )
            .await
            .unwrap();

        assert_eq!(result.details["action"], "create");
        assert_eq!(result.details["unit"]["title"], "Widget unit");
        let widgets = widgets.lock().unwrap();
        assert!(widgets.iter().any(|(key, content)| {
            key == "mana"
                && matches!(content, Some(WidgetContent::Lines(lines)) if lines.iter().any(|line| line.contains("mana delta: created 2 · Widget unit")))
        }));
    }

    #[tokio::test]
    async fn decision_add_sets_mana_delta_widget_and_action_details() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep, widgets) = ctx_with_ui(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_decision_widget",
                json!({ "action": "decision_add", "id": "1", "description": "Choose retry limit" }),
                ctx,
            )
            .await
            .unwrap();

        assert_eq!(result.details["action"], "decision_add");
        assert_eq!(result.details["unit"]["decisions"][0], "Choose retry limit");
        let widgets = widgets.lock().unwrap();
        assert!(widgets.iter().any(|(key, content)| {
            key == "mana"
                && matches!(content, Some(WidgetContent::Lines(lines)) if lines.iter().any(|line| line.contains("mana delta: decision added on 1 · Test unit")))
        }));
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
    async fn create_supports_rich_unit_fields() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_create_rich",
                json!({
                    "action": "create",
                    "title": "Rich unit",
                    "description": "Implement the thing",
                    "acceptance": "- works\n- tested",
                    "notes": "start here",
                    "design": "follow existing pattern",
                    "verify": "test -n ok",
                    "labels": ["feature", "backend"],
                    "deps": ["1"],
                    "paths": ["src/lib.rs", "src/auth.rs"],
                    "requires": ["auth-api"],
                    "produces": ["auth-fix"],
                    "decisions": ["Confirm whether auth should stay sync"],
                    "feature": true,
                    "fail_first": true,
                    "verify_timeout": 12,
                    "force": false
                }),
                ctx,
            )
            .await
            .unwrap();
        let unit = &result.details["unit"];
        assert_eq!(unit["acceptance"], "- works\n- tested");
        assert_eq!(unit["labels"][0], "feature");
        assert_eq!(unit["dependencies"][0], "1");
        assert_eq!(unit["paths"][0], "src/lib.rs");
        assert_eq!(unit["requires"][0], "auth-api");
        assert_eq!(unit["produces"][0], "auth-fix");
        assert_eq!(
            unit["decisions"][0],
            "Confirm whether auth should stay sync"
        );
        assert_eq!(unit["feature"], true);
        assert_eq!(unit["fail_first"], true);
        assert_eq!(unit["verify_timeout"], 12);
    }

    #[test]
    fn parent_placement_details_warns_when_parent_reason_missing() {
        let details = parent_placement_details(Some("304"), None);

        assert_eq!(details["parent"], json!("304"));
        assert_eq!(details["warning"], json!("parent_reason_missing"));
        assert!(
            details["hint"]
                .as_str()
                .unwrap_or_default()
                .contains("confirm it belongs")
        );
    }

    #[test]
    fn parent_placement_details_accepts_explicit_reason() {
        let details = parent_placement_details(
            Some("313"),
            Some("This is workflow reliability work, not tool schema audit."),
        );

        assert_eq!(details["warning"], serde_json::Value::Null);
        assert_eq!(
            details["parent_reason"],
            json!("This is workflow reliability work, not tool schema audit.")
        );
    }

    #[test]
    fn close_force_requires_reason_with_evidence() {
        let output = mana_close_force_reason_error("313.2");

        assert!(output.is_error);
        assert_eq!(output.details["action"], json!("close"));
        assert_eq!(output.details["missing"], json!(["reason"]));
        assert!(
            output
                .text_content()
                .unwrap_or_default()
                .contains("equivalent verify evidence")
        );
    }

    #[test]
    fn close_verify_errors_include_recovery_hint() {
        let output = mana_close_error_output(
            "313.2",
            "Verify command failed during close: cargo test -p imp-core one two --no-run"
                .to_string(),
        );

        assert!(output.is_error);
        assert_eq!(output.details["verify_related"], json!(true));
        assert_eq!(output.details["force_requires_reason"], json!(true));
        let hint = output.details["recovery_hint"].as_str().unwrap_or_default();
        assert!(hint.contains("equivalent focused checks"));
    }

    #[test]
    fn close_non_verify_errors_stay_plain() {
        let output = mana_close_error_output("313.2", "Unit not found".to_string());

        assert!(output.is_error);
        assert_eq!(output.details["verify_related"], json!(false));
        assert!(output.details["recovery_hint"].is_null());
        assert_eq!(output.text_content().unwrap_or_default(), "Unit not found");
    }

    #[test]
    fn mana_guide_outputs_concise_structured_topic() {
        let output = mana_guide_output(GuideTopic::Verify);

        assert!(!output.is_error);
        assert_eq!(output.details["action"], json!("guide"));
        assert_eq!(output.details["topic"], json!("verify"));
        assert!(output.details["guidance"].as_array().unwrap().len() <= 3);
        assert!(
            output
                .text_content()
                .unwrap_or_default()
                .contains("mana guide: verify")
        );
    }

    #[test]
    fn mana_template_outputs_task_template() {
        let output = mana_template_output(TemplateKind::Task, Some(GuideTopic::Verify));

        assert!(!output.is_error);
        assert_eq!(output.details["action"], json!("template"));
        assert_eq!(output.details["kind"], json!("task"));
        assert_eq!(output.details["template"]["fail_first"], json!(true));
        assert!(output.details["template"]["verify"].is_string());
    }

    #[test]
    fn mana_guide_and_template_validate_topic_and_kind() {
        assert!(parse_guide_topic(&json!({ "topic": "orchestrate" })).is_ok());
        assert!(
            parse_guide_topic(&json!({ "topic": "bad" }))
                .unwrap_err()
                .to_string()
                .contains("Invalid mana guide topic")
        );
        assert!(parse_template_kind(&json!({ "kind": "fact" })).is_ok());
        assert!(
            parse_template_kind(&json!({ "kind": "job" }))
                .unwrap_err()
                .to_string()
                .contains("Invalid mana template kind")
        );
    }

    #[test]
    fn mana_schema_uses_canonical_fields_only() {
        let schema = ManaTool::default().parameters();
        let properties = schema["properties"].as_object().unwrap();

        assert!(properties.contains_key("action"));
        assert!(properties.contains_key("id"));
        assert!(properties.contains_key("run_id"));
        assert!(properties.contains_key("title"));
        assert!(properties.contains_key("description"));
        assert!(properties.contains_key("acceptance"));
        assert!(properties.contains_key("verify"));
        assert!(properties.contains_key("notes"));
        assert!(properties.contains_key("decisions"));
        assert!(properties.contains_key("paths"));
        assert!(properties.contains_key("deps"));
        assert!(properties.contains_key("labels"));
        assert!(properties.contains_key("targets"));
        assert!(properties.contains_key("scope"));
        assert!(properties.contains_key("path"));
        assert!(properties.contains_key("parent_reason"));
        let actions = properties["action"]["enum"].as_array().unwrap();
        assert!(actions.iter().any(|value| value == "reparent"));

        assert!(!properties.contains_key("mana_scope"));
        assert!(!properties.contains_key("mana_dir"));
        assert!(!properties.contains_key("paths_csv"));
        assert!(!properties.contains_key("fact_title"));
        assert!(!properties.contains_key("add_label"));
        assert!(!properties.contains_key("remove_label"));

        let kind_enum = properties["kind"]["enum"].as_array().unwrap();
        assert!(kind_enum.iter().any(|value| value == "task"));
        assert!(!kind_enum.iter().any(|value| value == "job"));
    }

    #[test]
    fn mana_validation_teaches_required_fields() {
        let output = validate_mana_action("notes_append", &json!({ "id": "304" }))
            .expect("notes_append without notes should fail validation");

        assert!(output.is_error);
        assert_eq!(output.details["action"], json!("notes_append"));
        assert_eq!(output.details["missing"], json!(["notes"]));
        assert_eq!(output.details["canonical_fields"], json!(["id", "notes"]));
        assert!(
            output
                .text_content()
                .unwrap_or_default()
                .contains("requires id and notes")
        );
    }

    #[test]
    fn mana_validation_rejects_path_when_paths_is_intended() {
        let output = validate_mana_action(
            "create",
            &json!({ "title": "Doc task", "path": "src/lib.rs" }),
        )
        .expect("create with path attachment should teach paths");

        assert!(output.is_error);
        assert_eq!(output.details["invalid"], json!(["path"]));
        assert!(
            output
                .text_content()
                .unwrap_or_default()
                .contains("Use path for project/.mana location")
        );
    }

    #[test]
    fn mana_validation_allows_valid_create_and_decision_add() {
        assert!(validate_mana_action("create", &json!({ "title": "Build thing" })).is_none());
        assert!(
            validate_mana_action(
                "decision_add",
                &json!({ "id": "304", "decisions": ["Use canonical names"] }),
            )
            .is_none()
        );
    }

    #[test]
    fn mana_validation_requires_parent_for_reparent() {
        let output = validate_mana_action("reparent", &json!({ "id": "313.2" }))
            .expect("reparent without parent should fail validation");

        assert!(output.is_error);
        assert_eq!(output.details["missing"], json!(["parent"]));
        assert_eq!(
            output.details["canonical_fields"],
            json!(["id", "parent", "reason"])
        );
    }

    #[tokio::test]
    async fn reparent_moves_child_between_parents() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();

        let old_parent = tool
            .execute(
                "old_parent",
                json!({ "action": "create", "title": "Old Parent" }),
                ctx.clone(),
            )
            .await
            .unwrap();
        let old_parent_id = old_parent.details["unit"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let new_parent = tool
            .execute(
                "new_parent",
                json!({ "action": "create", "title": "New Parent" }),
                ctx.clone(),
            )
            .await
            .unwrap();
        let new_parent_id = new_parent.details["unit"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let child = tool
            .execute(
                "child",
                json!({ "action": "create", "title": "Child", "parent": old_parent_id, "parent_reason": "Initial grouping" }),
                ctx.clone(),
            )
            .await
            .unwrap();
        let child_id = child.details["unit"]["id"].as_str().unwrap().to_string();

        let moved = tool
            .execute(
                "move_child",
                json!({
                    "action": "reparent",
                    "id": child_id,
                    "parent": new_parent_id,
                    "reason": "Belongs under the new reliability epic"
                }),
                ctx.clone(),
            )
            .await
            .unwrap();

        assert!(!moved.is_error);
        assert_eq!(moved.details["action"], json!("reparent"));
        assert_eq!(moved.details["old_parent"], json!(old_parent_id));
        assert_eq!(moved.details["new_parent"], json!(new_parent_id));
        assert!(
            moved
                .text_content()
                .unwrap_or_default()
                .contains("reparented")
        );
    }

    #[tokio::test]
    async fn update_supports_acceptance_labels_and_decisions() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let _created = tool
            .execute(
                "call_create_update_target",
                json!({ "action": "create", "title": "Update target", "verify": "test -n ok" }),
                ctx,
            )
            .await
            .unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        let (ctx2, _keep2) = ctx_with_mode(dir2.path(), crate::config::AgentMode::Full);
        std::fs::write(
            dir2.path().join(".mana").join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        ).unwrap();
        let result = tool
            .execute(
                "call_update_rich",
                json!({
                    "action": "update",
                    "id": "1",
                    "acceptance": "must pass auth flow",
                    "labels": ["backend"],
                    "decisions": ["Choose retry limit"],
                    "resolve_decisions": []
                }),
                ctx2,
            )
            .await
            .unwrap();
        let unit = &result.details["unit"];
        assert_eq!(unit["acceptance"], "must pass auth flow");
        assert_eq!(unit["labels"][0], "backend");
        assert_eq!(unit["decisions"][0], "Choose retry limit");
    }

    #[tokio::test]
    async fn create_respects_verify_lint_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_create_lint",
                json!({ "action": "create", "title": "Weak verify", "verify": "echo done" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.is_error, "weak verify should be rejected by default");
        let text = result.text_content().unwrap_or("");
        assert!(text.contains("Verify command has lint errors") || text.contains("verify"));
    }

    #[tokio::test]
    async fn native_verify_reopen_and_fact_actions_work() {
        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: closed\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\nclosed_at: '2026-03-28T00:00:00Z'\nclose_reason: done\n---\n\nbody\n",
        ).unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let tool = ManaTool::default();
        let reopened = tool
            .execute("call_reopen", json!({ "action": "reopen", "id": "1" }), ctx)
            .await
            .unwrap();
        assert_eq!(reopened.details["unit"]["status"], "open");

        let dir2 = tempfile::tempdir().unwrap();
        let mana_dir2 = dir2.path().join(".mana");
        std::fs::create_dir_all(&mana_dir2).unwrap();
        std::fs::write(mana_dir2.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir2.join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        ).unwrap();
        let (tx2, _rx2) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx2, _cmd_rx2) = mpsc::channel(16);
        let ctx2 = ToolContext {
            cwd: dir2.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx2,
            command_tx: cmd_tx2,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let verify = tool
            .execute(
                "call_verify",
                json!({ "action": "verify", "id": "1" }),
                ctx2,
            )
            .await
            .unwrap();
        assert_eq!(verify.details["passed"], true);

        let dir3 = tempfile::tempdir().unwrap();
        let mana_dir3 = dir3.path().join(".mana");
        std::fs::create_dir_all(&mana_dir3).unwrap();
        std::fs::write(mana_dir3.join("config.yaml"), "project: test\nnext_id: 1\n").unwrap();
        let (tx3, _rx3) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx3, _cmd_rx3) = mpsc::channel(16);
        let ctx3 = ToolContext {
            cwd: dir3.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx3,
            command_tx: cmd_tx3,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let fact = tool.execute("call_fact", json!({ "action": "fact_create", "title": "Auth fact", "verify": "test -d .mana", "description": "fact body", "ttl_days": 7 }), ctx3).await.unwrap();
        assert_eq!(fact.details["unit"]["unit_type"], "fact");
    }

    #[tokio::test]
    async fn notes_append_is_safe_partial_update() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_notes_append",
                json!({
                    "action": "notes_append",
                    "id": "1",
                    "notes": "diagnosis from turn 2"
                }),
                ctx,
            )
            .await
            .unwrap();
        let unit = &result.details["unit"];
        assert_eq!(unit["title"], "Test unit");
        assert!(
            unit["notes"]
                .as_str()
                .unwrap_or("")
                .contains("diagnosis from turn 2")
        );
    }

    #[tokio::test]
    async fn decision_add_and_resolve_work() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let added = tool
            .execute(
                "call_decision_add",
                json!({
                    "action": "decision_add",
                    "id": "1",
                    "description": "Choose retry limit"
                }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(added.details["unit"]["decisions"][0], "Choose retry limit");

        let dir2 = tempfile::tempdir().unwrap();
        let (ctx2, _keep2) = ctx_with_mode(dir2.path(), crate::config::AgentMode::Full);
        std::fs::write(
            dir2.path().join(".mana").join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\ndecisions:\n  - Choose retry limit\n---\n\nbody\n",
        ).unwrap();
        let resolved = tool
            .execute(
                "call_decision_resolve",
                json!({
                    "action": "decision_resolve",
                    "id": "1",
                    "resolve_decisions": ["Choose retry limit"]
                }),
                ctx2,
            )
            .await
            .unwrap();
        let decisions = resolved.details["unit"]["decisions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(decisions.is_empty());
    }

    #[tokio::test]
    async fn show_returns_archived_unit_when_active_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir_all(mana_dir.join("archive/2026/04")).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("archive/2026/04/1-archived-unit.md"),
            "---\nid: '1'\ntitle: Archived unit\nstatus: closed\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nclosed_at: '2026-03-28T00:00:00Z'\nclose_reason: done\nis_archived: true\n---\n\nbody\n",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_show_archived",
                json!({ "action": "show", "id": "1" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.details["title"], "Archived unit");
        assert_eq!(result.details["is_archived"], true);
    }

    #[tokio::test]
    async fn tree_reports_archived_root_as_active_only_limitation() {
        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir_all(mana_dir.join("archive/2026/04")).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("archive/2026/04/1-archived-unit.md"),
            "---\nid: '1'\ntitle: Archived unit\nstatus: closed\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nclosed_at: '2026-03-28T00:00:00Z'\nclose_reason: done\nis_archived: true\n---\n\nbody\n",
        )
        .unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_tree_archived",
                json!({ "action": "tree", "id": "1" }),
                ctx,
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = result.text_content().unwrap_or("");
        assert!(text.contains("Archived unit 1 can be shown but not rendered in tree view"));
    }

    #[tokio::test]
    async fn root_scope_targets_outermost_mana() {
        let tower = tempfile::tempdir().unwrap();
        let root_mana = tower.path().join(".mana");
        std::fs::create_dir_all(&root_mana).unwrap();
        std::fs::write(root_mana.join("config.yaml"), "project: root\nnext_id: 2\n").unwrap();
        std::fs::write(
            root_mana.join("1-root-unit.md"),
            "---\nid: '1'\ntitle: Root unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        ).unwrap();
        let project = tower.path().join("imp");
        let project_mana = project.join(".mana");
        std::fs::create_dir_all(&project_mana).unwrap();
        std::fs::write(
            project_mana.join("config.yaml"),
            "project: nested\nnext_id: 2\n",
        )
        .unwrap();
        std::fs::write(
            project_mana.join("1-project-unit.md"),
            "---\nid: '1'\ntitle: Project unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        ).unwrap();
        let workdir = project.join("src");
        std::fs::create_dir_all(&workdir).unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: workdir,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_root_scope",
                json!({ "action": "show", "id": "1", "scope": "root" }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(result.details["title"], "Root unit");
    }

    #[tokio::test]
    async fn explicit_path_targets_project_outside_cwd_ancestry() {
        let outside = tempfile::tempdir().unwrap();
        let target_project = outside.path().join("other-project");
        let target_mana = target_project.join(".mana");
        std::fs::create_dir_all(&target_mana).unwrap();
        std::fs::write(
            target_mana.join("config.yaml"),
            "project: other\nnext_id: 2\n",
        )
        .unwrap();
        std::fs::write(
            target_mana.join("1-other-unit.md"),
            "---\nid: '1'\ntitle: Other unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        )
        .unwrap();

        let unrelated = tempfile::tempdir().unwrap();
        let workdir = unrelated.path().join("scratch");
        std::fs::create_dir_all(&workdir).unwrap();
        let (tx, _rx) = mpsc::channel::<ToolUpdate>(1);
        let (cmd_tx, _cmd_rx) = mpsc::channel(16);
        let ctx = ToolContext {
            cwd: workdir,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_explicit_path",
                json!({ "action": "show", "id": "1", "path": target_project }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(result.details["title"], "Other unit");
    }

    #[tokio::test]
    async fn worker_blocks_fact_create() {
        match run_with_mode("worker", "fact_create").await {
            ManaResult::ModeBlocked(_) => {}
            ManaResult::Attempted(out) => {
                panic!(
                    "worker should block 'fact_create', got: {:?}",
                    out.text_content()
                )
            }
        }
    }

    #[tokio::test]
    async fn worker_allows_verify() {
        match run_with_mode("worker", "verify").await {
            ManaResult::Attempted(_) => {}
            ManaResult::ModeBlocked(msg) => {
                panic!("worker should allow 'verify' but was blocked: {msg}")
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
            "status",
            "list",
            "show",
            "create",
            "close",
            "update",
            "run",
            "run_state",
            "evaluate",
            "claim",
            "release",
            "logs",
            "agents",
            "next",
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
        let prev = {
            let _guard = ENV_LOCK.lock().unwrap();
            let prev = std::env::var("IMP_MODE").ok();
            std::env::set_var("IMP_MODE", "full");
            prev
        };

        let result = run_with_ctx_mode(crate::config::AgentMode::Worker, "create").await;

        match prev {
            Some(v) => {
                let _guard = ENV_LOCK.lock().unwrap();
                std::env::set_var("IMP_MODE", v)
            }
            None => {
                let _guard = ENV_LOCK.lock().unwrap();
                std::env::remove_var("IMP_MODE")
            }
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
            "status",
            "list",
            "show",
            "create",
            "close",
            "update",
            "run",
            "run_state",
            "evaluate",
            "claim",
            "release",
            "logs",
            "agents",
            "next",
            "tree",
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
        let tool = ManaTool::default();
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
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_bg",
                json!({ "action": "run", "background": true, "dry_run": true }),
                ctx,
            )
            .await
            .unwrap();
        let text = result.text_content().unwrap_or("");
        assert!(text.contains("Started native mana orchestration in background"));
        assert_eq!(result.details["background"], true);
        assert!(result.details["run_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn background_run_enqueues_follow_up_on_completion_without_ui() {
        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
        std::fs::write(
            mana_dir.join("1-test-unit.md"),
            "---\nid: '1'\ntitle: Test unit\nstatus: open\npriority: 2\ncreated_at: '2026-03-28T00:00:00Z'\nupdated_at: '2026-03-28T00:00:00Z'\nverify: test -n \"ok\"\n---\n\nbody\n",
        )
        .unwrap();

        let (tx, _rx) = mpsc::channel::<ToolUpdate>(8);
        let (cmd_tx, mut cmd_rx) = mpsc::channel(8);
        let ctx = ToolContext {
            cwd: dir.path().to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            command_tx: cmd_tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(FileCache::new()),
            checkpoint_state: Arc::new(crate::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(FileTracker::new())),
            anchor_store: Arc::new(crate::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
            turn_mana_review: Arc::new(std::sync::Mutex::new(
                crate::mana_review::TurnManaReviewAccumulator::default(),
            )),
            config: Arc::new(crate::config::Config::default()),
        };

        let tool = ManaTool::default();
        let _ = tool
            .execute(
                "call_bg_follow_up",
                json!({ "action": "run", "background": true, "dry_run": true }),
                ctx,
            )
            .await
            .unwrap();

        let follow_up = tokio::time::timeout(std::time::Duration::from_secs(2), cmd_rx.recv())
            .await
            .expect("follow-up timeout")
            .expect("follow-up message");

        match follow_up {
            crate::agent::AgentCommand::FollowUp(text) => {
                assert!(
                    text.contains("Native mana orchestration finished"),
                    "text was: {text}"
                );
                assert!(
                    text.contains("Inspect with mana(action=\"run_state\")"),
                    "text was: {text}"
                );
            }
            other => panic!("expected follow-up, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn background_run_with_ui_does_not_enqueue_follow_up_on_completion() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep, _widgets) = ctx_with_ui(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let (cmd_tx, mut cmd_rx) = mpsc::channel(8);
        let ctx = ToolContext {
            command_tx: cmd_tx,
            ..ctx
        };

        let _ = tool
            .execute(
                "call_bg_follow_up_ui",
                json!({ "action": "run", "background": true, "dry_run": true }),
                ctx,
            )
            .await
            .unwrap();

        let follow_up =
            tokio::time::timeout(std::time::Duration::from_millis(700), cmd_rx.recv()).await;
        match follow_up {
            Err(_) | Ok(None) => {}
            Ok(Some(msg)) => panic!(
                "UI mode should rely on widget/status instead of queueing duplicate follow-up chat text, got: {msg:?}"
            ),
        }
    }

    #[tokio::test]
    async fn background_run_supports_explicit_targets() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();
        let result = tool
            .execute(
                "call_bg_targets",
                json!({ "action": "run", "background": true, "dry_run": true, "targets": ["1", "2"] }),
                ctx,
            )
            .await
            .unwrap();
        assert_eq!(result.details["target"]["kind"], "explicit");
        assert_eq!(result.details["target"]["ids"][0], "1");
        assert_eq!(result.details["target"]["ids"][1], "2");
    }

    #[tokio::test]
    async fn run_state_and_evaluate_report_native_run() {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, _keep) = ctx_with_mode(dir.path(), crate::config::AgentMode::Full);
        let tool = ManaTool::default();

        let run_result = tool
            .execute(
                "call_run",
                json!({ "action": "run", "background": false, "dry_run": true }),
                ctx,
            )
            .await
            .unwrap();
        let run_id = run_result.details["run_id"]
            .as_str()
            .expect("run_id")
            .to_string();

        let dir2 = tempfile::tempdir().unwrap();
        let (ctx2, _keep2) = ctx_with_mode(dir2.path(), crate::config::AgentMode::Full);
        let state = tool
            .execute(
                "call_state",
                json!({ "action": "run_state", "run_id": run_id.as_str() }),
                ctx2,
            )
            .await
            .unwrap();
        let state_text = state.text_content().unwrap_or("");
        assert!(
            state_text.contains("Native mana orchestration "),
            "state_text was: {state_text}"
        );
        assert!(
            state_text.contains("Worker runtime:"),
            "state_text was: {state_text}"
        );
        assert!(
            state_text.contains("Units:") || state_text.contains("Latest: Dry run:"),
            "state_text was: {state_text}"
        );
        assert!(
            state_text.contains("all ready units") || state_text.contains("unit"),
            "state_text was: {state_text}"
        );

        let dir3 = tempfile::tempdir().unwrap();
        let (ctx3, _keep3) = ctx_with_mode(dir3.path(), crate::config::AgentMode::Full);
        let evaluation = tool
            .execute(
                "call_eval",
                json!({ "action": "evaluate", "run_id": run_result.details["run_id"] }),
                ctx3,
            )
            .await
            .unwrap();
        let eval_text = evaluation.text_content().unwrap_or("");
        assert!(
            eval_text.contains("Native mana orchestration run ") && eval_text.contains("finished"),
            "eval_text was: {eval_text}"
        );
        assert!(
            eval_text.contains("Worker runtime:"),
            "eval_text was: {eval_text}"
        );
    }

    #[test]
    fn run_store_prefers_active_run_snapshot() {
        let mut store = ManaRunStore::default();
        let active_id = store.start_run(
            "all ready units".to_string(),
            true,
            &mana::commands::run::NativeRunParams {
                target: mana::commands::run::RunTarget::AllReady,
                jobs: 2,
                dry_run: false,
                loop_mode: false,
                keep_going: false,
                timeout: 30,
                idle_timeout: 5,
                json_stream: true,
                review: false,
            },
        );
        let finished_id = store.start_run(
            "unit 1".to_string(),
            false,
            &mana::commands::run::NativeRunParams {
                target: mana::commands::run::RunTarget::Unit("1".to_string()),
                jobs: 1,
                dry_run: true,
                loop_mode: false,
                keep_going: false,
                timeout: 30,
                idle_timeout: 5,
                json_stream: true,
                review: false,
            },
        );
        store.fail_run(&finished_id, "done".to_string());

        let latest = store.snapshot(None).expect("snapshot");
        assert_eq!(latest.run_id, active_id);
        assert_eq!(latest.status, "starting");
    }

    #[test]
    fn stream_event_line_formats_tool_activity() {
        let line = stream_event_line(&mana::stream::StreamEvent::UnitTool {
            id: "1".to_string(),
            tool_name: "read".to_string(),
            tool_count: 3,
            file_path: Some("src/lib.rs".to_string()),
        })
        .expect("line");
        assert!(line.contains("#3 read"));
        assert!(line.contains("src/lib.rs"));
    }

    #[test]
    fn evaluate_output_reports_failures() {
        let mut state = NativeRunState::new(
            "run-7".to_string(),
            "unit 7".to_string(),
            false,
            &mana::commands::run::NativeRunParams {
                target: mana::commands::run::RunTarget::Unit("7".to_string()),
                jobs: 1,
                dry_run: false,
                loop_mode: false,
                keep_going: false,
                timeout: 30,
                idle_timeout: 5,
                json_stream: true,
                review: false,
            },
        );
        state.status = "finished".to_string();
        state.summary.total_failed = 2;
        state.log_lines.push("✗ 7 failed verify".to_string());

        let output = evaluate_run_output(&state);
        let text = output.text_content().unwrap_or("");
        assert!(text.contains("2 failed unit"));
        assert!(text.contains("Latest: ✗ 7 failed verify"));
        assert!(text.contains("Next: Inspect failed units"));
        assert_eq!(
            output.details["recovery"]["retry_requires_unit_update"],
            json!(true)
        );
        assert!(
            output.details["recovery"]["next_actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action
                    .as_str()
                    .unwrap_or_default()
                    .contains("do not rerun unchanged"))
        );
    }

    #[test]
    fn run_state_output_includes_recovery_details() {
        let mut state = NativeRunState::new(
            "run-8".to_string(),
            "unit 8".to_string(),
            false,
            &mana::commands::run::NativeRunParams {
                target: mana::commands::run::RunTarget::Unit("8".to_string()),
                jobs: 1,
                dry_run: false,
                loop_mode: false,
                keep_going: false,
                timeout: 30,
                idle_timeout: 5,
                json_stream: true,
                review: false,
            },
        );
        state.status = "failed".to_string();
        state.summary.total_failed = 1;
        state.units.push(RunUnitStatus {
            id: "8".to_string(),
            title: "Failed unit".to_string(),
            status: "failed".to_string(),
            round: Some(1),
            agent: Some("imp-worker".to_string()),
            model: None,
            duration_secs: Some(3),
            tool_count: Some(2),
            turns: Some(1),
            failure_summary: Some("verify failed".to_string()),
            error: Some("exit 1".to_string()),
        });

        let output = run_state_output(&state);
        let text = output.text_content().unwrap_or_default();

        assert!(text.contains("Next: Inspect failed units"));
        assert_eq!(
            output.details["recovery"]["failed_units"][0]["id"],
            json!("8")
        );
        assert_eq!(
            output.details["recovery"]["retry_requires_unit_update"],
            json!(true)
        );
    }
    #[test]
    fn target_ids_from_run_target_extracts_explicit_units() {
        assert_eq!(
            target_ids_from_run_target(&RunTarget::Unit("273.3".to_string())),
            vec!["273.3".to_string()]
        );
        assert_eq!(
            target_ids_from_run_target(&RunTarget::Explicit(vec![
                "1".to_string(),
                "2".to_string()
            ])),
            vec!["1".to_string(), "2".to_string()]
        );
        assert!(target_ids_from_run_target(&RunTarget::AllReady).is_empty());
    }

    #[test]
    fn retry_guardrail_blocks_failed_unit_without_new_update() {
        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();
        mana_core::config::Config {
            project: "retry-test".to_string(),
            next_id: 1,
            auto_close_parent: true,
            run: None,
            plan: None,
            max_loops: 10,
            max_concurrent: 4,
            poll_interval: 30,
            extends: vec![],
            rules_file: None,
            file_locking: false,
            worktree: false,
            on_close: None,
            on_fail: None,
            verify_timeout: None,
            review: None,
            user: None,
            user_email: None,
            auto_commit: false,
            commit_template: None,
            research: None,
            run_model: None,
            plan_model: None,
            review_model: None,
            research_model: None,
            batch_verify: false,
            memory_reserve_mb: 0,
            notify: None,
        }
        .save(&mana_dir)
        .unwrap();

        let created = mana_core::api::create_unit(
            &mana_dir,
            mana_core::ops::create::CreateParams {
                title: "Retry target".to_string(),
                verify: Some("false".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        let id = created.unit.id;
        let now = chrono::Utc::now();
        let mut failed_unit = mana_core::ops::show::get(&mana_dir, &id).unwrap().unit;
        failed_unit
            .attempt_log
            .push(mana_core::unit::AttemptRecord {
                num: 1,
                outcome: mana_core::unit::AttemptOutcome::Failed,
                notes: Some("verify failed".to_string()),
                agent: Some("imp-test".to_string()),
                started_at: Some(now),
                finished_at: Some(now),
                autonomy_observation: None,
            });
        failed_unit.updated_at = now - chrono::Duration::milliseconds(1);
        let unit_path = mana_core::discovery::find_unit_file(&mana_dir, &id).unwrap();
        failed_unit.to_file(&unit_path).unwrap();

        let guardrail = retry_guardrail_for_targets(&mana_dir, std::slice::from_ref(&id))
            .unwrap()
            .expect("failed unchanged unit should require update");

        assert_eq!(guardrail["retry_requires_unit_update"], json!(true));
        assert_eq!(guardrail["blocked_units"][0]["id"], json!(id));
        assert!(
            guardrail["next_actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action.as_str().unwrap_or_default().contains("Append notes"))
        );
    }

    #[test]
    fn retry_guardrail_allows_failed_unit_after_update() {
        let dir = tempfile::tempdir().unwrap();
        let mana_dir = dir.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();
        mana_core::config::Config {
            project: "retry-test".to_string(),
            next_id: 1,
            auto_close_parent: true,
            run: None,
            plan: None,
            max_loops: 10,
            max_concurrent: 4,
            poll_interval: 30,
            extends: vec![],
            rules_file: None,
            file_locking: false,
            worktree: false,
            on_close: None,
            on_fail: None,
            verify_timeout: None,
            review: None,
            user: None,
            user_email: None,
            auto_commit: false,
            commit_template: None,
            research: None,
            run_model: None,
            plan_model: None,
            review_model: None,
            research_model: None,
            batch_verify: false,
            memory_reserve_mb: 0,
            notify: None,
        }
        .save(&mana_dir)
        .unwrap();

        let created = mana_core::api::create_unit(
            &mana_dir,
            mana_core::ops::create::CreateParams {
                title: "Retry target".to_string(),
                verify: Some("false".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        let id = created.unit.id;
        let now = chrono::Utc::now();
        let mut failed_unit = mana_core::ops::show::get(&mana_dir, &id).unwrap().unit;
        failed_unit
            .attempt_log
            .push(mana_core::unit::AttemptRecord {
                num: 1,
                outcome: mana_core::unit::AttemptOutcome::Failed,
                notes: Some("verify failed".to_string()),
                agent: Some("imp-test".to_string()),
                started_at: Some(now),
                finished_at: Some(now),
                autonomy_observation: None,
            });
        failed_unit.updated_at = now;
        let unit_path = mana_core::discovery::find_unit_file(&mana_dir, &id).unwrap();
        failed_unit.to_file(&unit_path).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        mana_core::api::update_unit(
            &mana_dir,
            &id,
            mana_core::ops::update::UpdateParams {
                notes: Some("Changed retry plan after failure".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let guardrail = retry_guardrail_for_targets(&mana_dir, &[id]).unwrap();

        assert!(guardrail.is_none());
    }
    fn native_run_params_for_test() -> mana::commands::run::NativeRunParams {
        mana::commands::run::NativeRunParams {
            target: mana::commands::run::RunTarget::AllReady,
            jobs: 1,
            dry_run: false,
            loop_mode: false,
            keep_going: false,
            timeout: 30,
            idle_timeout: 5,
            json_stream: true,
            review: false,
        }
    }

    #[test]
    fn mana_run_state_persists_material_events() {
        let mut store = ManaRunStore::default();
        let run_id = store.start_run(
            "all ready units".to_string(),
            true,
            &native_run_params_for_test(),
        );
        let before = store.snapshot(Some(&run_id)).unwrap().last_event_at_ms;

        store.update_with_event(
            &run_id,
            &mana::stream::StreamEvent::UnitTool {
                id: "1".to_string(),
                tool_name: "read".to_string(),
                tool_count: 1,
                file_path: Some("src/lib.rs".to_string()),
            },
        );

        let state = store.snapshot(Some(&run_id)).unwrap();
        assert_eq!(state.event_count, 1);
        assert!(state.last_event_at_ms >= before);
        assert!(state.log_lines.iter().any(|line| line.contains("#1 read")));
    }

    #[test]
    fn mana_run_state_marks_stale_running_runs_interrupted() {
        let mut store = ManaRunStore::default();
        let run_id = store.start_run(
            "all ready units".to_string(),
            true,
            &native_run_params_for_test(),
        );
        {
            let run = store
                .runs
                .iter_mut()
                .find(|run| run.run_id == run_id)
                .unwrap();
            run.status = "running".to_string();
            run.last_event_at_ms = unix_time_ms().saturating_sub(INTERRUPTED_RUN_STALE_MS + 1_000);
        }

        store.classify_stale_unfinished_runs();

        let state = store.snapshot(Some(&run_id)).unwrap();
        assert_eq!(state.status, "interrupted");
        assert!(state.error.as_deref().unwrap_or_default().contains("stale"));
        let output = run_state_output(&state);
        let text = output.text_content().unwrap_or_default();
        assert!(text.contains("Interrupted:"));
        assert_eq!(
            output.details["recovery"]["stale_workers"][0]["run_id"],
            run_id
        );
        assert!(
            output.details["recovery"]["next_actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action
                    .as_str()
                    .unwrap_or_default()
                    .contains("interrupted/stale"))
        );
    }
}
