use imp_core::personality::{
    PersonaFocus, PersonaRole, PersonalityBand, PersonalityConfig, PersonalityIdentity,
    PersonalityProfile, VoiceWord, WorkStyleWord,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalityScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalityField {
    Name,
    WorkStyle,
    Voice,
    Focus,
    Role,
    Autonomy,
    Verbosity,
    Caution,
    Warmth,
    PlanningDepth,
    Profile,
    Scope,
    Save,
    DeleteProfile,
}

const FIELDS: &[PersonalityField] = &[
    PersonalityField::Name,
    PersonalityField::WorkStyle,
    PersonalityField::Voice,
    PersonalityField::Focus,
    PersonalityField::Role,
    PersonalityField::Autonomy,
    PersonalityField::Verbosity,
    PersonalityField::Caution,
    PersonalityField::Warmth,
    PersonalityField::PlanningDepth,
    PersonalityField::Profile,
    PersonalityField::Scope,
    PersonalityField::Save,
    PersonalityField::DeleteProfile,
];

#[derive(Debug, Clone)]
pub struct PersonalityState {
    pub selected: usize,
    pub editing_name: bool,
    pub identity: PersonalityIdentity,
    pub sliders: imp_core::personality::PersonalitySliders,
    pub scope: PersonalityScope,
    pub profile_name: String,
    pub saved_profiles: Vec<String>,
    pub active_profile: Option<String>,
    pub dirty: bool,
}

impl PersonalityState {
    pub fn new(
        global: &PersonalityConfig,
        project: Option<&PersonalityConfig>,
        scope: PersonalityScope,
    ) -> Self {
        let config = match scope {
            PersonalityScope::Global => global,
            PersonalityScope::Project => project.unwrap_or(global),
        };
        let profile = config.effective_profile();
        Self {
            selected: 0,
            editing_name: false,
            identity: profile.identity.clone(),
            sliders: profile.sliders.clone(),
            scope,
            profile_name: config
                .profiles
                .active
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            saved_profiles: config.profiles.profile_names(),
            active_profile: config.profiles.active.clone(),
            dirty: false,
        }
    }

    pub fn current_field(&self) -> PersonalityField {
        FIELDS[self.selected]
    }

    pub fn move_up(&mut self) {
        self.editing_name = false;
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        self.editing_name = false;
        if self.selected + 1 < FIELDS.len() {
            self.selected += 1;
        }
    }

    pub fn cycle_forward(&mut self) {
        self.dirty = true;
        match self.current_field() {
            PersonalityField::Name => {
                self.editing_name = true;
            }
            PersonalityField::WorkStyle => {
                self.identity.work_style = next_work_style(self.identity.work_style.clone());
            }
            PersonalityField::Voice => {
                self.identity.voice = next_voice(self.identity.voice.clone());
            }
            PersonalityField::Focus => {
                self.identity.focus = next_focus(self.identity.focus.clone());
            }
            PersonalityField::Role => {
                self.identity.role = next_role(self.identity.role.clone());
            }
            PersonalityField::Autonomy => self.sliders.autonomy = next_band(self.sliders.autonomy),
            PersonalityField::Verbosity => {
                self.sliders.verbosity = next_band(self.sliders.verbosity)
            }
            PersonalityField::Caution => self.sliders.caution = next_band(self.sliders.caution),
            PersonalityField::Warmth => self.sliders.warmth = next_band(self.sliders.warmth),
            PersonalityField::PlanningDepth => {
                self.sliders.planning_depth = next_band(self.sliders.planning_depth)
            }
            PersonalityField::Profile => {
                self.profile_name = next_profile_name(&self.profile_name, &self.saved_profiles);
                self.active_profile = if self.profile_name == "default" {
                    None
                } else {
                    Some(self.profile_name.clone())
                };
            }
            PersonalityField::Scope => {
                self.scope = match self.scope {
                    PersonalityScope::Global => PersonalityScope::Project,
                    PersonalityScope::Project => PersonalityScope::Global,
                }
            }
            PersonalityField::Save => {}
            PersonalityField::DeleteProfile => {}
        }
    }

    pub fn cycle_backward(&mut self) {
        self.dirty = true;
        match self.current_field() {
            PersonalityField::Name => {
                self.editing_name = true;
            }
            PersonalityField::WorkStyle => {
                self.identity.work_style = prev_work_style(self.identity.work_style.clone());
            }
            PersonalityField::Voice => {
                self.identity.voice = prev_voice(self.identity.voice.clone());
            }
            PersonalityField::Focus => {
                self.identity.focus = prev_focus(self.identity.focus.clone());
            }
            PersonalityField::Role => {
                self.identity.role = prev_role(self.identity.role.clone());
            }
            PersonalityField::Autonomy => self.sliders.autonomy = prev_band(self.sliders.autonomy),
            PersonalityField::Verbosity => {
                self.sliders.verbosity = prev_band(self.sliders.verbosity)
            }
            PersonalityField::Caution => self.sliders.caution = prev_band(self.sliders.caution),
            PersonalityField::Warmth => self.sliders.warmth = prev_band(self.sliders.warmth),
            PersonalityField::PlanningDepth => {
                self.sliders.planning_depth = prev_band(self.sliders.planning_depth)
            }
            PersonalityField::Profile => {
                self.profile_name = prev_profile_name(&self.profile_name, &self.saved_profiles);
                self.active_profile = if self.profile_name == "default" {
                    None
                } else {
                    Some(self.profile_name.clone())
                };
            }
            PersonalityField::Scope => {
                self.scope = match self.scope {
                    PersonalityScope::Global => PersonalityScope::Project,
                    PersonalityScope::Project => PersonalityScope::Global,
                }
            }
            PersonalityField::Save => {}
            PersonalityField::DeleteProfile => {}
        }
    }

    pub fn push_char(&mut self, c: char) {
        if self.current_field() == PersonalityField::Name || self.editing_name {
            if !c.is_control() {
                self.identity.name.push(c);
                self.editing_name = true;
                self.dirty = true;
            }
        }
    }

    pub fn pop_char(&mut self) {
        if self.current_field() == PersonalityField::Name || self.editing_name {
            self.identity.name.pop();
            self.editing_name = true;
            self.dirty = true;
        }
    }

    pub fn save_to_config(&self, config: &mut PersonalityConfig) {
        let profile = self.profile();
        if self.profile_name == "default" {
            config.profile = profile;
            config.profiles.clear_active();
        } else {
            let saved_name = config
                .profiles
                .save_profile(self.profile_name.clone(), profile.clone());
            config.profile = profile;
            config.profiles.set_active(saved_name);
        }
    }

    pub fn delete_active_profile(&mut self, config: &mut PersonalityConfig) -> bool {
        let Some(active) = self.active_profile.clone() else {
            return false;
        };
        let deleted = config.profiles.delete_profile(&active);
        if deleted {
            self.saved_profiles = config.profiles.profile_names();
            self.active_profile = None;
            self.profile_name = "default".into();
            self.identity = config.profile.identity.clone();
            self.sliders = config.profile.sliders.clone();
            self.dirty = true;
        }
        deleted
    }

    pub fn profile(&self) -> PersonalityProfile {
        PersonalityProfile {
            identity: self.identity.clone(),
            sliders: self.sliders.clone(),
        }
    }

    pub fn set_profile_name(&mut self, name: impl Into<String>) {
        self.profile_name = name.into();
        self.active_profile = if self.profile_name == "default" {
            None
        } else {
            Some(self.profile_name.clone())
        };
        self.dirty = true;
    }

    pub fn sentence(&self) -> String {
        self.identity.render_sentence()
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
        if area.height < 12 || area.width < 40 {
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

        let sentence = Paragraph::new(self.state.sentence())
            .style(self.theme.style())
            .block(
                Block::default()
                    .title(" Identity sentence ")
                    .borders(Borders::ALL),
            );
        sentence.render(rows[0], buf);

        let scope = match self.state.scope {
            PersonalityScope::Global => "global",
            PersonalityScope::Project => "project",
        };
        let status = Paragraph::new(format!(
            "Scope: {}{}{}",
            scope,
            match &self.state.active_profile {
                Some(name) => format!("  • profile: {}", name),
                None => "  • profile: default".to_string(),
            },
            if self.state.dirty {
                "  • unsaved"
            } else {
                ""
            }
        ))
        .style(self.theme.muted_style());
        status.render(rows[1], buf);

        let mut lines = Vec::new();
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Name,
            "name",
            &self.state.identity.name,
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::WorkStyle,
            "work style",
            self.state.identity.work_style.as_str(),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Voice,
            "voice",
            self.state.identity.voice.as_str(),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Focus,
            "focus",
            self.state.identity.focus.as_str(),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Role,
            "role",
            self.state.identity.role.as_str(),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Autonomy,
            "autonomy",
            band_label(self.state.sliders.autonomy),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Verbosity,
            "verbosity",
            band_label(self.state.sliders.verbosity),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Caution,
            "caution",
            band_label(self.state.sliders.caution),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Warmth,
            "warmth",
            band_label(self.state.sliders.warmth),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::PlanningDepth,
            "planning",
            band_label(self.state.sliders.planning_depth),
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Profile,
            "profile",
            &self.state.profile_name,
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Scope,
            "scope",
            scope,
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::Save,
            "save",
            "apply changes",
        );
        push_field_line(
            &mut lines,
            self.state,
            PersonalityField::DeleteProfile,
            "delete",
            "delete active profile",
        );

        Paragraph::new(lines)
            .block(Block::default().title(" Fields ").borders(Borders::ALL))
            .render(rows[2], buf);

        Paragraph::new(
            "↑/↓ move  ←/→ change  type to edit name/profile  Enter apply/delete  Esc close",
        )
        .style(self.theme.muted_style())
        .render(rows[3], buf);
    }
}

fn push_field_line(
    lines: &mut Vec<Line<'static>>,
    state: &PersonalityState,
    field: PersonalityField,
    label: &str,
    value: &str,
) {
    let selected = state.current_field() == field;
    let indicator = if selected { "▸" } else { " " };
    let style = if selected {
        state_line_style_selected()
    } else {
        Default::default()
    };
    lines.push(Line::from(vec![
        Span::styled(format!("{} ", indicator), style),
        Span::styled(format!("{label:<12}"), style.add_modifier(Modifier::BOLD)),
        Span::styled(value.to_string(), style),
    ]));
}

fn state_line_style_selected() -> ratatui::style::Style {
    ratatui::style::Style::default().add_modifier(Modifier::REVERSED)
}

fn band_label(band: PersonalityBand) -> &'static str {
    band.ui_label()
}

