use std::collections::HashMap;
use std::path::{Path, PathBuf};

use imp_llm::ThinkingLevel;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::guardrails::GuardrailConfig;
use crate::hooks::HookDef;
use crate::personality::PersonalityConfig;
use crate::roles::RoleDef;
use crate::tools::web::types::WebConfig;

/// Agent mode — controls which tools and mana actions the agent may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AgentMode {
    /// Default. Full access to all tools. No filtering.
    #[default]
    Full,
    /// Unit executor. Read + write + bash. No mana create/run.
    Worker,
    /// Plans and executes via mana. Cannot touch files directly.
    Orchestrator,
    /// Decomposes work. Can read and create mana units. Cannot run them.
    Planner,
    /// Read-only inspector. No mutations, no mana.
    Reviewer,
    /// Batch inspector. Reads code and mana state, produces reports.
    Auditor,
}

impl AgentMode {
    /// Tool names this mode permits. An empty slice means "allow all" (Full).
    pub fn allowed_tool_names(&self) -> &'static [&'static str] {
        match self {
            AgentMode::Full => &[],
            AgentMode::Worker => &[
                "read",
                "scan",
                "web",
                "session_search",
                "write",
                "edit",
                "multi_edit",
                "bash",
                "mana",
                "memory",
                "ask",
            ],
            AgentMode::Orchestrator => &[
                "read",
                "scan",
                "web",
                "session_search",
                "mana",
                "ask",
            ],
            AgentMode::Planner => &[
                "read",
                "scan",
                "web",
                "session_search",
                "mana",
                "ask",
            ],
            AgentMode::Reviewer => &[
                "read",
                "scan",
                "web",
                "session_search",
                "ask",
            ],
            AgentMode::Auditor => &[
                "read",
                "scan",
                "web",
                "session_search",
                "mana",
            ],
        }
    }

    /// Returns true if the mode allows the named tool.
    pub fn allows_tool(&self, name: &str) -> bool {
        match self {
            AgentMode::Full => true,
            _ => self.allowed_tool_names().contains(&name),
        }
    }

    /// Mana sub-actions this mode permits. An empty slice means "allow all" (Full).
    pub fn allowed_mana_actions(&self) -> &'static [&'static str] {
        match self {
            AgentMode::Full => &[],
            AgentMode::Worker => &["show", "update", "status", "list", "logs", "next"],
            AgentMode::Orchestrator => &[
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
            ],
            AgentMode::Planner => &["status", "list", "show", "create", "next"],
            AgentMode::Reviewer => &[],
            AgentMode::Auditor => &["status", "list", "show", "logs", "agents", "next"],
        }
    }

    /// Returns true if the mode allows the named mana action.
    pub fn allows_mana_action(&self, action: &str) -> bool {
        match self {
            AgentMode::Full => true,
            AgentMode::Reviewer => false,
            _ => self.allowed_mana_actions().contains(&action),
        }
    }

    /// Parse a mode from a string name (e.g. `"worker"`, `"full"`).
    ///
    /// Returns `None` for unrecognised names. Used to read `IMP_MODE` from the
    /// environment without requiring a full `FromStr` implementation.
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "full" => Some(AgentMode::Full),
            "worker" => Some(AgentMode::Worker),
            "orchestrator" => Some(AgentMode::Orchestrator),
            "planner" => Some(AgentMode::Planner),
            "reviewer" => Some(AgentMode::Reviewer),
            "auditor" => Some(AgentMode::Auditor),
            _ => None,
        }
    }

    /// Mode-specific behavioral instruction for the system prompt, if any.
    pub fn instructions(&self) -> Option<&'static str> {
        match self {
            AgentMode::Full => None,
            AgentMode::Worker => Some(
                "You are a worker agent. Your job is to complete the assigned unit. \
                You may read files, write files, and run shell commands. \
                You may not create or run mana units — use `mana update` to report progress.",
            ),
            AgentMode::Orchestrator => Some(
                "You are an orchestrator agent. Your job is to plan and execute work \
                by creating and running mana units. You may not read or write files directly — \
                delegate all file work to worker agents via mana.",
            ),
            AgentMode::Planner => Some(
                "You are a planner agent. Your job is to decompose work into mana units. \
                You may read files and create units, but you may not run them — \
                a human or orchestrator will approve execution.",
            ),
            AgentMode::Reviewer => Some(
                "You are a reviewer agent. Your job is to read code and report findings. \
                You may not write files, run commands, or use mana.",
            ),
            AgentMode::Auditor => Some(
                "You are an auditor agent. Your job is to inspect code and mana state \
                and produce structured reports. You may read files and mana status, \
                but you may not modify anything.",
            ),
        }
    }
}

