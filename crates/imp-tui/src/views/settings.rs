use imp_core::config::{
    AnimationLevel, ChatToolDisplay, Config, ContextConfig, ContinuePolicy, ShellBackend,
    ShellConfig, SidebarStyle, ToolOutputDisplay,
};
use imp_core::tools::web::types::SearchProvider;
use imp_llm::auth::AuthStore;
use imp_llm::model::ModelMeta;
use imp_llm::ThinkingLevel;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// Which field in the settings panel is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Model,
    ChosenModels,
    Theme,
    ThinkingLevel,
    MaxTokens,
    MaxTurns,
    ObservationMask,
    ShellBackend,
    ReadMaxLines,
    SidebarWidth,
    WordWrap,
    Animations,
    AutoOpenSidebar,
    SidebarAutoOpenWidth,
    ThinkingLines,
    StreamingLines,
    MouseScrollLines,
    KeyboardScrollLines,
    ShowTimestamps,
    ShowCost,
    ShowContextUsage,
    NotifyOnAgentComplete,
    ContinuePolicy,
    WebSearchProvider,
    TavilyApiKey,
    ExaApiKey,
    Save,
}

const FIELDS: &[SettingsField] = &[
    SettingsField::Model,
    SettingsField::ChosenModels,
    SettingsField::Theme,
    SettingsField::ThinkingLevel,
    SettingsField::MaxTokens,
    SettingsField::MaxTurns,
    SettingsField::ObservationMask,
    SettingsField::ShellBackend,
    SettingsField::ReadMaxLines,
    SettingsField::SidebarWidth,
    SettingsField::WordWrap,
    SettingsField::Animations,
    SettingsField::AutoOpenSidebar,
    SettingsField::SidebarAutoOpenWidth,
    SettingsField::ThinkingLines,
    SettingsField::StreamingLines,
    SettingsField::MouseScrollLines,
    SettingsField::KeyboardScrollLines,
    SettingsField::ShowTimestamps,
    SettingsField::ShowCost,
    SettingsField::ShowContextUsage,
    SettingsField::NotifyOnAgentComplete,
    SettingsField::ContinuePolicy,
    SettingsField::WebSearchProvider,
    SettingsField::TavilyApiKey,
    SettingsField::ExaApiKey,
    SettingsField::Save,
];

/// State for the settings overlay.
#[derive(Debug, Clone)]
pub struct SettingsState {
    pub selected: usize,
    pub model: String,
    pub model_options: Vec<String>,
    pub chosen_models: Vec<String>,
    pub theme_name: String,
    pub theme_options: Vec<String>,
    pub thinking_level: ThinkingLevel,
    pub max_tokens: u32,
    pub max_turns: u32,
    pub observation_mask: f64,
    pub shell_backend: ShellBackend,
    pub sidebar_style: SidebarStyle,
    pub tool_output: ToolOutputDisplay,
    pub tool_output_lines: usize,
    pub read_max_lines: usize,
    pub sidebar_width: u16,
    pub word_wrap: bool,
    pub animations: AnimationLevel,
    pub chat_tool_display: ChatToolDisplay,
    pub auto_open_sidebar: bool,
    pub sidebar_auto_open_width: u16,
    pub thinking_lines: usize,
    pub streaming_lines: usize,
    pub mouse_scroll_lines: usize,
    pub keyboard_scroll_lines: usize,
    pub show_timestamps: bool,
    pub show_cost: bool,
    pub show_context_usage: bool,
    pub notify_on_agent_complete: bool,
    pub continue_policy: ContinuePolicy,
    pub web_search_provider: Option<SearchProvider>,
    pub tavily_api_key: String,
    pub exa_api_key: String,
    pub tavily_configured: bool,
    pub exa_configured: bool,
    pub editing_number: bool,
    pub edit_buffer: String,
    pub dirty: bool,
}

impl SettingsState {
    fn normalized_selected(&self) -> usize {
        self.selected.min(FIELDS.len().saturating_sub(1))
    }

