use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use mana::commands::agents::load_agents;
use mana::commands::logs::find_all_logs;
use mana::commands::next::ScoredUnit;
use mana::commands::run::{RunArgs, RunSummary, RunUnitStatus, RunView};
use mana::stream::StreamEvent;
use mana_core::ops::claim::ClaimParams;
use serde::Serialize;
use serde_json::json;

use super::{truncate_head, Tool, ToolContext, ToolOutput, ToolUpdate};
use crate::error::Result;
use crate::ui::{NotifyLevel, WidgetContent};
const MAX_OUTPUT_LINES: usize = 2000;
const MAX_OUTPUT_BYTES: usize = 50 * 1024;
const MAX_STORED_RUN_EVENTS: usize = 64;

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

#[derive(Debug, Clone, Serialize)]
struct NativeRunArgsView {
    id: Option<String>,
    jobs: u32,
    dry_run: bool,
    loop_mode: bool,
    keep_going: bool,
    timeout: u32,
    idle_timeout: u32,
    review: bool,
}

impl From<&RunArgs> for NativeRunArgsView {
    fn from(args: &RunArgs) -> Self {
        Self {
            id: args.id.clone(),
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

#[derive(Debug, Clone, Serialize)]
struct NativeRunState {
    run_id: String,
    scope: String,
    background: bool,
    status: String,
    error: Option<String>,
    started_at_ms: u128,
    finished_at_ms: Option<u128>,
    args: NativeRunArgsView,
    summary: RunSummary,
    units: Vec<RunUnitStatus>,
    log_lines: Vec<String>,
    event_count: usize,
}

impl NativeRunState {
    fn new(run_id: String, scope: String, background: bool, args: &RunArgs) -> Self {
        Self {
            run_id,
            scope,
            background,
            status: "starting".to_string(),
            error: None,
            started_at_ms: unix_time_ms(),
            finished_at_ms: None,
            args: NativeRunArgsView::from(args),
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
        if let Some(line) = stream_event_line(event) {
            self.log_lines.push(line);
            if self.log_lines.len() > MAX_STORED_RUN_EVENTS {
                let overflow = self.log_lines.len() - MAX_STORED_RUN_EVENTS;
                self.log_lines.drain(0..overflow);
            }
        }

        match event {
            StreamEvent::RunStart {
                total_units,
                total_rounds,
                units,
                ..
            } => {
                self.status = "running".to_string();
                self.summary.total_units = *total_units;
                self.summary.total_rounds = *total_rounds;
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
            StreamEvent::RunPlan { total_units, .. } => {
                self.status = "running".to_string();
                self.summary.total_units = (*total_units).max(self.summary.total_units);
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
            StreamEvent::DryRun { .. } => {
                self.status = "finished".to_string();
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
        self.summary = view.summary.clone();
        self.units = view.units.clone();
        self.status = "finished".to_string();
        self.error = None;
        self.finished_at_ms = Some(unix_time_ms());
    }

    fn fail(&mut self, error: String) {
        self.status = "failed".to_string();
        self.error = Some(error.clone());
        self.finished_at_ms = Some(unix_time_ms());
        self.log_lines.push(error);
        if self.log_lines.len() > MAX_STORED_RUN_EVENTS {
            let overflow = self.log_lines.len() - MAX_STORED_RUN_EVENTS;
            self.log_lines.drain(0..overflow);
        }
    }
}

#[derive(Debug, Default)]
struct ManaRunStore {
    next_id: u64,
    runs: Vec<NativeRunState>,
}

impl ManaRunStore {
    fn start_run(&mut self, scope: String, background: bool, args: &RunArgs) -> String {
        self.next_id += 1;
        let run_id = format!("run-{}", self.next_id);
        self.runs
            .push(NativeRunState::new(run_id.clone(), scope, background, args));
        self.trim_history();
        run_id
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
            if let Some(index) = self
                .runs
                .iter()
                .position(|run| run.status != "starting" && run.status != "running")
            {
                self.runs.remove(index);
            } else {
                break;
            }
        }
    }
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
    }
}

fn finish_run_in_store(store: &std::sync::Mutex<ManaRunStore>, run_id: &str, view: &RunView) {
    if let Ok(mut store) = store.lock() {
        store.finish_run(run_id, view);
    }
}

fn fail_run_in_store(store: &std::sync::Mutex<ManaRunStore>, run_id: &str, error: String) {
    if let Ok(mut store) = store.lock() {
        store.fail_run(run_id, error);
    }
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

fn background_run_started_output(scope: &str, run_id: &str, run_args: &RunArgs) -> ToolOutput {
    let text = format!(
        "Started mana run in background for {scope} as {run_id}. Use mana(action=\"run_state\", run_id=\"{run_id}\") for native status, mana(action=\"logs\", run_id=\"{run_id}\") for recent native events, and mana(action=\"agents\") / mana(action=\"logs\", id=...) for worker output."
    );
    ToolOutput {
        content: vec![imp_llm::ContentBlock::Text { text }],
        details: json!({
            "background": true,
            "run_id": run_id,
            "scope": scope,
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
    run_args: RunArgs,
    ctx: ToolContext,
    run_store: Arc<std::sync::Mutex<ManaRunStore>>,
    run_id: String,
) {
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
                Some(format!(
                    "inspect with mana run_state/logs (run_id={run_id})"
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
                fail_run_in_store(&run_store, &run_id, message.clone());
                ui.set_status("mana", Some(&message)).await;
                ui.set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
                    .await;
                ui.notify(&message, NotifyLevel::Error).await;
            }
            Err(join_err) => {
                let message = format!("mana: {scope} task failed: {join_err}");
                fail_run_in_store(&run_store, &run_id, message.clone());
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

fn run_state_snapshot(
    run_store: &Arc<std::sync::Mutex<ManaRunStore>>,
    run_id: Option<&str>,
) -> Option<NativeRunState> {
    run_store
        .lock()
        .ok()
        .and_then(|store| store.snapshot(run_id))
}

fn run_state_output(state: &NativeRunState) -> ToolOutput {
    let mut lines = vec![format!(
        "Mana run {}: {} · {}",
        state.run_id, state.scope, state.status
    )];
    lines.push(format!(
        "{} total · {} done · {} failed · {} awaiting verify · {} skipped",
        state.summary.total_units,
        state.summary.total_closed,
        state.summary.total_failed,
        state.summary.total_awaiting_verify,
        state.summary.total_skipped
    ));
    if let Some(last) = state.log_lines.last() {
        lines.push(format!("Latest: {last}"));
    }
    text_output(
        lines.join("\n"),
        serde_json::to_value(state).unwrap_or(serde_json::Value::Null),
    )
}

fn evaluate_run_output(state: &NativeRunState) -> ToolOutput {
    let headline = match state.status.as_str() {
        "starting" | "running" => {
            format!("Run {} is still running for {}.", state.run_id, state.scope)
        }
        "failed" => format!("Run {} failed for {}.", state.run_id, state.scope),
        _ if state.summary.total_failed > 0 => format!(
            "Run {} finished with {} failed unit(s).",
            state.run_id, state.summary.total_failed
        ),
        _ if state.summary.total_awaiting_verify > 0 => format!(
            "Run {} finished with {} unit(s) awaiting verify.",
            state.run_id, state.summary.total_awaiting_verify
        ),
        _ => format!(
            "Run {} finished successfully: {} unit(s) done.",
            state.run_id, state.summary.total_closed
        ),
    };

    let latest = state
        .log_lines
        .last()
        .map(|line| format!("Latest: {line}"))
        .unwrap_or_else(|| "Latest: (no stream events captured yet)".to_string());

    text_output(
        format!("{headline}\n{latest}"),
        serde_json::to_value(state).unwrap_or(serde_json::Value::Null),
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
            run_store: Arc::new(std::sync::Mutex::new(ManaRunStore::default())),
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
        "Work coordination substrate. Prefer this over bash for mana operations when an equivalent action exists: inspect units, create/update/claim/release work, inspect orchestration logs/agents, and run orchestration natively with in-session run state. Use for complex tasks or delegation. Load the `mana` skill when coordinating multi-step work or delegation to learn the workflow."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["status", "list", "show", "create", "close", "update", "run", "run_state", "evaluate", "claim", "release", "logs", "agents", "next", "tree"] },
                "id": { "type": "string" },
                "run_id": { "type": "string", "description": "Native in-session mana run ID, returned by action=run" },
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
                "background": { "type": "boolean", "description": "Run mana orchestration in the background and return immediately (default true unless dry_run=true)" },
                "jobs": { "type": "integer" },
                "dry_run": { "type": "boolean" },
                "loop": { "type": "boolean" },
                "keep_going": { "type": "boolean" },
                "timeout": { "type": "integer" },
                "idle_timeout": { "type": "integer" },
                "review": { "type": "boolean" }
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

                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| crate::error::Error::Tool("logs requires 'id' or 'run_id'".into()))?;
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
                let scope = run_args
                    .id
                    .as_deref()
                    .map(|id| format!("unit {id}"))
                    .unwrap_or_else(|| "all ready units".to_string());
                let run_id = {
                    let mut store = self.run_store.lock().map_err(|_| {
                        crate::error::Error::Tool("mana run state lock poisoned".into())
                    })?;
                    store.start_run(scope.clone(), background, &run_args)
                };

                if background {
                    let started = background_run_started_output(&scope, &run_id, &run_args);
                    spawn_background_run(
                        mana_dir.clone(),
                        run_args,
                        ctx,
                        self.run_store.clone(),
                        run_id,
                    );
                    return Ok(started);
                }

                send_update(
                    &ctx,
                    format!("Starting mana run {run_id}..."),
                    json!({"kind": "mana_run_status", "status": "starting", "run_id": run_id}),
                );
                ctx.ui
                    .set_widget(
                        "mana",
                        Some(mana_widget_lines(
                            format!("running mana ({run_id})"),
                            Some("native foreground orchestration".to_string()),
                        )),
                    )
                    .await;
                ctx.ui.set_status("mana", Some("mana: running")).await;

                let run_store = self.run_store.clone();
                let run_id_for_sink = run_id.clone();
                let update_tx = ctx.update_tx.clone();
                match mana::commands::run::run_with_stream_capture_and_sink(
                    &mana_dir,
                    run_args,
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
                                json!({"kind": "mana_run_view", "run_id": run_id, "view": view}),
                            );
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
                            details: json!({
                                "run_id": run_id,
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
                "Unknown action: {other}. Use: status, list, show, create, close, update, run, run_state, evaluate, claim, release, logs, agents, next, tree"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::mpsc;

    use super::{evaluate_run_output, stream_event_line, ManaRunStore, ManaTool, NativeRunState};
    use crate::tools::{FileCache, FileTracker, Tool, ToolContext, ToolUpdate};
    use crate::ui::NullInterface;

    enum ManaResult {
        ModeBlocked(String),
        Attempted(crate::tools::ToolOutput),
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        assert!(text.contains("Started mana run in background"));
        assert_eq!(result.details["background"], true);
        assert!(result.details["run_id"].as_str().is_some());
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
                json!({ "action": "run_state", "run_id": run_id }),
                ctx2,
            )
            .await
            .unwrap();
        assert!(state.text_content().unwrap_or("").contains("Mana run run-"));

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
        assert!(eval_text.contains("Run run-") && eval_text.contains("finished"));
    }

    #[test]
    fn run_store_prefers_active_run_snapshot() {
        let mut store = ManaRunStore::default();
        let active_id = store.start_run(
            "all ready units".to_string(),
            true,
            &mana::commands::run::RunArgs {
                id: None,
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
            &mana::commands::run::RunArgs {
                id: Some("1".to_string()),
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
            &mana::commands::run::RunArgs {
                id: Some("7".to_string()),
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
    }
}
