use imp_core::config::{
    AnimationLevel, ChatToolDisplay, Config, ContextConfig, ShellBackend, ShellConfig,
    SidebarStyle, ToolOutputDisplay,
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
    Theme,
    ThinkingLevel,
    MaxTurns,
    ObservationMask,
    ShellBackend,
    SidebarStyle,
    ToolOutput,
    ToolOutputLines,
    ReadMaxLines,
    SidebarWidth,
    WordWrap,
    Animations,
    ChatToolDisplay,
    AutoOpenSidebar,
    SidebarAutoOpenWidth,
    ThinkingLines,
    StreamingLines,
    MouseScrollLines,
    KeyboardScrollLines,
    ShowTimestamps,
    ShowCost,
    ShowContextUsage,
    WebSearchProvider,
    TavilyApiKey,
    ExaApiKey,
    Save,
}

const FIELDS: &[SettingsField] = &[
    SettingsField::Model,
    SettingsField::Theme,
    SettingsField::ThinkingLevel,
    SettingsField::MaxTurns,
    SettingsField::ObservationMask,
    SettingsField::ShellBackend,
    SettingsField::SidebarStyle,
    SettingsField::ToolOutput,
    SettingsField::ToolOutputLines,
    SettingsField::ReadMaxLines,
    SettingsField::SidebarWidth,
    SettingsField::WordWrap,
    SettingsField::Animations,
    SettingsField::ChatToolDisplay,
    SettingsField::AutoOpenSidebar,
    SettingsField::SidebarAutoOpenWidth,
    SettingsField::ThinkingLines,
    SettingsField::StreamingLines,
    SettingsField::MouseScrollLines,
    SettingsField::KeyboardScrollLines,
    SettingsField::ShowTimestamps,
    SettingsField::ShowCost,
    SettingsField::ShowContextUsage,
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
    pub theme_name: String,
    pub theme_options: Vec<String>,
    pub thinking_level: ThinkingLevel,
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
    pub fn new(config: &Config, model_name: &str, models: &[ModelMeta], auth_store: &AuthStore) -> Self {
        Self {
            selected: 0,
            model: model_name.to_string(),
            model_options: models.iter().map(|m| m.id.clone()).collect(),
            theme_name: config.theme.clone().unwrap_or_else(|| "default".into()),
            theme_options: vec!["default".into(), "light".into()],
            thinking_level: config.thinking.unwrap_or(ThinkingLevel::Medium),
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
            web_search_provider: config.web.search_provider,
            tavily_api_key: String::new(),
            exa_api_key: String::new(),
            tavily_configured: auth_store.stored.contains_key("tavily") || std::env::var("TAVILY_API_KEY").is_ok(),
            exa_configured: auth_store.stored.contains_key("exa") || std::env::var("EXA_API_KEY").is_ok(),
            editing_number: false,
            edit_buffer: String::new(),
            dirty: false,
        }
    }

    pub fn current_field(&self) -> SettingsField {
        FIELDS[self.selected]
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
                if let Some(idx) = self.model_options.iter().position(|m| *m == self.model) {
                    let next = (idx + 1) % self.model_options.len();
                    self.model = self.model_options[next].clone();
                }
            }
            SettingsField::Theme => {
                if let Some(idx) = self
                    .theme_options
                    .iter()
                    .position(|t| *t == self.theme_name)
                {
                    let next = (idx + 1) % self.theme_options.len();
                    self.theme_name = self.theme_options[next].clone();
                }
            }
            SettingsField::ThinkingLevel => {
                self.thinking_level = next_thinking(self.thinking_level);
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
            SettingsField::SidebarStyle => {
                self.sidebar_style = match self.sidebar_style {
                    SidebarStyle::Stream => SidebarStyle::Split,
                    SidebarStyle::Split => SidebarStyle::Stream,
                };
            }
            SettingsField::ToolOutput => {
                self.tool_output = match self.tool_output {
                    ToolOutputDisplay::Full => ToolOutputDisplay::Compact,
                    ToolOutputDisplay::Compact => ToolOutputDisplay::Collapsed,
                    ToolOutputDisplay::Collapsed => ToolOutputDisplay::Full,
                };
            }
            SettingsField::ToolOutputLines => {
                self.tool_output_lines = self.tool_output_lines.saturating_add(5).min(100);
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
            SettingsField::ChatToolDisplay => {
                self.chat_tool_display = match self.chat_tool_display {
                    ChatToolDisplay::Interleaved => ChatToolDisplay::Summary,
                    ChatToolDisplay::Summary => ChatToolDisplay::Hidden,
                    ChatToolDisplay::Hidden => ChatToolDisplay::Interleaved,
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
                if let Some(idx) = self.model_options.iter().position(|m| *m == self.model) {
                    let prev = if idx == 0 {
                        self.model_options.len() - 1
                    } else {
                        idx - 1
                    };
                    self.model = self.model_options[prev].clone();
                }
            }
            SettingsField::Theme => {
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
            SettingsField::ThinkingLevel => {
                self.thinking_level = prev_thinking(self.thinking_level);
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
            SettingsField::SidebarStyle => {
                self.sidebar_style = match self.sidebar_style {
                    SidebarStyle::Stream => SidebarStyle::Split,
                    SidebarStyle::Split => SidebarStyle::Stream,
                };
            }
            SettingsField::ToolOutput => {
                self.tool_output = match self.tool_output {
                    ToolOutputDisplay::Full => ToolOutputDisplay::Collapsed,
                    ToolOutputDisplay::Compact => ToolOutputDisplay::Full,
                    ToolOutputDisplay::Collapsed => ToolOutputDisplay::Compact,
                };
            }
            SettingsField::ToolOutputLines => {
                self.tool_output_lines = self.tool_output_lines.saturating_sub(5).max(5);
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
            SettingsField::ChatToolDisplay => {
                self.chat_tool_display = match self.chat_tool_display {
                    ChatToolDisplay::Interleaved => ChatToolDisplay::Hidden,
                    ChatToolDisplay::Summary => ChatToolDisplay::Interleaved,
                    ChatToolDisplay::Hidden => ChatToolDisplay::Summary,
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
            SettingsField::MaxTurns => {
                self.editing_number = true;
                self.edit_buffer = self.max_turns.to_string();
            }
            SettingsField::ObservationMask => {
                self.editing_number = true;
                self.edit_buffer = format!("{:.2}", self.observation_mask);
            }
            SettingsField::ToolOutputLines => {
                self.editing_number = true;
                self.edit_buffer = self.tool_output_lines.to_string();
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
            SettingsField::ToolOutputLines => {
                if let Ok(v) = self.edit_buffer.parse::<usize>() {
                    self.tool_output_lines = v.clamp(1, 100);
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
        config.theme = Some(self.theme_name.clone());
        config.thinking = Some(self.thinking_level);
        config.max_turns = Some(self.max_turns);
        config.context = ContextConfig {
            observation_mask_threshold: self.observation_mask,
            ..config.context.clone()
        };
        config.shell = ShellConfig {
            backend: self.shell_backend.clone(),
        };
        config.ui = imp_core::config::UiConfig {
            sidebar_style: self.sidebar_style,
            tool_output: self.tool_output,
            tool_output_lines: self.tool_output_lines,
            read_max_lines: self.read_max_lines,
            sidebar_width: self.sidebar_width,
            word_wrap: self.word_wrap,
            animations: self.animations,
            hide_tools_in_chat: self.chat_tool_display == ChatToolDisplay::Hidden,
            chat_tool_display: self.chat_tool_display,
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
        };
        config.web = imp_core::tools::web::types::WebConfig {
            search_provider: self.web_search_provider,
        };
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

        let mut row: u16 = 0;

        // Instructions
        let header = Line::from(Span::styled(
            "  ←/→ change value  Enter edit  Esc close",
            self.theme.muted_style(),
        ));
        buf.set_line(inner.x, inner.y + row, &header, inner.width);
        row += 2;

        // Model
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            0,
            "Model",
            &self.state.model,
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            1,
            "Theme",
            &self.state.theme_name,
            "← →",
        );

        // Thinking
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            2,
            "Thinking level",
            thinking_label(self.state.thinking_level),
            "← →",
        );

        // Max turns
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
            &mut row,
            3,
            "Max turns",
            &max_turns_val,
            "← → / type",
        );

        // Spacer before context section
        row += 1;

        // Observation mask
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
            &mut row,
            4,
            "Observation mask",
            &obs_val,
            "← →",
        );

        // Shell backend
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            5,
            "Shell backend",
            shell_label(&self.state.shell_backend),
            "← →",
        );

        // Spacer before UI section
        row += 1;

        // Sidebar style
        let sidebar_label = match self.state.sidebar_style {
            SidebarStyle::Stream => "stream",
            SidebarStyle::Split => "split",
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            6,
            "Sidebar style",
            sidebar_label,
            "← →",
        );

        // Tool output
        let tool_output_label = match self.state.tool_output {
            ToolOutputDisplay::Full => "full",
            ToolOutputDisplay::Compact => "compact",
            ToolOutputDisplay::Collapsed => "collapsed",
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            7,
            "Tool output",
            tool_output_label,
            "← →",
        );

        // Tool output lines
        let tol_val = if self.state.editing_number
            && self.state.current_field() == SettingsField::ToolOutputLines
        {
            format!("{}▎", self.state.edit_buffer)
        } else {
            self.state.tool_output_lines.to_string()
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            8,
            "Tool output lines",
            &tol_val,
            "← → / type",
        );

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
            &mut row,
            9,
            "Read max lines",
            &rml_val,
            "← → / type (0 = no limit)",
        );

        // Sidebar width
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
            &mut row,
            10,
            "Sidebar width",
            &sw_val,
            "← → / type",
        );

        // Word wrap
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            11,
            "Word wrap",
            if self.state.word_wrap { "on" } else { "off" },
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            12,
            "Animations",
            animation_label(self.state.animations),
            "← →",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            13,
            "Chat tool display",
            match self.state.chat_tool_display {
                ChatToolDisplay::Interleaved => "interleaved",
                ChatToolDisplay::Summary => "summary",
                ChatToolDisplay::Hidden => "hidden",
            },
            "← →",
        );
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            14,
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
            &mut row,
            15,
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
            &mut row,
            16,
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
            &mut row,
            17,
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
            &mut row,
            18,
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
            &mut row,
            19,
            "Keyboard scroll",
            &keyboard_scroll_val,
            "← → / type",
        );

        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            20,
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
            &mut row,
            21,
            "Show cost",
            if self.state.show_cost { "on" } else { "off" },
            "← →",
        );
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            22,
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
            &mut row,
            23,
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
            format!("{}▎", "•".repeat(self.state.tavily_api_key.chars().count().max(1)))
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            24,
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
            format!("{}▎", "•".repeat(self.state.exa_api_key.chars().count().max(1)))
        };
        render_field(
            self.state,
            self.theme,
            buf,
            inner,
            &mut row,
            25,
            "Exa API key",
            &exa_val,
            "Enter to edit",
        );
        // Spacer before save
        row += 1;

        // Save button
        if row < inner.height {
            let is_save = self.state.selected == FIELDS.len() - 1;
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
            buf.set_line(inner.x, inner.y + row, &line, inner.width);
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
    row: &mut u16,
    field_idx: usize,
    label: &str,
    value: &str,
    hint: &str,
) {
    if *row >= inner.height {
        return;
    }
    let is_selected = field_idx == state.selected;
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
    buf.set_line(inner.x, inner.y + *row, &line, inner.width);
    *row += 1;
}