    pub fn new(
        config: &Config,
        model_name: &str,
        models: &[ModelMeta],
        auth_store: &AuthStore,
    ) -> Self {
        Self {
            selected: 0,
            model: model_name.to_string(),
            model_options: models.iter().map(|m| m.id.clone()).collect(),
            chosen_models: config.enabled_models.clone().unwrap_or_default(),
            theme_name: config.theme.clone().unwrap_or_else(|| "default".into()),
            theme_options: vec!["default".into(), "light".into()],
            thinking_level: config.thinking.unwrap_or(ThinkingLevel::Medium),
            max_tokens: config.max_tokens.unwrap_or(4096),
            max_turns: config.max_turns.unwrap_or(100),
            observation_mask: config.context.observation_mask_threshold,
            shell_backend: config.shell.backend.clone(),
            sidebar_style: config.ui.sidebar_style,
            tool_output: config.ui.tool_output,
            tool_output_lines: config.ui.tool_output_lines,
            read_max_lines: config.ui.read_max_lines,
            sidebar_width: config.ui.sidebar_width,
            word_wrap: config.ui.word_wrap,
            animations: config.ui.animations,
            chat_tool_display: config.ui.effective_chat_tool_display(),
            auto_open_sidebar: config.ui.auto_open_sidebar,
            sidebar_auto_open_width: config.ui.sidebar_auto_open_width,
            thinking_lines: config.ui.thinking_lines,
            streaming_lines: config.ui.streaming_lines,
            mouse_scroll_lines: config.ui.mouse_scroll_lines,
            keyboard_scroll_lines: config.ui.keyboard_scroll_lines,
            show_timestamps: config.ui.show_timestamps,
            show_cost: config.ui.show_cost,
            show_context_usage: config.ui.show_context_usage,
            notify_on_agent_complete: config.ui.notify_on_agent_complete,
            continue_policy: config.ui.continue_policy,
            web_search_provider: config.web.search_provider,
            tavily_api_key: String::new(),
            exa_api_key: String::new(),
            tavily_configured: auth_store.stored.contains_key("tavily")
                || std::env::var("TAVILY_API_KEY").is_ok(),
            exa_configured: auth_store.stored.contains_key("exa")
                || std::env::var("EXA_API_KEY").is_ok(),
            editing_number: false,
            edit_buffer: String::new(),
            dirty: false,
        }
    }

    pub fn current_field(&self) -> SettingsField {
        FIELDS[self.normalized_selected()]
    }

    pub fn move_up(&mut self) {
        self.commit_edit();
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        self.commit_edit();
        if self.selected + 1 < FIELDS.len() {
            self.selected += 1;
        }
    }

    /// Cycle the current field's value forward.
    pub fn cycle_forward(&mut self) {
        self.dirty = true;
        match self.current_field() {
            SettingsField::Model => {
                if !self.model_options.is_empty() {
                    if let Some(idx) = self.model_options.iter().position(|m| *m == self.model) {
                        let next = (idx + 1) % self.model_options.len();
                        self.model = self.model_options[next].clone();
                    }
                }
            }
            SettingsField::ChosenModels => {
                self.toggle_current_model_in_chosen();
            }
            SettingsField::Theme => {
                if !self.theme_options.is_empty() {
                    if let Some(idx) = self
                        .theme_options
                        .iter()
                        .position(|t| *t == self.theme_name)
                    {
                        let next = (idx + 1) % self.theme_options.len();
                        self.theme_name = self.theme_options[next].clone();
                    }
                }
            }
            SettingsField::ThinkingLevel => {
                self.thinking_level = next_thinking(self.thinking_level);
            }
            SettingsField::MaxTokens => {
                self.max_tokens = self.max_tokens.saturating_add(256).min(128_000);
            }
            SettingsField::ShellBackend => {
                self.shell_backend = next_shell(&self.shell_backend);
            }
            SettingsField::MaxTurns => {
                self.max_turns = self.max_turns.saturating_add(10);
            }
            SettingsField::ObservationMask => {
                self.observation_mask = (self.observation_mask + 0.05).min(1.0);
            }
            SettingsField::ReadMaxLines => {
                self.read_max_lines = self.read_max_lines.saturating_add(100);
            }
            SettingsField::SidebarWidth => {
                self.sidebar_width = (self.sidebar_width + 5).min(80);
            }
            SettingsField::WordWrap => {
                self.word_wrap = !self.word_wrap;
            }
            SettingsField::Animations => {
                self.animations = match self.animations {
                    AnimationLevel::None => AnimationLevel::Spinner,
                    AnimationLevel::Spinner => AnimationLevel::Minimal,
                    AnimationLevel::Minimal => AnimationLevel::None,
                };
            }
            SettingsField::AutoOpenSidebar => {
                self.auto_open_sidebar = !self.auto_open_sidebar;
            }
            SettingsField::SidebarAutoOpenWidth => {
                self.sidebar_auto_open_width = (self.sidebar_auto_open_width + 10).min(240);
            }
            SettingsField::ThinkingLines => {
                self.thinking_lines = self.thinking_lines.saturating_add(1).min(20);
            }
            SettingsField::StreamingLines => {
                self.streaming_lines = self.streaming_lines.saturating_add(1).min(20);
            }
            SettingsField::MouseScrollLines => {
                self.mouse_scroll_lines = self.mouse_scroll_lines.saturating_add(1).min(20);
            }
            SettingsField::KeyboardScrollLines => {
                self.keyboard_scroll_lines = self.keyboard_scroll_lines.saturating_add(5).min(100);
            }
            SettingsField::ShowTimestamps => {
                self.show_timestamps = !self.show_timestamps;
            }
            SettingsField::ShowCost => {
                self.show_cost = !self.show_cost;
            }
            SettingsField::ShowContextUsage => {
                self.show_context_usage = !self.show_context_usage;
            }
            SettingsField::NotifyOnAgentComplete => {
                self.notify_on_agent_complete = !self.notify_on_agent_complete;
            }
            SettingsField::ContinuePolicy => {
                self.continue_policy = match self.continue_policy {
                    ContinuePolicy::Disabled => ContinuePolicy::Conservative,
                    ContinuePolicy::Conservative => ContinuePolicy::Balanced,
                    ContinuePolicy::Balanced => ContinuePolicy::Aggressive,
                    ContinuePolicy::Aggressive => ContinuePolicy::Disabled,
                };
            }
            SettingsField::WebSearchProvider => {
                self.web_search_provider = match self.web_search_provider {
                    None => Some(SearchProvider::Tavily),
                    Some(SearchProvider::Tavily) => Some(SearchProvider::Exa),
                    Some(SearchProvider::Exa) => Some(SearchProvider::Linkup),
                    Some(SearchProvider::Linkup) => Some(SearchProvider::Perplexity),
                    Some(SearchProvider::Perplexity) => None,
                };
            }
            SettingsField::TavilyApiKey => {}
            SettingsField::ExaApiKey => {}
            SettingsField::Save => {}
        }
    }