/// Shell backend selection for the Bash tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ShellBackend {
    /// Use `sh -c` (default, always available).
    #[default]
    Sh,
    /// Use the rush library API (`rush::run`). Falls back to `sh` if
    /// the `rush-backend` feature is not compiled in.
    Rush,
    /// Connect to a running rush daemon over Unix socket. Falls back to `sh`
    /// if the daemon is not reachable.
    RushDaemon,
}

/// Shell-related configuration for the Bash tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellConfig {
    /// Which shell backend to use. Defaults to `"sh"`.
    #[serde(default)]
    pub backend: ShellBackend,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            backend: ShellBackend::Sh,
        }
    }
}

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Default model (alias or full ID).
    pub model: Option<String>,

    /// Default thinking level.
    pub thinking: Option<ThinkingLevel>,

    /// Maximum agent turns.
    pub max_turns: Option<u32>,

    /// Active tool names (None = all).
    pub tools: Option<Vec<String>>,

    /// Named roles.
    #[serde(default)]
    pub roles: HashMap<String, RoleDef>,

    /// Hook definitions.
    #[serde(default)]
    pub hooks: Vec<HookDef>,

    /// Context management settings.
    #[serde(default)]
    pub context: ContextConfig,

    /// Shell backend settings.
    #[serde(default)]
    pub shell: ShellConfig,

    /// Engineering guardrails — profile-aware guidance and post-write checks.
    #[serde(default)]
    pub guardrails: GuardrailConfig,

    /// Agent mode — controls tool and mana action access.
    #[serde(default)]
    pub mode: AgentMode,

    /// Enabled models for the model selector (None = show all).
    /// Entries can be canonical IDs or aliases (e.g. "sonnet", "claude-sonnet-4-6").
    #[serde(default)]
    pub enabled_models: Option<Vec<String>>,

    /// Theme name ("default", "light", or custom).
    pub theme: Option<String>,

    /// Learning loop settings (memory, skill nudges).
    #[serde(default)]
    pub learning: LearningConfig,

    /// UI display settings.
    #[serde(default)]
    pub ui: UiConfig,

    /// Web tool settings.
    #[serde(default)]
    pub web: WebConfig,

    /// Personality settings, including identity sentence and saved profiles.
    #[serde(default)]
    pub personality: PersonalityConfig,
}

// ── UI configuration ────────────────────────────────────────────

/// How the sidebar displays tool calls.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarStyle {
    /// Chronological stream of tool calls with inline results.
    #[default]
    Stream,
    /// Master-detail split: tool list (top) + selected output (bottom).
    Split,
}

/// How much tool output to show per tool call.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolOutputDisplay {
    /// Show all output lines (scrollable).
    Full,
    /// Show first N lines per tool (configurable via `tool_output_lines`).
    #[default]
    Compact,
    /// Headers only — expand on click/enter.
    Collapsed,
}

/// How tool calls appear inside the chat transcript.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatToolDisplay {
    /// Show tool calls inline where they occurred, preserving chronological order.
    #[default]
    Interleaved,
    /// Show a compact header in chat and leave details to the sidebar.
    Summary,
    /// Hide tool calls in chat entirely.
    Hidden,
}

/// UI animation intensity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnimationLevel {
    /// No animated motion; show static state labels only.
    None,
    /// Basic spinner-only motion.
    Spinner,
    /// Restrained motion with concise state-specific labels.
    #[default]
    #[serde(alias = "full")]
    Minimal,
}

