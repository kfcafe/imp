use imp_llm::model::ModelMeta;
use imp_llm::ThinkingLevel;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// Providers the welcome flow supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeProvider {
    Anthropic,
    OpenAI,
    Google,
}

impl WelcomeProvider {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Anthropic => "Anthropic",
            Self::OpenAI => "OpenAI",
            Self::Google => "Google",
        }
    }

    pub fn env_var(&self) -> &'static str {
        match self {
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Google => "GOOGLE_API_KEY",
        }
    }

    pub fn provider_id(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
        }
    }

    pub fn key_url(&self) -> &'static str {
        match self {
            Self::Anthropic => "console.anthropic.com/settings/keys",
            Self::OpenAI => "platform.openai.com/api-keys",
            Self::Google => "aistudio.google.dev/apikey",
        }
    }

    pub fn default_model_alias(&self) -> &'static str {
        match self {
            Self::Anthropic => "sonnet",
            Self::OpenAI => "gpt4o",
            Self::Google => "gemini-pro",
        }
    }
}

const PROVIDERS: &[WelcomeProvider] = &[
    WelcomeProvider::Anthropic,
    WelcomeProvider::OpenAI,
    WelcomeProvider::Google,
];

/// Which step of the welcome flow the user is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeStep {
    /// Splash / introduction.
    Welcome,
    /// Choose provider and enter API key.
    ProviderAuth,
    /// Pick default model and thinking level.
    ModelThinking,
    /// Summary and quick tips.
    Done,
}

const STEPS: &[WelcomeStep] = &[
    WelcomeStep::Welcome,
    WelcomeStep::ProviderAuth,
    WelcomeStep::ModelThinking,
    WelcomeStep::Done,
];

/// Detected state for each provider — whether an env var or stored credential exists.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub provider: WelcomeProvider,
    pub env_detected: bool,
    pub stored: bool,
}

impl ProviderStatus {
    pub fn has_auth(&self) -> bool {
        self.env_detected || self.stored
    }
}

/// State for the welcome overlay.
#[derive(Debug, Clone)]
pub struct WelcomeState {
    pub step: usize,
    /// Provider list with detection status.
    pub providers: Vec<ProviderStatus>,
    /// Currently selected provider index.
    pub provider_selected: usize,
    /// API key input buffer (masked display).
    pub key_input: String,
    /// Whether the key input field is active.
    pub key_editing: bool,
    /// Error message for invalid key input.
    pub key_error: Option<String>,
    /// Available models for the selected provider.
    pub models: Vec<ModelMeta>,
    /// Selected model index.
    pub model_selected: usize,
    /// Selected thinking level.
    pub thinking_level: ThinkingLevel,
    /// Whether auth was resolved (env or input).
    pub auth_resolved: bool,
    /// The resolved API key (if entered manually).
    pub resolved_key: Option<String>,
}

impl WelcomeState {
    /// Create welcome state, detecting existing auth.
    pub fn new(all_models: &[ModelMeta]) -> Self {
        let providers: Vec<ProviderStatus> = PROVIDERS
            .iter()
            .map(|p| {
                let env_detected = std::env::var(p.env_var()).is_ok();
                ProviderStatus {
                    provider: *p,
                    env_detected,
                    stored: false,
                }
            })
            .collect();

        // Pre-select the first provider with auth, or Anthropic by default.
        let provider_selected = providers.iter().position(|p| p.has_auth()).unwrap_or(0);

        let selected_provider = providers[provider_selected].provider;
        let models = filter_models_for_provider(all_models, selected_provider);

        Self {
            step: 0,
            providers,
            provider_selected,
            key_input: String::new(),
            key_editing: false,
            key_error: None,
            models,
            model_selected: 0,
            thinking_level: ThinkingLevel::Medium,
            auth_resolved: false,
            resolved_key: None,
        }
    }

    /// Mark a provider as having a stored credential.
    pub fn mark_stored(&mut self, provider: WelcomeProvider) {
        for p in &mut self.providers {
            if p.provider == provider {
                p.stored = true;
            }
        }
    }