    /// Cycle the current field's value backward.
    pub fn cycle_backward(&mut self) {
        self.dirty = true;
        match self.current_field() {
            SettingsField::Model => {
                if !self.model_options.is_empty() {
                    if let Some(idx) = self.model_options.iter().position(|m| *m == self.model) {
                        let prev = if idx == 0 {
                            self.model_options.len() - 1
                        } else {
                            idx - 1
                        };
                        self.model = self.model_options[prev].clone();
                    }
                }
            }
            SettingsField::ChosenModels => {
                self.toggle_current_model_in_chosen();
            }
            SettingsField::Theme => {
                if !self.theme_options.is_empty() {
                    if let Some(idx) = self
                        .theme_options
                        .iter()
                        .position(|t| *t == self.theme_name)
                    {
                        let prev = if idx == 0 {
                            self.theme_options.len() - 1
                        } else {
                            idx - 1
                        };
                        self.theme_name = self.theme_options[prev].clone();
                    }
                }
            }
            SettingsField::ThinkingLevel => {
                self.thinking_level = prev_thinking(self.thinking_level);
            }
            SettingsField::MaxTokens => {
                self.max_tokens = self.max_tokens.saturating_sub(256).max(1);
            }
            SettingsField::ShellBackend => {
                self.shell_backend = prev_shell(&self.shell_backend);
            }
            SettingsField::MaxTurns => {
                self.max_turns = self.max_turns.saturating_sub(10).max(1);
            }
            SettingsField::ObservationMask => {
                self.observation_mask = (self.observation_mask - 0.05).max(0.0);
            }
            SettingsField::ReadMaxLines => {
                self.read_max_lines = self.read_max_lines.saturating_sub(100);
            }
            SettingsField::SidebarWidth => {
                self.sidebar_width = self.sidebar_width.saturating_sub(5).max(20);
            }
            SettingsField::WordWrap => {
                self.word_wrap = !self.word_wrap;
            }
            SettingsField::Animations => {
                self.animations = match self.animations {
                    AnimationLevel::None => AnimationLevel::Minimal,
                    AnimationLevel::Spinner => AnimationLevel::None,
                    AnimationLevel::Minimal => AnimationLevel::Spinner,
                };
            }
            SettingsField::AutoOpenSidebar => {
                self.auto_open_sidebar = !self.auto_open_sidebar;
            }
            SettingsField::SidebarAutoOpenWidth => {
                self.sidebar_auto_open_width =
                    self.sidebar_auto_open_width.saturating_sub(10).max(40);
            }
            SettingsField::ThinkingLines => {
                self.thinking_lines = self.thinking_lines.saturating_sub(1).max(1);
            }
            SettingsField::StreamingLines => {
                self.streaming_lines = self.streaming_lines.saturating_sub(1).max(1);
            }
            SettingsField::MouseScrollLines => {
                self.mouse_scroll_lines = self.mouse_scroll_lines.saturating_sub(1).max(1);
            }
            SettingsField::KeyboardScrollLines => {
                self.keyboard_scroll_lines = self.keyboard_scroll_lines.saturating_sub(5).max(5);
            }
            SettingsField::ShowTimestamps => {
                self.show_timestamps = !self.show_timestamps;
            }
            SettingsField::ShowCost => {
                self.show_cost = !self.show_cost;
            }
            SettingsField::ShowContextUsage => {
                self.show_context_usage = !self.show_context_usage;
            }
            SettingsField::NotifyOnAgentComplete => {
                self.notify_on_agent_complete = !self.notify_on_agent_complete;
            }
            SettingsField::ContinuePolicy => {
                self.continue_policy = match self.continue_policy {
                    ContinuePolicy::Disabled => ContinuePolicy::Aggressive,
                    ContinuePolicy::Conservative => ContinuePolicy::Disabled,
                    ContinuePolicy::Balanced => ContinuePolicy::Conservative,
                    ContinuePolicy::Aggressive => ContinuePolicy::Balanced,
                };
            }
            SettingsField::WebSearchProvider => {
                self.web_search_provider = match self.web_search_provider {
                    None => Some(SearchProvider::Perplexity),
                    Some(SearchProvider::Tavily) => None,
                    Some(SearchProvider::Exa) => Some(SearchProvider::Tavily),
                    Some(SearchProvider::Linkup) => Some(SearchProvider::Exa),
                    Some(SearchProvider::Perplexity) => Some(SearchProvider::Linkup),
                };
            }
            SettingsField::TavilyApiKey => {}
            SettingsField::ExaApiKey => {}
            SettingsField::Save => {}
        }
    }

