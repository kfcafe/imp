use eframe::egui;
use imp_core::runtime::{
    RuntimeArtifactRef, RuntimeEvent, RuntimeEventKind, RuntimePhase, RuntimeStateAccumulator,
    RuntimeStateSnapshot, RuntimeToolStatus,
};

const ACCENT: egui::Color32 = egui::Color32::from_rgb(139, 92, 246);
const PANEL: egui::Color32 = egui::Color32::from_rgb(17, 21, 27);
const PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(13, 17, 23);
const MUTED: egui::Color32 = egui::Color32::from_rgb(139, 152, 168);
const GOOD: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
const WARN: egui::Color32 = egui::Color32::from_rgb(245, 158, 11);
const BLUE: egui::Color32 = egui::Color32::from_rgb(56, 189, 248);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiRuntimeViewModel {
    pub run_label: String,
    pub phase: RuntimePhase,
    pub phase_label: String,
    pub model: Option<String>,
    pub worktree_label: Option<String>,
    pub evidence_paths: Vec<String>,
    pub active_tools: Vec<String>,
    pub completed_tools: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub status_lines: Vec<String>,
}

impl GuiRuntimeViewModel {
    pub fn from_snapshot(snapshot: &RuntimeStateSnapshot) -> Self {
        let run_label = snapshot
            .workflow
            .run_id
            .clone()
            .unwrap_or_else(|| "run: unsaved".into());
        let worktree_label = snapshot.workspace.worktree.as_ref().map(|worktree| {
            format!(
                "{} @ {}",
                worktree.metadata.branch,
                worktree.metadata.worktree_path.display()
            )
        });
        let evidence_paths = snapshot
            .evidence_refs
            .iter()
            .map(artifact_display_label)
            .collect();
        let active_tools = snapshot
            .active_tools
            .iter()
            .map(|tool| format!("{} · {}", tool.name, tool.id))
            .collect();
        let completed_tools = snapshot
            .completed_tools
            .iter()
            .map(|tool| format!("{} · {}", tool.name, tool_status_label(tool.status)))
            .collect();
        let mut status_lines = snapshot
            .status_items
            .iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>();
        status_lines.sort();

        Self {
            run_label,
            phase: snapshot.phase,
            phase_label: phase_label(snapshot.phase).into(),
            model: snapshot.workflow.model.clone(),
            worktree_label,
            evidence_paths,
            active_tools,
            completed_tools,
            warnings: snapshot.warnings.clone(),
            errors: snapshot.errors.clone(),
            status_lines,
        }
    }
}

#[derive(Debug, Clone)]
struct WorkItem {
    id: String,
    title: String,
    status: String,
    summary: String,
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    title: String,
    detail: String,
    status: EventStatus,
}

#[derive(Debug, Clone, Copy)]
enum EventStatus {
    Done,
    Running,
    Pending,
}

pub struct ImpGuiApp {
    project_name: String,
    selected_work_index: usize,
    work_items: Vec<WorkItem>,
    timeline: Vec<TimelineEvent>,
    runtime_accumulator: RuntimeStateAccumulator,
    runtime_view: GuiRuntimeViewModel,
    terminal_output: String,
    diff_preview: String,
}

impl ImpGuiApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        configure_style(&creation_context.egui_ctx);
        let runtime_accumulator = demo_runtime_accumulator();
        let runtime_view = GuiRuntimeViewModel::from_snapshot(&runtime_accumulator.snapshot());
        Self {
            project_name: "imp".to_owned(),
            selected_work_index: 0,
            work_items: vec![
                WorkItem {
                    id: "83".into(),
                    title: "Cursor bounds hardening".into(),
                    status: "active".into(),
                    summary: "Verify panic-prone text-box state".into(),
                },
                WorkItem {
                    id: "272.1".into(),
                    title: "YouTube transcript".into(),
                    status: "claimed".into(),
                    summary: "Pure HTTP extraction".into(),
                },
                WorkItem {
                    id: "46.1".into(),
                    title: "Runtime safety gaps".into(),
                    status: "planning".into(),
                    summary: "Backlog reconciliation".into(),
                },
            ],
            timeline: vec![
                TimelineEvent {
                    title: "Runtime snapshot".into(),
                    detail: "GUI derives reusable run state from imp_core::runtime::RuntimeStateSnapshot.".into(),
                    status: EventStatus::Done,
                },
                TimelineEvent {
                    title: "Event reducer".into(),
                    detail: "RuntimeEvent streams reduce through RuntimeStateAccumulator before rendering.".into(),
                    status: EventStatus::Done,
                },
                TimelineEvent {
                    title: "Frontend state".into(),
                    detail: "GUI keeps layout/selection local; core owns semantic run facts.".into(),
                    status: EventStatus::Running,
                },
                TimelineEvent {
                    title: "Live agent bridge".into(),
                    detail: "Future work: subscribe to real runtime events instead of demo fixture data.".into(),
                    status: EventStatus::Pending,
                },
            ],
            runtime_accumulator,
            runtime_view,
            terminal_output: "$ cargo test -p imp-gui\nchecking runtime snapshot adapter...\n".to_owned(),
            diff_preview: "RuntimeStateSnapshot -> GuiRuntimeViewModel\n  phase\n  tools\n  worktree\n  evidence\n".to_owned(),
        }
    }
}