fn next_band(band: PersonalityBand) -> PersonalityBand {
    match band {
        PersonalityBand::VeryLow => PersonalityBand::Low,
        PersonalityBand::Low => PersonalityBand::Medium,
        PersonalityBand::Medium => PersonalityBand::High,
        PersonalityBand::High => PersonalityBand::VeryHigh,
        PersonalityBand::VeryHigh => PersonalityBand::VeryLow,
    }
}

fn prev_band(band: PersonalityBand) -> PersonalityBand {
    match band {
        PersonalityBand::VeryLow => PersonalityBand::VeryHigh,
        PersonalityBand::Low => PersonalityBand::VeryLow,
        PersonalityBand::Medium => PersonalityBand::Low,
        PersonalityBand::High => PersonalityBand::Medium,
        PersonalityBand::VeryHigh => PersonalityBand::High,
    }
}

fn next_profile_name(current: &str, saved_profiles: &[String]) -> String {
    let mut names = vec!["default".to_string()];
    names.extend(saved_profiles.iter().cloned());
    let idx = names.iter().position(|name| name == current).unwrap_or(0);
    names[(idx + 1) % names.len()].clone()
}

fn prev_profile_name(current: &str, saved_profiles: &[String]) -> String {
    let mut names = vec!["default".to_string()];
    names.extend(saved_profiles.iter().cloned());
    let idx = names.iter().position(|name| name == current).unwrap_or(0);
    names[(idx + names.len() - 1) % names.len()].clone()
}