/// UI display configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiConfig {
    /// Sidebar layout style.
    #[serde(default)]
    pub sidebar_style: SidebarStyle,

    /// How much tool output to show.
    #[serde(default)]
    pub tool_output: ToolOutputDisplay,

    /// Max lines per tool in compact mode. Default: 10.
    #[serde(default = "default_tool_output_lines")]
    pub tool_output_lines: usize,

    /// Max lines the read tool returns before truncating. 0 disables line
    /// truncation for file reads. Default: 500.
    #[serde(default = "default_read_max_lines")]
    pub read_max_lines: usize,

    /// Sidebar width as percentage of screen (20-80). Default: 40.
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,

    /// Word-wrap long lines in tool output. Default: true.
    #[serde(default = "default_true")]
    pub word_wrap: bool,

    /// Animation intensity for the TUI. Default: minimal.
    #[serde(default)]
    pub animations: AnimationLevel,

    /// Legacy compatibility flag for older configs. Prefer `chat_tool_display`.
    #[serde(default)]
    pub hide_tools_in_chat: bool,

    /// How tool calls should appear in the chat transcript.
    #[serde(default)]
    pub chat_tool_display: ChatToolDisplay,

    /// Auto-open the sidebar on the first tool call. Default: true.
    #[serde(default = "default_true")]
    pub auto_open_sidebar: bool,

    /// Minimum terminal width to auto-open sidebar. Default: 120.
    #[serde(default = "default_sidebar_auto_open_width")]
    pub sidebar_auto_open_width: u16,

    /// Number of thinking lines to show in the rolling tail. Default: 5.
    #[serde(default = "default_thinking_lines")]
    pub thinking_lines: usize,

    /// Number of streaming tool output lines to retain. Default: 5.
    #[serde(default = "default_streaming_lines")]
    pub streaming_lines: usize,

    /// Mouse wheel scroll speed in lines. Default: 3.
    #[serde(default = "default_mouse_scroll_lines")]
    pub mouse_scroll_lines: usize,

    /// Keyboard/page scroll speed in lines. Default: 20.
    #[serde(default = "default_keyboard_scroll_lines")]
    pub keyboard_scroll_lines: usize,

    /// Deprecated: mouse capture is now always enabled. This field is retained
    /// only for backwards-compatible deserialization of existing config files.
    #[serde(default)]
    #[doc(hidden)]
    pub mouse_capture: bool,

    /// Show timestamps in chat. Default: false.
    #[serde(default)]
    pub show_timestamps: bool,

    /// Show cost in the top bar. Default: true.
    #[serde(default = "default_true")]
    pub show_cost: bool,

    /// Show context usage in the top bar. Default: true.
    #[serde(default = "default_true")]
    pub show_context_usage: bool,

    /// Emit a terminal bell when an agent run fully completes in the TUI.
    /// Default: true.
    #[serde(default = "default_true")]
    pub notify_on_agent_complete: bool,
}

fn default_tool_output_lines() -> usize {
    10
}
fn default_read_max_lines() -> usize {
    500
}
fn default_sidebar_width() -> u16 {
    40
}
fn default_sidebar_auto_open_width() -> u16 {
    120
}
fn default_thinking_lines() -> usize {
    5
}
fn default_streaming_lines() -> usize {
    5
}
fn default_mouse_scroll_lines() -> usize {
    3
}
fn default_keyboard_scroll_lines() -> usize {
    20
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_style: SidebarStyle::default(),
            tool_output: ToolOutputDisplay::default(),
            tool_output_lines: 10,
            read_max_lines: 500,
            sidebar_width: 40,
            word_wrap: true,
            animations: AnimationLevel::Minimal,
            hide_tools_in_chat: false,
            chat_tool_display: ChatToolDisplay::default(),
            auto_open_sidebar: true,
            sidebar_auto_open_width: 120,
            thinking_lines: 5,
            streaming_lines: 5,
            mouse_scroll_lines: 3,
            keyboard_scroll_lines: 20,
            mouse_capture: false,
            show_timestamps: false,
            show_cost: true,
            show_context_usage: true,
            notify_on_agent_complete: true,
        }
    }
}

impl UiConfig {
    pub fn effective_chat_tool_display(&self) -> ChatToolDisplay {
        if self.hide_tools_in_chat {
            ChatToolDisplay::Hidden
        } else {
            self.chat_tool_display
        }
    }
}