    /// Begin direct numeric input for the current field.
    pub fn start_edit(&mut self) {
        match self.current_field() {
            SettingsField::MaxTokens => {
                self.editing_number = true;
                self.edit_buffer = self.max_tokens.to_string();
            }
            SettingsField::MaxTurns => {
                self.editing_number = true;
                self.edit_buffer = self.max_turns.to_string();
            }
            SettingsField::ObservationMask => {
                self.editing_number = true;
                self.edit_buffer = format!("{:.2}", self.observation_mask);
            }
            SettingsField::ReadMaxLines => {
                self.editing_number = true;
                self.edit_buffer = self.read_max_lines.to_string();
            }
            SettingsField::SidebarWidth => {
                self.editing_number = true;
                self.edit_buffer = self.sidebar_width.to_string();
            }
            SettingsField::TavilyApiKey => {
                self.editing_number = false;
                self.edit_buffer = self.tavily_api_key.clone();
            }
            SettingsField::ExaApiKey => {
                self.editing_number = false;
                self.edit_buffer = self.exa_api_key.clone();
            }
            _ => {
                // For enum/bool fields, Enter cycles forward
                self.cycle_forward();
            }
        }
    }

    pub fn push_char(&mut self, c: char) {
        if self.editing_number {
            if c.is_ascii_digit() || c == '.' {
                self.edit_buffer.push(c);
            }
            return;
        }

        match self.current_field() {
            SettingsField::TavilyApiKey => {
                self.tavily_api_key.push(c);
                self.dirty = true;
            }
            SettingsField::ExaApiKey => {
                self.exa_api_key.push(c);
                self.dirty = true;
            }
            SettingsField::ChosenModels => {
                if !c.is_control() {
                    let lower = c.to_ascii_lowercase();
                    if let Some(next) = self
                        .model_options
                        .iter()
                        .find(|m| m.to_ascii_lowercase().starts_with(lower))
                    {
                        self.model = next.clone();
                    }
                }
            }
            _ => {}
        }
    }

    pub fn pop_char(&mut self) {
        if self.editing_number {
            self.edit_buffer.pop();
            return;
        }

        match self.current_field() {
            SettingsField::TavilyApiKey => {
                self.tavily_api_key.pop();
                self.dirty = true;
            }
            SettingsField::ExaApiKey => {
                self.exa_api_key.pop();
                self.dirty = true;
            }
            _ => {}
        }
    }

    /// Commit the edit buffer to the underlying field value.
    pub fn commit_edit(&mut self) {
        if !self.editing_number {
            return;
        }
        self.editing_number = false;
        self.dirty = true;
        match self.current_field() {
            SettingsField::MaxTokens => {
                if let Ok(v) = self.edit_buffer.parse::<u32>() {
                    self.max_tokens = v.max(1);
                }
            }
            SettingsField::MaxTurns => {
                if let Ok(v) = self.edit_buffer.parse::<u32>() {
                    self.max_turns = v.max(1);
                }
            }
            SettingsField::ObservationMask => {
                if let Ok(v) = self.edit_buffer.parse::<f64>() {
                    self.observation_mask = v.clamp(0.0, 1.0);
                }
            }
            SettingsField::ReadMaxLines => {
                if let Ok(v) = self.edit_buffer.parse::<usize>() {
                    self.read_max_lines = v;
                }
            }
            SettingsField::SidebarWidth => {
                if let Ok(v) = self.edit_buffer.parse::<u16>() {
                    self.sidebar_width = v.clamp(20, 80);
                }
            }
            _ => {}
        }
        self.edit_buffer.clear();
    }

