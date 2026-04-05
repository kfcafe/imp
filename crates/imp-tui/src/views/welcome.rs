use imp_llm::auth::AuthStore;
use imp_llm::model::{ModelMeta, ProviderMeta, ProviderRegistry};
use imp_llm::ThinkingLevel;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// Which step of the welcome flow the user is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeStep {
    /// Splash / introduction.
    Welcome,
    /// Choose provider and enter API key.
    ProviderAuth,
    /// Pick default model and thinking level.
    ModelThinking,
    /// Optional web search provider setup.
    WebSearch,
    /// Summary and quick tips.
    Done,
}

const STEPS: &[WelcomeStep] = &[
    WelcomeStep::Welcome,
    WelcomeStep::ProviderAuth,
    WelcomeStep::ModelThinking,
    WelcomeStep::WebSearch,
    WelcomeStep::Done,
];

/// Detected state for each provider — whether an env var or stored credential exists.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub meta: ProviderMeta,
    pub env_detected: bool,
    pub stored: bool,
}

impl ProviderStatus {
    pub fn has_auth(&self) -> bool {
        self.env_detected || self.stored
    }
}

#[derive(Debug, Clone)]
pub struct WebProviderStatus {
    pub id: &'static str,
    pub label: &'static str,
    pub env_key: &'static str,
    pub docs_url: &'static str,
    pub env_detected: bool,
    pub stored: bool,
}

impl WebProviderStatus {
    pub fn has_auth(&self) -> bool {
        self.id == "none" || self.env_detected || self.stored
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
    /// Optional web search providers for the built-in `web` tool.
    pub web_providers: Vec<WebProviderStatus>,
    /// Selected web provider index.
    pub web_provider_selected: usize,
    /// Optional web provider key input.
    pub web_key_input: String,
    /// Resolved web provider id.
    pub resolved_web_provider: Option<String>,
    /// Resolved web provider key (if entered manually).
    pub resolved_web_key: Option<String>,
}

impl WelcomeState {
    /// Create welcome state, detecting existing auth from env vars for all registered providers.
    pub fn new(all_models: &[ModelMeta]) -> Self {
        let registry = ProviderRegistry::with_builtins();
        let auth_path = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
            .unwrap_or_else(|_| std::path::PathBuf::from(".config"))
            .join("imp")
            .join("auth.json");
        let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
        let providers: Vec<ProviderStatus> = registry
            .list()
            .iter()
            .map(|meta| {
                let env_detected = meta.env_vars.iter().any(|v| std::env::var(v).is_ok());
                ProviderStatus {
                    meta: meta.clone(),
                    env_detected,
                    stored: auth_store.stored.contains_key(meta.id),
                }
            })
            .collect();

        // Pre-select the first provider with auth, or the first provider (Anthropic) by default.
        let provider_selected = providers.iter().position(|p| p.has_auth()).unwrap_or(0);

        let selected_id = providers[provider_selected].meta.id;
        let models = filter_models_for_provider(all_models, selected_id);

        let web_providers = vec![
            WebProviderStatus {
                id: "none",
                label: "Skip for now",
                env_key: "",
                docs_url: "",
                env_detected: false,
                stored: false,
            },
            WebProviderStatus {
                id: "tavily",
                label: "Tavily",
                env_key: "TAVILY_API_KEY",
                docs_url: "https://app.tavily.com/home",
                env_detected: std::env::var("TAVILY_API_KEY").is_ok(),
                stored: auth_store.stored.contains_key("tavily"),
            },
            WebProviderStatus {
                id: "exa",
                label: "Exa",
                env_key: "EXA_API_KEY",
                docs_url: "https://dashboard.exa.ai/api-keys",
                env_detected: std::env::var("EXA_API_KEY").is_ok(),
                stored: auth_store.stored.contains_key("exa"),
            },
        ];
        let web_provider_selected = web_providers.iter().position(|p| p.has_auth()).unwrap_or(0);

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
            web_providers,
            web_provider_selected,
            web_key_input: String::new(),
            resolved_web_provider: None,
            resolved_web_key: None,
        }
    }