    pub fn current_step(&self) -> WelcomeStep {
        STEPS[self.step]
    }

    pub fn selected_provider(&self) -> &ProviderStatus {
        &self.providers[self.provider_selected]
    }

    pub fn selected_provider_id(&self) -> WelcomeProvider {
        self.providers[self.provider_selected].provider
    }

    pub fn selected_model(&self) -> Option<&ModelMeta> {
        self.models.get(self.model_selected)
    }

    pub fn advance(&mut self) {
        if self.step + 1 < STEPS.len() {
            self.step += 1;
        }
    }

    pub fn go_back(&mut self) {
        if self.step > 0 {
            self.step -= 1;
        }
    }

    pub fn provider_up(&mut self) {
        if self.provider_selected > 0 {
            self.provider_selected -= 1;
            self.on_provider_changed();
        }
    }

    pub fn provider_down(&mut self) {
        if self.provider_selected + 1 < self.providers.len() {
            self.provider_selected += 1;
            self.on_provider_changed();
        }
    }

    pub fn model_up(&mut self) {
        if self.model_selected > 0 {
            self.model_selected -= 1;
        }
    }

    pub fn model_down(&mut self) {
        if self.model_selected + 1 < self.models.len() {
            self.model_selected += 1;
        }
    }

    pub fn cycle_thinking(&mut self) {
        self.thinking_level = match self.thinking_level {
            ThinkingLevel::Off => ThinkingLevel::Low,
            ThinkingLevel::Minimal => ThinkingLevel::Low,
            ThinkingLevel::Low => ThinkingLevel::Medium,
            ThinkingLevel::Medium => ThinkingLevel::High,
            ThinkingLevel::High => ThinkingLevel::XHigh,
            ThinkingLevel::XHigh => ThinkingLevel::Off,
        };
    }

    pub fn cycle_thinking_back(&mut self) {
        self.thinking_level = match self.thinking_level {
            ThinkingLevel::Off => ThinkingLevel::XHigh,
            ThinkingLevel::Minimal => ThinkingLevel::Off,
            ThinkingLevel::Low => ThinkingLevel::Off,
            ThinkingLevel::Medium => ThinkingLevel::Low,
            ThinkingLevel::High => ThinkingLevel::Medium,
            ThinkingLevel::XHigh => ThinkingLevel::High,
        };
    }

    pub fn push_key_char(&mut self, c: char) {
        self.key_input.push(c);
        self.key_error = None;
    }

    pub fn pop_key_char(&mut self) {
        self.key_input.pop();
        self.key_error = None;
    }

    /// Check whether auth is available for the current provider (env or entered key).
    pub fn check_auth_resolved(&mut self) -> bool {
        let status = &self.providers[self.provider_selected];
        if status.has_auth() {
            self.auth_resolved = true;
            self.resolved_key = None;
            return true;
        }
        if !self.key_input.trim().is_empty() {
            self.auth_resolved = true;
            self.resolved_key = Some(self.key_input.trim().to_string());
            return true;
        }
        self.key_error = Some("Please enter an API key or set the environment variable.".into());
        false
    }

    pub fn update_models(&mut self, all_models: &[ModelMeta]) {
        let provider = self.selected_provider_id();
        self.models = filter_models_for_provider(all_models, provider);
        self.model_selected = 0;
    }

    fn on_provider_changed(&mut self) {
        self.key_input.clear();
        self.key_error = None;
        self.key_editing = false;
        self.auth_resolved = false;
        self.resolved_key = None;
    }
}

fn filter_models_for_provider(
    all_models: &[ModelMeta],
    provider: WelcomeProvider,
) -> Vec<ModelMeta> {
    all_models
        .iter()
        .filter(|m| m.provider == provider.provider_id())
        .cloned()
        .collect()
}