    /// Write current settings into a Config for saving and in-session use.
    pub fn apply_to_config(&self, config: &mut Config) {
        config.model = Some(self.model.clone());
        config.enabled_models = if self.chosen_models.is_empty() {
            None
        } else {
            Some(self.chosen_models.clone())
        };
        config.theme = Some(self.theme_name.clone());
        config.thinking = Some(self.thinking_level);
        config.max_tokens = Some(self.max_tokens);
        config.max_turns = Some(self.max_turns);
        config.context = ContextConfig {
            observation_mask_threshold: self.observation_mask,
            ..config.context.clone()
        };
        config.shell = ShellConfig {
            backend: self.shell_backend.clone(),
        };
        config.ui = imp_core::config::UiConfig {
            sidebar_style: SidebarStyle::Inspector,
            tool_output: ToolOutputDisplay::Full,
            tool_output_lines: self.tool_output_lines,
            read_max_lines: self.read_max_lines,
            sidebar_width: self.sidebar_width,
            word_wrap: self.word_wrap,
            animations: self.animations,
            hide_tools_in_chat: false,
            chat_tool_display: ChatToolDisplay::Summary,
            auto_open_sidebar: self.auto_open_sidebar,
            sidebar_auto_open_width: self.sidebar_auto_open_width,
            thinking_lines: self.thinking_lines,
            streaming_lines: self.streaming_lines,
            mouse_scroll_lines: self.mouse_scroll_lines,
            keyboard_scroll_lines: self.keyboard_scroll_lines,
            mouse_capture: config.ui.mouse_capture,
            show_timestamps: self.show_timestamps,
            show_cost: self.show_cost,
            show_context_usage: self.show_context_usage,
            notify_on_agent_complete: self.notify_on_agent_complete,
            continue_policy: self.continue_policy,
        };
        config.web = imp_core::tools::web::types::WebConfig {
            search_provider: self.web_search_provider,
        };
    }
    fn model_is_chosen(&self, model_id: &str) -> bool {
        self.chosen_models.iter().any(|m| m == model_id)
    }

    fn toggle_current_model_in_chosen(&mut self) {
        let model = self.model.clone();
        if let Some(idx) = self.chosen_models.iter().position(|m| m == &model) {
            self.chosen_models.remove(idx);
        } else {
            self.chosen_models.push(model);
        }
    }

    fn chosen_models_summary(&self) -> String {
        if self.chosen_models.is_empty() {
            "all models".to_string()
        } else {
            format!("{} chosen", self.chosen_models.len())
        }
    }
}

fn next_thinking(level: ThinkingLevel) -> ThinkingLevel {
    match level {
        ThinkingLevel::Off => ThinkingLevel::Low,
        ThinkingLevel::Minimal => ThinkingLevel::Low,
        ThinkingLevel::Low => ThinkingLevel::Medium,
        ThinkingLevel::Medium => ThinkingLevel::High,
        ThinkingLevel::High => ThinkingLevel::XHigh,
        ThinkingLevel::XHigh => ThinkingLevel::Off,
    }
}

fn prev_thinking(level: ThinkingLevel) -> ThinkingLevel {
    match level {
        ThinkingLevel::Off => ThinkingLevel::XHigh,
        ThinkingLevel::Minimal => ThinkingLevel::Off,
        ThinkingLevel::Low => ThinkingLevel::Off,
        ThinkingLevel::Medium => ThinkingLevel::Low,
        ThinkingLevel::High => ThinkingLevel::Medium,
        ThinkingLevel::XHigh => ThinkingLevel::High,
    }
}

fn next_shell(backend: &ShellBackend) -> ShellBackend {
    match backend {
        ShellBackend::Sh => ShellBackend::Rush,
        ShellBackend::Rush => ShellBackend::RushDaemon,
        ShellBackend::RushDaemon => ShellBackend::Sh,
    }
}

fn prev_shell(backend: &ShellBackend) -> ShellBackend {
    match backend {
        ShellBackend::Sh => ShellBackend::RushDaemon,
        ShellBackend::Rush => ShellBackend::Sh,
        ShellBackend::RushDaemon => ShellBackend::Rush,
    }
}

fn thinking_label(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::Off => "Off",
        ThinkingLevel::Minimal => "Minimal",
        ThinkingLevel::Low => "Low",
        ThinkingLevel::Medium => "Medium",
        ThinkingLevel::High => "High",
        ThinkingLevel::XHigh => "XHigh",
    }
}

fn shell_label(backend: &ShellBackend) -> &'static str {
    match backend {
        ShellBackend::Sh => "sh",
        ShellBackend::Rush => "rush",
        ShellBackend::RushDaemon => "rush-daemon",
    }
}