macro_rules! cycle_enum {
    ($name:ident, $ty:ty, [$($variant:expr),+ $(,)?]) => {
        fn $name(value: $ty) -> $ty {
            let values = [$($variant),+];
            let idx = values.iter().position(|v| *v == value).unwrap_or(0);
            values[(idx + 1) % values.len()].clone()
        }
    };
}

macro_rules! cycle_enum_prev {
    ($name:ident, $ty:ty, [$($variant:expr),+ $(,)?]) => {
        fn $name(value: $ty) -> $ty {
            let values = [$($variant),+];
            let idx = values.iter().position(|v| *v == value).unwrap_or(0);
            values[(idx + values.len() - 1) % values.len()].clone()
        }
    };
}

cycle_enum!(
    next_work_style,
    WorkStyleWord,
    [
        WorkStyleWord::Practical,
        WorkStyleWord::Careful,
        WorkStyleWord::Disciplined,
        WorkStyleWord::Methodical,
        WorkStyleWord::Focused,
        WorkStyleWord::Thorough,
        WorkStyleWord::Precise,
        WorkStyleWord::Deliberate,
        WorkStyleWord::Skeptical,
        WorkStyleWord::Patient,
    ]
);
cycle_enum_prev!(
    prev_work_style,
    WorkStyleWord,
    [
        WorkStyleWord::Practical,
        WorkStyleWord::Careful,
        WorkStyleWord::Disciplined,
        WorkStyleWord::Methodical,
        WorkStyleWord::Focused,
        WorkStyleWord::Thorough,
        WorkStyleWord::Precise,
        WorkStyleWord::Deliberate,
        WorkStyleWord::Skeptical,
        WorkStyleWord::Patient,
    ]
);
cycle_enum!(
    next_voice,
    VoiceWord,
    [
        VoiceWord::Concise,
        VoiceWord::Clear,
        VoiceWord::Direct,
        VoiceWord::Calm,
        VoiceWord::Thoughtful,
        VoiceWord::Collaborative,
        VoiceWord::Structured,
        VoiceWord::Friendly,
        VoiceWord::Terse,
        VoiceWord::Warm,
    ]
);
cycle_enum_prev!(
    prev_voice,
    VoiceWord,
    [
        VoiceWord::Concise,
        VoiceWord::Clear,
        VoiceWord::Direct,
        VoiceWord::Calm,
        VoiceWord::Thoughtful,
        VoiceWord::Collaborative,
        VoiceWord::Structured,
        VoiceWord::Friendly,
        VoiceWord::Terse,
        VoiceWord::Warm,
    ]
);
cycle_enum!(
    next_focus,
    PersonaFocus,
    [
        PersonaFocus::Coding,
        PersonaFocus::Engineering,
        PersonaFocus::Software,
        PersonaFocus::Debugging,
        PersonaFocus::Research,
        PersonaFocus::Writing,
        PersonaFocus::Planning,
        PersonaFocus::Operations,
        PersonaFocus::Analysis,
        PersonaFocus::General,
    ]
);
cycle_enum_prev!(
    prev_focus,
    PersonaFocus,
    [
        PersonaFocus::Coding,
        PersonaFocus::Engineering,
        PersonaFocus::Software,
        PersonaFocus::Debugging,
        PersonaFocus::Research,
        PersonaFocus::Writing,
        PersonaFocus::Planning,
        PersonaFocus::Operations,
        PersonaFocus::Analysis,
        PersonaFocus::General,
    ]
);
cycle_enum!(
    next_role,
    PersonaRole,
    [
        PersonaRole::Agent,
        PersonaRole::Assistant,
        PersonaRole::Worker,
        PersonaRole::Collaborator,
        PersonaRole::Partner,
        PersonaRole::Reviewer,
        PersonaRole::Planner,
    ]
);
cycle_enum_prev!(
    prev_role,
    PersonaRole,
    [
        PersonaRole::Agent,
        PersonaRole::Assistant,
        PersonaRole::Worker,
        PersonaRole::Collaborator,
        PersonaRole::Partner,
        PersonaRole::Reviewer,
        PersonaRole::Planner,
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personality_state_sentence_updates_with_identity_changes() {
        let global = PersonalityConfig::default();
        let state = PersonalityState::new(&global, None, PersonalityScope::Global);
        assert_eq!(
            state.sentence(),
            "You are imp, a practical, concise, coding agent."
        );
    }

    #[test]
    fn personality_state_can_save_named_profile_to_config() {
        let global = PersonalityConfig::default();
        let mut state = PersonalityState::new(&global, None, PersonalityScope::Global);
        state.set_profile_name("Builder");
        state.identity.name = "Patch".into();

        let mut config = PersonalityConfig::default();
        state.save_to_config(&mut config);

        assert_eq!(config.profiles.active.as_deref(), Some("Builder"));
        assert!(config.profiles.saved.contains_key("Builder"));
        assert_eq!(config.profiles.saved["Builder"].identity.name, "Patch");
    }
}