impl eframe::App for ImpGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.runtime_view =
            GuiRuntimeViewModel::from_snapshot(&self.runtime_accumulator.snapshot());
        self.render_top_bar(ctx);
        self.render_left_sidebar(ctx);
        self.render_right_inspector(ctx);
        self.render_bottom_panel(ctx);
        self.render_central_panel(ctx);
    }
}

impl ImpGuiApp {
    fn render_top_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar")
            .exact_height(44.0)
            .frame(panel_frame(egui::Color32::from_rgb(9, 11, 15)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.colored_label(ACCENT, "●");
                    ui.strong("imp workbench");
                    pill(ui, &format!("project: {}", self.project_name), BLUE);
                    pill(
                        ui,
                        &self.runtime_view.phase_label,
                        phase_color(self.runtime_view.phase),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.button("New task");
                        let _ = ui.button("Approve tools");
                        ui.label(egui::RichText::new(&self.runtime_view.run_label).color(MUTED));
                    });
                });
            });
    }

    fn render_left_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("work_sidebar")
            .exact_width(270.0)
            .frame(panel_frame(PANEL))
            .show(ctx, |ui| {
                section_heading(ui, "Next work");
                for (index, item) in self.work_items.iter().enumerate() {
                    let selected = index == self.selected_work_index;
                    if work_item_button(ui, item, selected).clicked() {
                        self.selected_work_index = index;
                    }
                }
                section_heading(ui, "Runtime state");
                for line in self.runtime_view.status_lines.iter().take(5) {
                    nav_card(ui, "status", line);
                }
                if let Some(model) = &self.runtime_view.model {
                    nav_card(ui, "Model", model);
                }
            });
    }

    fn render_right_inspector(&self, ctx: &egui::Context) {
        let selected = &self.work_items[self.selected_work_index];
        egui::SidePanel::right("inspector")
            .exact_width(360.0)
            .frame(panel_frame(PANEL))
            .show(ctx, |ui| {
                section_heading(ui, "Selected task");
                card(ui, |ui| {
                    ui.strong(format!("{} · {}", selected.id, selected.title));
                    key_value(ui, "Status", &selected.status);
                    key_value(ui, "Assignee", "imp");
                    key_value(ui, "Risk", "medium: runtime path");
                    key_value(ui, "Verify", "cargo test -p imp-gui");
                });
                section_heading(ui, "Runtime snapshot");
                card(ui, |ui| {
                    key_value(ui, "Phase", &self.runtime_view.phase_label);
                    key_value(
                        ui,
                        "Active tools",
                        &self.runtime_view.active_tools.len().to_string(),
                    );
                    key_value(
                        ui,
                        "Completed",
                        &self.runtime_view.completed_tools.len().to_string(),
                    );
                    if let Some(worktree) = &self.runtime_view.worktree_label {
                        key_value(ui, "Worktree", worktree);
                    }
                });
                section_heading(ui, "Evidence");
                if self.runtime_view.evidence_paths.is_empty() {
                    nav_card(
                        ui,
                        "No evidence yet",
                        "Runtime snapshots will surface artifact refs here",
                    );
                } else {
                    for path in &self.runtime_view.evidence_paths {
                        nav_card(ui, "Artifact", path);
                    }
                }
            });
    }

    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(220.0)
            .frame(panel_frame(PANEL))
            .show(ctx, |ui| {
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        section_heading(ui, "Runtime adapter");
                        code_block(ui, &mut self.diff_preview);
                    });
                    columns[1].vertical(|ui| {
                        section_heading(ui, "Verify output");
                        code_block(ui, &mut self.terminal_output);
                    });
                });
            });
    }

    fn render_central_panel(&self, ctx: &egui::Context) {
        let selected = &self.work_items[self.selected_work_index];
        egui::CentralPanel::default()
            .frame(panel_frame(PANEL))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(&selected.title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        pill(ui, "snapshot-backed", GOOD);
                        pill(ui, "scope locked", MUTED);
                    });
                });
                ui.horizontal(|ui| {
                    for tab in ["Narrative", "Runtime", "Tools", "Review"] {
                        let _ = ui.selectable_label(tab == "Runtime", tab);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for event in &self.timeline {
                        timeline_card(ui, event);
                    }
                    card(ui, |ui| {
                        ui.strong("Runtime summary");
                        ui.label(format!(
                            "{} · {} active tools · {} completed tools",
                            self.runtime_view.phase_label,
                            self.runtime_view.active_tools.len(),
                            self.runtime_view.completed_tools.len()
                        ));
                        for warning in &self.runtime_view.warnings {
                            ui.colored_label(WARN, warning);
                        }
                        for error in &self.runtime_view.errors {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                    });
                });
            });
    }
}

