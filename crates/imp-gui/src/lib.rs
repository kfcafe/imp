use eframe::egui;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(139, 92, 246);
const PANEL: egui::Color32 = egui::Color32::from_rgb(17, 21, 27);
const PANEL_DARK: egui::Color32 = egui::Color32::from_rgb(13, 17, 23);
const MUTED: egui::Color32 = egui::Color32::from_rgb(139, 152, 168);
const GOOD: egui::Color32 = egui::Color32::from_rgb(34, 197, 94);
const WARN: egui::Color32 = egui::Color32::from_rgb(245, 158, 11);
const BLUE: egui::Color32 = egui::Color32::from_rgb(56, 189, 248);

#[derive(Debug, Clone)]
struct WorkItem {
    id: &'static str,
    title: &'static str,
    status: &'static str,
    summary: &'static str,
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    title: &'static str,
    detail: &'static str,
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
    terminal_output: String,
    diff_preview: String,
}

impl ImpGuiApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        configure_style(&creation_context.egui_ctx);

        Self {
            project_name: "imp".to_owned(),
            selected_work_index: 0,
            work_items: vec![
                WorkItem {
                    id: "83",
                    title: "Cursor bounds hardening",
                    status: "active",
                    summary: "Verify panic-prone text-box state",
                },
                WorkItem {
                    id: "272.1",
                    title: "YouTube transcript",
                    status: "claimed",
                    summary: "Pure HTTP extraction",
                },
                WorkItem {
                    id: "46.1",
                    title: "Runtime safety gaps",
                    status: "planning",
                    summary: "Backlog reconciliation",
                },
            ],
            timeline: vec![
                TimelineEvent {
                    title: "Scope lock",
                    detail: "Modify the selected unit only; keep runtime integration explicit and reviewable.",
                    status: EventStatus::Done,
                },
                TimelineEvent {
                    title: "Inspect project context",
                    detail: "Load AGENTS.md, mana unit details, relevant code, and verification command before editing.",
                    status: EventStatus::Done,
                },
                TimelineEvent {
                    title: "Run focused verify",
                    detail: "Execute the narrowest useful command and stream output into the workbench.",
                    status: EventStatus::Running,
                },
                TimelineEvent {
                    title: "Summarize outcome",
                    detail: "Return DONE, DONE_WITH_CONCERNS, BLOCKED, or NEEDS_CONTEXT with evidence.",
                    status: EventStatus::Pending,
                },
            ],
            terminal_output: "$ cargo check -p imp-gui\nchecking imp-gui...\n".to_owned(),
            diff_preview: "fn apply_edit(&mut self, edit: Edit) {\n-    self.cursor = next_cursor;\n+    self.cursor = self.normalize_cursor(next_cursor);\n}\n".to_owned(),
        }
    }
}

impl eframe::App for ImpGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let _ = ui.button("New task");
                        let _ = ui.button("Approve tools");
                        pill(ui, "verifying", GOOD);
                        ui.label(egui::RichText::new("Run #1842").color(MUTED));
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

                section_heading(ui, "Memory");
                nav_card(ui, "Decisions", "3 open · 18 resolved");
                nav_card(ui, "Facts", "verified project claims");
                nav_card(ui, "Rules", "AGENTS.md + local conventions");

                section_heading(ui, "Agents");
                nav_card(ui, "imp", "online · editing + verify");
                nav_card(ui, "reviewer", "idle · available for focused review");
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
                    key_value(ui, "Status", selected.status);
                    key_value(ui, "Assignee", "imp");
                    key_value(ui, "Risk", "medium: runtime path");
                    key_value(ui, "Verify", "cargo check -p imp-gui");
                });

                section_heading(ui, "Acceptance");
                card(ui, |ui| {
                    ui.label("The GUI shell compiles, opens independently, and preserves existing CLI/TUI behavior.");
                });

                section_heading(ui, "Files");
                nav_card(ui, "crates/imp-gui", "new egui app crate");
                nav_card(ui, "Cargo.toml", "workspace registration");

                section_heading(ui, "Promote");
                ui.horizontal_wrapped(|ui| {
                    let _ = ui.button("Create follow-up task");
                    let _ = ui.button("Record decision");
                });
            });
    }

    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(220.0)
            .frame(panel_frame(PANEL))
            .show(ctx, |ui| {
                ui.columns(2, |columns| {
                    columns[0].vertical(|ui| {
                        section_heading(ui, "Diff preview");
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
                    ui.heading(selected.title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        pill(ui, "tests passing", GOOD);
                        pill(ui, "scope locked", MUTED);
                    });
                });
                ui.horizontal(|ui| {
                    for tab in ["Narrative", "Plan", "Tools", "Review"] {
                        let _ = ui.selectable_label(tab == "Narrative", tab);
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for event in &self.timeline {
                        timeline_card(ui, event);
                    }
                    card(ui, |ui| {
                        ui.strong("Agent message draft");
                        ui.label("DONE — GUI shell is compiling. Next: wire the app to real mana units and agent events.");
                    });
                });
            });
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
                    ui.label(egui::RichText::new(item.status).small().color(WARN));
                });
            });
            ui.label(egui::RichText::new(item.summary).color(MUTED));
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
                ui.strong(event.title);
                ui.label(egui::RichText::new(event.detail).color(MUTED));
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