/// Learning loop configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearningConfig {
    /// Master switch for memory + skill nudges. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Tool call count before suggesting skill creation. Default: 8.
    #[serde(default = "default_nudge_threshold")]
    pub skill_nudge_threshold: u32,

    /// Character limit for memory.md. Default: 2200.
    #[serde(default = "default_memory_limit")]
    pub memory_char_limit: usize,

    /// Character limit for user.md. Default: 1400.
    #[serde(default = "default_user_limit")]
    pub user_char_limit: usize,
}

fn default_true() -> bool {
    true
}
fn default_nudge_threshold() -> u32 {
    8
}
fn default_memory_limit() -> usize {
    2200
}
fn default_user_limit() -> usize {
    1400
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            skill_nudge_threshold: 8,
            memory_char_limit: 2200,
            user_char_limit: 1400,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextConfig {
    /// Mask old tool outputs at this ratio (default: 0.6).
    pub observation_mask_threshold: f64,

    /// Keep last N turns unmasked (default: 10).
    pub mask_window: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            observation_mask_threshold: 0.6,
            mask_window: 10,
        }
    }
}

impl Config {
    /// Load config from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Resolve the full config by merging: defaults < user < project < env < CLI.
    pub fn resolve(user_config_dir: &Path, project_dir: Option<&Path>) -> Result<Self> {
        let mut config = Self::default();

        // User config
        let user_path = user_config_dir.join("config.toml");
        if user_path.exists() {
            let user = Self::load(&user_path)?;
            config.merge(user);
        }

        // Project config
        if let Some(project) = project_dir {
            let project_path = project.join(".imp").join("config.toml");
            if project_path.exists() {
                let project = Self::load(&project_path)?;
                config.merge(project);
            }
        }

        // Env overrides
        if let Ok(model) = std::env::var("IMP_MODEL") {
            config.model = Some(model);
        }
        if let Ok(thinking) = std::env::var("IMP_THINKING") {
            config.thinking = parse_thinking_level(&thinking);
        }
        if let Ok(mode) = std::env::var("IMP_MODE") {
            if let Some(m) = parse_agent_mode(&mode) {
                config.mode = m;
            }
        }
        if let Ok(provider) = std::env::var("IMP_WEB_PROVIDER") {
            config.web.search_provider = match provider.to_lowercase().as_str() {
                "tavily" => Some(crate::tools::web::types::SearchProvider::Tavily),
                "exa" => Some(crate::tools::web::types::SearchProvider::Exa),
                "linkup" => Some(crate::tools::web::types::SearchProvider::Linkup),
                "perplexity" => Some(crate::tools::web::types::SearchProvider::Perplexity),
                _ => config.web.search_provider,
            };
        }

        Ok(config)
    }

    fn merge(&mut self, other: Config) {
        if other.model.is_some() {
            self.model = other.model;
        }
        if other.thinking.is_some() {
            self.thinking = other.thinking;
        }
        if other.max_turns.is_some() {
            self.max_turns = other.max_turns;
        }
        if other.tools.is_some() {
            self.tools = other.tools;
        }
        if other.context != ContextConfig::default() {
            self.context = other.context;
        }
        if other.shell != ShellConfig::default() {
            self.shell = other.shell;
        }
        self.guardrails.merge(other.guardrails);
        if other.mode != AgentMode::default() {
            self.mode = other.mode;
        }
        if other.enabled_models.is_some() {
            self.enabled_models = other.enabled_models;
        }
        if other.ui != UiConfig::default() {
            self.ui = other.ui;
        }
        if other.web != WebConfig::default() {
            self.web = other.web;
        }
        if other.personality != PersonalityConfig::default() {
            self.personality.merge(other.personality);
        }
        self.roles.extend(other.roles);
        self.hooks.extend(other.hooks);
    }

    /// Default user config directory.
    pub fn user_config_dir() -> PathBuf {
        dirs_path("config")
    }

    /// Default session directory.
    pub fn session_dir() -> PathBuf {
        dirs_path("data").join("sessions")
    }

    /// Save config to a TOML file. Creates parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| crate::error::Error::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Path to the user config.toml file.
    pub fn user_config_path() -> PathBuf {
        Self::user_config_dir().join("config.toml")
    }
}