fn demo_runtime_accumulator() -> RuntimeStateAccumulator {
    let mut accumulator = RuntimeStateAccumulator::new("demo-runtime");
    accumulator.apply(&RuntimeEvent {
        run_id: "demo-runtime".into(),
        sequence: 1,
        kind: RuntimeEventKind::AgentStarted {
            model: "openrouter/demo".into(),
        },
        ..RuntimeEvent::default()
    });
    accumulator.apply(&RuntimeEvent {
        run_id: "demo-runtime".into(),
        sequence: 2,
        kind: RuntimeEventKind::ToolStarted {
            tool_call: imp_core::runtime::RuntimeToolCall {
                id: "tool-1".into(),
                name: "bash".into(),
                status: RuntimeToolStatus::Running,
                args_preview: Some("cargo test -p imp-gui".into()),
                ..imp_core::runtime::RuntimeToolCall::default()
            },
        },
        ..RuntimeEvent::default()
    });
    accumulator.apply(&RuntimeEvent {
        run_id: "demo-runtime".into(),
        sequence: 3,
        kind: RuntimeEventKind::EvidenceUpdated {
            artifact: RuntimeArtifactRef {
                kind: "runtime-doc".into(),
                path: "docs/runtime-event-state-api.md".into(),
                summary: Some("Runtime API docs".into()),
            },
        },
        ..RuntimeEvent::default()
    });
    accumulator
}

fn artifact_display_label(artifact: &RuntimeArtifactRef) -> String {
    match &artifact.summary {
        Some(summary) => format!("{} · {}", artifact.path.display(), summary),
        None => artifact.path.display().to_string(),
    }
}

fn phase_label(phase: RuntimePhase) -> &'static str {
    match phase {
        RuntimePhase::Idle => "idle",
        RuntimePhase::Starting => "starting",
        RuntimePhase::Running => "running",
        RuntimePhase::WaitingForTool => "waiting for tool",
        RuntimePhase::WaitingForApproval => "waiting for approval",
        RuntimePhase::Verifying => "verifying",
        RuntimePhase::Completed => "completed",
        RuntimePhase::Failed => "failed",
        RuntimePhase::Blocked => "blocked",
    }
}

fn phase_color(phase: RuntimePhase) -> egui::Color32 {
    match phase {
        RuntimePhase::Completed => GOOD,
        RuntimePhase::Failed | RuntimePhase::Blocked => WARN,
        RuntimePhase::Running
        | RuntimePhase::WaitingForTool
        | RuntimePhase::WaitingForApproval
        | RuntimePhase::Verifying => BLUE,
        RuntimePhase::Idle | RuntimePhase::Starting => MUTED,
    }
}

fn tool_status_label(status: RuntimeToolStatus) -> &'static str {
    match status {
        RuntimeToolStatus::Pending => "pending",
        RuntimeToolStatus::Running => "running",
        RuntimeToolStatus::Succeeded => "succeeded",
        RuntimeToolStatus::Failed => "failed",
        RuntimeToolStatus::Cancelled => "cancelled",
    }
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(7, 9, 13);
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(24, 18, 36);
    ctx.set_style(style);
}

fn panel_frame(fill: egui::Color32) -> egui::Frame {
    egui::Frame::default()
        .fill(fill)
        .inner_margin(egui::Margin::same(10))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(39, 49, 61)))
}

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .small()
            .color(MUTED),
    );
    ui.add_space(4.0);
}