    /// Mark a provider as having a stored credential.
    pub fn mark_stored(&mut self, provider_id: &str) {
        for p in &mut self.providers {
            if p.meta.id == provider_id {
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

    /// Return the selected provider's id string.
    pub fn selected_provider_id(&self) -> &str {
        self.providers[self.provider_selected].meta.id
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
    }

    pub fn pop_key_char(&mut self) {
        self.key_input.pop();
    }

    /// Check whether auth is available for the current provider (env or entered key).
    pub fn check_auth_resolved(&mut self) -> Result<(), String> {
        let status = &self.providers[self.provider_selected];
        if status.has_auth() {
            self.auth_resolved = true;
            self.resolved_key = None;
            return Ok(());
        }
        if !self.key_input.trim().is_empty() {
            self.auth_resolved = true;
            self.resolved_key = Some(self.key_input.trim().to_string());
            return Ok(());
        }
        Err("Please enter an API key or set the environment variable.".into())
    }

    pub fn update_models(&mut self, all_models: &[ModelMeta]) {
        let id = self.selected_provider_id().to_string();
        self.models = filter_models_for_provider(all_models, &id);
        self.model_selected = 0;
    }

    pub fn selected_web_provider(&self) -> &WebProviderStatus {
        &self.web_providers[self.web_provider_selected]
    }

    pub fn web_provider_up(&mut self) {
        if self.web_provider_selected > 0 {
            self.web_provider_selected -= 1;
            self.on_web_provider_changed();
        }
    }

    pub fn web_provider_down(&mut self) {
        if self.web_provider_selected + 1 < self.web_providers.len() {
            self.web_provider_selected += 1;
            self.on_web_provider_changed();
        }
    }

    pub fn push_web_key_char(&mut self, c: char) {
        self.web_key_input.push(c);
    }

    pub fn pop_web_key_char(&mut self) {
        self.web_key_input.pop();
    }

    pub fn check_web_auth_resolved(&mut self) -> Result<(), String> {
        let (provider_id, has_auth) = {
            let status = self.selected_web_provider();
            (status.id.to_string(), status.has_auth())
        };
        self.resolved_web_provider = Some(provider_id.clone());
        if provider_id == "none" {
            self.resolved_web_key = None;
            return Ok(());
        }
        if has_auth {
            self.resolved_web_key = None;
            return Ok(());
        }
        if !self.web_key_input.trim().is_empty() {
            self.resolved_web_key = Some(self.web_key_input.trim().to_string());
            return Ok(());
        }
        Err("Enter a web search API key or choose Skip for now.".into())
    }

    fn on_provider_changed(&mut self) {
        self.key_input.clear();
        self.key_editing = false;
        self.auth_resolved = false;
        self.resolved_key = None;
    }

    fn on_web_provider_changed(&mut self) {
        self.web_key_input.clear();
        self.resolved_web_key = None;
        self.resolved_web_provider = None;
    }
}

fn filter_models_for_provider(all_models: &[ModelMeta], provider_id: &str) -> Vec<ModelMeta> {
    all_models
        .iter()
        .filter(|m| m.provider == provider_id)
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

    // Check if any registered provider has auth via env var.
    let registry = ProviderRegistry::with_builtins();
    let has_env = registry
        .list()
        .iter()
        .any(|meta| meta.env_vars.iter().any(|v| std::env::var(v).is_ok()));

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
            WelcomeStep::WebSearch => self.render_web_search(inner, buf),
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
                // Show the first env var that is set, or the primary one
                let detected_var = status
                    .meta
                    .env_vars
                    .iter()
                    .find(|v| std::env::var(v).is_ok())
                    .copied()
                    .unwrap_or(status.meta.env_vars.first().copied().unwrap_or(""));
                format!("  ({} detected ✓)", detected_var)
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
                Span::styled(status.meta.name, label_style),
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

            // Key URL hint — use the provider's docs_url
            let url_line = Line::from(vec![
                Span::styled("  Get a key: ", self.theme.muted_style()),
                Span::styled(
                    selected.meta.docs_url,
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

    fn render_web_search(&self, area: Rect, buf: &mut Buffer) {
        let mut row: u16 = 0;
        let x = area.x;

        let title = Line::from(Span::styled(
            "  Optional web search setup",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        buf.set_line(x, area.y + row, &title, area.width);
        row += 1;

        let subtitle = Line::from(Span::styled(
            "  Add Tavily or Exa now so the web tool can search immediately.",
            self.theme.muted_style(),
        ));
        buf.set_line(x, area.y + row, &subtitle, area.width);
        row += 2;

        for (i, provider) in self.state.web_providers.iter().enumerate() {
            if row >= area.height.saturating_sub(6) {
                break;
            }
            let is_selected = i == self.state.web_provider_selected;
            let marker = if is_selected { "▸ " } else { "  " };
            let mut status = String::new();
            if provider.id == "none" {
                status = "  (skip)".to_string();
            } else if provider.env_detected {
                status = format!("  ({} detected ✓)", provider.env_key);
            } else if provider.stored {
                status = "  (saved ✓)".to_string();
            }
            let label_style = if is_selected {
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::styled(format!("  {marker}"), self.theme.accent_style()),
                Span::styled(provider.label, label_style),
                Span::styled(status, self.theme.success_style()),
            ]);
            buf.set_line(x, area.y + row, &line, area.width);
            row += 1;
        }

        row += 1;
        let selected = self.state.selected_web_provider();
        if selected.id != "none" && !selected.has_auth() {
            let prompt_line =
                Line::from(vec![Span::styled("  API Key: ", self.theme.muted_style())]);
            buf.set_line(x, area.y + row, &prompt_line, area.width);
            row += 1;

            let display_key = if self.state.web_key_input.is_empty() {
                "  ┌─ paste your key here ─────────────────┐".to_string()
            } else {
                let masked: String = self
                    .state
                    .web_key_input
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i < 6 { c } else { '•' })
                    .collect();
                format!(
                    "  ┌ {masked}▎{} ┐",
                    " ".repeat(40usize.saturating_sub(masked.len() + 1))
                )
            };
            let key_style = if self.state.web_key_input.is_empty() {
                self.theme.muted_style()
            } else {
                Style::default()
            };
            let key_line = Line::from(Span::styled(display_key, key_style));
            buf.set_line(x, area.y + row, &key_line, area.width);
            row += 1;

            let url_line = Line::from(vec![
                Span::styled("  Get a key: ", self.theme.muted_style()),
                Span::styled(selected.docs_url, Style::default().fg(self.theme.accent)),
            ]);
            buf.set_line(x, area.y + row, &url_line, area.width);
        } else if selected.id == "none" {
            let ready = Line::from(vec![
                Span::styled("  ↷ ", self.theme.muted_style()),
                Span::styled(
                    "Skipping web search setup for now.",
                    self.theme.muted_style(),
                ),
            ]);
            buf.set_line(x, area.y + row, &ready, area.width);
        } else {
            let ready = Line::from(vec![
                Span::styled("  ✓ ", self.theme.success_style()),
                Span::styled("Web search provider is ready.", self.theme.muted_style()),
            ]);
            buf.set_line(x, area.y + row, &ready, area.width);
        }

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

        // Summary — use the provider's human-readable name from ProviderMeta
        let provider_name = self.state.providers[self.state.provider_selected].meta.name;
        let web_provider_name = self
            .state
            .resolved_web_provider
            .as_deref()
            .filter(|id| *id != "none")
            .map(|id| {
                self.state
                    .web_providers
                    .iter()
                    .find(|provider| provider.id == id)
                    .map(|provider| provider.label)
                    .unwrap_or(id)
            })
            .unwrap_or("not configured");
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
            format!("  Provider:  {provider_name}"),
            format!("  Model:     {model_name}"),
            format!("  Thinking:  {thinking_label}"),
            format!("  Web:       {web_provider_name}"),
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