fn animation_label(level: AnimationLevel) -> &'static str {
    match level {
        AnimationLevel::None => "none",
        AnimationLevel::Spinner => "spinner",
        AnimationLevel::Minimal => "minimal",
    }
}

enum SettingsRow {
    Header,
    Field(usize),
    Save,
}

fn visit_settings_rows(mut visit: impl FnMut(SettingsRow, u16)) {
    let mut row: u16 = 0;
    visit(SettingsRow::Header, row);
    row += 2;

    for field_idx in 0..=5 {
        visit(SettingsRow::Field(field_idx), row);
        row += 1;
    }

    row += 1;

    for field_idx in 6..=7 {
        visit(SettingsRow::Field(field_idx), row);
        row += 1;
    }

    row += 1;

    for field_idx in 8..=29 {
        visit(SettingsRow::Field(field_idx), row);
        row += 1;
    }

    row += 1;
    visit(SettingsRow::Save, row);
}

fn total_settings_rows() -> u16 {
    let mut total = 0;
    visit_settings_rows(|_, row| {
        total = row.saturating_add(1);
    });
    total
}

fn selected_settings_row(selected: usize) -> u16 {
    let selected = selected.min(FIELDS.len().saturating_sub(1));
    let mut selected_row = 0;
    visit_settings_rows(|entry, row| match entry {
        SettingsRow::Field(field_idx) if field_idx == selected => selected_row = row,
        SettingsRow::Save if selected == FIELDS.len().saturating_sub(1) => selected_row = row,
        _ => {}
    });
    selected_row
}

fn settings_scroll_offset(selected: usize, visible_rows: u16) -> u16 {
    if visible_rows == 0 {
        return 0;
    }

    let total_rows = total_settings_rows();
    if total_rows <= visible_rows {
        return 0;
    }

    let selected_row = selected_settings_row(selected);
    let desired = selected_row.saturating_sub(visible_rows.saturating_sub(1));
    desired.min(total_rows.saturating_sub(visible_rows))
}

fn scrolled_screen_y(inner: Rect, logical_row: u16, scroll_offset: u16) -> Option<u16> {
    if logical_row < scroll_offset {
        return None;
    }

    let visible_row = logical_row - scroll_offset;
    if visible_row >= inner.height {
        return None;
    }

    Some(inner.y + visible_row)
}

/// Settings overlay widget.
pub struct SettingsView<'a> {
    state: &'a SettingsState,
    theme: &'a Theme,
}

impl<'a> SettingsView<'a> {
    pub fn new(state: &'a SettingsState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for SettingsView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 10 || area.width < 30 {
            return;
        }

        Clear.render(area, buf);

        let title = if self.state.dirty {
            " Settings * "
        } else {
            " Settings "
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let total_rows = total_settings_rows();
        let scroll_offset = settings_scroll_offset(self.state.normalized_selected(), inner.height);

        let mut row: u16 = 0;

        let header = Line::from(Span::styled(
            "  ↑/↓ move  ←/→ change  Enter edit  Esc close",
            self.theme.muted_style(),
        ));
        if let Some(y) = scrolled_screen_y(inner, row, scroll_offset) {
            buf.set_line(inner.x, y, &header, inner.width);
        }
        row += 2;

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            0,
            "Model",
            &self.state.model,
            "← →",
        );