/// Detect whether this is a first run that needs the welcome flow.
///
/// Returns true when there is no user config AND no working auth for any
/// supported provider.
pub fn needs_welcome(config_dir: &std::path::Path, auth_path: &std::path::Path) -> bool {
    let config_exists = config_dir.join("config.toml").exists();
    if config_exists {
        return false;
    }

    // Check if any provider has auth via env var or stored credential.
    let has_env = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GOOGLE_API_KEY"]
        .iter()
        .any(|var| std::env::var(var).is_ok());

    let has_stored = auth_path.exists()
        && std::fs::read_to_string(auth_path)
            .map(|s| s.trim().len() > 2) // not empty JSON "{}"
            .unwrap_or(false);

    !has_env && !has_stored
}

// ── View widget ─────────────────────────────────────────────────

/// Welcome overlay widget.
pub struct WelcomeView<'a> {
    state: &'a WelcomeState,
    theme: &'a Theme,
}

impl<'a> WelcomeView<'a> {
    pub fn new(state: &'a WelcomeState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for WelcomeView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 10 || area.width < 30 {
            return;
        }

        Clear.render(area, buf);

        let step_indicator = format!(" Welcome ({}/{}) ", self.state.step + 1, STEPS.len());
        let block = Block::default()
            .title(step_indicator)
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        match self.state.current_step() {
            WelcomeStep::Welcome => self.render_welcome(inner, buf),
            WelcomeStep::ProviderAuth => self.render_provider_auth(inner, buf),
            WelcomeStep::ModelThinking => self.render_model_thinking(inner, buf),
            WelcomeStep::Done => self.render_done(inner, buf),
        }
    }
}