fn dirs_path(kind: &str) -> PathBuf {
    match kind {
        "config" => {
            if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
                PathBuf::from(dir).join("imp")
            } else if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".config").join("imp")
            } else {
                PathBuf::from(".config").join("imp")
            }
        }
        "data" => {
            if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
                PathBuf::from(dir).join("imp")
            } else if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".local").join("share").join("imp")
            } else {
                PathBuf::from(".local").join("share").join("imp")
            }
        }
        _ => PathBuf::from("."),
    }
}

fn parse_agent_mode(s: &str) -> Option<AgentMode> {
    match s.to_lowercase().as_str() {
        "full" => Some(AgentMode::Full),
        "worker" => Some(AgentMode::Worker),
        "orchestrator" => Some(AgentMode::Orchestrator),
        "planner" => Some(AgentMode::Planner),
        "reviewer" => Some(AgentMode::Reviewer),
        "auditor" => Some(AgentMode::Auditor),
        _ => None,
    }
}

fn parse_thinking_level(s: &str) -> Option<ThinkingLevel> {
    match s.to_lowercase().as_str() {
        "off" => Some(ThinkingLevel::Off),
        "minimal" => Some(ThinkingLevel::Minimal),
        "low" => Some(ThinkingLevel::Low),
        "medium" => Some(ThinkingLevel::Medium),
        "high" => Some(ThinkingLevel::High),
        "xhigh" => Some(ThinkingLevel::XHigh),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn config_default_values() {
        let config = Config::default();
        assert!(config.model.is_none());
        assert!(config.thinking.is_none());
        assert!(config.max_turns.is_none());
        assert!(config.tools.is_none());
        assert_eq!(config.ui.read_max_lines, 500);
        assert_eq!(config.web, WebConfig::default());
        assert_eq!(config.personality, PersonalityConfig::default());
        assert!(config.roles.is_empty());
        assert!(config.hooks.is_empty());
        assert!((config.context.observation_mask_threshold - 0.6).abs() < f64::EPSILON);
        assert_eq!(config.context.mask_window, 10);
        assert_eq!(config.guardrails, GuardrailConfig::default());
    }

    #[test]
    fn config_load_from_toml() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
model = "sonnet"
thinking = "high"
max_turns = 50
tools = ["read", "write", "bash"]

[guardrails]
enabled = true
level = "enforce"
profile = "zig"
critical_paths = ["src/**"]
after_write = ["zig fmt --check ."]

[context]
observation_mask_threshold = 0.5
mask_window = 5

[web]
search_provider = "exa"
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.model.as_deref(), Some("sonnet"));
        assert_eq!(config.thinking, Some(ThinkingLevel::High));
        assert_eq!(config.max_turns, Some(50));
        assert_eq!(config.tools.as_ref().unwrap().len(), 3);
        assert_eq!(config.guardrails.enabled, Some(true));
        assert_eq!(config.ui.read_max_lines, 500);
        assert_eq!(
            config.guardrails.profile,
            Some(crate::guardrails::GuardrailProfile::Zig)
        );
        assert_eq!(
            config.guardrails.after_write,
            Some(vec!["zig fmt --check .".into()])
        );
        assert_eq!(
            config.web.search_provider,
            Some(crate::tools::web::types::SearchProvider::Exa)
        );
        assert!((config.context.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.context.mask_window, 5);
    }

    #[test]
    fn config_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("nonexistent.toml");
        let config = Config::load(&config_path).unwrap();
        assert!(config.model.is_none());
    }

    #[test]
    fn config_loads_personality_section() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
[personality.profile.identity]
name = "Nova"
work_style = "careful"
voice = "clear"
focus = "research"
role = "assistant"

[personality.profile.sliders]
autonomy = "low"
verbosity = "high"
caution = "very-high"
warmth = "high"
planning_depth = "very-high"

[personality.profiles]
active = "researcher"