        let chosen_hint = if self.state.model_is_chosen(&self.state.model) {
            "← → toggle current"
        } else {
            "← → add current"
        };
        let chosen_summary = self.state.chosen_models_summary();
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            1,
            "Chosen models",
            &chosen_summary,
            chosen_hint,
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            2,
            "Theme",
            &self.state.theme_name,
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            3,
            "Thinking level",
            thinking_label(self.state.thinking_level),
            "← →",
        );

        let max_tokens_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::MaxTokens
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.max_tokens.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            4,
            "Max tokens",
            &max_tokens_val,
            "← → / type",
        );

        let max_turns_val =
            if self.state.editing_number && self.state.current_field() == SettingsField::MaxTurns {
                format!("{}▎", self.state.edit_buffer)
            } else {
                self.state.max_turns.to_string()
            };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            5,
            "Max turns",
            &max_turns_val,
            "← → / type",
        );

        row += 1;

        let obs_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::ObservationMask
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            format!("{:.0}%", self.state.observation_mask * 100.0)
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            6,
            "Observation mask",
            &obs_val,
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            7,
            "Shell backend",
            shell_label(&self.state.shell_backend),
            "← →",
        );

        row += 1;

        let rml_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::ReadMaxLines
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.read_max_lines.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            8,
            "Read max lines",
            &rml_val,
            "← → / type (0 = no limit)",
        );

        let sw_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::SidebarWidth
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            format!("{}%", self.state.sidebar_width)
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            9,
            "Inspector width",
            &sw_val,
            "← → / type",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            10,
            "Word wrap",
            if self.state.word_wrap { "on" } else { "off" },
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            11,
            "Animations",
            animation_label(self.state.animations),
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            16,
            "Auto-open sidebar",
            if self.state.auto_open_sidebar {
                "on"
            } else {
                "off"
            },
            "← →",
        );

        let sao_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::SidebarAutoOpenWidth
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.sidebar_auto_open_width.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            17,
            "Auto-open width",
            &sao_val,
            "← → / type",
        );

        let thinking_lines_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::ThinkingLines
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.thinking_lines.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            18,
            "Thinking lines",
            &thinking_lines_val,
            "← → / type",
        );

        let streaming_lines_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::StreamingLines
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.streaming_lines.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            19,
            "Streaming lines",
            &streaming_lines_val,
            "← → / type",
        );

        let mouse_scroll_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::MouseScrollLines
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.mouse_scroll_lines.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            20,
            "Mouse scroll",
            &mouse_scroll_val,
            "← → / type",
        );

        let keyboard_scroll_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::KeyboardScrollLines
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.keyboard_scroll_lines.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            21,
            "Keyboard scroll",
            &keyboard_scroll_val,
            "← → / type",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            22,
            "Show timestamps",
            if self.state.show_timestamps {
                "on"
            } else {
                "off"
            },
            "← →",
        );
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            23,
            "Show cost",
            if self.state.show_cost { "on" } else { "off" },
            "← →",
        );
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            24,
            "Show context",
            if self.state.show_context_usage {
                "on"
            } else {
                "off"
            },
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            25,
            "Bell on done",
            if self.state.notify_on_agent_complete {
                "on"
            } else {
                "off"
            },
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            26,
            "Auto-continue",
            match self.state.continue_policy {
                ContinuePolicy::Disabled => "disabled",
                ContinuePolicy::Conservative => "conservative",
                ContinuePolicy::Balanced => "balanced",
                ContinuePolicy::Aggressive => "aggressive",
            },
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            27,
            "Web provider",
            match self.state.web_search_provider {
                None => "auto",
                Some(SearchProvider::Tavily) => "tavily",
                Some(SearchProvider::Exa) => "exa",
                Some(SearchProvider::Linkup) => "linkup",
                Some(SearchProvider::Perplexity) => "perplexity",
            },
            "← →",
        );

        let tavily_val = if self.state.tavily_api_key.is_empty() {
            if self.state.tavily_configured {
                "configured (press Enter to replace)".to_string()
            } else {
                "not set".to_string()
            }
        } else {
            format!(
                "{}▎",
                "•".repeat(self.state.tavily_api_key.chars().count().max(1))
            )
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            28,
            "Tavily API key",
            &tavily_val,
            "Enter to edit",
        );

        let exa_val = if self.state.exa_api_key.is_empty() {
            if self.state.exa_configured {
                "configured (press Enter to replace)".to_string()
            } else {
                "not set".to_string()
            }
        } else {
            format!(
                "{}▎",
                "•".repeat(self.state.exa_api_key.chars().count().max(1))
            )
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            scroll_offset,
            &mut row,
            29,
            "Exa API key",
            &exa_val,
            "Enter to edit",
        );
        row += 1;

        if let Some(y) = scrolled_screen_y(inner, row, scroll_offset) {
            let is_save = self.state.normalized_selected() == FIELDS.len() - 1;
            let save_style = if is_save {
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                self.theme.muted_style()
            };
            let marker = if is_save { "▸ " } else { "  " };
            let dirty_hint = if self.state.dirty {
                " (unsaved changes)"
            } else {
                ""
            };
            let line = Line::from(vec![
                Span::styled(marker, self.theme.accent_style()),
                Span::styled("[ Save to config.toml ]", save_style),
                Span::styled(dirty_hint, self.theme.warning_style()),
            ]);
            buf.set_line(inner.x, y, &line, inner.width);
        }

        if scroll_offset > 0 {
            let hint = Line::from(Span::styled("↑ more", self.theme.muted_style()));
            buf.set_line(inner.x + inner.width.saturating_sub(7), inner.y, &hint, 7);
        }
        if scroll_offset + inner.height < total_rows {
            let hint = Line::from(Span::styled("↓ more", self.theme.muted_style()));
            let y = inner.y + inner.height.saturating_sub(1);
            buf.set_line(inner.x + inner.width.saturating_sub(7), y, &hint, 7);
        }
    }
}