impl WelcomeView<'_> {
    fn render_welcome(&self, area: Rect, buf: &mut Buffer) {
        let mut row: u16 = 0;
        let center_x = area.x;

        // ASCII art logo
        let logo = [
            "  ╔╗    ╔╗  ",
            "  ║╚════╝║  ",
            "  ║ ■  ■ ║  ",
            "╔═╩══════╩═╗",
            "║    imp    ║",
            "╚══════════╝",
        ];

        // Center the logo
        for line in &logo {
            if row >= area.height {
                return;
            }
            let offset = area.width.saturating_sub(line.len() as u16) / 2;
            let styled = Line::from(Span::styled(*line, self.theme.accent_style()));
            buf.set_line(center_x + offset, area.y + row, &styled, area.width);
            row += 1;
        }

        row += 1; // spacer

        let lines = [
            (
                "Welcome to imp — an AI coding agent.",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            ("", Style::default()),
            (
                "Let's get you set up. This takes about 30 seconds.",
                self.theme.muted_style(),
            ),
        ];

        for (text, style) in &lines {
            if row >= area.height {
                return;
            }
            let offset = area.width.saturating_sub(text.len() as u16) / 2;
            let line = Line::from(Span::styled(*text, *style));
            buf.set_line(center_x + offset, area.y + row, &line, area.width);
            row += 1;
        }

        // Footer
        if area.height > row + 2 {
            let footer_y = area.y + area.height - 1;
            let footer = Line::from(vec![
                Span::styled("  Enter ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Continue", self.theme.muted_style()),
                Span::raw("    "),
                Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Skip", self.theme.muted_style()),
            ]);
            buf.set_line(center_x, footer_y, &footer, area.width);
        }
    }

    fn render_provider_auth(&self, area: Rect, buf: &mut Buffer) {
        let mut row: u16 = 0;
        let x = area.x;

        // Title
        let title = Line::from(Span::styled(
            "  Choose your AI provider",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        buf.set_line(x, area.y + row, &title, area.width);
        row += 2;

        // Provider list
        for (i, status) in self.state.providers.iter().enumerate() {
            if row >= area.height.saturating_sub(4) {
                break;
            }
            let is_selected = i == self.state.provider_selected;
            let marker = if is_selected { "▸ " } else { "  " };

            let auth_hint = if status.env_detected {
                format!("  ({} detected ✓)", status.provider.env_var())
            } else if status.stored {
                "  (saved ✓)".to_string()
            } else {
                String::new()
            };

            let label_style = if is_selected {
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!("  {marker}"), self.theme.accent_style()),
                Span::styled(status.provider.label(), label_style),
                Span::styled(auth_hint, self.theme.success_style()),
            ]);
            buf.set_line(x, area.y + row, &line, area.width);
            row += 1;
        }

        row += 1;

        // API key input (only if selected provider has no auth)
        let selected = self.state.selected_provider();
        if !selected.has_auth() {
            let prompt_line =
                Line::from(vec![Span::styled("  API Key: ", self.theme.muted_style())]);
            buf.set_line(x, area.y + row, &prompt_line, area.width);
            row += 1;

            // Key input field (masked)
            let display_key = if self.state.key_input.is_empty() {
                "  ┌─ paste your key here ─────────────────┐".to_string()
            } else {
                let masked: String = self
                    .state
                    .key_input
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i < 6 { c } else { '•' })
                    .collect();
                format!(
                    "  ┌ {masked}▎{} ┐",
                    " ".repeat(40usize.saturating_sub(masked.len() + 1))
                )
            };
            let key_style = if self.state.key_input.is_empty() {
                self.theme.muted_style()
            } else {
                Style::default()
            };
            let key_line = Line::from(Span::styled(display_key, key_style));
            buf.set_line(x, area.y + row, &key_line, area.width);
            row += 1;

            // Key URL hint
            let url_line = Line::from(vec![
                Span::styled("  Get a key: ", self.theme.muted_style()),
                Span::styled(
                    selected.provider.key_url(),
                    Style::default().fg(self.theme.accent),
                ),
            ]);
            buf.set_line(x, area.y + row, &url_line, area.width);
            row += 1;

            // Error
            if let Some(ref error) = self.state.key_error {
                row += 1;
                let error_line =
                    Line::from(Span::styled(format!("  {error}"), self.theme.error_style()));
                buf.set_line(x, area.y + row, &error_line, area.width);
            }
        } else {
            let ready = Line::from(vec![
                Span::styled("  ✓ ", self.theme.success_style()),
                Span::styled("Ready to connect.", self.theme.muted_style()),
            ]);
            buf.set_line(x, area.y + row, &ready, area.width);
        }

        // Footer
        if area.height > 2 {
            let footer_y = area.y + area.height - 1;
            let footer = Line::from(vec![
                Span::styled("  Enter ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Continue", self.theme.muted_style()),
                Span::raw("    "),
                Span::styled("↑↓ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Select provider", self.theme.muted_style()),
                Span::raw("    "),
                Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Back", self.theme.muted_style()),
            ]);
            buf.set_line(x, footer_y, &footer, area.width);
        }
    }

    fn render_model_thinking(&self, area: Rect, buf: &mut Buffer) {
        let mut row: u16 = 0;
        let x = area.x;

        let title = Line::from(Span::styled(
            "  Default model & thinking level",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        buf.set_line(x, area.y + row, &title, area.width);
        row += 2;

        // Model list
        let subtitle = Line::from(Span::styled("  Model:", self.theme.muted_style()));
        buf.set_line(x, area.y + row, &subtitle, area.width);
        row += 1;

        let visible_models = 6usize;
        let start = self.state.model_selected.saturating_sub(visible_models / 2);
        let end = (start + visible_models).min(self.state.models.len());
        let start = end.saturating_sub(visible_models);

        for (display_i, model_i) in (start..end).enumerate() {
            if row >= area.height.saturating_sub(6) {
                break;
            }
            let model = &self.state.models[model_i];
            let is_selected = model_i == self.state.model_selected;
            let marker = if is_selected { "▸ " } else { "  " };

            let name_style = if is_selected {
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let context_str = format!("{}k", model.context_window / 1000);
            let price_str = format!(
                "${:.2}/{:.2}",
                model.pricing.input_per_mtok, model.pricing.output_per_mtok
            );

            let line = Line::from(vec![
                Span::styled(format!("    {marker}"), self.theme.accent_style()),
                Span::styled(format!("{:<36}", &model.name), name_style),
                Span::styled(format!("{context_str:>5}"), self.theme.muted_style()),
                Span::raw("  "),
                Span::styled(price_str, self.theme.muted_style()),
            ]);
            buf.set_line(x, area.y + row, &line, area.width);
            row += 1;
            let _ = display_i; // used for loop count
        }

        row += 1;

        // Thinking level
        let thinking_label = match self.state.thinking_level {
            ThinkingLevel::Off => "Off",
            ThinkingLevel::Minimal => "Minimal",
            ThinkingLevel::Low => "Low",
            ThinkingLevel::Medium => "Medium",
            ThinkingLevel::High => "High",
            ThinkingLevel::XHigh => "XHigh",
        };
        let thinking_line = Line::from(vec![
            Span::styled("  Thinking:  ", self.theme.muted_style()),
            Span::styled("← ", self.theme.accent_style()),
            Span::styled(
                thinking_label,
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" →", self.theme.accent_style()),
        ]);
        buf.set_line(x, area.y + row, &thinking_line, area.width);
        row += 2;

        // Hint
        let hint = Line::from(Span::styled(
            "  You can change these anytime with Ctrl+L and Shift+Tab.",
            self.theme.muted_style(),
        ));
        if row < area.height {
            buf.set_line(x, area.y + row, &hint, area.width);
        }

        // Footer
        if area.height > 2 {
            let footer_y = area.y + area.height - 1;
            let footer = Line::from(vec![
                Span::styled("  Enter ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Continue", self.theme.muted_style()),
                Span::raw("    "),
                Span::styled("↑↓ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Model", self.theme.muted_style()),
                Span::raw("    "),
                Span::styled("←→ ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Thinking", self.theme.muted_style()),
                Span::raw("    "),
                Span::styled("Esc ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Back", self.theme.muted_style()),
            ]);
            buf.set_line(x, footer_y, &footer, area.width);
        }
    }

    fn render_done(&self, area: Rect, buf: &mut Buffer) {
        let mut row: u16 = 0;
        let x = area.x;

        // Checkmark header
        let header = Line::from(Span::styled(
            "  ✓ You're all set.",
            Style::default()
                .fg(self.theme.success)
                .add_modifier(Modifier::BOLD),
        ));
        buf.set_line(x, area.y + row, &header, area.width);
        row += 2;

        // Summary
        let provider = self.state.selected_provider_id();
        let model_name = self
            .state
            .selected_model()
            .map(|m| m.name.as_str())
            .unwrap_or("default");
        let thinking_label = match self.state.thinking_level {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::XHigh => "xhigh",
        };

        let summary_lines = [
            format!("  Provider:  {}", provider.label()),
            format!("  Model:     {model_name}"),
            format!("  Thinking:  {thinking_label}"),
        ];

        for line_text in &summary_lines {
            if row >= area.height {
                return;
            }
            let line = Line::from(Span::styled(line_text.as_str(), Style::default()));
            buf.set_line(x, area.y + row, &line, area.width);
            row += 1;
        }

        row += 1;

        // Config path
        let config_hint = Line::from(Span::styled(
            "  Config saved to ~/.config/imp/config.toml",
            self.theme.muted_style(),
        ));
        if row < area.height {
            buf.set_line(x, area.y + row, &config_hint, area.width);
            row += 1;
        }

        row += 1;

        // Quick tips
        let tips_header = Line::from(Span::styled(
            "  Quick tips:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        if row < area.height {
            buf.set_line(x, area.y + row, &tips_header, area.width);
            row += 1;
        }

        let tips = [
            ("Enter", "Send a message"),
            ("Ctrl+C", "Clear / Abort / Quit"),
            ("Ctrl+L", "Switch model"),
            ("Shift+Tab", "Cycle thinking level"),
            ("@file", "Attach file context"),
            ("/command", "Slash commands"),
        ];

        for (key, desc) in &tips {
            if row >= area.height.saturating_sub(2) {
                break;
            }
            let line = Line::from(vec![
                Span::styled(format!("    {key:<12}"), self.theme.accent_style()),
                Span::styled(*desc, self.theme.muted_style()),
            ]);
            buf.set_line(x, area.y + row, &line, area.width);
            row += 1;
        }

        // Footer
        if area.height > 2 {
            let footer_y = area.y + area.height - 1;
            let footer = Line::from(vec![
                Span::styled("  Enter ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("Start using imp", self.theme.muted_style()),
            ]);
            buf.set_line(x, footer_y, &footer, area.width);
        }
    }
}