[personality.profiles.saved.researcher.identity]
name = "Nova"
work_style = "careful"
voice = "clear"
focus = "research"
role = "assistant"
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.personality.profile.identity.name, "Nova");
        assert_eq!(
            config.personality.profile.identity.render_sentence(),
            "You are Nova, a careful, clear, research assistant."
        );
        assert_eq!(
            config.personality.profiles.active.as_deref(),
            Some("researcher")
        );
        assert!(config.personality.profiles.saved.contains_key("researcher"));
    }

    #[test]
    fn config_merge_personality_project_overrides_user_and_keeps_saved_profiles() {
        let mut user = Config::default();
        user.personality.profile.identity.name = "imp".into();
        user.personality.profiles.active = Some("builder".into());
        user.personality.profiles.saved.insert(
            "builder".into(),
            crate::personality::PersonalityProfile::default(),
        );

        let mut project = Config::default();
        project.personality.profile.identity.name = "Patch".into();
        project.personality.profiles.active = Some("reviewer".into());
        project.personality.profiles.saved.insert(
            "reviewer".into(),
            crate::personality::PersonalityProfile::default(),
        );

        user.merge(project);

        assert_eq!(user.personality.profile.identity.name, "Patch");
        assert_eq!(
            user.personality.profiles.active.as_deref(),
            Some("reviewer")
        );
        assert!(user.personality.profiles.saved.contains_key("builder"));
        assert!(user.personality.profiles.saved.contains_key("reviewer"));
    }

    #[test]
    fn config_merge_project_overrides_user() {
        let mut user = Config {
            model: Some("haiku".into()),
            max_turns: Some(20),
            ..Default::default()
        };

        let project = Config {
            model: Some("sonnet".into()),
            max_turns: None, // not set → user value preserved
            ..Default::default()
        };

        user.merge(project);
        assert_eq!(user.model.as_deref(), Some("sonnet"));
        assert_eq!(user.max_turns, Some(20));
    }

    #[test]
    fn config_merge_roles_extend() {
        let mut base = Config::default();
        base.roles.insert(
            "worker".into(),
            RoleDef {
                model: Some("haiku".into()),
                thinking: None,
                tools: None,
                readonly: false,
                instructions: None,
                max_turns: None,
            },
        );

        let overlay = Config {
            roles: {
                let mut m = HashMap::new();
                m.insert(
                    "reviewer".into(),
                    RoleDef {
                        model: Some("sonnet".into()),
                        thinking: Some(ThinkingLevel::High),
                        tools: None,
                        readonly: true,
                        instructions: None,
                        max_turns: None,
                    },
                );
                m
            },
            ..Default::default()
        };

        base.merge(overlay);
        assert!(base.roles.contains_key("worker"));
        assert!(base.roles.contains_key("reviewer"));
    }

    #[test]
    fn config_merge_hooks_extend() {
        let mut base = Config::default();
        base.hooks.push(HookDef {
            event: "after_file_write".into(),
            match_pattern: None,
            action: "log".into(),
            command: None,
            blocking: false,
            threshold: None,
        });

        let overlay = Config {
            hooks: vec![HookDef {
                event: "before_tool_call".into(),
                match_pattern: None,
                action: "block".into(),
                command: None,
                blocking: true,
                threshold: None,
            }],
            ..Default::default()
        };

        base.merge(overlay);
        assert_eq!(base.hooks.len(), 2);
    }

    #[test]
    fn config_merge_context_overrides_default() {
        let mut base = Config::default();

        let overlay = Config {
            context: ContextConfig {
                observation_mask_threshold: 0.5,
                mask_window: 5,
            },
            ..Default::default()
        };

        base.merge(overlay);
        assert!((base.context.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(base.context.mask_window, 5);
    }

    #[test]
    fn config_merge_guardrails_preserves_unspecified_fields() {
        let mut base = Config::default();
        base.guardrails.enabled = Some(true);
        base.guardrails.profile = Some(crate::guardrails::GuardrailProfile::Rust);
        base.guardrails.critical_paths = Some(vec!["src/**".into()]);

        let mut overlay = Config::default();
        overlay.guardrails.level = Some(crate::guardrails::GuardrailLevel::Enforce);
        overlay.guardrails.after_write = Some(vec!["cargo test".into()]);

        base.merge(overlay);

        assert_eq!(base.guardrails.enabled, Some(true));
        assert_eq!(
            base.guardrails.profile,
            Some(crate::guardrails::GuardrailProfile::Rust)
        );
        assert_eq!(base.guardrails.critical_paths, Some(vec!["src/**".into()]));
        assert_eq!(
            base.guardrails.level,
            Some(crate::guardrails::GuardrailLevel::Enforce)
        );
        assert_eq!(base.guardrails.after_write, Some(vec!["cargo test".into()]));
    }

    #[test]
    fn config_resolve_user_then_project() {
        // Clean env to avoid interference from parallel tests
        std::env::remove_var("IMP_MODEL");
        std::env::remove_var("IMP_THINKING");

        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("user");
        let project_dir = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(project_dir.join(".imp")).unwrap();

        // User config: model=haiku, max_turns=20, custom context
        fs::write(
            user_dir.join("config.toml"),
            r#"
model = "haiku"
max_turns = 20

[context]
observation_mask_threshold = 0.55
mask_window = 9
"#,
        )
        .unwrap();

        // Project config: model=sonnet (overrides user), custom context overrides user context
        fs::write(
            project_dir.join(".imp").join("config.toml"),
            r#"
model = "sonnet"

[context]
observation_mask_threshold = 0.5
mask_window = 5
"#,
        )
        .unwrap();

        let config = Config::resolve(&user_dir, Some(&project_dir)).unwrap();
        assert_eq!(config.model.as_deref(), Some("sonnet"));
        assert_eq!(config.max_turns, Some(20));
        assert!((config.context.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.context.mask_window, 5);
    }

    #[test]
    fn config_resolve_env_overrides() {
        // Test env override logic without relying on process-global state
        // (env vars are inherently racy in parallel tests).
        // We test that the override *mechanism* works by simulating it.
        let mut config = Config {
            model: Some("haiku".into()),
            thinking: Some(ThinkingLevel::Low),
            ..Default::default()
        };

        // Simulate IMP_MODEL override
        let env_model = "opus";
        config.model = Some(env_model.into());

        // Simulate IMP_THINKING override
        let env_thinking = "high";
        config.thinking = parse_thinking_level(env_thinking);

        assert_eq!(config.model.as_deref(), Some("opus"));
        assert_eq!(config.thinking, Some(ThinkingLevel::High));
    }

    #[test]
    fn config_resolve_missing_files_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let config = Config::resolve(dir.path(), None).unwrap();
        assert!(config.model.is_none());
        assert!(config.thinking.is_none());
        assert!(config.max_turns.is_none());
    }

    #[test]
    fn config_load_with_roles_and_hooks() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
model = "sonnet"

[roles.coder]
model = "opus"
thinking = "high"
readonly = false

[roles.reader]
readonly = true

[[hooks]]
event = "after_file_write"
action = "log"
blocking = false
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.roles.len(), 2);
        assert!(config.roles.contains_key("coder"));
        assert!(config.roles.contains_key("reader"));
        assert_eq!(config.roles["coder"].model.as_deref(), Some("opus"));
        assert!(config.roles["reader"].readonly);
        assert_eq!(config.hooks.len(), 1);
        assert_eq!(config.hooks[0].event, "after_file_write");
    }

    #[test]
    fn config_parse_thinking_levels() {
        assert_eq!(parse_thinking_level("off"), Some(ThinkingLevel::Off));
        assert_eq!(
            parse_thinking_level("minimal"),
            Some(ThinkingLevel::Minimal)
        );
        assert_eq!(parse_thinking_level("low"), Some(ThinkingLevel::Low));
        assert_eq!(parse_thinking_level("medium"), Some(ThinkingLevel::Medium));
        assert_eq!(parse_thinking_level("high"), Some(ThinkingLevel::High));
        assert_eq!(parse_thinking_level("xhigh"), Some(ThinkingLevel::XHigh));
        assert_eq!(parse_thinking_level("OFF"), Some(ThinkingLevel::Off));
        assert_eq!(parse_thinking_level("High"), Some(ThinkingLevel::High));
        assert_eq!(parse_thinking_level("invalid"), None);
        assert_eq!(parse_thinking_level(""), None);
    }

    #[test]
    fn config_partial_toml_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
