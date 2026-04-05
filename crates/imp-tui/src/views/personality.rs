use std::path::PathBuf;

use imp_core::personality::{
    default_soul_markdown, generated_tunable_line, replace_tunable_line, soul_identity_text,
    tunable_state_for_label, SoulTunableState,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::theme::Theme;
use crate::views::editor::EditorState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalityScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalityTab {
    Builder,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalityField {
    Scope,
    Autonomy,
    Brevity,
    Caution,
    Warmth,
    Planning,
    Save,
}

const FIELDS: &[PersonalityField] = &[
    PersonalityField::Scope,
    PersonalityField::Autonomy,
    PersonalityField::Brevity,
    PersonalityField::Caution,
    PersonalityField::Warmth,
    PersonalityField::Planning,
    PersonalityField::Save,
];

#[derive(Debug, Clone)]
pub struct PendingOverwrite {
    pub label: &'static str,
    pub replacement_line: String,
    pub diff_preview: String,
}

#[derive(Debug, Clone)]
pub struct PersonalityState {
    pub selected: usize,
    pub scope: PersonalityScope,
    pub tab: PersonalityTab,
    pub editor: EditorState,
    pub dirty_global: bool,
    pub dirty_project: bool,
    pub pending_overwrite: Option<PendingOverwrite>,
    global_path: PathBuf,
    project_path: PathBuf,
    global_source: String,
    project_source: String,
}

impl PersonalityState {
    pub fn new(cwd: PathBuf, scope: PersonalityScope) -> Self {
        let global_path = imp_core::config::Config::user_config_dir().join("soul.md");
        let project_path = cwd.join(".imp").join("soul.md");
        let global_source = std::fs::read_to_string(&global_path).unwrap_or_else(|_| default_soul_markdown());
        let project_source = std::fs::read_to_string(&project_path).unwrap_or_else(|_| default_soul_markdown());
        let mut editor = EditorState::new();
        editor.set_content(match scope {
            PersonalityScope::Global => &global_source,
            PersonalityScope::Project => &project_source,
        });
        Self {
            selected: 0,
            scope,
            tab: PersonalityTab::Builder,
            editor,
            dirty_global: false,
            dirty_project: false,
            pending_overwrite: None,
            global_path,
            project_path,
            global_source,
            project_source,
        }
    }

    pub fn current_field(&self) -> PersonalityField {
        FIELDS[self.selected]
    }

    pub fn current_path(&self) -> &PathBuf {
        match self.scope {
            PersonalityScope::Global => &self.global_path,
            PersonalityScope::Project => &self.project_path,
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self.scope {
            PersonalityScope::Global => self.dirty_global,
            PersonalityScope::Project => self.dirty_project,
        }
    }

    fn set_dirty(&mut self, dirty: bool) {
        match self.scope {
            PersonalityScope::Global => self.dirty_global = dirty,
            PersonalityScope::Project => self.dirty_project = dirty,
        }
    }

    fn sync_editor_to_scope_store(&mut self) {
        match self.scope {
            PersonalityScope::Global => self.global_source = self.editor.content().to_string(),
            PersonalityScope::Project => self.project_source = self.editor.content().to_string(),
        }
    }

    fn load_scope_into_editor(&mut self) {
        let content = match self.scope {
            PersonalityScope::Global => self.global_source.as_str(),
            PersonalityScope::Project => self.project_source.as_str(),
        };
        self.editor.set_content(content);
    }

    pub fn sentence(&self) -> String {
        soul_identity_text(self.editor.content())
    }

    pub fn move_up(&mut self) {
        if self.tab == PersonalityTab::Builder && self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.tab == PersonalityTab::Builder && self.selected + 1 < FIELDS.len() {
            self.selected += 1;
        }
    }

    pub fn switch_tab(&mut self) {
        self.pending_overwrite = None;
        self.tab = match self.tab {
            PersonalityTab::Builder => PersonalityTab::Source,
            PersonalityTab::Source => PersonalityTab::Builder,
        };
    }

    pub fn toggle_scope(&mut self) {
        self.sync_editor_to_scope_store();
        self.scope = match self.scope {
            PersonalityScope::Global => PersonalityScope::Project,
            PersonalityScope::Project => PersonalityScope::Global,
        };
        self.load_scope_into_editor();
        self.pending_overwrite = None;
    }

    pub fn tunable_display(&self, label: &'static str) -> &'static str {
        match tunable_state_for_label(self.editor.content(), label) {
            SoulTunableState::Preset(0) => "very low",
            SoulTunableState::Preset(1) => "low",
            SoulTunableState::Preset(2) => "balanced",
            SoulTunableState::Preset(3) => "high",
            SoulTunableState::Preset(4) => "very high",
            SoulTunableState::Preset(_) => "preset",
            SoulTunableState::Edited => "edited",
            SoulTunableState::Missing => "missing",
        }
    }

    fn cycle_tunable(&mut self, label: &'static str, forward: bool) {
        let state = tunable_state_for_label(self.editor.content(), label);
        let next_idx = match state {
            SoulTunableState::Preset(idx) => {
                if forward { (idx + 1) % 5 } else { (idx + 4) % 5 }
            }
            SoulTunableState::Missing => {
                if forward { 0 } else { 4 }
            }
            SoulTunableState::Edited => {
                if forward { 0 } else { 4 }
            }
        };
        let Some(new_line) = generated_tunable_line(label, next_idx) else {
            return;
        };

        if matches!(state, SoulTunableState::Edited) {
            let current = imp_core::personality::parse_tunables_section(self.editor.content())
                .get(label)
                .cloned()
                .unwrap_or_default();
            self.pending_overwrite = Some(PendingOverwrite {
                label,
                replacement_line: new_line.clone(),
                diff_preview: format!("- {label}: {current}\n+ {new_line}"),
            });
            return;
        }

        let updated = replace_tunable_line(self.editor.content(), label, &new_line);
        self.editor.set_content(&updated);
        self.sync_editor_to_scope_store();
        self.set_dirty(true);
    }

    pub fn cycle_forward(&mut self) {
        if self.tab != PersonalityTab::Builder {
            return;
        }
        match self.current_field() {
            PersonalityField::Scope => self.toggle_scope(),
            PersonalityField::Autonomy => self.cycle_tunable("Autonomy", true),
            PersonalityField::Brevity => self.cycle_tunable("Brevity", true),
            PersonalityField::Caution => self.cycle_tunable("Caution", true),
            PersonalityField::Warmth => self.cycle_tunable("Warmth", true),
            PersonalityField::Planning => self.cycle_tunable("Planning", true),
            PersonalityField::Save => {}
        }
    }

    pub fn cycle_backward(&mut self) {
        if self.tab != PersonalityTab::Builder {
            return;
        }
        match self.current_field() {
            PersonalityField::Scope => self.toggle_scope(),
            PersonalityField::Autonomy => self.cycle_tunable("Autonomy", false),
            PersonalityField::Brevity => self.cycle_tunable("Brevity", false),
            PersonalityField::Caution => self.cycle_tunable("Caution", false),
            PersonalityField::Warmth => self.cycle_tunable("Warmth", false),
            PersonalityField::Planning => self.cycle_tunable("Planning", false),
            PersonalityField::Save => {}
        }
    }

    pub fn confirm_overwrite(&mut self) {
        let Some(pending) = self.pending_overwrite.take() else {
            return;
        };
        let updated = replace_tunable_line(self.editor.content(), pending.label, &pending.replacement_line);
        self.editor.set_content(&updated);
        self.sync_editor_to_scope_store();
        self.set_dirty(true);
    }

    pub fn cancel_overwrite(&mut self) {
        self.pending_overwrite = None;
    }

    pub fn save_success(&mut self) {
        self.sync_editor_to_scope_store();
        self.set_dirty(false);
    }

    pub fn insert_char(&mut self, c: char) {
        self.editor.insert_char(c);
        self.sync_editor_to_scope_store();
        self.set_dirty(true);
    }

    pub fn insert_newline(&mut self) {
        self.editor.insert_newline();
        self.sync_editor_to_scope_store();
        self.set_dirty(true);
    }

    pub fn pop_char(&mut self) {
        self.editor.delete_back();
        self.sync_editor_to_scope_store();
        self.set_dirty(true);
    }

    pub fn move_left(&mut self) {
        self.editor.move_left();
    }

    pub fn move_right(&mut self) {
        self.editor.move_right();
    }
}

pub struct PersonalityView<'a> {
    state: &'a PersonalityState,
    theme: &'a Theme,
}

impl<'a> PersonalityView<'a> {
    pub fn new(state: &'a PersonalityState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for PersonalityView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 12 || area.width < 50 {
            return;
        }

        Clear.render(area, buf);
        let block = Block::default()
            .title(" Personality ")
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(8),
                Constraint::Length(2),
            ])
            .split(inner);

        Paragraph::new(self.state.sentence())
            .style(self.theme.style())
            .block(Block::default().title(" Identity ").borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .render(rows[0], buf);

        let scope = match self.state.scope {
            PersonalityScope::Global => "global",
            PersonalityScope::Project => "project",
        };
        let tab = match self.state.tab {
            PersonalityTab::Builder => "builder",
            PersonalityTab::Source => "source",
        };
        Paragraph::new(format!(
            "Scope: {scope}  •  Tab: {tab}  •  Path: {}{}",
            self.state.current_path().display(),
            if self.state.is_dirty() { "  • unsaved" } else { "" }
        ))
        .style(self.theme.muted_style())
        .render(rows[1], buf);

        match self.state.tab {
            PersonalityTab::Builder => render_builder(rows[2], buf, self.state),
            PersonalityTab::Source => render_source(rows[2], buf, self.state),
        }

        let hints = if self.state.pending_overwrite.is_some() {
            "Enter/Y: confirm overwrite  Esc/N: cancel"
        } else {
            match self.state.tab {
                PersonalityTab::Builder => {
                    "Tab: source  ↑/↓ move  ←/→ change  Enter on save to write file  Ctrl-S save  Esc close"
                }
                PersonalityTab::Source => {
                    "Tab: builder  type to edit  arrows move  Enter newline  Backspace delete  Ctrl-S save  Esc close"
                }
            }
        };
        Paragraph::new(hints)
            .style(self.theme.muted_style())
            .render(rows[3], buf);

        if let Some(pending) = &self.state.pending_overwrite {
            let modal = centered_rect(70, 40, area);
            Clear.render(modal, buf);
            Paragraph::new(pending.diff_preview.clone())
                .block(Block::default().title(" Confirm overwrite ").borders(Borders::ALL))
                .wrap(Wrap { trim: false })
                .render(modal, buf);
        }
    }
}

fn render_builder(area: Rect, buf: &mut Buffer, state: &PersonalityState) {
    let mut lines = Vec::new();
    push_field_line(lines.as_mut(), state, PersonalityField::Scope, "scope", match state.scope {
        PersonalityScope::Global => "global",
        PersonalityScope::Project => "project",
    });
    push_field_line(lines.as_mut(), state, PersonalityField::Autonomy, "autonomy", state.tunable_display("Autonomy"));
    push_field_line(lines.as_mut(), state, PersonalityField::Brevity, "brevity", state.tunable_display("Brevity"));
    push_field_line(lines.as_mut(), state, PersonalityField::Caution, "caution", state.tunable_display("Caution"));
    push_field_line(lines.as_mut(), state, PersonalityField::Warmth, "warmth", state.tunable_display("Warmth"));
    push_field_line(lines.as_mut(), state, PersonalityField::Planning, "planning", state.tunable_display("Planning"));
    push_field_line(lines.as_mut(), state, PersonalityField::Save, "save", "write soul.md");

    Paragraph::new(lines)
        .block(Block::default().title(" Builder ").borders(Borders::ALL))
        .render(area, buf);
}

fn render_source(area: Rect, buf: &mut Buffer, state: &PersonalityState) {
    Paragraph::new(state.editor.content().to_string())
        .block(Block::default().title(" Source ").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn push_field_line(
    lines: &mut Vec<Line<'static>>,
    state: &PersonalityState,
    field: PersonalityField,
    label: &str,
    value: &str,
) {
    let selected = state.tab == PersonalityTab::Builder && state.current_field() == field;
    let indicator = if selected { "▸" } else { " " };
    let style = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::styled(format!("{} ", indicator), style),
        Span::styled(format!("{label:<12}"), style.add_modifier(Modifier::BOLD)),
        Span::styled(value.to_string(), style),
    ]));
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personality_state_defaults_to_generated_soul() {
        let tmp = tempfile::tempdir().unwrap();
        let state = PersonalityState::new(tmp.path().to_path_buf(), PersonalityScope::Global);
        assert!(state.sentence().contains("You are imp"));
        assert_eq!(state.tunable_display("Autonomy"), "high");
    }

    #[test]
    fn personality_state_marks_custom_lines_as_edited() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = PersonalityState::new(tmp.path().to_path_buf(), PersonalityScope::Global);
        state.editor.set_content("# Soul\n\nYou are imp.\n\n## Tunables\n\n- Autonomy: custom autonomy line\n");
        assert_eq!(state.tunable_display("Autonomy"), "edited");
    }
}