/// Render one settings field row.
#[allow(clippy::too_many_arguments)]
fn render_field(
    state: &SettingsState,
    theme: &Theme,
    buf: &mut Buffer,
    inner: Rect,
    scroll_offset: u16,
    row: &mut u16,
    field_idx: usize,
    label: &str,
    value: &str,
    hint: &str,
) {
    let logical_row = *row;
    let Some(screen_y) = scrolled_screen_y(inner, logical_row, scroll_offset) else {
        *row += 1;
        return;
    };

    let is_selected = field_idx == state.normalized_selected();
    let marker = if is_selected { "▸ " } else { "  " };

    let label_style = if is_selected {
        theme.selected_style()
    } else {
        Style::default()
    };
    let value_style = if is_selected {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let label_width = 22;
    let line = Line::from(vec![
        Span::styled(marker, theme.accent_style()),
        Span::styled(format!("{label:<label_width$}"), label_style),
        Span::styled(value, value_style),
        Span::raw("  "),
        Span::styled(hint, theme.muted_style()),
    ]);
    buf.set_line(inner.x, screen_y, &line, inner.width);
    *row += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_core::config::Config;
    use imp_llm::auth::AuthStore;
    use imp_llm::model::ModelRegistry;

    #[test]
    fn applying_settings_forces_primary_inspector_display_model() {
        let registry = ModelRegistry::with_builtins();
        let models = registry.list().to_vec();
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let mut config = Config::default();
        let state = SettingsState::new(&config, &models[0].id, &models, &auth_store);

        state.apply_to_config(&mut config);

        assert_eq!(config.ui.sidebar_style, SidebarStyle::Inspector);
        assert_eq!(config.ui.tool_output, ToolOutputDisplay::Full);
        assert_eq!(config.ui.chat_tool_display, ChatToolDisplay::Summary);
        assert!(!config.ui.hide_tools_in_chat);
    }

    #[test]
    fn save_field_scrolls_into_view_on_short_panels() {
        assert_eq!(selected_settings_row(FIELDS.len() - 1), 35);
        assert_eq!(total_settings_rows(), 36);
        assert_eq!(settings_scroll_offset(FIELDS.len() - 1, 10), 26);
    }

    #[test]
    fn top_fields_do_not_scroll_when_visible() {
        assert_eq!(selected_settings_row(0), 2);
        assert_eq!(settings_scroll_offset(0, 10), 0);
        assert_eq!(settings_scroll_offset(5, 10), 0);
    }

    #[test]
    fn current_field_clamps_stale_selection() {
        let registry = ModelRegistry::with_builtins();
        let models = registry.list().to_vec();
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let state = SettingsState {
            selected: usize::MAX,
            ..SettingsState::new(&Config::default(), &models[0].id, &models, &auth_store)
        };

        assert_eq!(state.current_field(), SettingsField::Save);
    }

    #[test]
    fn cycle_model_is_safe_with_empty_model_options() {
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let mut state = SettingsState::new(&Config::default(), "custom-model", &[], &auth_store);
        state.selected = 0;
        state.model_options.clear();

        state.cycle_forward();
        state.cycle_backward();

        assert_eq!(state.model, "custom-model");
    }

    #[test]
    fn chosen_models_round_trip_into_config() {
        let registry = ModelRegistry::with_builtins();
        let models = registry.list().to_vec();
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let mut config = Config::default();
        let mut state = SettingsState::new(&config, &models[0].id, &models, &auth_store);

        state.selected = 1;
        state.cycle_forward();
        assert_eq!(state.chosen_models, vec![models[0].id.clone()]);

        state.apply_to_config(&mut config);
        assert_eq!(config.enabled_models, Some(vec![models[0].id.clone()]));
    }

    #[test]
    fn bell_setting_round_trips_into_config() {
        let registry = ModelRegistry::with_builtins();
        let models = registry.list().to_vec();
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let mut config = Config::default();
        let state = SettingsState {
            notify_on_agent_complete: false,
            ..SettingsState::new(&config, &models[0].id, &models, &auth_store)
        };

        state.apply_to_config(&mut config);
        assert!(!config.ui.notify_on_agent_complete);
    }

    #[test]
    fn continue_policy_round_trips_into_config() {
        let registry = ModelRegistry::with_builtins();
        let models = registry.list().to_vec();
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let mut config = Config::default();
        let state = SettingsState {
            continue_policy: ContinuePolicy::Balanced,
            ..SettingsState::new(&config, &models[0].id, &models, &auth_store)
        };

        state.apply_to_config(&mut config);
        assert_eq!(config.ui.continue_policy, ContinuePolicy::Balanced);
    }

    #[test]
    fn empty_chosen_models_means_all_models() {
        let registry = ModelRegistry::with_builtins();
        let models = registry.list().to_vec();
        let auth_store = AuthStore::new(std::path::PathBuf::from("/tmp/auth.json"));
        let mut config = Config::default();
        let state = SettingsState::new(&config, &models[0].id, &models, &auth_store);

        state.apply_to_config(&mut config);
        assert_eq!(config.enabled_models, None);
    }
}