model = "sonnet"
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.model.as_deref(), Some("sonnet"));
        // Unspecified fields use defaults
        assert!(config.thinking.is_none());
        assert!(config.max_turns.is_none());
        assert!((config.context.observation_mask_threshold - 0.6).abs() < f64::EPSILON);
    }

    // --- AgentMode tests ---

    #[test]
    fn agent_mode_default_is_full() {
        let config = Config::default();
        assert_eq!(config.mode, AgentMode::Full);
        assert_eq!(AgentMode::default(), AgentMode::Full);
    }

    #[test]
    fn agent_mode_full_allows_all_tools() {
        let mode = AgentMode::Full;
        assert!(mode.allows_tool("anything"));
        assert!(mode.allows_tool("read"));
        assert!(mode.allows_tool("bash"));
        assert!(mode.allows_tool("nonexistent_future_tool"));
        assert_eq!(mode.allowed_tool_names(), &[] as &[&str]);
    }

    #[test]
    fn agent_mode_orchestrator_allows_read() {
        let mode = AgentMode::Orchestrator;
        assert!(mode.allows_tool("read"));
        assert!(mode.allows_tool("scan"));
        assert!(mode.allows_tool("web"));
        assert!(mode.allows_tool("session_search"));
        assert!(mode.allows_tool("mana"));
        assert!(mode.allows_tool("ask"));
    }

    #[test]
    fn agent_mode_orchestrator_blocks_write() {
        let mode = AgentMode::Orchestrator;
        assert!(!mode.allows_tool("write"));
        assert!(!mode.allows_tool("edit"));
        assert!(!mode.allows_tool("multi_edit"));
        assert!(!mode.allows_tool("bash"));
    }

    #[test]
    fn agent_mode_planner_allows_mana_create() {
        let mode = AgentMode::Planner;
        assert!(mode.allows_mana_action("create"));
        assert!(mode.allows_mana_action("status"));
        assert!(mode.allows_mana_action("list"));
        assert!(mode.allows_mana_action("show"));
    }

    #[test]
    fn agent_mode_planner_blocks_mana_close() {
        let mode = AgentMode::Planner;
        assert!(!mode.allows_mana_action("close"));
        assert!(!mode.allows_mana_action("run"));
        assert!(!mode.allows_mana_action("update"));
    }

    #[test]
    fn agent_mode_worker_blocks_mana_create() {
        let mode = AgentMode::Worker;
        assert!(!mode.allows_mana_action("create"));
        assert!(!mode.allows_mana_action("run"));
        assert!(!mode.allows_mana_action("close"));
    }

    #[test]
    fn agent_mode_worker_allows_mana_update() {
        let mode = AgentMode::Worker;
        assert!(mode.allows_mana_action("update"));
        assert!(mode.allows_mana_action("show"));
        assert!(mode.allows_mana_action("status"));
        assert!(mode.allows_mana_action("list"));
    }

    #[test]
    fn agent_mode_reviewer_no_mana() {
        let mode = AgentMode::Reviewer;
        assert!(!mode.allows_mana_action("status"));
        assert!(!mode.allows_mana_action("list"));
        assert!(!mode.allows_mana_action("show"));
        assert!(!mode.allows_mana_action("create"));
        assert!(!mode.allows_mana_action("run"));
        // Reviewer also has no mana tool access
        assert!(!mode.allows_tool("mana"));
    }

    #[test]
    fn agent_mode_auditor_mana_readonly() {
        let mode = AgentMode::Auditor;
        assert!(mode.allows_mana_action("status"));
        assert!(mode.allows_mana_action("list"));
        assert!(mode.allows_mana_action("show"));
        assert!(!mode.allows_mana_action("create"));
        assert!(!mode.allows_mana_action("close"));
        assert!(!mode.allows_mana_action("run"));
        assert!(!mode.allows_mana_action("update"));
    }

    #[test]
    fn agent_mode_config_deserialize() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, r#"mode = "orchestrator""#).unwrap();
        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.mode, AgentMode::Orchestrator);
    }

    #[test]
    fn agent_mode_instructions() {
        assert!(AgentMode::Full.instructions().is_none());
        assert!(AgentMode::Worker.instructions().is_some());
        assert!(AgentMode::Orchestrator.instructions().is_some());
        assert!(AgentMode::Planner.instructions().is_some());
        assert!(AgentMode::Reviewer.instructions().is_some());
        assert!(AgentMode::Auditor.instructions().is_some());

        // Spot-check content is mode-specific
        let worker = AgentMode::Worker.instructions().unwrap();
        assert!(worker.contains("worker"));
        let reviewer = AgentMode::Reviewer.instructions().unwrap();
        assert!(reviewer.contains("reviewer") || reviewer.contains("read"));
    }
}