fn pill(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::default()
        .stroke(egui::Stroke::new(1.0, color.gamma_multiply(0.6)))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(color));
        });
}

fn work_item_button(ui: &mut egui::Ui, item: &WorkItem, selected: bool) -> egui::Response {
    let fill = if selected {
        egui::Color32::from_rgb(24, 18, 36)
    } else {
        PANEL_DARK
    };

    egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0,
            if selected {
                ACCENT
            } else {
                egui::Color32::TRANSPARENT
            },
        ))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(9))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(format!("{} {}", item.id, item.title));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&item.status).small().color(WARN));
                });
            });
            ui.label(egui::RichText::new(&item.summary).color(MUTED));
        })
        .response
        .interact(egui::Sense::click())
}

fn nav_card(ui: &mut egui::Ui, title: &str, detail: &str) {
    card(ui, |ui| {
        ui.strong(title);
        ui.label(egui::RichText::new(detail).color(MUTED));
    });
}

fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::default()
        .fill(PANEL_DARK)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(39, 49, 61)))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(11))
        .show(ui, add_contents);
    ui.add_space(8.0);
}

fn key_value(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [90.0, 18.0],
            egui::Label::new(egui::RichText::new(key).color(MUTED)),
        );
        ui.label(value);
    });
}

fn timeline_card(ui: &mut egui::Ui, event: &TimelineEvent) {
    card(ui, |ui| {
        ui.horizontal_top(|ui| {
            let (symbol, color) = match event.status {
                EventStatus::Done => ("●", GOOD),
                EventStatus::Running => ("●", BLUE),
                EventStatus::Pending => ("○", MUTED),
            };
            ui.colored_label(color, symbol);
            ui.vertical(|ui| {
                ui.strong(&event.title);
                ui.label(egui::RichText::new(&event.detail).color(MUTED));
            });
        });
    });
}

fn code_block(ui: &mut egui::Ui, text: &mut String) {
    egui::Frame::default()
        .fill(egui::Color32::from_rgb(7, 9, 13))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(8),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_core::runtime::{RuntimeToolCall, RuntimeWorktreeState};
    use imp_core::workflow::WorktreeRunMetadata;

    #[test]
    fn gui_view_model_projects_runtime_snapshot_without_tui_types() {
        let mut snapshot = RuntimeStateSnapshot::default();
        snapshot.workflow.run_id = Some("run-1".into());
        snapshot.workflow.model = Some("openrouter/test".into());
        snapshot.phase = RuntimePhase::WaitingForTool;
        snapshot.active_tools.push(RuntimeToolCall {
            id: "tool-1".into(),
            name: "bash".into(),
            status: RuntimeToolStatus::Running,
            ..RuntimeToolCall::default()
        });
        snapshot.workspace.worktree = Some(RuntimeWorktreeState {
            metadata: WorktreeRunMetadata {
                worktree_path: "/tmp/imp-worktree".into(),
                branch: "imp/run/test".into(),
                ..WorktreeRunMetadata::default()
            },
            ..RuntimeWorktreeState::default()
        });
        snapshot.evidence_refs.push(RuntimeArtifactRef {
            kind: "worktree-diff".into(),
            path: ".imp/runs/run-1/worktree/diff.patch".into(),
            summary: Some("patch".into()),
        });
        snapshot
            .status_items
            .insert("phase".into(), "waiting".into());

        let view = GuiRuntimeViewModel::from_snapshot(&snapshot);
        assert_eq!(view.run_label, "run-1");
        assert_eq!(view.phase, RuntimePhase::WaitingForTool);
        assert_eq!(view.model.as_deref(), Some("openrouter/test"));
        assert_eq!(view.active_tools, vec!["bash · tool-1".to_string()]);
        assert_eq!(
            view.worktree_label.as_deref(),
            Some("imp/run/test @ /tmp/imp-worktree")
        );
        assert_eq!(view.evidence_paths.len(), 1);
        assert!(view
            .status_lines
            .iter()
            .any(|line| line == "phase: waiting"));
    }

    #[test]
    fn gui_demo_runtime_uses_core_accumulator() {
        let snapshot = demo_runtime_accumulator().snapshot();
        let view = GuiRuntimeViewModel::from_snapshot(&snapshot);
        assert_eq!(view.run_label, "demo-runtime");
        assert_eq!(view.phase, RuntimePhase::WaitingForTool);
        assert_eq!(view.active_tools, vec!["bash · tool-1".to_string()]);
        assert_eq!(view.evidence_paths.len(), 1);
    }
}
