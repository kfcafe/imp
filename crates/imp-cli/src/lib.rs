use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStage {
    ProcessStart,
    CwdResolved,
    ConfigResolved,
    SessionReady,
    AuthLoaded,
    ModelRegistryReady,
    ModelResolved,
    ProviderReady,
    ApiKeyResolved,
    AgentBuilt,
    PromptReady,
    RunLoopStarted,
}

impl StartupStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProcessStart => "process_start",
            Self::CwdResolved => "cwd_resolved",
            Self::ConfigResolved => "config_resolved",
            Self::SessionReady => "session_ready",
            Self::AuthLoaded => "auth_loaded",
            Self::ModelRegistryReady => "model_registry_ready",
            Self::ModelResolved => "model_resolved",
            Self::ProviderReady => "provider_ready",
            Self::ApiKeyResolved => "api_key_resolved",
            Self::AgentBuilt => "agent_built",
            Self::PromptReady => "prompt_ready",
            Self::RunLoopStarted => "run_loop_started",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupTiming {
    pub stage: StartupStage,
    pub since_start_ms: u64,
    pub since_previous_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadlessOutputMode {
    Json,
    Human,
}

#[derive(Debug)]
struct StartupTimer {
    started_at: std::time::Instant,
    last_mark_at: std::time::Instant,
    enabled: bool,
}

impl StartupTimer {
    fn new(enabled: bool) -> Self {
        let now = std::time::Instant::now();
        Self {
            started_at: now,
            last_mark_at: now,
            enabled,
        }
    }

    fn mark(&mut self, stage: StartupStage) -> Option<StartupTiming> {
        if !self.enabled {
            return None;
        }
        let now = std::time::Instant::now();
        let timing = StartupTiming {
            stage,
            since_start_ms: now.duration_since(self.started_at).as_millis() as u64,
            since_previous_ms: now.duration_since(self.last_mark_at).as_millis() as u64,
        };
        self.last_mark_at = now;
        Some(timing)
    }
}

use async_trait::async_trait;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::ffi::OsString;
use imp_core::agent::{Agent, AgentCommand, AgentEvent, AgentHandle};
use imp_core::config::{AnimationLevel, Config, ToolOutputDisplay};
use imp_core::format_error_for_display;
use imp_core::tools::web::types::SearchProvider;

use imp_core::imp_session::{
    resolve_runtime_connection, ImpSession, ResolvedRuntimeConnection, RuntimeConnectionIntent,
    SessionChoice, SessionOptions,
};
use imp_core::personality::{
    default_soul_markdown, generated_tunable_line, replace_tunable_line, soul_identity_text,
    tunable_state_for_label, SoulTunableState,
};
use imp_core::resources::{discover_project_soul, suggested_project_soul_path};
use imp_core::session::{SessionEntry, SessionManager};
use imp_core::ui::{ComponentSpec, NotifyLevel, SelectOption, UserInterface, WidgetContent};
use imp_core::usage::{UsageCostBreakdown, UsageRecordSource, UsageTokens};
use imp_core::TimingEvent;
use imp_llm::auth::{AuthStore, StoredCredential};
use imp_llm::model::{ModelRegistry, ProviderMeta, ProviderRegistry};
use imp_llm::oauth::anthropic::AnthropicOAuth;
use imp_llm::oauth::chatgpt::ChatGptOAuth;
use imp_llm::oauth::kimi_code::KimiCodeOAuth;
use imp_llm::provider::ThinkingLevel;
use imp_llm::providers::create_provider;
use imp_llm::{truncate_chars_with_suffix, Message, Model, StreamEvent};
use imp_tui::animation::{activity_label as tui_activity_label, ActivitySurface, AnimationState};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

mod usage_report;

#[derive(Debug)]
struct ShellLiveness {
    enabled: bool,
    tick: u64,
    visible_len: usize,
}

impl ShellLiveness {
    fn new() -> Self {
        Self {
            enabled: io::stderr().is_terminal(),
            tick: 0,
            visible_len: 0,
        }
    }

    fn render(&mut self, state: AnimationState) {
        if !self.enabled {
            return;
        }
        let label = tui_activity_label(
            state,
            self.tick,
            AnimationLevel::Minimal,
            ActivitySurface::Chat,
        );
        self.tick = self.tick.wrapping_add(1);
        if label.is_empty() {
            self.clear();
            return;
        }
        let padded = if self.visible_len > label.len() {
            format!(
                "{label}{:width$}",
                "",
                width = self.visible_len - label.len()
            )
        } else {
            label.clone()
        };
        eprint!("\r{padded}");
        io::stderr().flush().ok();
        self.visible_len = label.len();
    }

    fn clear(&mut self) {
        if !self.enabled || self.visible_len == 0 {
            return;
        }
        eprint!("\r{:width$}\r", "", width = self.visible_len);
        io::stderr().flush().ok();
        self.visible_len = 0;
    }

    fn settle_line(&mut self) {
        if !self.enabled || self.visible_len == 0 {
            return;
        }
        eprintln!();
        self.visible_len = 0;
    }
}

fn shell_animation_state(
    printed_thinking: bool,
    printed_any_text: bool,
    active_tools: u32,
) -> AnimationState {
    if active_tools > 0 {
        AnimationState::ExecutingTools { active_tools }
    } else if printed_any_text {
        AnimationState::Streaming
    } else if printed_thinking {
        AnimationState::Thinking
    } else {
        AnimationState::WaitingForResponse
    }
}

fn bump_shell_liveness(
    liveness: &mut ShellLiveness,
    printed_thinking: bool,
    printed_any_text: bool,
    active_tools: u32,
) {
    let state = shell_animation_state(printed_thinking, printed_any_text, active_tools);
    liveness.render(state);
}

fn clear_shell_liveness_for_output(
    liveness: &mut ShellLiveness,
    printed_trailing_newline: &mut bool,
) {
    if liveness.visible_len > 0 {
        liveness.settle_line();
        *printed_trailing_newline = true;
    }
}

/// A coding agent engine
#[derive(Parser)]
#[command(name = "imp", version, about)]
struct Cli {
    /// Print response and exit (non-interactive mode)
    #[arg(short, long)]
    print: Option<String>,

    /// LLM provider (anthropic, openai, google)
    #[arg(long)]
    provider: Option<String>,

    /// Model to use (alias or full ID)
    #[arg(short, long)]
    model: Option<String>,

    /// Thinking level: off, minimal, low, medium, high, xhigh
    #[arg(long)]
    thinking: Option<String>,

    /// API key override
    #[arg(long)]
    api_key: Option<String>,

    /// Continue most recent session
    #[arg(short, long)]
    #[clap(name = "continue")]
    cont: bool,

    /// Browse and select a session to resume
    #[arg(short, long)]
    resume: bool,

    /// Use a specific session file
    #[arg(long)]
    session: Option<PathBuf>,

    /// Ephemeral mode (no session persistence)
    #[arg(long)]
    no_session: bool,

    /// Enable specific tools (comma-separated)
    #[arg(long)]
    tools: Option<String>,

    /// Disable all built-in tools
    #[arg(long)]
    no_tools: bool,

    /// Replace default system prompt
    #[arg(long)]
    system_prompt: Option<String>,

    /// Output mode: interactive, rpc, json
    #[arg(long, default_value = "interactive")]
    mode: String,

    /// Maximum turns before stopping (default: 50)
    #[arg(long)]
    max_turns: Option<u32>,

    /// Max output tokens per response
    #[arg(long)]
    max_tokens: Option<u32>,

    /// Verbose startup logging
    #[arg(long)]
    verbose: bool,

    /// List available models
    #[arg(long)]
    list_models: bool,

    /// File arguments (@file includes file content)
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args, Debug, Clone)]
struct HeadlessManaArgs {
    /// Mana unit ID to run
    unit_id: String,
    /// Explicit path to the .mana directory for canonical unit loading
    #[arg(long)]
    mana_dir: Option<PathBuf>,
    /// Defer verify/close to compatibility orchestrators instead of verifying inline
    #[arg(long)]
    defer_verify: bool,
}

#[derive(Args, Debug, Clone)]
struct ManaNamespaceArgs {
    /// Mana operator verb or unit ID
    target: String,
    /// Additional arguments for reserved mana namespace verbs
    args: Vec<String>,
    /// Explicit path to the .mana directory for canonical unit loading
    #[arg(long)]
    mana_dir: Option<PathBuf>,
    /// Defer verify/close to compatibility orchestrators instead of verifying inline
    #[arg(long)]
    defer_verify: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the CLI-first interactive chat shell
    Chat,
    /// Open the fullscreen terminal UI explicitly
    Tui,
    /// Open the viewer/inspector surface (planned; not fully implemented yet)
    View {
        /// Viewer area to open (planned: sessions, tree, logs, checkpoints)
        area: Option<String>,
    },
    /// Edit a guided subset of imp settings in the terminal
    Settings,
    /// Edit personality/soul settings in the terminal
    Personality,
    /// Run the terminal-native setup wizard
    Setup,
    /// Log in to a provider. OAuth is supported for Anthropic, OpenAI/ChatGPT, and Kimi Code.
    Login {
        /// Provider to configure (`anthropic`, `openai`, `kimi`, or `kimi-code`). Defaults to anthropic.
        provider: Option<String>,
    },
    /// Save, list, or remove API credentials in secure imp auth storage
    Secrets {
        #[command(subcommand)]
        command: Option<SecretsCommand>,
        /// Provider/service to configure (e.g. tavily, exa, resend, my-service)
        provider: Option<String>,
    },
    /// Edit configuration
    Config,
    /// Enter the mana-aware operator namespace. Use `imp mana <unit-id>` to run one unit.
    Mana(ManaNamespaceArgs),
    /// Compatibility alias for `imp mana <unit-id>` during migration.
    #[command(hide = true)]
    Run(HeadlessManaArgs),
    /// Usage reporting and export
    Usage {
        #[command(subcommand)]
        command: UsageCommand,
    },
    /// Import skills and config from other agents (pi, Claude Code, Codex)
    Import {
        /// Only detect — don't copy anything
        #[arg(long)]
        dry_run: bool,
        /// Import from a specific agent: pi, claude, codex
        #[arg(long)]
        from: Option<String>,
        /// Skip the confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Install this build to the user-visible `imp` command path
    InstallLocal {
        /// Print the chosen install destination without writing it
        #[arg(long)]
        dry_run: bool,
        /// Explicit install destination path
        #[arg(long)]
        dest: Option<PathBuf>,
    },
    /// Save a web search provider API key into imp auth storage
    WebLogin {
        /// Search provider to configure (tavily, exa, linkup, perplexity)
        provider: String,
    },
}

#[derive(Subcommand, Debug)]
enum SecretsCommand {
    /// List configured secret providers/services
    List,
    /// Alias for list
    Ls,
    /// Show metadata for one configured provider/service
    Show {
        /// Provider/service to inspect
        provider: String,
    },
    /// Alias for show
    Inspect {
        /// Provider/service to inspect
        provider: String,
    },
    /// Remove a configured provider/service from secure storage
    Remove {
        /// Provider/service to remove
        provider: String,
    },
    /// Alias for remove
    Rm {
        /// Provider/service to remove
        provider: String,
    },
    /// Configure or update a provider/service's secret fields
    Set {
        /// Provider/service to configure (e.g. tavily, exa, resend, my-service)
        provider: String,
    },
}

#[derive(Subcommand, Debug)]
enum UsageCommand {
    /// Show overall usage totals
    Summary(UsageReportArgs),
    /// Show usage grouped by day
    Daily(UsageReportArgs),
    /// Show usage grouped by model
    Models(UsageReportArgs),
    /// Show usage grouped by session
    Sessions(UsageReportArgs),
    /// Export usage records in a machine-friendly format
    Export(UsageExportArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
enum UsageExportFormat {
    Json,
}

#[derive(Debug, Clone, Args)]
struct UsageReportArgs {
    /// Include records on or after this unix timestamp or YYYY-MM-DD date
    #[arg(long)]
    since: Option<String>,
    /// Include records before this unix timestamp or date
    #[arg(long)]
    until: Option<String>,
    /// Only include this provider
    #[arg(long)]
    provider: Option<String>,
    /// Only include this model
    #[arg(long)]
    model: Option<String>,
    /// Only include this session id or path fragment
    #[arg(long)]
    session: Option<String>,
    /// Emit JSON instead of a human table when supported
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct UsageExportArgs {
    #[command(flatten)]
    filters: UsageReportArgs,
    /// Export format
    #[arg(long, value_enum, default_value_t = UsageExportFormat::Json)]
    format: UsageExportFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsageReportKind {
    Summary,
    Daily,
    Models,
    Sessions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum UsageGroupKind {
    Day,
    Model,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundKind {
    Since,
    Until,
}

#[derive(Debug, Clone)]
struct UsageFilters {
    since: Option<u64>,
    until: Option<u64>,
    provider: Option<String>,
    model: Option<String>,
    session: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct UsageTotalsRow {
    requests: usize,
    tokens: UsageTokens,
    cost: UsageCostBreakdown,
}

#[derive(Debug, Clone, Serialize)]
struct UsageGroupRow {
    group: String,
    group_kind: UsageGroupKind,
    provider: Option<String>,
    model: Option<String>,
    session_id: Option<String>,
    session_path: Option<String>,
    day: Option<String>,
    totals: UsageTotalsRow,
}

#[derive(Debug, Clone, Serialize)]
struct UsageSessionSummary {
    session_id: Option<String>,
    session_path: Option<String>,
    messages: usize,
    first_timestamp: Option<u64>,
    last_timestamp: Option<u64>,
    first_day: Option<String>,
    last_day: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UsageFilterSummary {
    since: Option<u64>,
    until: Option<u64>,
    provider: Option<String>,
    model: Option<String>,
    session: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UsageSummaryJson {
    report: &'static str,
    generated_at: u64,
    filters: UsageFilterSummary,
    totals: UsageTotalsRow,
    sessions: usize,
    providers: usize,
    models: usize,
    canonical_records: usize,
    legacy_records: usize,
}

#[derive(Debug, Clone, Serialize)]
struct UsageGroupedJson {
    report: &'static str,
    generated_at: u64,
    filters: UsageFilterSummary,
    totals: UsageTotalsRow,
    rows: Vec<UsageGroupRow>,
}

#[derive(Debug, Clone, Serialize)]
struct UsageExportJson {
    report: &'static str,
    generated_at: u64,
    filters: UsageFilterSummary,
    totals: UsageTotalsRow,
    records: Vec<UsageExportRecord>,
}

#[derive(Debug, Clone, Serialize)]
struct UsageExportRecord {
    request_id: String,
    recorded_at: u64,
    day: String,
    provider: Option<String>,
    model: Option<String>,
    session: UsageSessionSummary,
    source: UsageRecordSource,
    tokens: UsageTokens,
    cost: Option<UsageCostBreakdown>,
    assistant_message_id: Option<String>,
    turn_index: Option<u32>,
    entry_id: String,
    parent_id: Option<String>,
}

fn split_path_entries(path: Option<OsString>) -> Vec<PathBuf> {
    path.as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .collect()
}

fn find_imp_on_path_from(path: Option<OsString>) -> Option<PathBuf> {
    split_path_entries(path)
        .into_iter()
        .map(|dir| dir.join("imp"))
        .find(|candidate| candidate.is_file())
}

fn path_contains_dir(path: Option<OsString>, dir: &std::path::Path) -> bool {
    split_path_entries(path).into_iter().any(|entry| entry == dir)
}

fn preferred_user_install_path(home: &std::path::Path, path: Option<OsString>) -> PathBuf {
    let home_bin = home.join("bin");
    if path_contains_dir(path.clone(), &home_bin) {
        return home_bin.join("imp");
    }

    let local_bin = home.join(".local/bin");
    if path_contains_dir(path.clone(), &local_bin) {
        return local_bin.join("imp");
    }

    home.join(".cargo/bin/imp")
}

fn resolve_install_destination(
    home: &std::path::Path,
    path: Option<OsString>,
    active_imp: Option<PathBuf>,
    dest_override: Option<PathBuf>,
) -> PathBuf {
    if let Some(dest) = dest_override {
        return dest;
    }

    if let Some(active) = active_imp {
        if active.starts_with(home) {
            return active;
        }
    }

    preferred_user_install_path(home, path)
}

fn install_binary_to(source: &std::path::Path, dest: &std::path::Path) -> io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Install destination has no parent: {}", dest.display()),
        )
    })?;
    std::fs::create_dir_all(parent)?;

    let temp = dest.with_extension("tmp");
    std::fs::copy(source, &temp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&temp, perms)?;
    }
    std::fs::rename(&temp, dest)?;
    Ok(())
}

fn run_install_local(dest_override: Option<PathBuf>, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    let path_env = std::env::var_os("PATH");
    let active_imp = find_imp_on_path_from(path_env.clone());
    let dest = resolve_install_destination(&home, path_env, active_imp.clone(), dest_override);

    if dry_run {
        println!("{}", dest.display());
        return Ok(());
    }

    install_binary_to(&current_exe, &dest)?;

    println!("Installed imp to {}", dest.display());
    if let Some(previous) = active_imp {
        if previous != dest {
            println!(
                "Updated active-user install target instead of Cargo bin shadow path. Previous `imp` path was {}.",
                previous.display()
            );
        }
    }

    let resolved_after = find_imp_on_path_from(std::env::var_os("PATH"));
    match resolved_after {
        Some(path) if path == dest => {
            println!("`imp` now resolves to {}", path.display());
        }
        Some(path) => {
            println!(
                "Installed to {}, but `imp` still resolves to {}. Adjust PATH or rerun with --dest {}.",
                dest.display(),
                path.display(),
                path.display()
            );
        }
        None => {
            println!(
                "Installed to {}, but `imp` is not currently on PATH. Add {} to PATH.",
                dest.display(),
                dest.parent().map(|p| p.display().to_string()).unwrap_or_default()
            );
        }
    }

    Ok(())
}

pub async fn run() {
    let cli = Cli::parse();

    // Dispatch subcommands first
    if let Some(command) = &cli.command {
        match command {
            Commands::Chat => {
                if let Err(e) = run_chat_mode(&cli).await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Tui => {
                if let Err(e) = run_interactive(&cli).await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::View { area } => {
                if let Err(e) = run_view_mode(&cli, area.as_deref()).await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Settings => {
                if let Err(e) = run_settings_mode() {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Personality => {
                if let Err(e) = run_personality_mode() {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Setup => {
                if let Err(e) = run_setup_mode().await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Login { provider } => {
                let provider_name = provider.as_deref().unwrap_or("anthropic");
                if let Err(e) = run_login(provider_name).await {
                    eprintln!("Login failed: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Secrets { command, provider } => {
                if let Err(e) = run_secrets_command(command.as_ref(), provider.as_deref()).await {
                    eprintln!("Secrets command failed: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Config => {
                let config_dir = Config::user_config_dir();
                let config_path = config_dir.join("config.toml");
                println!("{}", config_path.display());
                return;
            }
            Commands::Mana(ManaNamespaceArgs {
                target,
                args,
                mana_dir,
                defer_verify,
            }) => match target.as_str() {
                "status" | "show" | "logs" | "run" => {
                    if let Err(e) = run_reserved_mana_namespace_command(target, args).await {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                    return;
                }
                _ => {
                    if !args.is_empty() {
                        eprintln!(
                            "Error: unexpected extra arguments after mana unit id `{target}`: {}",
                            args.join(" ")
                        );
                        std::process::exit(1);
                    }
                    match run_headless_mode(&cli, target, mana_dir.as_deref(), *defer_verify).await
                    {
                        Ok(true) => return,
                        Ok(false) => std::process::exit(1),
                        Err(e) => {
                            eprintln!("Error: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            },
            Commands::Run(HeadlessManaArgs {
                unit_id,
                mana_dir,
                defer_verify,
            }) => {
                match run_headless_mode(&cli, unit_id, mana_dir.as_deref(), *defer_verify).await {
                    Ok(true) => return,
                    Ok(false) => std::process::exit(1),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Commands::Usage { command } => {
                if let Err(e) = usage_report::run_usage_command(command) {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::Import { dry_run, from, yes } => {
                run_import(*dry_run, from.as_deref(), *yes);
                return;
            }
            Commands::InstallLocal { dry_run, dest } => {
                if let Err(e) = run_install_local(dest.clone(), *dry_run) {
                    eprintln!("Install failed: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Commands::WebLogin { provider } => {
                if let Err(e) = run_web_login(provider).await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
                return;
            }
        }
    }

    // List models
    if cli.list_models {
        run_list_models();
        return;
    }

    if cli.mode == "chat" {
        if let Err(e) = run_chat_mode(&cli).await {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Expand @file args into file content context
    let file_context = expand_file_args(&cli.args);

    // Read from stdin if piped
    let stdin_content = {
        if !std::io::stdin().is_terminal() {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok();
            if buf.is_empty() {
                None
            } else {
                Some(buf)
            }
        } else {
            None
        }
    };

    // Print mode
    if let Some(ref prompt) = cli.print {
        let full_prompt = build_full_prompt(prompt, &file_context, &stdin_content);
        if let Err(e) = run_print_mode(&cli, &full_prompt).await {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // If stdin was piped without -p, run in print mode with stdin as prompt
    if let Some(ref stdin) = stdin_content {
        let remaining: Vec<&str> = cli.args.iter().map(|s| s.as_str()).collect();
        let instruction = if remaining.is_empty() {
            String::new()
        } else {
            remaining.join(" ")
        };
        let full_prompt = build_full_prompt(&instruction, &file_context, &Some(stdin.clone()));
        if let Err(e) = run_print_mode(&cli, &full_prompt).await {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Default interactive mode: fullscreen TUI
    if cli.mode == "interactive" {
        if let Err(e) = run_interactive(&cli).await {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // RPC / JSON modes (JSON-lines stdin/stdout protocol)
    match cli.mode.as_str() {
        "rpc" | "json" => {
            if let Err(e) = run_rpc_mode(&cli).await {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("Unknown mode: {other}. Use interactive, chat, rpc, or json.");
            std::process::exit(1);
        }
    }
}

fn format_price(price: f64) -> String {
    if price == 0.0 {
        "n/a".to_string()
    } else {
        format!("${price:.2}")
    }
}

fn run_list_models() {
    let registry = ModelRegistry::with_builtins();
    let models = registry.list();

    println!(
        "{:<40} {:<12} {:>8} {:>10} {:>10}",
        "MODEL", "PROVIDER", "CONTEXT", "$/M IN", "$/M OUT"
    );
    println!("{}", "-".repeat(84));

    for m in models {
        println!(
            "{:<40} {:<12} {:>7}k {:>10} {:>10}",
            m.id,
            m.provider,
            m.context_window / 1000,
            format_price(m.pricing.input_per_mtok),
            format_price(m.pricing.output_per_mtok),
        );
    }
}

fn oauth_login_success_message(auth_store: &AuthStore, provider: &str) -> String {
    auth_store
        .oauth_display_info(provider)
        .map(|info| info.login_message(provider))
        .unwrap_or_else(|| format!("Logged in to {provider} successfully."))
}

fn provider_alias(name: &str) -> String {
    match name.trim().to_lowercase().as_str() {
        "kimi" => "moonshot".to_string(),
        other => other.to_string(),
    }
}

fn kimi_api_login_success_message(auth_store: &AuthStore) -> String {
    let registry = ProviderRegistry::with_builtins();
    let provider = registry
        .find("moonshot")
        .expect("moonshot provider should exist");

    let auth_kind = match auth_store.stored.get("moonshot") {
        Some(StoredCredential::SecretFields { fields })
            if fields.len() == 1 && fields.first().map(String::as_str) == Some("api_key") =>
        {
            "API key"
        }
        Some(StoredCredential::ApiKey { .. }) => "API key",
        Some(StoredCredential::SecretFields { .. }) => "credentials",
        Some(StoredCredential::OAuth(_)) => "credentials",
        None => "credentials",
    };

    format!(
        "Configured {} in imp's secure auth store using a {}. You can now run `imp -m kimi` or `imp -m kimi-k2.6`.",
        provider.name, auth_kind
    )
}

fn search_provider_from_name(name: &str) -> Option<SearchProvider> {
    match name.trim().to_lowercase().as_str() {
        "tavily" => Some(SearchProvider::Tavily),
        "exa" => Some(SearchProvider::Exa),
        "linkup" => Some(SearchProvider::Linkup),
        "perplexity" => Some(SearchProvider::Perplexity),
        _ => None,
    }
}

fn search_provider_docs_url(provider: SearchProvider) -> &'static str {
    match provider {
        SearchProvider::Tavily => "https://app.tavily.com/home",
        SearchProvider::Exa => "https://dashboard.exa.ai/api-keys",
        SearchProvider::Linkup => "https://app.linkup.so/api-keys",
        SearchProvider::Perplexity => "https://www.perplexity.ai/settings/api",
    }
}

fn parse_secret_field_names(input: &str) -> Vec<String> {
    let names: Vec<String> = input
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect();
    if names.is_empty() {
        vec!["api_key".to_string()]
    } else {
        names
    }
}

fn prompt_for_secret_fields(
    _provider_name: &str,
    display_name: &str,
    docs_hint: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    if docs_hint.is_empty() {
        eprintln!("Saving credentials for {display_name}.");
    } else {
        eprintln!("Saving credentials for {display_name}. Get them at: {docs_hint}");
    }

    eprintln!("Field names (comma-separated) [api_key]:");
    eprint!("> ");
    io::stdout().flush().ok();
    let mut field_input = String::new();
    std::io::stdin().read_line(&mut field_input)?;
    let field_names = parse_secret_field_names(&field_input);

    let mut fields = HashMap::new();
    for field in field_names {
        eprintln!("Enter {field}:");
        eprint!("> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let value = input.trim().to_string();
        if value.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("No value entered for {field}. Aborting."),
            )
            .into());
        }
        fields.insert(field, value);
    }

    Ok(fields)
}

async fn run_web_login(provider_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let provider = search_provider_from_name(provider_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Unknown web provider: {provider_name}. Use one of: tavily, exa, linkup, perplexity"
            ),
        )
    })?;

    let auth_path = imp_core::storage::global_auth_path();
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    let _env_key = provider.env_key_name();
    let fields = prompt_for_secret_fields(
        provider.name(),
        provider.name(),
        search_provider_docs_url(provider),
    )?;

    auth_store.store_secret_fields(provider.name(), fields)?;
    eprintln!(
        "Credentials saved for {} in secure imp auth storage (metadata: {}).",
        provider.name(),
        auth_path.display()
    );
    eprintln!(
        "The web tool will now auto-detect {} without requiring an exported env var.",
        provider.name()
    );

    Ok(())
}

async fn run_secrets_command(
    command: Option<&SecretsCommand>,
    provider: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Some(SecretsCommand::List) | Some(SecretsCommand::Ls) => run_secrets_list(),
        Some(SecretsCommand::Show { provider }) | Some(SecretsCommand::Inspect { provider }) => {
            run_secrets_show(provider)
        }
        Some(SecretsCommand::Remove { provider }) | Some(SecretsCommand::Rm { provider }) => {
            run_secrets_remove(provider)
        }
        Some(SecretsCommand::Set { provider }) => run_secrets_login(provider).await,
        None => {
            let provider = provider.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: imp secrets <provider> | imp secrets list | imp secrets show <provider> | imp secrets rm <provider>",
                )
            })?;
            run_secrets_login(provider).await
        }
    }
}

fn run_secrets_list() -> Result<(), Box<dyn std::error::Error>> {
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let auth_path = imp_core::storage::global_auth_path();
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));

    if auth_store.stored.is_empty() {
        println!("No saved credentials.");
        return Ok(());
    }

    let registry = ProviderRegistry::with_builtins();
    let mut rows: Vec<(String, String, String)> = auth_store
        .stored
        .iter()
        .map(|(name, entry)| {
            let display_name = registry
                .find(name)
                .map(|meta| meta.name.to_string())
                .unwrap_or_else(|| name.clone());
            let kind = match entry {
                StoredCredential::OAuth(_) => "oauth".to_string(),
                StoredCredential::ApiKey { .. } => "api_key".to_string(),
                StoredCredential::SecretFields { fields } => {
                    if fields.len() == 1 && fields.first().map(String::as_str) == Some("api_key") {
                        "api_key".to_string()
                    } else {
                        format!("{} fields", fields.len())
                    }
                }
            };
            let fields = match entry {
                StoredCredential::OAuth(_) => "access_token".to_string(),
                StoredCredential::ApiKey { .. } => "api_key".to_string(),
                StoredCredential::SecretFields { fields } => fields.join(", "),
            };
            (name.clone(), display_name, kind + "|" + &fields)
        })
        .collect();

    rows.sort_by(|a, b| a.0.cmp(&b.0));

    let provider_w = rows
        .iter()
        .map(|(id, display, _)| format!("{} ({})", display, id).len())
        .max()
        .unwrap_or(8)
        .max("Provider".len());
    let kind_w = rows
        .iter()
        .filter_map(|(_, _, payload)| payload.split_once('|').map(|(kind, _)| kind.len()))
        .max()
        .unwrap_or(4)
        .max("Kind".len());

    println!(
        "{:<provider_w$}  {:<kind_w$}  {}",
        "Provider",
        "Kind",
        "Fields",
        provider_w = provider_w,
        kind_w = kind_w
    );
    println!(
        "{:-<provider_w$}  {:-<kind_w$}  {:-<6}",
        "",
        "",
        "",
        provider_w = provider_w,
        kind_w = kind_w
    );

    for (id, display_name, payload) in rows {
        let (kind, fields) = payload.split_once('|').unwrap_or(("", ""));
        println!(
            "{:<provider_w$}  {:<kind_w$}  {}",
            format!("{} ({})", display_name, id),
            kind,
            fields,
            provider_w = provider_w,
            kind_w = kind_w
        );
    }

    Ok(())
}

fn run_secrets_show(provider: &str) -> Result<(), Box<dyn std::error::Error>> {
    let provider = provider_alias(provider);
    let auth_path = imp_core::storage::global_auth_path();
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
    let registry = ProviderRegistry::with_builtins();

    let entry = auth_store.stored.get(&provider).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("No saved credentials for {provider}."),
        )
    })?;

    let display_name = registry
        .find(&provider)
        .map(|meta| meta.name.to_string())
        .unwrap_or_else(|| provider.to_string());

    let (kind, fields): (&str, Vec<String>) = match entry {
        StoredCredential::OAuth(_) => ("oauth", vec!["access_token".into()]),
        StoredCredential::ApiKey { .. } => ("api_key", vec!["api_key".into()]),
        StoredCredential::SecretFields { fields } => ("secret_fields", fields.clone()),
    };

    println!("Provider : {} ({})", display_name, provider);
    println!("Kind     : {}", kind);
    println!("Fields   : {}", fields.join(", "));
    println!("Storage  : secure keychain + auth metadata");
    println!("Values   : hidden");
    Ok(())
}

fn run_secrets_remove(provider: &str) -> Result<(), Box<dyn std::error::Error>> {
    let provider = provider_alias(provider);
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let auth_path = imp_core::storage::global_auth_path();
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
    auth_store.remove(&provider)?;
    eprintln!("Removed saved credentials for {provider}.");
    Ok(())
}

async fn run_secrets_login(provider_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let auth_path = imp_core::storage::global_auth_path();
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    let canonical_provider = provider_alias(provider_name);
    let registry = ProviderRegistry::with_builtins();
    let provider_meta = registry.find(&canonical_provider);
    let display_name = provider_meta
        .map(|p| p.name)
        .unwrap_or(&canonical_provider);
    let docs_hint = provider_meta.map(|p| p.docs_url).unwrap_or("");

    let fields = prompt_for_secret_fields(&canonical_provider, display_name, docs_hint)?;
    auth_store.store_secret_fields(&canonical_provider, fields)?;
    eprintln!("Credentials saved for {display_name}.");
    Ok(())
}

/// Try to import existing Kimi CLI OAuth credentials from `~/.kimi/credentials/kimi-code.json`.
fn try_import_kimi_cli_credentials() -> Option<imp_llm::auth::OAuthCredential> {
    let path = std::path::PathBuf::from(
        std::env::var_os("HOME")?
    ).join(".kimi").join("credentials").join("kimi-code.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let access_token = json["access_token"].as_str()?.to_string();
    let refresh_token = json["refresh_token"].as_str()?.to_string();
    let expires_at = json["expires_at"].as_f64()? as u64;
    Some(imp_llm::auth::OAuthCredential {
        access_token,
        refresh_token,
        expires_at,
    })
}

async fn run_login(provider_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let auth_path = imp_core::storage::global_auth_path();
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    let canonical_provider = provider_alias(provider_name);
    // For login, "kimi" maps to the Kimi Code OAuth flow rather than Moonshot API key.
    let login_provider = if provider_name.trim().eq_ignore_ascii_case("kimi") {
        "kimi-code"
    } else {
        &canonical_provider
    };

    if login_provider == "anthropic" {
        let oauth = AnthropicOAuth::new();

        eprintln!("Opening browser for Anthropic login...");
        eprintln!("If the browser doesn't open, visit the URL printed below.");

        let credential = oauth
            .login(
                |url| {
                    eprintln!("\n{url}\n");
                    let _ = open_url(url);
                },
                || async {
                    eprintln!("Paste the authorization code or redirect URL:");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).ok()?;
                    let trimmed = input.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                },
            )
            .await?;

        auth_store.store(
            "anthropic",
            imp_llm::auth::StoredCredential::OAuth(credential),
        )?;
        eprintln!("{}", oauth_login_success_message(&auth_store, "anthropic"));
    } else if login_provider == "openai" || login_provider == "openai-codex" {
        let oauth = ChatGptOAuth::new();

        eprintln!("Opening browser for OpenAI / ChatGPT login...");
        eprintln!("If the browser doesn't open, visit the URL printed below.");

        let credential = oauth
            .login(
                |url| {
                    eprintln!("\n{url}\n");
                    let _ = open_url(url);
                },
                || async {
                    eprintln!("Paste the authorization code or redirect URL:");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).ok()?;
                    let trimmed = input.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                },
            )
            .await?;

        auth_store.store(
            "openai",
            imp_llm::auth::StoredCredential::OAuth(credential.clone()),
        )?;
        auth_store.store(
            "openai-codex",
            imp_llm::auth::StoredCredential::OAuth(credential),
        )?;
        eprintln!(
            "{}",
            oauth_login_success_message(&auth_store, "openai-codex")
        );
    } else if login_provider == "kimi-code" {
        // Try importing existing kimi-cli credentials first.
        if let Some(credential) = try_import_kimi_cli_credentials() {
            auth_store.store(
                "kimi-code",
                imp_llm::auth::StoredCredential::OAuth(credential),
            )?;
            eprintln!("Imported Kimi Code credentials from kimi-cli.");
            eprintln!("{}", oauth_login_success_message(&auth_store, "kimi-code"));
            return Ok(());
        }

        let oauth = KimiCodeOAuth::new();

        eprintln!("Opening browser for Kimi Code login...");
        eprintln!("If the browser doesn't open, visit the URL printed below.");

        let credential = oauth
            .login(
                |url| {
                    eprintln!("\n{url}\n");
                    let _ = open_url(url);
                },
                |msg| {
                    eprintln!("{msg}");
                },
            )
            .await?;

        auth_store.store(
            "kimi-code",
            imp_llm::auth::StoredCredential::OAuth(credential),
        )?;
        eprintln!("{}", oauth_login_success_message(&auth_store, "kimi-code"));
    } else if canonical_provider == "moonshot" {
        let registry = ProviderRegistry::with_builtins();
        let provider = registry
            .find("moonshot")
            .expect("moonshot provider should exist");

        eprintln!("Kimi uses a generated API key, not an OAuth browser flow in imp.");
        eprintln!(
            "Open {} to create a key from Moonshot / Kimi or Kimi Code, then paste it below.",
            provider.docs_url
        );
        let _ = open_url(&format!("https://{}", provider.docs_url));

        let value = prompt_input_line("api_key> ")?;
        if value.trim().is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No api_key entered. Aborting.").into());
        }
        auth_store.store_secret_fields(
            "moonshot",
            HashMap::from([("api_key".to_string(), value)]),
        )?;
        eprintln!("{}", kimi_api_login_success_message(&auth_store));
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "`imp login {provider_name}` is not supported. Use one of: anthropic, openai, kimi. For API-only providers, use `imp secrets {}`.",
                provider_alias(provider_name)
            ),
        )
        .into());
    }

    Ok(())
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()?;
    }
    Ok(())
}

fn provider_has_auth(auth_store: &AuthStore, meta: &ProviderMeta) -> bool {
    meta.env_vars.iter().any(|v| std::env::var(v).is_ok())
        || auth_store.stored.contains_key(meta.id)
}

fn save_auth_secret_fields(
    provider: &str,
    fields: HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let auth_path = imp_core::storage::global_auth_path();
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
    auth_store.store_secret_fields(provider, fields)?;
    Ok(())
}

async fn run_setup_mode() -> Result<(), Box<dyn std::error::Error>> {
    let _ = imp_core::storage::reconcile_legacy_into_global_root();
    let cwd = std::env::current_dir()?;
    let mut config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;
    let auth_path = imp_core::storage::global_auth_path();
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
    let provider_registry = ProviderRegistry::with_builtins();
    let model_registry = ModelRegistry::with_builtins();

    println!("imp setup");
    println!("=========");

    let providers = provider_registry.list();
    println!("Providers:");
    for (idx, provider) in providers.iter().enumerate() {
        let status = if provider_has_auth(&auth_store, provider) {
            "configured"
        } else {
            "needs auth"
        };
        println!("{}. {} [{}]", idx + 1, provider.name, status);
    }
    let provider_choice = prompt_input_line("Select provider> ")?;
    let provider_index = provider_choice
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(1))
        .filter(|idx| *idx < providers.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid provider selection"))?;
    let provider = &providers[provider_index];

    if !provider_has_auth(&auth_store, provider) {
        println!("No auth detected for {}.", provider.name);
        if provider.id == "anthropic" || provider.id == "openai" || provider.id == "kimi-code" {
            let mode = prompt_input_line("Choose auth mode [login|key]> ")?;
            if mode.eq_ignore_ascii_case("login") {
                run_login(provider.id).await?;
            } else {
                println!("Enter api_key for {}", provider.name);
                let value = prompt_input_line("api_key> ")?;
                save_auth_secret_fields(
                    provider.id,
                    HashMap::from([("api_key".to_string(), value)]),
                )?;
            }
        } else {
            println!("Get a key at: {}", provider.docs_url);
            let value = prompt_input_line("api_key> ")?;
            save_auth_secret_fields(provider.id, HashMap::from([("api_key".to_string(), value)]))?;
        }
    }

    let models = model_registry.list_by_provider(provider.id);
    if models.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No built-in models found for provider {}", provider.id),
        )
        .into());
    }

    println!();
    println!("Models for {}:", provider.name);
    for (idx, model) in models.iter().enumerate().take(20) {
        println!("{}. {} ({})", idx + 1, model.name, model.id);
    }
    let model_choice = prompt_input_line("Select model> ")?;
    let model_index = model_choice
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(1))
        .filter(|idx| *idx < models.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid model selection"))?;
    let model = models[model_index];
    config.model = Some(model.id.clone());

    println!();
    println!("Thinking levels:");
    println!("1. off");
    println!("2. minimal");
    println!("3. low");
    println!("4. medium");
    println!("5. high");
    println!("6. xhigh");
    let thinking_choice = prompt_input_line("Select thinking> ")?;
    config.thinking = Some(match thinking_choice.trim() {
        "1" => ThinkingLevel::Off,
        "2" => ThinkingLevel::Minimal,
        "3" => ThinkingLevel::Low,
        "4" => ThinkingLevel::Medium,
        "5" => ThinkingLevel::High,
        "6" => ThinkingLevel::XHigh,
        _ => {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "Invalid thinking selection").into(),
            )
        }
    });

    println!();
    println!("Web search provider:");
    println!("1. none");
    println!("2. tavily");
    println!("3. exa");
    println!("4. linkup");
    println!("5. perplexity");
    let web_choice = prompt_input_line("Select web search provider> ")?;
    config.web.search_provider = match web_choice.trim() {
        "1" | "" => None,
        "2" => Some(SearchProvider::Tavily),
        "3" => Some(SearchProvider::Exa),
        "4" => Some(SearchProvider::Linkup),
        "5" => Some(SearchProvider::Perplexity),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid web provider selection",
            )
            .into())
        }
    };

    if let Some(web_provider) = config.web.search_provider {
        let web_provider_name = web_provider.name();
        let web_has_auth = auth_store
            .resolve_secret_field(web_provider_name, "api_key")
            .is_ok()
            || std::env::var(web_provider.env_key_name()).is_ok();
        if !web_has_auth {
            println!("No key detected for web provider {}.", web_provider_name);
            println!("Get a key at: {}", search_provider_docs_url(web_provider));
            let value = prompt_input_line("api_key> ")?;
            save_auth_secret_fields(
                web_provider_name,
                HashMap::from([("api_key".to_string(), value)]),
            )?;
        }
    }

    let saved_path = save_user_config(&config)?;
    println!();
    println!("Setup complete.");
    println!("Config saved to {}", saved_path.display());
    println!("Provider: {}", provider.name);
    println!("Model: {}", model.id);
    println!(
        "Thinking: {}",
        config.thinking.map(thinking_level_label).unwrap_or("off")
    );
    println!(
        "Web search: {}",
        web_search_provider_label(config.web.search_provider)
    );

    Ok(())
}

fn parse_thinking_level(s: &str) -> ThinkingLevel {
    match s.to_lowercase().as_str() {
        "off" => ThinkingLevel::Off,
        "minimal" => ThinkingLevel::Minimal,
        "low" => ThinkingLevel::Low,
        "medium" => ThinkingLevel::Medium,
        "high" => ThinkingLevel::High,
        "xhigh" => ThinkingLevel::XHigh,
        _ => ThinkingLevel::Off,
    }
}

fn resolve_model_and_provider(
    cli: &Cli,
    config: &Config,
    registry: &ModelRegistry,
    auth_store: &AuthStore,
) -> Result<(String, String), String> {
    let ResolvedRuntimeConnection {
        model_id,
        provider_name,
    } = resolve_runtime_connection(
        RuntimeConnectionIntent {
            model_hint: cli.model.as_deref(),
            config_model: config.model.as_deref(),
            provider_override: cli.provider.as_deref(),
            api_key_override_present: cli.api_key.is_some(),
        },
        auth_store,
        registry,
    )?;

    Ok((model_id, provider_name))
}

async fn resolve_provider_api_key(
    auth_store: &mut AuthStore,
    provider_name: &str,
) -> Result<imp_llm::auth::ApiKey, imp_llm::Error> {
    match provider_name {
        "openai-codex" => auth_store.resolve_chatgpt_oauth().await,
        "anthropic" | "kimi-code" => auth_store.resolve_with_refresh(provider_name).await,
        _ => auth_store.resolve(provider_name),
    }
}

fn worker_status_counts_as_success(status: imp_core::mana_worker::WorkerStatus) -> bool {
    matches!(
        status,
        imp_core::mana_worker::WorkerStatus::Completed
            | imp_core::mana_worker::WorkerStatus::AwaitingVerify
    )
}

async fn run_headless_mode(
    cli: &Cli,
    unit_id: &str,
    mana_dir_override: Option<&Path>,
    defer_verify: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let cwd = std::env::current_dir()?;
    emit_startup_timing(&mut startup_timer, StartupStage::CwdResolved);

    // Load the unit via canonical mana-core APIs for the primary single-unit
    // imp-run path instead of ad hoc markdown scanning.
    let assignment =
        imp_core::mana_worker::load_assignment_with_mana_dir(&cwd, unit_id, mana_dir_override)?;
    let config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;
    emit_startup_timing(&mut startup_timer, StartupStage::ConfigResolved);
    emit_startup_timing(&mut startup_timer, StartupStage::ModelRegistryReady);
    emit_startup_timing(&mut startup_timer, StartupStage::AuthLoaded);
    emit_startup_timing(&mut startup_timer, StartupStage::ModelResolved);
    emit_startup_timing(&mut startup_timer, StartupStage::ProviderReady);
    emit_startup_timing(&mut startup_timer, StartupStage::ApiKeyResolved);

    let options = imp_core::mana_worker::WorkerRunOptions {
        cwd: cwd.clone(),
        model_override: None,
        model: cli.model.clone().or_else(|| assignment.model.clone()),
        provider: cli.provider.clone(),
        api_key: cli.api_key.clone(),
        thinking: cli
            .thinking
            .as_ref()
            .map(|thinking| parse_thinking_level(thinking)),
        max_turns: cli.max_turns.or(config.max_turns),
        max_tokens: cli.max_tokens.or(config.max_tokens),
        system_prompt: cli.system_prompt.clone(),
        no_tools: cli.no_tools,
        mana_dir_override: mana_dir_override.map(Path::to_path_buf),
        defer_verify,
        lua_loader: build_lua_loader(cli.no_tools, cwd.clone()),
    };

    let mut prepared = imp_core::mana_worker::prepare_worker_run(assignment, options).await?;
    emit_startup_timing(&mut startup_timer, StartupStage::AgentBuilt);

    for warning in &prepared.prefill_warnings {
        eprintln!("[imp] context prefill: {warning}");
    }
    if !prepared.prefilled_files.is_empty() {
        eprintln!(
            "[imp] prefilled {} files (~{} tokens)",
            prepared.prefilled_files.len(),
            prepared.estimated_prefill_tokens
        );
    }

    emit_startup_timing(&mut startup_timer, StartupStage::PromptReady);
    prepared
        .session
        .prompt(&prepared.prompt)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    emit_startup_timing(&mut startup_timer, StartupStage::RunLoopStarted);

    let output_mode = determine_headless_output_mode(&cli.mode, io::stdout().is_terminal());
    let mut printed_trailing_newline = false;

    while let Some(event) = prepared.session.recv_event().await {
        match output_mode {
            HeadlessOutputMode::Json => print_json_event(&event)?,
            HeadlessOutputMode::Human => print_headless_human_event(
                &event,
                !cli.no_tools,
                cli.verbose,
                &mut printed_trailing_newline,
            )?,
        }
    }

    prepared
        .session
        .wait()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    let outcome = imp_core::mana_worker::finalize_worker_run(prepared).await?;
    if let Some(summary) = &outcome.result.summary {
        eprintln!("[worker] {summary}");
    }
    if let Some(error) = &outcome.result.error {
        eprintln!("[worker error] {error}");
    }

    Ok(worker_status_counts_as_success(outcome.result.status))
}

fn build_lua_loader(no_tools: bool, cwd: PathBuf) -> Option<imp_core::tools::LuaToolLoader> {
    if no_tools {
        return None;
    }

    fn init_lua_tools(
        cwd: PathBuf,
        policy: &imp_core::config::LuaCapabilityPolicy,
        tools: &mut imp_core::tools::ToolRegistry,
    ) {
        let user_config_dir = Config::user_config_dir();
        imp_lua::init_lua_extensions(&user_config_dir, Some(&cwd), tools, policy);
    }

    Some(Arc::new(move |policy, tools| {
        init_lua_tools(cwd.clone(), policy, tools)
    }))
}

fn emit_startup_timing(timer: &mut StartupTimer, stage: StartupStage) {
    if let Some(timing) = timer.mark(stage) {
        eprintln!(
            "[startup stage={} total={}ms delta={}ms]",
            timing.stage.as_str(),
            timing.since_start_ms,
            timing.since_previous_ms,
        );
    }
}

fn format_timing_event(timing: &TimingEvent) -> String {
    format!(
        "[timing turn={} stage={} turn={}ms llm={}ms]",
        timing.turn,
        timing.stage.as_str(),
        timing.since_turn_start_ms,
        timing.since_llm_request_start_ms,
    )
}

async fn run_reserved_mana_namespace_command(
    target: &str,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let rendered_args = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    Err(format!(
        "`imp mana {target}{rendered_args}` is reserved for a future mana-aware operator command. For now, use `mana {target}{rendered_args}` directly or `imp mana <unit-id>` for single-unit worker execution."
    )
    .into())
}

fn determine_headless_output_mode(cli_mode: &str, stdout_is_terminal: bool) -> HeadlessOutputMode {
    match cli_mode {
        "json" | "rpc" => HeadlessOutputMode::Json,
        _ if stdout_is_terminal => HeadlessOutputMode::Human,
        _ => HeadlessOutputMode::Json,
    }
}

fn print_headless_human_event(
    event: &AgentEvent,
    show_tools: bool,
    verbose: bool,
    printed_trailing_newline: &mut bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        AgentEvent::MessageDelta { delta } => match delta {
            StreamEvent::TextDelta { text } => {
                print!("{text}");
                io::stdout().flush()?;
                *printed_trailing_newline = false;
            }
            StreamEvent::ThinkingDelta { text } => eprint!("{text}"),
            _ => {}
        },
        AgentEvent::ToolExecutionStart {
            tool_name, args, ..
        } if show_tools => {
            let summary = match tool_name.as_str() {
                "bash" => args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(|c| truncate_chars_with_suffix(c, 60, "…"))
                    .unwrap_or_default(),
                "read" | "write" | "edit" => args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                "scan" => args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => String::new(),
            };
            if summary.is_empty() {
                eprintln!("[tool: {tool_name}]");
            } else {
                eprintln!("[tool: {tool_name} {summary}]");
            }
        }
        AgentEvent::ToolExecutionEnd { result, .. } if show_tools => {
            if result.is_error {
                let text: String = result
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                if !text.is_empty() {
                    eprintln!("[error: {}]", truncate_chars_with_suffix(&text, 100, ""));
                }
            }
        }
        AgentEvent::TurnEnd { .. } => {
            if !*printed_trailing_newline {
                println!();
                *printed_trailing_newline = true;
            }
        }
        AgentEvent::Error { error } => {
            eprintln!("Error: {}", format_error_for_display(error));
        }
        AgentEvent::Timing { timing } => {
            if verbose {
                eprintln!("{}", format_timing_event(timing));
            }
        }
        AgentEvent::AgentEnd { usage, cost } => {
            eprintln!(
                "\n[tokens: ↑{} ↓{} | cost: ${:.4}]",
                usage.input_tokens, usage.output_tokens, cost.total
            );
        }
        _ => {}
    }

    Ok(())
}

fn print_json_event(event: &AgentEvent) -> Result<(), Box<dyn std::error::Error>> {
    let value = match event {
        AgentEvent::AgentStart { model, timestamp } => {
            json!({ "type": "agent_start", "model": model, "timestamp": timestamp })
        }
        AgentEvent::AgentEnd { usage, cost } => {
            json!({ "type": "agent_end", "usage": usage, "cost": cost })
        }
        AgentEvent::TurnStart { index } => json!({ "type": "turn_start", "index": index }),
        AgentEvent::TurnAssessment { index, assessment } => json!({
            "type": "turn_assessment",
            "index": index,
            "assessment": next_action_assessment_to_json(assessment),
        }),
        AgentEvent::TurnEnd { index, message, .. } => {
            json!({ "type": "turn_end", "index": index, "message": message })
        }
        AgentEvent::MessageStart { message } => {
            json!({ "type": "message_start", "message": message })
        }
        AgentEvent::MessageDelta { delta } => stream_event_to_json(delta),
        AgentEvent::MessageEnd { message } => json!({ "type": "message_end", "message": message }),
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } => {
            json!({
                "type": "tool_execution_start",
                "tool_call_id": tool_call_id,
                "tool": tool_name,
                "args": args,
            })
        }
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            result,
        } => {
            json!({
                "type": "tool_execution_end",
                "tool_call_id": tool_call_id,
                "result": result,
            })
        }
        AgentEvent::Timing { timing } => json!({
            "type": "timing",
            "turn": timing.turn,
            "stage": timing.stage.as_str(),
            "since_turn_start_ms": timing.since_turn_start_ms,
            "since_llm_request_start_ms": timing.since_llm_request_start_ms,
        }),
        AgentEvent::Warning { message } => json!({ "type": "warning", "message": message }),
        AgentEvent::Error { error } => json!({ "type": "error", "error": error }),
        AgentEvent::ToolOutputDelta { .. } => return Ok(()), // handled in TUI only
    };

    let line = serde_json::to_string(&value)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{line}")?;
    stdout.flush()?;
    Ok(())
}

fn stream_event_to_json(event: &StreamEvent) -> serde_json::Value {
    match event {
        StreamEvent::MessageStart { model } => {
            json!({ "type": "message_start", "model": model })
        }
        StreamEvent::TextDelta { text } => json!({ "type": "text_delta", "text": text }),
        StreamEvent::ThinkingDelta { text } => {
            json!({ "type": "thinking_delta", "text": text })
        }
        StreamEvent::ToolCall {
            id,
            name,
            arguments,
        } => {
            json!({
                "type": "tool_call",
                "id": id,
                "tool": name,
                "args": arguments,
            })
        }
        StreamEvent::MessageEnd { message } => {
            json!({ "type": "message_end", "message": message })
        }
        StreamEvent::Error { error } => json!({ "type": "stream_error", "error": error }),
    }
}

#[derive(Debug)]
enum RpcInputCommand {
    Prompt(String),
    Cancel,
    Steer(String),
    FollowUp(String),
}

type UiResponseMap = Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>;
type RpcAgentJoinHandle = JoinHandle<(Agent, imp_core::Result<()>)>;

struct RpcUi {
    stdout_tx: mpsc::Sender<Value>,
    pending: UiResponseMap,
    next_request_id: Arc<AtomicU64>,
}

impl RpcUi {
    fn new(stdout_tx: mpsc::Sender<Value>) -> Self {
        Self {
            stdout_tx,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_request_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn pending(&self) -> UiResponseMap {
        self.pending.clone()
    }

    async fn emit(&self, value: Value) {
        let _ = self.stdout_tx.send(value).await;
    }

    async fn request(&self, method: &str, params: Value) -> Option<Value> {
        let id = format!("q{}", self.next_request_id.fetch_add(1, Ordering::Relaxed));
        let (response_tx, response_rx) = oneshot::channel();

        self.pending.lock().await.insert(id.clone(), response_tx);
        self.emit(json!({
            "type": "ui_request",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await;

        match tokio::time::timeout(Duration::from_secs(60), response_rx).await {
            Ok(Ok(result)) => Some(result),
            Ok(Err(_)) | Err(_) => {
                self.pending.lock().await.remove(&id);
                None
            }
        }
    }
}

#[async_trait]
impl UserInterface for RpcUi {
    fn has_ui(&self) -> bool {
        true
    }

    async fn notify(&self, message: &str, level: NotifyLevel) {
        self.emit(json!({
            "type": "ui_request",
            "method": "notify",
            "params": {
                "message": message,
                "level": serde_json::to_value(level).unwrap_or(Value::Null),
            }
        }))
        .await;
    }

    async fn confirm(&self, title: &str, message: &str) -> Option<bool> {
        self.request(
            "confirm",
            json!({
                "title": title,
                "message": message,
            }),
        )
        .await?
        .as_bool()
    }

    async fn select_with_context(
        &self,
        title: &str,
        context: &str,
        options: &[SelectOption],
    ) -> Option<usize> {
        let result = self
            .request(
                "select",
                json!({
                    "title": title,
                    "context": context,
                    "options": serde_json::to_value(options).unwrap_or_else(|_| json!([])),
                }),
            )
            .await?;

        result.as_u64().map(|index| index as usize)
    }

    async fn input_with_context(
        &self,
        title: &str,
        context: &str,
        placeholder: &str,
    ) -> Option<String> {
        self.request(
            "input",
            json!({
                "title": title,
                "context": context,
                "placeholder": placeholder,
            }),
        )
        .await?
        .as_str()
        .map(ToOwned::to_owned)
    }

    async fn set_status(&self, key: &str, text: Option<&str>) {
        self.emit(json!({
            "type": "ui_request",
            "method": "set_status",
            "params": {
                "key": key,
                "text": text,
            }
        }))
        .await;
    }

    async fn set_widget(&self, key: &str, content: Option<WidgetContent>) {
        self.emit(json!({
            "type": "ui_request",
            "method": "set_widget",
            "params": {
                "key": key,
                "content": serde_json::to_value(content).unwrap_or(Value::Null),
            }
        }))
        .await;
    }

    async fn custom(&self, component: ComponentSpec) -> Option<Value> {
        self.request(
            "custom",
            json!({
                "component": serde_json::to_value(component).unwrap_or(Value::Null),
            }),
        )
        .await
    }
}

async fn run_rpc_mode(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let cwd = std::env::current_dir()?;
    emit_startup_timing(&mut startup_timer, StartupStage::CwdResolved);
    let config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;
    emit_startup_timing(&mut startup_timer, StartupStage::ConfigResolved);
    let registry = ModelRegistry::with_builtins();
    emit_startup_timing(&mut startup_timer, StartupStage::ModelRegistryReady);

    let stdout_tx = spawn_json_lines_stdout_writer();
    let rpc_ui = Arc::new(RpcUi::new(stdout_tx.clone()));

    let (command_tx, mut command_rx) = mpsc::channel(64);
    tokio::spawn(read_rpc_stdin(
        command_tx,
        rpc_ui.pending(),
        stdout_tx.clone(),
    ));

    let mut history: Vec<Message> = Vec::new();
    let mut queued_followups: VecDeque<String> = VecDeque::new();
    let mut active_command_tx: Option<mpsc::Sender<AgentCommand>> = None;
    let mut active_join: Option<RpcAgentJoinHandle> = None;
    let mut stdin_closed = false;

    loop {
        if let Some(join_handle) = active_join.as_mut() {
            tokio::select! {
                maybe_command = command_rx.recv() => {
                    match maybe_command {
                        Some(command) => {
                            process_rpc_command(
                                command,
                                cli,
                                &cwd,
                                &config,
                                &registry,
                                &stdout_tx,
                                &rpc_ui,
                                &history,
                                &mut queued_followups,
                                &mut active_command_tx,
                                &mut active_join,
                            ).await?;
                        }
                        None => stdin_closed = true,
                    }
                }
                join_result = join_handle => {
                    active_join = None;
                    active_command_tx = None;

                    match join_result {
                        Ok((agent, _result)) => {
                            history = agent.messages;
                        }
                        Err(error) => {
                            emit_protocol_error(&stdout_tx, format!("agent task failed: {error}")).await;
                        }
                    }

                    if let Some(prompt) = queued_followups.pop_front() {
                        let (command_tx, join_handle) = spawn_rpc_agent(
                            cli,
                            &cwd,
                            &config,
                            &registry,
                            history.clone(),
                            rpc_ui.clone(),
                            stdout_tx.clone(),
                            prompt,
                        )?;
                        active_command_tx = Some(command_tx);
                        active_join = Some(join_handle);
                    } else if stdin_closed {
                        break;
                    }
                }
            }
        } else {
            match command_rx.recv().await {
                Some(command) => {
                    process_rpc_command(
                        command,
                        cli,
                        &cwd,
                        &config,
                        &registry,
                        &stdout_tx,
                        &rpc_ui,
                        &history,
                        &mut queued_followups,
                        &mut active_command_tx,
                        &mut active_join,
                    )
                    .await?;
                }
                None => break,
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_rpc_command(
    command: RpcInputCommand,
    cli: &Cli,
    cwd: &Path,
    config: &Config,
    registry: &ModelRegistry,
    stdout_tx: &mpsc::Sender<Value>,
    rpc_ui: &Arc<RpcUi>,
    history: &[Message],
    queued_followups: &mut VecDeque<String>,
    active_command_tx: &mut Option<mpsc::Sender<AgentCommand>>,
    active_join: &mut Option<RpcAgentJoinHandle>,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RpcInputCommand::Prompt(content) => {
            if active_join.is_some() {
                queued_followups.push_back(content);
            } else {
                let (command_tx, join_handle) = spawn_rpc_agent(
                    cli,
                    cwd,
                    config,
                    registry,
                    history.to_vec(),
                    rpc_ui.clone(),
                    stdout_tx.clone(),
                    content,
                )?;
                *active_command_tx = Some(command_tx);
                *active_join = Some(join_handle);
            }
        }
        RpcInputCommand::Cancel => {
            if let Some(command_tx) = active_command_tx.as_ref() {
                let _ = command_tx.send(AgentCommand::Cancel).await;
            }
        }
        RpcInputCommand::Steer(content) => {
            if let Some(command_tx) = active_command_tx.as_ref() {
                let _ = command_tx.send(AgentCommand::Steer(content)).await;
            } else {
                emit_protocol_error(stdout_tx, "cannot steer without an active agent").await;
            }
        }
        RpcInputCommand::FollowUp(content) => {
            if active_join.is_some() {
                queued_followups.push_back(content);
            } else {
                let (command_tx, join_handle) = spawn_rpc_agent(
                    cli,
                    cwd,
                    config,
                    registry,
                    history.to_vec(),
                    rpc_ui.clone(),
                    stdout_tx.clone(),
                    content,
                )?;
                *active_command_tx = Some(command_tx);
                *active_join = Some(join_handle);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_rpc_agent(
    cli: &Cli,
    cwd: &Path,
    config: &Config,
    registry: &ModelRegistry,
    history: Vec<Message>,
    rpc_ui: Arc<RpcUi>,
    stdout_tx: mpsc::Sender<Value>,
    prompt: String,
) -> Result<(mpsc::Sender<AgentCommand>, RpcAgentJoinHandle), Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let (mut agent, handle) = create_rpc_agent(cli, cwd, config, registry, history, rpc_ui)?;
    let command_tx = handle.command_tx.clone();

    tokio::spawn(forward_rpc_events(handle, stdout_tx));

    emit_startup_timing(&mut startup_timer, StartupStage::PromptReady);
    let join_handle = tokio::spawn(async move {
        let result = agent.run(prompt).await;
        (agent, result)
    });
    emit_startup_timing(&mut startup_timer, StartupStage::RunLoopStarted);

    Ok((command_tx, join_handle))
}

fn create_rpc_agent(
    cli: &Cli,
    cwd: &Path,
    config: &Config,
    registry: &ModelRegistry,
    history: Vec<Message>,
    rpc_ui: Arc<RpcUi>,
) -> Result<(Agent, AgentHandle), Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let auth_path = imp_core::storage::global_auth_path();
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
    emit_startup_timing(&mut startup_timer, StartupStage::AuthLoaded);
    let (model_id, provider_name) =
        resolve_model_and_provider(cli, config, registry, &auth_store).map_err(io::Error::other)?;
    emit_startup_timing(&mut startup_timer, StartupStage::ModelResolved);

    let provider = create_provider(&provider_name)
        .ok_or_else(|| io::Error::other(format!("Unknown provider: {provider_name}")))?;
    emit_startup_timing(&mut startup_timer, StartupStage::ProviderReady);

    let meta = registry
        .resolve_meta(&model_id, Some(&provider_name))
        .ok_or_else(|| io::Error::other(format!("Model not found: {model_id}")))?;

    if let Some(ref key) = cli.api_key {
        auth_store.set_runtime_key(&provider_name, key.clone());
    }

    let api_key = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(resolve_provider_api_key(&mut auth_store, &provider_name))
    })?;
    emit_startup_timing(&mut startup_timer, StartupStage::ApiKeyResolved);
    let model = Model {
        meta,
        provider: Arc::from(provider),
    };

    // Apply CLI thinking level override to config.
    let mut agent_config = config.clone();
    if let Some(ref thinking) = cli.thinking {
        agent_config.thinking = Some(parse_thinking_level(thinking));
    }

    let rpc_ui_clone = rpc_ui.clone() as Arc<dyn UserInterface>;
    let lua_cwd = cwd.to_path_buf();
    let mut builder =
        imp_core::builder::AgentBuilder::new(agent_config, cwd.to_path_buf(), model, api_key)
            .lua_tool_loader(move |policy, tools| {
                let user_config_dir = Config::user_config_dir();
                imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools, policy);
            });
    if let Some(ref prompt) = cli.system_prompt {
        builder = builder.system_prompt(prompt.clone());
    }
    let (mut agent, handle) = builder.build()?;
    emit_startup_timing(&mut startup_timer, StartupStage::AgentBuilt);
    agent.ui = rpc_ui_clone;
    agent.messages = history;

    Ok((agent, handle))
}

fn spawn_json_lines_stdout_writer() -> mpsc::Sender<Value> {
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<Value>(256);

    tokio::spawn(async move {
        let mut stdout = BufWriter::new(tokio::io::stdout());
        while let Some(value) = stdout_rx.recv().await {
            let Ok(line) = serde_json::to_string(&value) else {
                continue;
            };

            if stdout.write_all(line.as_bytes()).await.is_err() {
                break;
            }
            if stdout.write_all(b"\n").await.is_err() {
                break;
            }
            if stdout.flush().await.is_err() {
                break;
            }
        }
    });

    stdout_tx
}

async fn read_rpc_stdin(
    command_tx: mpsc::Sender<RpcInputCommand>,
    pending_ui: UiResponseMap,
    stdout_tx: mpsc::Sender<Value>,
) {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(trimmed) {
                    Ok(value) => {
                        if value.get("type").and_then(Value::as_str) == Some("ui_response") {
                            if let Err(error) = deliver_ui_response(value, &pending_ui).await {
                                emit_protocol_error(&stdout_tx, error).await;
                            }
                            continue;
                        }

                        match parse_rpc_command(&value) {
                            Ok(command) => {
                                if command_tx.send(command).await.is_err() {
                                    break;
                                }
                            }
                            Err(error) => emit_protocol_error(&stdout_tx, error).await,
                        }
                    }
                    Err(error) => {
                        emit_protocol_error(&stdout_tx, format!("invalid JSON input: {error}"))
                            .await;
                    }
                }
            }
            Ok(None) => break,
            Err(error) => {
                emit_protocol_error(&stdout_tx, format!("stdin read failed: {error}")).await;
                break;
            }
        }
    }
}

fn parse_rpc_command(value: &Value) -> Result<RpcInputCommand, String> {
    let command_type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing command type".to_string())?;

    match command_type {
        "prompt" => Ok(RpcInputCommand::Prompt(required_rpc_content(value)?)),
        "cancel" => Ok(RpcInputCommand::Cancel),
        "steer" => Ok(RpcInputCommand::Steer(required_rpc_content(value)?)),
        "followup" => Ok(RpcInputCommand::FollowUp(required_rpc_content(value)?)),
        other => Err(format!("unknown command type: {other}")),
    }
}

fn required_rpc_content(value: &Value) -> Result<String, String> {
    value
        .get("content")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "missing string field: content".to_string())
}

async fn deliver_ui_response(value: Value, pending_ui: &UiResponseMap) -> Result<(), String> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "ui_response missing id".to_string())?
        .to_string();
    let result = value.get("result").cloned().unwrap_or(Value::Null);

    let response_tx = pending_ui
        .lock()
        .await
        .remove(&id)
        .ok_or_else(|| format!("unknown ui_response id: {id}"))?;

    response_tx
        .send(result)
        .map_err(|_| format!("failed to deliver ui_response: {id}"))
}

async fn forward_rpc_events(mut handle: AgentHandle, stdout_tx: mpsc::Sender<Value>) {
    while let Some(event) = handle.event_rx.recv().await {
        let _ = stdout_tx.send(rpc_agent_event_to_json(&event)).await;
    }
}

fn rpc_agent_event_to_json(event: &AgentEvent) -> Value {
    match event {
        AgentEvent::AgentStart { model, timestamp } => json!({
            "type": "agent_start",
            "model": model,
            "timestamp": timestamp,
        }),
        AgentEvent::AgentEnd { usage, cost } => json!({
            "type": "agent_end",
            "usage": usage,
            "cost": cost,
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
            "cache_read_tokens": usage.cache_read_tokens,
            "cache_write_tokens": usage.cache_write_tokens,
            "cost_total": cost.total,
        }),
        AgentEvent::TurnStart { index } => json!({ "type": "turn_start", "index": index }),
        AgentEvent::TurnAssessment { index, assessment } => json!({
            "type": "turn_assessment",
            "index": index,
            "assessment": next_action_assessment_to_json(assessment),
        }),
        AgentEvent::TurnEnd { index, message, .. } => {
            json!({ "type": "turn_end", "index": index, "message": message })
        }
        AgentEvent::MessageStart { message } => {
            json!({ "type": "message_start", "message": message })
        }
        AgentEvent::MessageDelta { delta } => rpc_stream_event_to_json(delta),
        AgentEvent::MessageEnd { message } => json!({ "type": "message_end", "message": message }),
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } => json!({
            "type": "tool_execution_start",
            "tool_call_id": tool_call_id,
            "tool_name": tool_name,
            "args": args,
        }),
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            result,
        } => json!({
            "type": "tool_execution_end",
            "tool_call_id": tool_call_id,
            "tool_name": result.tool_name,
            "is_error": result.is_error,
            "content": result.content,
            "details": result.details,
            "timestamp": result.timestamp,
        }),
        AgentEvent::Timing { timing } => json!({
            "type": "timing",
            "turn": timing.turn,
            "stage": timing.stage.as_str(),
            "since_turn_start_ms": timing.since_turn_start_ms,
            "since_llm_request_start_ms": timing.since_llm_request_start_ms,
        }),
        AgentEvent::Warning { message } => {
            json!({ "type": "warning", "message": message })
        }
        AgentEvent::Error { error } => json!({ "type": "error", "error": error }),
        AgentEvent::ToolOutputDelta { tool_call_id, text } => {
            json!({ "type": "tool_output_delta", "tool_call_id": tool_call_id, "text": text })
        }
    }
}

fn next_action_assessment_to_json(assessment: &imp_core::agent::NextActionAssessment) -> Value {
    let chosen_action = match &assessment.chosen_action {
        imp_core::agent::NextActionDebugView::Continue { prompt, reason } => json!({
            "kind": "continue",
            "prompt": prompt,
            "reason": reason,
        }),
        imp_core::agent::NextActionDebugView::Stop { reason } => json!({
            "kind": "stop",
            "reason": reason,
        }),
    };

    json!({
        "runtime": {
            "repeated_action": assessment.runtime.repeated_action,
            "execution_stop_reason": assessment.runtime.execution_stop_reason,
            "work_completed": assessment.runtime.work_completed,
            "no_progress": assessment.runtime.no_progress,
        },
        "mana": {
            "stop_reason": assessment.mana.stop_reason,
        },
        "text_fallback": {
            "planner_stop_reason": assessment.text_fallback.planner_stop_reason,
            "execution_stop_reason": assessment.text_fallback.execution_stop_reason,
        },
        "continue_recommendation": assessment.continue_recommendation.as_ref().map(|recommendation| json!({
            "prompt": recommendation.prompt,
            "reason": recommendation.reason,
        })),
        "chosen_action": chosen_action,
    })
}

fn rpc_stream_event_to_json(event: &StreamEvent) -> Value {
    match event {
        StreamEvent::MessageStart { model } => json!({ "type": "message_start", "model": model }),
        StreamEvent::TextDelta { text } => json!({ "type": "text_delta", "text": text }),
        StreamEvent::ThinkingDelta { text } => json!({ "type": "thinking_delta", "text": text }),
        StreamEvent::ToolCall {
            id,
            name,
            arguments,
        } => json!({
            "type": "tool_call",
            "id": id,
            "name": name,
            "arguments": arguments,
        }),
        StreamEvent::MessageEnd { message } => json!({ "type": "message_end", "message": message }),
        StreamEvent::Error { error } => json!({ "type": "stream_error", "error": error }),
    }
}

async fn emit_protocol_error(stdout_tx: &mpsc::Sender<Value>, error: impl Into<String>) {
    let _ = stdout_tx
        .send(json!({
            "type": "protocol_error",
            "error": error.into(),
        }))
        .await;
}

async fn run_print_mode(cli: &Cli, prompt: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let cwd = std::env::current_dir()?;
    emit_startup_timing(&mut startup_timer, StartupStage::CwdResolved);
    let config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;
    emit_startup_timing(&mut startup_timer, StartupStage::ConfigResolved);

    emit_startup_timing(&mut startup_timer, StartupStage::ModelRegistryReady);
    emit_startup_timing(&mut startup_timer, StartupStage::AuthLoaded);
    emit_startup_timing(&mut startup_timer, StartupStage::ModelResolved);
    emit_startup_timing(&mut startup_timer, StartupStage::ProviderReady);
    emit_startup_timing(&mut startup_timer, StartupStage::ApiKeyResolved);

    let session_choice = if cli.no_session {
        SessionChoice::InMemory
    } else if cli.cont {
        SessionChoice::Continue
    } else if let Some(ref path) = cli.session {
        SessionChoice::Open(path.clone())
    } else {
        SessionChoice::New
    };

    let mut options = SessionOptions {
        cwd: cwd.clone(),
        model: cli.model.clone(),
        provider: cli.provider.clone(),
        api_key: cli.api_key.clone(),
        thinking: cli
            .thinking
            .as_ref()
            .map(|thinking| parse_thinking_level(thinking)),
        max_turns: cli.max_turns.or(config.max_turns),
        max_tokens: cli.max_tokens.or(config.max_tokens),
        system_prompt: cli.system_prompt.clone(),
        no_tools: cli.no_tools,
        session: session_choice,
        ..Default::default()
    };
    emit_startup_timing(&mut startup_timer, StartupStage::SessionReady);

    if !cli.no_tools {
        options.lua_loader = build_lua_loader(false, std::env::current_dir().unwrap_or_default());
    }

    let mut session = ImpSession::create(options)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    emit_startup_timing(&mut startup_timer, StartupStage::AgentBuilt);

    emit_startup_timing(&mut startup_timer, StartupStage::PromptReady);
    session
        .prompt(prompt)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    emit_startup_timing(&mut startup_timer, StartupStage::RunLoopStarted);

    let mut printed_trailing_newline = false;

    while let Some(event) = session.recv_event().await {
        match event {
            AgentEvent::MessageDelta { delta } => match delta {
                StreamEvent::TextDelta { text } => {
                    print!("{text}");
                    printed_trailing_newline = false;
                }
                StreamEvent::ThinkingDelta { text } => eprint!("{text}"),
                _ => {}
            },
            AgentEvent::ToolExecutionStart {
                tool_name, args, ..
            } if !cli.no_tools => {
                let summary = match tool_name.as_str() {
                    "bash" => args
                        .get("command")
                        .and_then(|v| v.as_str())
                        .map(|c| truncate_chars_with_suffix(c, 60, "…"))
                        .unwrap_or_default(),
                    "read" | "write" | "edit" => args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    "scan" => args
                        .get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    _ => String::new(),
                };
                if summary.is_empty() {
                    eprintln!("[tool: {tool_name}]");
                } else {
                    eprintln!("[tool: {tool_name} {summary}]");
                }
            }
            AgentEvent::ToolExecutionEnd { result, .. } if !cli.no_tools => {
                if result.is_error {
                    let text: String = result
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    if !text.is_empty() {
                        eprintln!("[error: {}]", truncate_chars_with_suffix(&text, 100, ""));
                    }
                }
            }
            AgentEvent::TurnEnd { .. } => {
                if !printed_trailing_newline {
                    println!();
                    printed_trailing_newline = true;
                }
            }
            AgentEvent::Error { error } => {
                eprintln!("Error: {}", format_error_for_display(&error));
            }
            AgentEvent::Timing { timing } => {
                if cli.verbose {
                    eprintln!("{}", format_timing_event(&timing));
                }
            }
            AgentEvent::AgentEnd { usage, cost } => {
                eprintln!(
                    "\n[tokens: ↑{} ↓{} | cost: ${:.4}]",
                    usage.input_tokens, usage.output_tokens, cost.total
                );
            }
            _ => {}
        }
    }

    session
        .wait()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    Ok(())
}

struct CliTerminalUi;

#[async_trait]
impl UserInterface for CliTerminalUi {
    fn has_ui(&self) -> bool {
        true
    }

    async fn notify(&self, message: &str, level: NotifyLevel) {
        let prefix = match level {
            NotifyLevel::Info => "info",
            NotifyLevel::Warning => "warning",
            NotifyLevel::Error => "error",
        };
        eprintln!("[{prefix}] {message}");
    }

    async fn confirm(&self, title: &str, message: &str) -> Option<bool> {
        let title = title.to_string();
        let message = message.to_string();
        tokio::task::spawn_blocking(move || {
            eprintln!("\n{title}");
            if !message.trim().is_empty() {
                eprintln!("{message}");
            }
            eprint!("Proceed? [Y/n] ");
            io::stdout().flush().ok()?;
            let mut input = String::new();
            let bytes = io::stdin().read_line(&mut input).ok()?;
            if bytes == 0 {
                return None;
            }
            let answer = input.trim().to_lowercase();
            Some(!matches!(answer.as_str(), "n" | "no"))
        })
        .await
        .ok()
        .flatten()
    }

    async fn select_with_context(
        &self,
        title: &str,
        context: &str,
        options: &[SelectOption],
    ) -> Option<usize> {
        let title = title.to_string();
        let context = context.to_string();
        let options = options.to_vec();
        tokio::task::spawn_blocking(move || {
            eprintln!("\n{title}");
            if !context.trim().is_empty() {
                eprintln!("{context}");
            }
            for (idx, option) in options.iter().enumerate() {
                eprintln!("{}. {}", idx + 1, option.label);
                if let Some(description) = &option.description {
                    if !description.trim().is_empty() {
                        eprintln!("   {description}");
                    }
                }
            }
            eprint!("Select> ");
            io::stdout().flush().ok()?;
            let mut input = String::new();
            let bytes = io::stdin().read_line(&mut input).ok()?;
            if bytes == 0 {
                return None;
            }
            let index: usize = input.trim().parse().ok()?;
            index.checked_sub(1).filter(|idx| *idx < options.len())
        })
        .await
        .ok()
        .flatten()
    }

    async fn input_with_context(
        &self,
        title: &str,
        context: &str,
        placeholder: &str,
    ) -> Option<String> {
        let title = title.to_string();
        let context = context.to_string();
        let placeholder = placeholder.to_string();
        tokio::task::spawn_blocking(move || {
            eprintln!("\n{title}");
            if !context.trim().is_empty() {
                eprintln!("{context}");
            }
            if !placeholder.trim().is_empty() {
                eprintln!("placeholder: {placeholder}");
            }
            eprint!("> ");
            io::stdout().flush().ok()?;
            let mut input = String::new();
            let bytes = io::stdin().read_line(&mut input).ok()?;
            if bytes == 0 {
                return None;
            }
            Some(input.trim().to_string())
        })
        .await
        .ok()
        .flatten()
    }

    async fn set_status(&self, _key: &str, _text: Option<&str>) {}

    async fn set_widget(&self, _key: &str, _content: Option<WidgetContent>) {}

    async fn custom(&self, _component: ComponentSpec) -> Option<serde_json::Value> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChatShellCommand {
    Help(Option<String>),
    Quit,
    Status,
    New,
    Resume,
    Compact,
    Settings,
    Personality,
    Setup,
    View(Option<String>),
    Model(Option<String>),
    Thinking(Option<String>),
    Unknown(String),
}

fn parse_chat_shell_command(input: &str) -> Option<ChatShellCommand> {
    let raw = input
        .strip_prefix(':')
        .or_else(|| input.strip_prefix('/'))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some(ChatShellCommand::Help(None));
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next().unwrap_or_default();
    let rest = parts.collect::<Vec<_>>().join(" ");
    let arg = if rest.is_empty() { None } else { Some(rest) };

    Some(match command {
        "help" | "h" => ChatShellCommand::Help(arg),
        "quit" | "q" | "exit" => ChatShellCommand::Quit,
        "status" => ChatShellCommand::Status,
        "new" => ChatShellCommand::New,
        "resume" => ChatShellCommand::Resume,
        "compact" => ChatShellCommand::Compact,
        "settings" => ChatShellCommand::Settings,
        "personality" => ChatShellCommand::Personality,
        "setup" => ChatShellCommand::Setup,
        "view" => ChatShellCommand::View(arg),
        "model" => ChatShellCommand::Model(arg),
        "thinking" => ChatShellCommand::Thinking(arg),
        _ => ChatShellCommand::Unknown(trimmed.to_string()),
    })
}

fn parse_thinking_level_strict(raw: &str) -> Option<ThinkingLevel> {
    match raw.trim().to_lowercase().as_str() {
        "off" => Some(ThinkingLevel::Off),
        "minimal" => Some(ThinkingLevel::Minimal),
        "low" => Some(ThinkingLevel::Low),
        "medium" => Some(ThinkingLevel::Medium),
        "high" => Some(ThinkingLevel::High),
        "xhigh" => Some(ThinkingLevel::XHigh),
        _ => None,
    }
}

fn tool_output_display_label(display: ToolOutputDisplay) -> &'static str {
    match display {
        ToolOutputDisplay::Full => "full",
        ToolOutputDisplay::Compact => "compact",
        ToolOutputDisplay::Collapsed => "collapsed",
    }
}

fn parse_tool_output_display(raw: &str) -> Option<ToolOutputDisplay> {
    match raw.trim().to_lowercase().as_str() {
        "full" => Some(ToolOutputDisplay::Full),
        "compact" => Some(ToolOutputDisplay::Compact),
        "collapsed" => Some(ToolOutputDisplay::Collapsed),
        _ => None,
    }
}

fn web_search_provider_label(provider: Option<SearchProvider>) -> &'static str {
    match provider {
        Some(provider) => provider.name(),
        None => "none",
    }
}

fn prompt_input_line(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    eprint!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    let bytes = io::stdin().read_line(&mut input)?;
    if bytes == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "stdin closed").into());
    }
    Ok(input.trim().to_string())
}

fn prompt_optional_input_line(prompt: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    eprint!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    let bytes = io::stdin().read_line(&mut input)?;
    if bytes == 0 {
        return Ok(None);
    }
    Ok(Some(input.trim().to_string()))
}

fn save_user_config(config: &Config) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = imp_core::storage::global_config_path();
    config
        .save(&path)
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    Ok(path)
}

fn print_settings_summary(config: &Config, config_path: &Path) {
    println!("Settings ({})", config_path.display());
    println!("================");
    println!(
        "1. model               {}",
        config.model.as_deref().unwrap_or("(unset)")
    );
    println!(
        "2. thinking            {}",
        config
            .thinking
            .map(thinking_level_label)
            .unwrap_or("(unset)")
    );
    println!(
        "3. max_tokens          {}",
        config
            .max_tokens
            .map(|v| v.to_string())
            .unwrap_or_else(|| "(unset)".to_string())
    );
    println!(
        "4. max_turns           {}",
        config
            .max_turns
            .map(|v| v.to_string())
            .unwrap_or_else(|| "(unset)".to_string())
    );
    println!(
        "5. tool_output         {}",
        tool_output_display_label(config.ui.tool_output)
    );
    println!(
        "6. web_search_provider {}",
        web_search_provider_label(config.web.search_provider)
    );
    println!("s. save and exit");
    println!("q. quit without saving");
}

fn resolve_project_soul_path(cwd: &Path) -> PathBuf {
    discover_project_soul(cwd)
        .map(|soul| soul.path)
        .unwrap_or_else(|| suggested_project_soul_path(cwd))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersonalityScopeCli {
    Global,
    Project,
}

impl PersonalityScopeCli {
    fn toggle(self) -> Self {
        match self {
            Self::Global => Self::Project,
            Self::Project => Self::Global,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersonalityModeCli {
    Builder,
    Source,
}

fn tunable_cli_label(state: SoulTunableState) -> &'static str {
    match state {
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

fn personality_scope_paths(cwd: &Path) -> (PathBuf, PathBuf) {
    (
        Config::user_config_dir().join("soul.md"),
        resolve_project_soul_path(cwd),
    )
}

fn load_soul_content(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|_| default_soul_markdown())
}

fn save_soul_content(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = if content.trim().is_empty() {
        default_soul_markdown()
    } else {
        content.to_string()
    };
    std::fs::write(path, body)?;
    Ok(())
}

fn open_in_editor(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = ProcessCommand::new(&editor).arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("editor `{editor}` exited with {status}")).into())
    }
}

fn print_personality_builder(scope: PersonalityScopeCli, path: &Path, content: &str) {
    println!();
    println!("Personality — builder mode");
    println!("scope: {}", scope.label());
    println!("path: {}", path.display());
    println!("identity: {}", soul_identity_text(content));
    println!("1. scope               {}", scope.label());
    println!(
        "2. autonomy            {}",
        tunable_cli_label(tunable_state_for_label(content, "Autonomy"))
    );
    println!(
        "3. brevity             {}",
        tunable_cli_label(tunable_state_for_label(content, "Brevity"))
    );
    println!(
        "4. caution             {}",
        tunable_cli_label(tunable_state_for_label(content, "Caution"))
    );
    println!(
        "5. warmth              {}",
        tunable_cli_label(tunable_state_for_label(content, "Warmth"))
    );
    println!(
        "6. planning            {}",
        tunable_cli_label(tunable_state_for_label(content, "Planning"))
    );
    println!("7. switch to source mode");
    println!("s. save and exit");
    println!("q. quit without saving");
}

fn print_personality_source(scope: PersonalityScopeCli, path: &Path, content: &str) {
    println!();
    println!("Personality — source mode");
    println!("scope: {}", scope.label());
    println!("path: {}", path.display());
    println!("identity: {}", soul_identity_text(content));
    println!("Preview:");
    for line in content.lines().take(16) {
        println!("  {line}");
    }
    if content.lines().count() > 16 {
        println!("  …");
    }
    println!("1. switch scope");
    println!("2. switch to builder mode");
    println!("3. open in $EDITOR");
    println!("4. reset to default soul");
    println!("s. save and exit");
    println!("q. quit without saving");
}

fn cycle_personality_tunable(content: &str, label: &str) -> String {
    let next_idx = match tunable_state_for_label(content, label) {
        SoulTunableState::Preset(idx) => (idx + 1) % 5,
        SoulTunableState::Missing | SoulTunableState::Edited => 0,
    };
    let new_line = generated_tunable_line(label, next_idx)
        .unwrap_or_else(|| format!("- {label}: {}", next_idx));
    replace_tunable_line(content, label, &new_line)
}

fn run_personality_mode() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let (global_path, project_path) = personality_scope_paths(&cwd);
    let mut global_content = load_soul_content(&global_path);
    let mut project_content = load_soul_content(&project_path);
    let mut scope = if discover_project_soul(&cwd).is_some() {
        PersonalityScopeCli::Project
    } else {
        PersonalityScopeCli::Global
    };
    let mut mode = PersonalityModeCli::Builder;

    loop {
        let (path, content) = match scope {
            PersonalityScopeCli::Global => (&global_path, &mut global_content),
            PersonalityScopeCli::Project => (&project_path, &mut project_content),
        };

        match mode {
            PersonalityModeCli::Builder => print_personality_builder(scope, path, content),
            PersonalityModeCli::Source => print_personality_source(scope, path, content),
        }

        let Some(choice) = prompt_optional_input_line("Select field> ")? else {
            println!();
            return Ok(());
        };

        match mode {
            PersonalityModeCli::Builder => match choice.trim() {
                "1" => scope = scope.toggle(),
                "2" => {
                    *content = cycle_personality_tunable(content, "Autonomy");
                    println!("Updated autonomy.");
                }
                "3" => {
                    *content = cycle_personality_tunable(content, "Brevity");
                    println!("Updated brevity.");
                }
                "4" => {
                    *content = cycle_personality_tunable(content, "Caution");
                    println!("Updated caution.");
                }
                "5" => {
                    *content = cycle_personality_tunable(content, "Warmth");
                    println!("Updated warmth.");
                }
                "6" => {
                    *content = cycle_personality_tunable(content, "Planning");
                    println!("Updated planning.");
                }
                "7" => mode = PersonalityModeCli::Source,
                "s" | "save" => {
                    save_soul_content(path, content)?;
                    println!("Soul saved to {}", path.display());
                    return Ok(());
                }
                "q" | "quit" => {
                    println!("Discarded personality changes.");
                    return Ok(());
                }
                other => println!("Unknown selection: {other}"),
            },
            PersonalityModeCli::Source => match choice.trim() {
                "1" => scope = scope.toggle(),
                "2" => mode = PersonalityModeCli::Builder,
                "3" => {
                    save_soul_content(path, content)?;
                    open_in_editor(path)?;
                    *content = load_soul_content(path);
                    println!("Reloaded soul from {}", path.display());
                }
                "4" => {
                    *content = default_soul_markdown();
                    println!("Reset current scope to default soul markdown in memory.");
                }
                "s" | "save" => {
                    save_soul_content(path, content)?;
                    println!("Soul saved to {}", path.display());
                    return Ok(());
                }
                "q" | "quit" => {
                    println!("Discarded personality changes.");
                    return Ok(());
                }
                other => println!("Unknown selection: {other}"),
            },
        }
    }
}

fn run_settings_mode() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let config_path = imp_core::storage::global_config_path();
    let mut config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;

    loop {
        println!();
        print_settings_summary(&config, &config_path);
        let Some(choice) = prompt_optional_input_line("Select field> ")? else {
            println!();
            return Ok(());
        };

        match choice.trim() {
            "1" => {
                let value = prompt_input_line("model> ")?;
                if value.is_empty() {
                    config.model = None;
                    println!("Cleared model.");
                } else {
                    config.model = Some(value);
                    println!("Updated model.");
                }
            }
            "2" => {
                let value = prompt_input_line("thinking [off|minimal|low|medium|high|xhigh]> ")?;
                if value.is_empty() {
                    config.thinking = None;
                    println!("Cleared thinking level.");
                } else if let Some(level) = parse_thinking_level_strict(&value) {
                    config.thinking = Some(level);
                    println!("Updated thinking level.");
                } else {
                    println!("Unknown thinking level: {value}");
                }
            }
            "3" => {
                let value = prompt_input_line("max_tokens> ")?;
                if value.is_empty() {
                    config.max_tokens = None;
                    println!("Cleared max_tokens.");
                } else if let Ok(parsed) = value.parse::<u32>() {
                    config.max_tokens = Some(parsed.max(1));
                    println!("Updated max_tokens.");
                } else {
                    println!("Expected a positive integer.");
                }
            }
            "4" => {
                let value = prompt_input_line("max_turns> ")?;
                if value.is_empty() {
                    config.max_turns = None;
                    println!("Cleared max_turns.");
                } else if let Ok(parsed) = value.parse::<u32>() {
                    config.max_turns = Some(parsed.max(1));
                    println!("Updated max_turns.");
                } else {
                    println!("Expected a positive integer.");
                }
            }
            "5" => {
                let value = prompt_input_line("tool_output [full|compact|collapsed]> ")?;
                if let Some(display) = parse_tool_output_display(&value) {
                    config.ui.tool_output = display;
                    println!("Updated tool output display.");
                } else {
                    println!("Expected one of: full, compact, collapsed.");
                }
            }
            "6" => {
                let value =
                    prompt_input_line("web_search_provider [none|tavily|exa|linkup|perplexity]> ")?;
                match value.trim().to_lowercase().as_str() {
                    "" | "none" => {
                        config.web.search_provider = None;
                        println!("Cleared web search provider.");
                    }
                    "tavily" => {
                        config.web.search_provider = Some(SearchProvider::Tavily);
                        println!("Updated web search provider.");
                    }
                    "exa" => {
                        config.web.search_provider = Some(SearchProvider::Exa);
                        println!("Updated web search provider.");
                    }
                    "linkup" => {
                        config.web.search_provider = Some(SearchProvider::Linkup);
                        println!("Updated web search provider.");
                    }
                    "perplexity" => {
                        config.web.search_provider = Some(SearchProvider::Perplexity);
                        println!("Updated web search provider.");
                    }
                    _ => println!("Expected one of: none, tavily, exa, linkup, perplexity."),
                }
            }
            "s" | "save" => {
                let saved_path = save_user_config(&config)?;
                println!("Saved settings to {}", saved_path.display());
                return Ok(());
            }
            "q" | "quit" => {
                println!("Discarded settings changes.");
                return Ok(());
            }
            other => {
                println!("Unknown selection: {other}");
            }
        }
    }
}

fn thinking_level_label(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::Off => "off",
        ThinkingLevel::Minimal => "minimal",
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
        ThinkingLevel::XHigh => "xhigh",
    }
}

fn shell_session_choice(cli: &Cli) -> SessionChoice {
    if cli.no_session {
        SessionChoice::InMemory
    } else if cli.cont {
        SessionChoice::Continue
    } else if let Some(ref path) = cli.session {
        SessionChoice::Open(path.clone())
    } else {
        SessionChoice::New
    }
}

fn shell_project_label(cwd: &Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| cwd.display().to_string())
}

fn shell_session_label(session: &ImpSession) -> String {
    session
        .session_manager()
        .title(32)
        .or_else(|| session.session_manager().session_id())
        .unwrap_or_else(|| "in-memory".to_string())
}

fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        singular.to_string()
    } else {
        plural.to_string()
    }
}

fn format_chat_status_line(session: &ImpSession) -> String {
    let project = shell_project_label(session.cwd());
    let model = truncate_chars_with_suffix(&session.model().meta.id, 20, "…");
    let thinking = session
        .config()
        .thinking
        .map(thinking_level_label)
        .unwrap_or("off");
    let session_label = shell_session_label(session);

    format!("[{project} · {model} · {thinking} · {session_label}]")
}

#[derive(Debug, Default)]
struct ChatTurnSummaryState {
    tool_calls: usize,
    read_paths: std::collections::HashSet<String>,
    changed_paths: std::collections::HashSet<String>,
    other_tools: usize,
    had_tool_error: bool,
    mana_review_state: Option<imp_core::mana_review::ManaReviewState>,
}

impl ChatTurnSummaryState {
    fn record_tool_start(&mut self, tool_name: &str, args: &Value) {
        self.tool_calls += 1;
        match tool_name {
            "read" => {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    self.read_paths.insert(abbreviate_home_path(path));
                } else {
                    self.other_tools += 1;
                }
            }
            "write" | "edit" | "multi_edit" => {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    self.changed_paths.insert(abbreviate_home_path(path));
                } else {
                    self.other_tools += 1;
                }
            }
            _ => {
                self.other_tools += 1;
            }
        }
    }

    fn summary_line(&self, cost_total: f64) -> String {
        let mut parts = Vec::new();

        if self.tool_calls == 0 {
            parts.push("analysis only".to_string());
        } else {
            if !self.read_paths.is_empty() {
                let count = self.read_paths.len();
                parts.push(format!(
                    "{count} {} read",
                    pluralize(count, "file", "files")
                ));
            }
            if !self.changed_paths.is_empty() {
                let count = self.changed_paths.len();
                parts.push(format!(
                    "{count} {} changed",
                    pluralize(count, "file", "files")
                ));
            }
            if self.other_tools > 0 {
                let count = self.other_tools;
                parts.push(format!(
                    "{count} {} used",
                    pluralize(count, "tool", "tools")
                ));
            }
        }

        if self.had_tool_error {
            parts.push("errors surfaced".to_string());
        }

        match self.mana_review_state {
            Some(imp_core::mana_review::ManaReviewState::NeedsDecision) => {
                parts.push("review required".to_string())
            }
            Some(imp_core::mana_review::ManaReviewState::Changed) => {
                parts.push("durable changes captured".to_string())
            }
            _ => {}
        }

        parts.push(format!("cost ${cost_total:.4}"));
        format!("summary: {}", parts.join(" · "))
    }
}

fn abbreviate_home_path(path: &str) -> String {
    for prefix in ["/Users/", "/home/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            if let Some((_, suffix)) = rest.split_once('/') {
                return format!("~/{suffix}");
            }
            return "~".to_string();
        }
    }
    path.to_string()
}

fn print_chat_status(session: &ImpSession) {
    println!("{}", format_chat_status_line(session));
}

fn print_chat_status_detail(session: &ImpSession) {
    let project = shell_project_label(session.cwd());
    let model = &session.model().meta.id;
    let provider = &session.model().meta.provider;
    let thinking = session
        .config()
        .thinking
        .map(thinking_level_label)
        .unwrap_or("off");
    let session_id = session
        .session_manager()
        .session_id()
        .unwrap_or_else(|| "in-memory".to_string());
    let title = shell_session_label(session);
    let path = session
        .session_manager()
        .path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(in-memory)".to_string());
    println!("Status");
    println!("  project     {project}");
    println!("  model       {model}");
    println!("  provider    {provider}");
    println!("  thinking    {thinking}");
    println!("  session     {title}");
    println!("  session id  {session_id}");
    println!("  path        {path}");
}

async fn build_chat_session(
    cli: &Cli,
    session_choice: SessionChoice,
) -> Result<ImpSession, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;

    let mut options = SessionOptions {
        cwd: cwd.clone(),
        model: cli.model.clone(),
        provider: cli.provider.clone(),
        api_key: cli.api_key.clone(),
        thinking: cli
            .thinking
            .as_ref()
            .map(|thinking| parse_thinking_level(thinking)),
        max_turns: cli.max_turns.or(config.max_turns),
        max_tokens: cli.max_tokens.or(config.max_tokens),
        system_prompt: cli.system_prompt.clone(),
        no_tools: cli.no_tools,
        session: session_choice,
        ui: Some(Arc::new(CliTerminalUi) as Arc<dyn UserInterface>),
        ..Default::default()
    };

    if !cli.no_tools {
        options.lua_loader = build_lua_loader(false, cwd.clone());
    }

    ImpSession::create(options)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })
}

fn format_chat_tool_summary(tool_name: &str, args: &Value) -> String {
    match tool_name {
        "bash" => {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start();
            if command.is_empty() {
                String::new()
            } else if command.starts_with("rg ")
                || command.starts_with("grep ")
                || command.starts_with("fd ")
                || command.starts_with("find ")
                || command == "find"
                || command.starts_with("ls ")
                || command == "ls"
            {
                "search".to_string()
            } else if command.contains("check")
                || command.contains("test")
                || command.contains("verify")
                || command.contains("lint")
            {
                "check".to_string()
            } else {
                "run".to_string()
            }
        }
        "read" | "write" | "edit" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(abbreviate_home_path)
            .unwrap_or_default(),
        "scan" => {
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match action {
                "extract" => args
                    .get("files")
                    .and_then(|v| v.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(abbreviate_home_path)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| "extract".to_string()),
                "scan" => args
                    .get("directory")
                    .and_then(|v| v.as_str())
                    .map(abbreviate_home_path)
                    .unwrap_or_default(),
                _ => {
                    if action == tool_name {
                        String::new()
                    } else {
                        action.to_string()
                    }
                }
            }
        }
        "mana" => args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

async fn execute_chat_shell_command(
    session: &mut ImpSession,
    cli: &Cli,
    command: ChatShellCommand,
) -> Result<bool, Box<dyn std::error::Error>> {
    match command {
        ChatShellCommand::Help(topic) => {
            match topic.as_deref() {
                Some("model") => {
                    println!(
                        "Current model: {}\nUsage: :model <name>\nHint: run `imp --list-models` for the full built-in catalog.",
                        session.model().meta.id
                    );
                }
                Some("thinking") => {
                    let current = session
                        .config()
                        .thinking
                        .map(thinking_level_label)
                        .unwrap_or("off");
                    println!(
                        "Current thinking level: {current}\nUsage: :thinking <off|minimal|low|medium|high|xhigh>"
                    );
                }
                _ => {
                    println!(
                        "Shell commands\n  :help [topic]      Show commands and quick guidance\n  :status            Show current shell/session status\n  :new               Start a fresh session\n  :resume            Continue the most recent session for this cwd\n  :compact           Compact older context (planned)\n  :settings          Edit a guided subset of imp settings\n  :personality       Edit soul/personality tunables and source\n  :setup             Run the setup wizard\n  :view <area>       Open `imp view` for sessions, tree, logs, or checkpoints\n  :model <name>      Switch model for later prompts\n  :thinking <level>  Set thinking level for later prompts\n  :quit              Exit chat\n\nCompatibility\n  Slash-prefixed forms like `/help` and `/view` still work during migration,\n  but `:` is the preferred shell grammar."
                    );
                }
            }
            Ok(true)
        }
        ChatShellCommand::Quit => Ok(false),
        ChatShellCommand::Status => {
            print_chat_status_detail(session);
            Ok(true)
        }
        ChatShellCommand::New => {
            let replacement = build_chat_session(cli, SessionChoice::New).await?;
            *session = replacement;
            println!("Started a fresh session.");
            Ok(true)
        }
        ChatShellCommand::Resume => {
            let replacement = build_chat_session(cli, SessionChoice::Continue).await?;
            let resumed = shell_session_label(&replacement);
            let session_id = replacement
                .session_manager()
                .session_id()
                .unwrap_or_else(|| "in-memory".to_string());
            *session = replacement;
            println!("Resumed session: {resumed} ({session_id})");
            Ok(true)
        }
        ChatShellCommand::Compact => {
            println!("Compaction isn't available in the shell yet.");
            println!("For now, use `imp tui` or inspect session history with `imp view sessions`.");
            println!("A shell-native compaction flow is planned.");
            Ok(true)
        }
        ChatShellCommand::Settings => {
            run_settings_mode()?;
            Ok(true)
        }
        ChatShellCommand::Personality => {
            run_personality_mode()?;
            Ok(true)
        }
        ChatShellCommand::Setup => {
            run_setup_mode().await?;
            Ok(true)
        }
        ChatShellCommand::View(area) => {
            let area = area.unwrap_or_else(|| "sessions".to_string());
            println!("Opening `imp view {area}`...");
            run_view_mode(cli, Some(area.as_str())).await?;
            println!("Returned from viewer.");
            Ok(true)
        }
        ChatShellCommand::Model(None) => {
            let mut models: Vec<String> = session
                .model_registry()
                .list()
                .iter()
                .map(|meta| meta.id.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            models.sort();
            let preview = models.into_iter().take(12).collect::<Vec<_>>().join(", ");
            println!(
                "Current model: {}\nUsage: :model <name>\nExamples: {}{}",
                session.model().meta.id,
                preview,
                if preview.is_empty() { "" } else { ", ..." }
            );
            Ok(true)
        }
        ChatShellCommand::Model(Some(name)) => {
            session
                .set_model(name.trim())
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
            println!("Model set to {}", session.model().meta.id);
            Ok(true)
        }
        ChatShellCommand::Thinking(None) => {
            let current = session
                .config()
                .thinking
                .map(thinking_level_label)
                .unwrap_or("off");
            println!(
                "Current thinking level: {current}\nUsage: :thinking <off|minimal|low|medium|high|xhigh>"
            );
            Ok(true)
        }
        ChatShellCommand::Thinking(Some(level)) => {
            let Some(parsed) = parse_thinking_level_strict(level.trim()) else {
                println!("Unknown thinking level: {}", level.trim());
                println!("Expected one of: off, minimal, low, medium, high, xhigh");
                return Ok(true);
            };
            session.set_thinking(parsed);
            println!("Thinking level set to {}", thinking_level_label(parsed));
            Ok(true)
        }
        ChatShellCommand::Unknown(raw) => {
            println!("Unknown shell command: :{raw}");
            println!("Type :help for available commands.");
            Ok(true)
        }
    }
}

async fn run_chat_prompt(
    session: &mut ImpSession,
    cli: &Cli,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    session
        .prompt(prompt)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    let mut printed_trailing_newline = true;
    let mut printed_thinking = false;
    let mut printed_any_text = false;
    let mut active_tools = 0u32;
    let mut liveness = ShellLiveness::new();
    let mut turn_summary = ChatTurnSummaryState::default();

    while let Some(event) = session.recv_event().await {
        bump_shell_liveness(
            &mut liveness,
            printed_thinking,
            printed_any_text,
            active_tools,
        );
        match event {
            AgentEvent::MessageDelta { delta } => match delta {
                StreamEvent::TextDelta { text } => {
                    clear_shell_liveness_for_output(&mut liveness, &mut printed_trailing_newline);
                    print!("{text}");
                    io::stdout().flush().ok();
                    printed_trailing_newline = false;
                    printed_any_text = true;
                }
                StreamEvent::ThinkingDelta { .. } => {
                    printed_thinking = true;
                    bump_shell_liveness(
                        &mut liveness,
                        printed_thinking,
                        printed_any_text,
                        active_tools,
                    );
                }
                _ => {}
            },
            AgentEvent::ToolExecutionStart {
                tool_name, args, ..
            } if !cli.no_tools => {
                active_tools = active_tools.saturating_add(1);
                clear_shell_liveness_for_output(&mut liveness, &mut printed_trailing_newline);
                turn_summary.record_tool_start(&tool_name, &args);
                let summary = format_chat_tool_summary(&tool_name, &args);
                if summary.is_empty() {
                    eprintln!("tool: {tool_name}");
                } else {
                    eprintln!("tool: {tool_name} {summary}");
                }
            }
            AgentEvent::ToolExecutionEnd { result, .. } if !cli.no_tools => {
                active_tools = active_tools.saturating_sub(1);
                if result.is_error {
                    let text: String = result
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    clear_shell_liveness_for_output(&mut liveness, &mut printed_trailing_newline);
                    if !text.is_empty() {
                        eprintln!("error: {}", truncate_chars_with_suffix(&text, 160, "…"));
                        eprintln!("next: deeper log viewing is planned under `imp view logs`.");
                    }
                }
            }
            AgentEvent::TurnEnd { .. } => {
                liveness.clear();
                if !printed_trailing_newline {
                    println!();
                    printed_trailing_newline = true;
                }
            }
            AgentEvent::Error { error } => {
                clear_shell_liveness_for_output(&mut liveness, &mut printed_trailing_newline);
                eprintln!("error: {}", format_error_for_display(&error));
            }
            AgentEvent::Timing { timing } => {
                if cli.verbose {
                    clear_shell_liveness_for_output(&mut liveness, &mut printed_trailing_newline);
                    eprintln!("{}", format_timing_event(&timing));
                }
            }
            AgentEvent::AgentEnd { usage: _, cost } => {
                clear_shell_liveness_for_output(&mut liveness, &mut printed_trailing_newline);
                eprintln!("{}", turn_summary.summary_line(cost.total));
                break;
            }
            _ => {}
        }
    }

    session
        .wait()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    Ok(())
}

async fn run_chat_mode(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let mut session = build_chat_session(cli, shell_session_choice(cli)).await?;

    println!("imp chat");
    println!("  Use :help for commands. Ctrl-D exits.");

    loop {
        print_chat_status(&session);
        print!("imp> ");
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes = io::stdin().read_line(&mut input)?;
        if bytes == 0 {
            println!();
            break;
        }

        let input = input.trim_end_matches(['\r', '\n']).to_string();
        if input.trim().is_empty() {
            continue;
        }

        if let Some(command) = parse_chat_shell_command(&input) {
            if !execute_chat_shell_command(&mut session, cli, command).await? {
                break;
            }
            continue;
        }

        run_chat_prompt(&mut session, cli, &input).await?;
    }

    Ok(())
}

async fn run_view_mode(_cli: &Cli, area: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let session_dir = imp_core::storage::global_sessions_dir();

    match area.unwrap_or("sessions") {
        "sessions" => {
            let sessions = SessionManager::list(&session_dir)?;
            if sessions.is_empty() {
                println!("No saved sessions found.");
                return Ok(());
            }

            println!("Sessions\n========");
            for (idx, session) in sessions.iter().enumerate().take(20) {
                let title = session.title(72).unwrap_or_else(|| session.id.clone());
                let project = shell_project_label(Path::new(&session.cwd));
                println!("{}. {}", idx + 1, title);
                println!("   id: {}", session.id);
                println!("   project: {}", project);
                println!("   path: {}", session.path.display());
                println!("   messages: {}", session.message_count);
                if let Some(summary) = &session.summary {
                    println!(
                        "   summary: {}",
                        truncate_chars_with_suffix(summary, 120, "…")
                    );
                }
            }
            if sessions.len() > 20 {
                println!("… {} more session(s)", sessions.len() - 20);
            }
            Ok(())
        }
        "tree" => {
            let session =
                SessionManager::continue_recent(&cwd, &session_dir)?.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "No recent session found for this working directory.",
                    )
                })?;
            let tree = session.get_tree();
            if tree.is_empty() {
                println!("No session history yet.");
                return Ok(());
            }

            println!("Session tree\n============");
            print_tree_nodes(&tree, 0);
            Ok(())
        }
        "logs" => {
            let session =
                SessionManager::continue_recent(&cwd, &session_dir)?.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "No recent session found for this working directory.",
                    )
                })?;
            println!("Session log\n===========");
            for entry in session.entries().iter().rev().take(40).rev() {
                println!("{}", summarize_session_entry(entry));
            }
            Ok(())
        }
        "checkpoints" => {
            let session =
                SessionManager::continue_recent(&cwd, &session_dir)?.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "No recent session found for this working directory.",
                    )
                })?;
            let checkpoints = session.checkpoint_records();
            if checkpoints.is_empty() {
                println!("No checkpoints recorded in the most recent session for this working directory.");
                return Ok(());
            }

            println!("Checkpoints\n===========");
            for checkpoint in checkpoints {
                let label = checkpoint
                    .label
                    .as_deref()
                    .map(|label| format!(" — {label}"))
                    .unwrap_or_default();
                println!(
                    "- {}{} ({} file{})",
                    checkpoint.checkpoint_id,
                    label,
                    checkpoint.files.len(),
                    if checkpoint.files.len() == 1 { "" } else { "s" }
                );
                for file in checkpoint.files.iter().take(8) {
                    println!("    {file}");
                }
                if checkpoint.files.len() > 8 {
                    println!("    … {} more", checkpoint.files.len() - 8);
                }
            }
            Ok(())
        }
        other => {
            eprintln!(
                "Unknown viewer area: {other}. Use one of: sessions, tree, logs, checkpoints."
            );
            Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown viewer area").into())
        }
    }
}

fn print_tree_nodes(nodes: &[imp_core::session::TreeNode], depth: usize) {
    for node in nodes {
        let indent = "  ".repeat(depth);
        let summary = match &node.entry {
            SessionEntry::Header { cwd, .. } => format!("header {cwd}"),
            SessionEntry::SessionMeta { name, summary, .. } => format!(
                "session-meta {}{}",
                name.as_deref().unwrap_or("(unnamed)"),
                summary
                    .as_deref()
                    .map(|s| format!(" — {}", truncate_chars_with_suffix(s, 60, "…")))
                    .unwrap_or_default()
            ),
            SessionEntry::Message { message, .. } => summarize_message_for_view(message),
            SessionEntry::Compaction { summary, .. } => {
                format!(
                    "compaction {}",
                    truncate_chars_with_suffix(summary, 60, "…")
                )
            }
            SessionEntry::Label { label, .. } => format!("label {label}"),
            SessionEntry::Custom { custom_type, .. } => format!("custom {custom_type}"),
        };
        println!("{indent}- {summary}");
        print_tree_nodes(&node.children, depth + 1);
    }
}

fn summarize_message_for_view(message: &Message) -> String {
    let text_content = |message: &Message| -> Option<String> {
        let blocks = match message {
            Message::User(user) => &user.content,
            Message::Assistant(assistant) => &assistant.content,
            Message::ToolResult(result) => &result.content,
        };
        blocks.iter().find_map(|block| match block {
            imp_llm::ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
    };

    match message {
        Message::User(user) => format!(
            "user {}",
            truncate_chars_with_suffix(
                &text_content(message).unwrap_or_else(|| {
                    user.content
                        .iter()
                        .filter_map(|block| match block {
                            imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }),
                80,
                "…"
            )
        ),
        Message::Assistant(_) => format!(
            "assistant {}",
            truncate_chars_with_suffix(&text_content(message).unwrap_or_default(), 80, "…")
        ),
        Message::ToolResult(result) => format!(
            "tool-result {}",
            truncate_chars_with_suffix(
                &text_content(message).unwrap_or_else(|| result.tool_call_id.clone()),
                80,
                "…"
            )
        ),
    }
}

fn summarize_session_entry(entry: &SessionEntry) -> String {
    match entry {
        SessionEntry::Header { cwd, .. } => format!("header cwd={cwd}"),
        SessionEntry::SessionMeta { name, summary, .. } => format!(
            "session-meta name={} summary={}",
            name.as_deref().unwrap_or("(unnamed)"),
            summary.as_deref().unwrap_or("(none)")
        ),
        SessionEntry::Message { message, .. } => summarize_message_for_view(message),
        SessionEntry::Compaction { summary, .. } => {
            format!(
                "compaction {}",
                truncate_chars_with_suffix(summary, 100, "…")
            )
        }
        SessionEntry::Label { label, .. } => format!("label {label}"),
        SessionEntry::Custom { custom_type, .. } => format!("custom {custom_type}"),
    }
}

async fn run_interactive(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let interactive_result = async {
        let cwd = std::env::current_dir()?;
        let config = Config::resolve(&imp_core::storage::global_root(), Some(&cwd))?;

        let registry = ModelRegistry::with_builtins();

        let session = if cli.no_session {
            SessionManager::in_memory()
        } else if cli.cont {
            // Continue most recent session
            match SessionManager::continue_recent(&cwd, &imp_core::storage::global_sessions_dir())?
            {
                Some(session) => session,
                None => SessionManager::new(&cwd, &imp_core::storage::global_sessions_dir())?,
            }
        } else if let Some(ref path) = cli.session {
            SessionManager::open(path)?
        } else {
            // New persistent session
            SessionManager::new(&cwd, &imp_core::storage::global_sessions_dir())?
        };

        let mut runner =
            imp_tui::interactive::InteractiveRunner::new(config, session, registry, cwd)?;

        // Apply CLI overrides
        if let Some(ref model) = cli.model {
            runner.app_mut().model_name = model.clone();
        }
        if let Some(ref thinking) = cli.thinking {
            runner.app_mut().thinking_level = parse_thinking_level(thinking);
        }
        if cli.max_turns.is_some() {
            runner.app_mut().max_turns_override = cli.max_turns;
        }

        runner.run_guarded().await.map_err(Into::into)
    }
    .await;

    interactive_result
}

/// Expand @file arguments into file content blocks.
/// Returns a string with each file's content wrapped in XML-like tags.
fn expand_file_args(args: &[String]) -> String {
    let mut parts = Vec::new();
    for arg in args {
        if let Some(path_str) = arg.strip_prefix('@') {
            let path = std::path::Path::new(path_str);
            // Expand ~ in @~/path
            let resolved = if let Some(rest) = path_str.strip_prefix("~/") {
                if let Ok(home) = std::env::var("HOME") {
                    std::path::PathBuf::from(home).join(rest)
                } else {
                    path.to_path_buf()
                }
            } else {
                path.to_path_buf()
            };
            match std::fs::read_to_string(&resolved) {
                Ok(content) => {
                    parts.push(format!(
                        "<file path=\"{}\">\n{}\n</file>",
                        resolved.display(),
                        content.trim_end()
                    ));
                }
                Err(e) => {
                    eprintln!("Warning: cannot read {}: {e}", resolved.display());
                }
            }
        }
    }
    parts.join("\n\n")
}

/// Build the full prompt from user text, @file context, and stdin.
fn build_full_prompt(prompt: &str, file_context: &str, stdin: &Option<String>) -> String {
    let mut parts = Vec::new();
    if !file_context.is_empty() {
        parts.push(file_context.to_string());
    }
    if let Some(ref content) = stdin {
        parts.push(format!("<stdin>\n{}\n</stdin>", content.trim_end()));
    }
    if !prompt.is_empty() {
        parts.push(prompt.to_string());
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures_util::Stream;
    use imp_llm::auth::{OAuthCredential, StoredCredential};
    use imp_llm::message::{AssistantMessage, ContentBlock, StopReason};
    use imp_llm::provider::{Context as ProviderContext, Provider, RequestOptions, ThinkingLevel};
    use imp_llm::stream::StreamEvent;
    use imp_llm::{Model, ModelMeta};
    use mana_core::unit::Unit;
    use serde_json::json;
    use std::pin::Pin;
    use std::sync::Arc;

    /// Helper: build a minimal Cli struct with defaults for testing.
    fn default_cli() -> Cli {
        Cli {
            print: None,
            provider: None,
            model: None,
            thinking: None,
            api_key: None,
            cont: false,
            resume: false,
            session: None,
            no_session: false,
            tools: None,
            no_tools: false,
            system_prompt: None,
            mode: "interactive".to_string(),
            max_turns: None,
            max_tokens: None,
            verbose: false,
            list_models: false,
            args: Vec::new(),
            command: None,
        }
    }

    fn empty_auth_store() -> AuthStore {
        AuthStore::new(std::path::PathBuf::from("auth.json"))
    }

    struct StaticTestProvider {
        text: String,
    }

    #[async_trait]
    impl Provider for StaticTestProvider {
        fn stream(
            &self,
            model: &Model,
            _context: ProviderContext,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            let message = AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: self.text.clone(),
                }],
                usage: None,
                stop_reason: StopReason::EndTurn,
                timestamp: 0,
            };
            let events = vec![
                Ok(StreamEvent::MessageStart {
                    model: model.meta.id.clone(),
                }),
                Ok(StreamEvent::TextDelta {
                    text: self.text.clone(),
                }),
                Ok(StreamEvent::MessageEnd { message }),
            ];
            Box::pin(futures_util::stream::iter(events))
        }

        async fn resolve_auth(
            &self,
            _auth: &AuthStore,
        ) -> imp_llm::Result<imp_llm::auth::ApiKey> {
            Ok("test-key".to_string())
        }

        fn id(&self) -> &str {
            "test-provider"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    fn static_test_model(text: &str) -> Model {
        Model {
            meta: ModelMeta {
                id: "test-model".to_string(),
                provider: "test-provider".to_string(),
                name: "Test Model".to_string(),
                context_window: 4096,
                max_output_tokens: 1024,
                pricing: Default::default(),
                capabilities: Default::default(),
            },
            provider: Arc::new(StaticTestProvider {
                text: text.to_string(),
            }),
        }
    }

    fn write_test_mana_unit(
        root: &std::path::Path,
        id: &str,
        title_slug: &str,
        title: &str,
        description: &str,
        verify: &str,
    ) {
        let mana_dir = root.join(".mana");
        std::fs::create_dir_all(&mana_dir).unwrap();
        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();

        let mut unit = Unit::new(id, title);
        unit.description = Some(description.to_string());
        unit.verify = Some(verify.to_string());
        unit.updated_at = unit.created_at;
        unit.to_file(&mana_dir.join(format!("{id}-{title_slug}.md")))
            .unwrap();
    }

    #[test]
    fn cli_parses_mana_namespace_unit_target_for_headless_worker() {
        let cli = Cli::try_parse_from(["imp", "mana", "5.1"]).expect("parse mana unit target");
        match cli.command {
            Some(Commands::Mana(args)) => {
                assert_eq!(args.target, "5.1");
                assert!(args.args.is_empty());
                assert!(args.mana_dir.is_none());
                assert!(!args.defer_verify);
            }
            other => panic!("expected mana namespace subcommand, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_run_as_compatibility_alias_for_headless_mana_worker() {
        let cli =
            Cli::try_parse_from(["imp", "run", "5.1", "--defer-verify"]).expect("parse run alias");
        match cli.command {
            Some(Commands::Run(args)) => {
                assert_eq!(args.unit_id, "5.1");
                assert!(args.defer_verify);
            }
            other => panic!("expected run compatibility alias, got {other:?}"),
        }
    }

    #[test]
    fn cli_parses_reserved_mana_namespace_verb_with_passthrough_args() {
        let cli = Cli::try_parse_from(["imp", "mana", "show", "28.1"]).expect("parse mana show");
        match cli.command {
            Some(Commands::Mana(args)) => {
                assert_eq!(args.target, "show");
                assert_eq!(args.args, vec!["28.1".to_string()]);
            }
            other => panic!("expected mana namespace args, got {other:?}"),
        }
    }

    #[test]
    fn reserved_mana_namespace_commands_error_clearly() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let err = rt
            .block_on(run_reserved_mana_namespace_command("status", &[]))
            .expect_err("reserved command should error for now");
        let text = err.to_string();
        assert!(text.contains("reserved for a future mana-aware operator command"));
        assert!(text.contains("use `mana status` directly"));
    }

    #[test]
    fn worker_status_counts_as_success_for_completed_and_awaiting_verify_only() {
        assert!(worker_status_counts_as_success(
            imp_core::mana_worker::WorkerStatus::Completed
        ));
        assert!(worker_status_counts_as_success(
            imp_core::mana_worker::WorkerStatus::AwaitingVerify
        ));
        assert!(!worker_status_counts_as_success(
            imp_core::mana_worker::WorkerStatus::Failed
        ));
        assert!(!worker_status_counts_as_success(
            imp_core::mana_worker::WorkerStatus::Blocked
        ));
        assert!(!worker_status_counts_as_success(
            imp_core::mana_worker::WorkerStatus::Cancelled
        ));
    }

    #[tokio::test]
    async fn worker_run_end_to_end_with_model_override_closes_verified_unit() {
        let temp = tempfile::tempdir().unwrap();
        write_test_mana_unit(
            temp.path(),
            "1",
            "test-unit",
            "Test unit",
            "Say hi and finish.",
            "test -n ok",
        );

        let assignment = imp_core::mana_worker::load_assignment_with_mana_dir(
            temp.path(),
            "1",
            Some(temp.path().join(".mana").as_path()),
        )
        .expect("load assignment");

        let options = imp_core::mana_worker::WorkerRunOptions {
            cwd: temp.path().to_path_buf(),
            model_override: Some(static_test_model("done")),
            model: None,
            provider: None,
            api_key: None,
            system_prompt: None,
            thinking: Some(ThinkingLevel::Off),
            max_turns: Some(2),
            max_tokens: None,
            no_tools: false,
            mana_dir_override: Some(temp.path().join(".mana")),
            defer_verify: false,
            lua_loader: None,
        };

        let mut prepared = imp_core::mana_worker::prepare_worker_run(assignment, options)
            .await
            .expect("prepare worker run");
        prepared
            .session
            .prompt(&prepared.prompt)
            .await
            .expect("prompt session");
        prepared.session.wait().await.expect("wait for session");
        let outcome = imp_core::mana_worker::finalize_worker_run(prepared)
            .await
            .expect("finalize worker run");

        assert_eq!(outcome.result.status, imp_core::mana_worker::WorkerStatus::Completed);
        assert_eq!(outcome.verify_passed, Some(true));
        assert!(outcome.closed_after_verify);

        let archived = mana_core::ops::show::get(&temp.path().join(".mana"), "1")
            .expect("show closed unit");
        assert!(archived.unit.is_archived);
        assert_eq!(archived.unit.status.to_string(), "closed");
    }

    #[tokio::test]
    async fn run_headless_no_tools_worker_single_turn_exits_cleanly() {
        let temp = tempfile::tempdir().unwrap();
        write_test_mana_unit(
            temp.path(),
            "1",
            "headless-no-tools",
            "Headless no-tools unit",
            "Check mana status and finish.",
            "test -n ok",
        );

        let assignment = imp_core::mana_worker::load_assignment_with_mana_dir(
            temp.path(),
            "1",
            Some(temp.path().join(".mana").as_path()),
        )
        .expect("load assignment");

        let options = imp_core::mana_worker::WorkerRunOptions {
            cwd: temp.path().to_path_buf(),
            model_override: Some(static_test_model("SMOKE_OK")),
            model: None,
            provider: None,
            api_key: None,
            system_prompt: None,
            thinking: Some(ThinkingLevel::Off),
            max_turns: Some(1),
            max_tokens: None,
            no_tools: true,
            mana_dir_override: Some(temp.path().join(".mana")),
            defer_verify: true,
            lua_loader: None,
        };

        let mut prepared = imp_core::mana_worker::prepare_worker_run(assignment, options)
            .await
            .expect("prepare worker run");
        prepared
            .session
            .prompt(&prepared.prompt)
            .await
            .expect("prompt session");

        let events = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut events = Vec::new();
            while let Some(event) = prepared.session.recv_event().await {
                events.push(event);
            }
            events
        })
        .await
        .expect("headless event stream should complete promptly");

        tokio::time::timeout(std::time::Duration::from_secs(5), prepared.session.wait())
            .await
            .expect("headless session wait should complete promptly")
            .expect("wait for session");

        let saw_expected_text = events.iter().any(|event| {
            matches!(
                event,
                AgentEvent::MessageDelta {
                    delta: StreamEvent::TextDelta { text },
                } if text.contains("SMOKE_OK")
            )
        });
        let saw_max_turn_error = events.iter().any(|event| {
            matches!(event, AgentEvent::Error { error } if error.contains("Max turns exceeded"))
        });

        assert!(saw_expected_text);
        assert!(!saw_max_turn_error);
    }

    #[test]
    fn determine_headless_output_mode_prefers_human_for_terminal_run() {
        assert_eq!(
            determine_headless_output_mode("interactive", true),
            HeadlessOutputMode::Human
        );
        assert_eq!(
            determine_headless_output_mode("anything-else", true),
            HeadlessOutputMode::Human
        );
    }

    #[test]
    fn determine_headless_output_mode_keeps_json_for_piped_or_explicit_protocol_modes() {
        assert_eq!(
            determine_headless_output_mode("interactive", false),
            HeadlessOutputMode::Json
        );
        assert_eq!(
            determine_headless_output_mode("json", true),
            HeadlessOutputMode::Json
        );
        assert_eq!(
            determine_headless_output_mode("rpc", true),
            HeadlessOutputMode::Json
        );
    }

    #[test]
    fn parse_chat_shell_command_supports_colon_and_slash_prefix() {
        assert_eq!(
            parse_chat_shell_command(":help"),
            Some(ChatShellCommand::Help(None))
        );
        assert_eq!(
            parse_chat_shell_command("/quit"),
            Some(ChatShellCommand::Quit)
        );
        assert_eq!(
            parse_chat_shell_command(":status"),
            Some(ChatShellCommand::Status)
        );
    }

    #[test]
    fn parse_chat_shell_command_parses_model_and_thinking_args() {
        assert_eq!(
            parse_chat_shell_command(":model sonnet"),
            Some(ChatShellCommand::Model(Some("sonnet".to_string())))
        );
        assert_eq!(
            parse_chat_shell_command(":thinking high"),
            Some(ChatShellCommand::Thinking(Some("high".to_string())))
        );
        assert_eq!(
            parse_chat_shell_command(":view logs"),
            Some(ChatShellCommand::View(Some("logs".to_string())))
        );
        assert_eq!(
            parse_chat_shell_command(":settings"),
            Some(ChatShellCommand::Settings)
        );
        assert_eq!(
            parse_chat_shell_command(":personality"),
            Some(ChatShellCommand::Personality)
        );
        assert_eq!(
            parse_chat_shell_command(":setup"),
            Some(ChatShellCommand::Setup)
        );
        assert_eq!(
            parse_chat_shell_command(":resume"),
            Some(ChatShellCommand::Resume)
        );
    }

    #[test]
    fn parse_chat_shell_command_returns_unknown_for_unrecognized_commands() {
        assert_eq!(
            parse_chat_shell_command(":mystery abc"),
            Some(ChatShellCommand::Unknown("mystery abc".to_string()))
        );
    }

    #[test]
    fn provider_has_auth_detects_stored_credentials() {
        let path = std::path::PathBuf::from("auth.json");
        let mut auth_store = AuthStore::new(path);
        auth_store
            .store(
                "anthropic",
                StoredCredential::ApiKey {
                    key: "sk-test".into(),
                },
            )
            .unwrap();
        let provider = ProviderRegistry::with_builtins()
            .find("anthropic")
            .unwrap()
            .clone();
        assert!(provider_has_auth(&auth_store, &provider));
    }

    #[test]
    fn kimi_provider_alias_maps_to_moonshot() {
        assert_eq!(provider_alias("kimi"), "moonshot");
        assert_eq!(provider_alias("moonshot"), "moonshot");
    }

    #[test]
    fn run_secrets_show_accepts_kimi_alias() {
        assert_eq!(provider_alias("kimi"), "moonshot");
    }

    #[test]
    fn tunable_cli_label_formats_builder_states() {
        assert_eq!(tunable_cli_label(SoulTunableState::Preset(2)), "balanced");
        assert_eq!(tunable_cli_label(SoulTunableState::Edited), "edited");
        assert_eq!(tunable_cli_label(SoulTunableState::Missing), "missing");
    }

    #[test]
    fn cycle_personality_tunable_updates_markdown() {
        let content = default_soul_markdown();
        let updated = cycle_personality_tunable(&content, "Warmth");
        assert_ne!(updated, content);
        assert!(updated.contains("- Warmth:"));
    }

    #[test]
    fn parse_tool_output_display_accepts_known_values() {
        assert_eq!(
            parse_tool_output_display("full"),
            Some(ToolOutputDisplay::Full)
        );
        assert_eq!(
            parse_tool_output_display("compact"),
            Some(ToolOutputDisplay::Compact)
        );
        assert_eq!(
            parse_tool_output_display("collapsed"),
            Some(ToolOutputDisplay::Collapsed)
        );
        assert_eq!(parse_tool_output_display("mystery"), None);
    }

    #[test]
    fn web_search_provider_label_formats_none_and_provider_names() {
        assert_eq!(web_search_provider_label(None), "none");
        assert_eq!(web_search_provider_label(Some(SearchProvider::Exa)), "exa");
    }

    #[test]
    fn resolve_install_destination_prefers_active_user_imp_path() {
        let home = PathBuf::from("/Users/test");
        let path = Some(OsString::from("/Users/test/bin:/Users/test/.cargo/bin:/usr/bin"));
        let active_imp = Some(PathBuf::from("/Users/test/bin/imp"));

        let dest = resolve_install_destination(&home, path, active_imp, None);
        assert_eq!(dest, PathBuf::from("/Users/test/bin/imp"));
    }
    #[test]
    fn resolve_install_destination_falls_back_to_path_preference_when_imp_missing() {
        let home = PathBuf::from("/Users/test");
        let path = Some(OsString::from("/Users/test/bin:/Users/test/.cargo/bin:/usr/bin"));

        let dest = resolve_install_destination(&home, path, None, None);
        assert_eq!(dest, PathBuf::from("/Users/test/bin/imp"));
    }
    #[test]
    fn resolve_install_destination_uses_cargo_bin_when_no_user_bin_is_on_path() {
        let home = PathBuf::from("/Users/test");
        let path = Some(OsString::from("/usr/local/bin:/usr/bin"));

        let dest = resolve_install_destination(&home, path, None, None);
        assert_eq!(dest, PathBuf::from("/Users/test/.cargo/bin/imp"));
    }
    #[test]
    fn parse_thinking_level_strict_rejects_unknown_values() {
        assert_eq!(
            parse_thinking_level_strict("medium"),
            Some(ThinkingLevel::Medium)
        );
        assert_eq!(parse_thinking_level_strict("turbo"), None);
    }

    #[test]
    fn shell_session_choice_prefers_continue_and_open_over_new() {
        let mut cli = default_cli();
        assert!(matches!(shell_session_choice(&cli), SessionChoice::New));

        cli.no_session = true;
        assert!(matches!(
            shell_session_choice(&cli),
            SessionChoice::InMemory
        ));

        cli.no_session = false;
        cli.cont = true;
        assert!(matches!(
            shell_session_choice(&cli),
            SessionChoice::Continue
        ));

        cli.cont = false;
        cli.session = Some(PathBuf::from("session.jsonl"));
        assert!(matches!(
            shell_session_choice(&cli),
            SessionChoice::Open(path) if path == PathBuf::from("session.jsonl")
        ));
    }

    #[test]
    fn thinking_level_label_matches_expected_strings() {
        assert_eq!(thinking_level_label(ThinkingLevel::Off), "off");
        assert_eq!(thinking_level_label(ThinkingLevel::Minimal), "minimal");
        assert_eq!(thinking_level_label(ThinkingLevel::Low), "low");
        assert_eq!(thinking_level_label(ThinkingLevel::Medium), "medium");
        assert_eq!(thinking_level_label(ThinkingLevel::High), "high");
        assert_eq!(thinking_level_label(ThinkingLevel::XHigh), "xhigh");
    }

    // ── parse_thinking_level ───────────────────────────────────────

    #[test]
    fn parse_thinking_level_all_variants() {
        assert!(matches!(parse_thinking_level("off"), ThinkingLevel::Off));
        assert!(matches!(
            parse_thinking_level("minimal"),
            ThinkingLevel::Minimal
        ));
        assert!(matches!(parse_thinking_level("low"), ThinkingLevel::Low));
        assert!(matches!(
            parse_thinking_level("medium"),
            ThinkingLevel::Medium
        ));
        assert!(matches!(parse_thinking_level("high"), ThinkingLevel::High));
        assert!(matches!(
            parse_thinking_level("xhigh"),
            ThinkingLevel::XHigh
        ));
    }

    #[test]
    fn parse_thinking_level_unknown_defaults_to_off() {
        assert!(matches!(parse_thinking_level("turbo"), ThinkingLevel::Off));
        assert!(matches!(parse_thinking_level(""), ThinkingLevel::Off));
    }

    #[test]
    fn parse_thinking_level_case_insensitive() {
        assert!(matches!(parse_thinking_level("HIGH"), ThinkingLevel::High));
        assert!(matches!(
            parse_thinking_level("Medium"),
            ThinkingLevel::Medium
        ));
    }

    // ── resolve_model_and_provider ─────────────────────────────────

    #[test]
    fn resolve_model_sonnet_alias() {
        let cli = default_cli();
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let auth_store = empty_auth_store();
        let (model_id, provider) =
            resolve_model_and_provider(&cli, &config, &registry, &auth_store).unwrap();
        // Default is "sonnet"
        assert!(
            model_id.contains("sonnet"),
            "expected sonnet, got {model_id}"
        );
        assert_eq!(provider, "anthropic");
    }

    #[test]
    fn resolve_model_haiku_alias() {
        let mut cli = default_cli();
        cli.model = Some("haiku".to_string());
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let auth_store = empty_auth_store();
        let (model_id, provider) =
            resolve_model_and_provider(&cli, &config, &registry, &auth_store).unwrap();
        assert!(model_id.contains("haiku"), "expected haiku, got {model_id}");
        assert_eq!(provider, "anthropic");
    }

    #[test]
    fn resolve_model_unknown_alias_errors() {
        let mut cli = default_cli();
        cli.model = Some("nonexistent-xyz".to_string());
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let auth_store = empty_auth_store();
        let result = resolve_model_and_provider(&cli, &config, &registry, &auth_store);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown model"));
    }

    #[test]
    fn resolve_model_allows_custom_openai_model() {
        let mut cli = default_cli();
        cli.model = Some("gpt-4o".to_string());
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let auth_store = empty_auth_store();
        let (model_id, provider) =
            resolve_model_and_provider(&cli, &config, &registry, &auth_store).unwrap();
        assert_eq!(model_id, "gpt-4o");
        assert_eq!(provider, "openai");
    }

    #[test]
    fn resolve_model_cli_overrides_config() {
        let mut cli = default_cli();
        cli.model = Some("haiku".to_string());
        let mut config = Config::default();
        config.model = Some("sonnet".to_string());
        let registry = ModelRegistry::with_builtins();
        let auth_store = empty_auth_store();
        let (model_id, _) =
            resolve_model_and_provider(&cli, &config, &registry, &auth_store).unwrap();
        assert!(
            model_id.contains("haiku"),
            "CLI --model should override config"
        );
    }

    #[test]
    fn resolve_model_cli_provider_override() {
        let mut cli = default_cli();
        cli.provider = Some("openai".to_string());
        // Use default sonnet — provider override just changes provider name
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let auth_store = empty_auth_store();
        let (_, provider) =
            resolve_model_and_provider(&cli, &config, &registry, &auth_store).unwrap();
        assert_eq!(provider, "openai");
    }

    #[test]
    fn resolve_model_prefers_chatgpt_provider_when_only_oauth_is_available() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut auth_store = AuthStore::new(path);
        auth_store
            .store(
                "openai",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "oauth-token".into(),
                    refresh_token: "refresh-token".into(),
                    expires_at: imp_llm::now() + 3600,
                }),
            )
            .unwrap();

        let mut config = Config::default();
        config.model = Some("gpt-5.4".to_string());
        let registry = ModelRegistry::with_builtins();

        let (model_id, provider) =
            resolve_model_and_provider(&default_cli(), &config, &registry, &auth_store).unwrap();
        assert_eq!(model_id, "gpt-5.4");
        assert_eq!(provider, "openai-codex");
    }

    #[test]
    fn resolve_model_keeps_openai_when_api_key_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut auth_store = AuthStore::new(path);
        auth_store
            .store(
                "openai",
                StoredCredential::ApiKey {
                    key: "sk-openai".into(),
                },
            )
            .unwrap();
        auth_store
            .store(
                "openai-codex",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "oauth-token".into(),
                    refresh_token: "refresh-token".into(),
                    expires_at: imp_llm::now() + 3600,
                }),
            )
            .unwrap();

        let mut config = Config::default();
        config.model = Some("gpt-5.4".to_string());
        let registry = ModelRegistry::with_builtins();

        let (model_id, provider) =
            resolve_model_and_provider(&default_cli(), &config, &registry, &auth_store).unwrap();
        assert_eq!(model_id, "gpt-5.4");
        assert_eq!(provider, "openai");
    }

    #[test]
    fn resolve_custom_openai_model_does_not_switch_to_chatgpt_provider() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let mut auth_store = AuthStore::new(path);
        auth_store
            .store(
                "openai",
                StoredCredential::OAuth(OAuthCredential {
                    access_token: "oauth-token".into(),
                    refresh_token: "refresh-token".into(),
                    expires_at: imp_llm::now() + 3600,
                }),
            )
            .unwrap();

        let mut config = Config::default();
        config.model = Some("gpt-4o".to_string());
        let registry = ModelRegistry::with_builtins();

        let (model_id, provider) =
            resolve_model_and_provider(&default_cli(), &config, &registry, &auth_store).unwrap();
        assert_eq!(model_id, "gpt-4o");
        assert_eq!(provider, "openai");
    }

    // ── parse_rpc_command ──────────────────────────────────────────

    #[test]
    fn parse_rpc_prompt_command() {
        let value = json!({"type": "prompt", "content": "hello"});
        let cmd = parse_rpc_command(&value).unwrap();
        assert!(matches!(cmd, RpcInputCommand::Prompt(ref s) if s == "hello"));
    }

    #[test]
    fn parse_rpc_cancel_command() {
        let value = json!({"type": "cancel"});
        let cmd = parse_rpc_command(&value).unwrap();
        assert!(matches!(cmd, RpcInputCommand::Cancel));
    }

    #[test]
    fn parse_rpc_steer_command() {
        let value = json!({"type": "steer", "content": "also do X"});
        let cmd = parse_rpc_command(&value).unwrap();
        assert!(matches!(cmd, RpcInputCommand::Steer(ref s) if s == "also do X"));
    }

    #[test]
    fn parse_rpc_followup_command() {
        let value = json!({"type": "followup", "content": "next step"});
        let cmd = parse_rpc_command(&value).unwrap();
        assert!(matches!(cmd, RpcInputCommand::FollowUp(ref s) if s == "next step"));
    }

    #[test]
    fn parse_rpc_unknown_type_errors() {
        let value = json!({"type": "bogus"});
        let result = parse_rpc_command(&value);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown command type"));
    }

    #[test]
    fn parse_rpc_missing_type_errors() {
        let value = json!({"content": "hello"});
        let result = parse_rpc_command(&value);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing command type"));
    }

    #[test]
    fn parse_rpc_prompt_missing_content_errors() {
        let value = json!({"type": "prompt"});
        let result = parse_rpc_command(&value);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing string field"));
    }

    // ── rpc_stream_event_to_json ───────────────────────────────────

    #[test]
    fn rpc_stream_event_text_delta() {
        let event = StreamEvent::TextDelta {
            text: "hello".to_string(),
        };
        let json = rpc_stream_event_to_json(&event);
        assert_eq!(json["type"], "text_delta");
        assert_eq!(json["text"], "hello");
    }

    #[test]
    fn rpc_stream_event_tool_call() {
        let event = StreamEvent::ToolCall {
            id: "call_1".to_string(),
            name: "bash".to_string(),
            arguments: json!({"command": "ls"}),
        };
        let json = rpc_stream_event_to_json(&event);
        assert_eq!(json["type"], "tool_call");
        assert_eq!(json["name"], "bash");
        assert_eq!(json["arguments"]["command"], "ls");
    }

    // ── rpc_agent_event_to_json ────────────────────────────────────

    #[test]
    fn rpc_agent_event_tool_execution_start() {
        let event = AgentEvent::ToolExecutionStart {
            tool_call_id: "call_42".to_string(),
            tool_name: "read".to_string(),
            args: json!({"path": "/tmp/test.txt"}),
        };
        let json = rpc_agent_event_to_json(&event);
        assert_eq!(json["type"], "tool_execution_start");
        assert_eq!(json["tool_name"], "read");
        assert_eq!(json["args"]["path"], "/tmp/test.txt");
    }

    #[test]
    fn rpc_agent_event_agent_end() {
        let usage = imp_llm::Usage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 100,
            cache_write_tokens: 50,
        };
        let cost = imp_llm::Cost {
            input: 0.003,
            output: 0.0075,
            cache_read: 0.00003,
            cache_write: 0.0001875,
            total: 0.0107175,
        };
        let event = AgentEvent::AgentEnd { usage, cost };
        let json = rpc_agent_event_to_json(&event);
        assert_eq!(json["type"], "agent_end");
        assert_eq!(json["input_tokens"], 1000);
        assert_eq!(json["output_tokens"], 500);
        assert_eq!(json["cache_read_tokens"], 100);
        assert_eq!(json["cost_total"], 0.0107175);
    }

    #[test]
    fn rpc_agent_event_timing() {
        let event = AgentEvent::Timing {
            timing: TimingEvent {
                turn: 2,
                stage: imp_core::TimingStage::FirstTextDelta,
                since_turn_start_ms: 150,
                since_llm_request_start_ms: 120,
            },
        };
        let json = rpc_agent_event_to_json(&event);
        assert_eq!(json["type"], "timing");
        assert_eq!(json["turn"], 2);
        assert_eq!(json["stage"], "first_text_delta");
        assert_eq!(json["since_turn_start_ms"], 150);
        assert_eq!(json["since_llm_request_start_ms"], 120);
    }

    #[test]
    fn startup_stage_names_are_stable() {
        assert_eq!(StartupStage::ProcessStart.as_str(), "process_start");
        assert_eq!(StartupStage::RunLoopStarted.as_str(), "run_loop_started");
    }

    #[test]
    fn imp_cli_uses_canonical_mana_worker_prompt_and_context_helpers() {
        let assignment = imp_core::mana_worker::WorkerAssignment {
            id: "42".to_string(),
            title: "Fix the widget".to_string(),
            description: "The widget is broken.\nPlease fix it.".to_string(),
            acceptance: Some("Widget tests pass".to_string()),
            verify: Some("cargo test -p imp-cli".to_string()),
            notes: Some("Check the edge case.".to_string()),
            decisions: vec!["Keep the CLI thin".to_string()],
            dependencies: Vec::new(),
            paths: vec!["imp/crates/imp-cli/src/main.rs".to_string()],
            files: Vec::new(),
            attempts: vec![imp_core::mana_worker::WorkerAttempt {
                number: 1,
                outcome: "failed".to_string(),
                summary: "timed out".to_string(),
            }],
            workspace_root: PathBuf::from("/tmp"),
            model: None,
        };

        let prompt = imp_core::mana_worker::build_task_prompt(&assignment);
        let context = imp_core::mana_worker::build_task_context(&assignment);

        assert!(prompt.starts_with("Task: Fix the widget"));
        assert!(prompt.contains("Notes:\nCheck the edge case."));
        assert!(prompt.contains("Previous attempts:"));
        assert!(prompt.contains("Verify command: cargo test -p imp-cli"));

        assert_eq!(context.title, "Fix the widget");
        assert_eq!(context.acceptance.as_deref(), Some("Widget tests pass"));
        assert_eq!(context.verify.as_deref(), Some("cargo test -p imp-cli"));
        assert_eq!(
            context.context_paths,
            vec!["imp/crates/imp-cli/src/main.rs"]
        );
        assert!(context
            .constraints
            .iter()
            .any(|constraint| constraint.contains("verify command")));
    }
}

fn run_import(dry_run: bool, from: Option<&str>, auto_yes: bool) {
    use imp_core::import::{
        detect_sources, import_agents_md, import_skills, AgentSource, SkipReason,
    };

    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => {
            eprintln!("Cannot determine home directory");
            std::process::exit(1);
        }
    };

    let sources = detect_sources(&home);

    // Filter by --from if specified
    let sources: Vec<_> = if let Some(filter) = from {
        let target = match filter.to_lowercase().as_str() {
            "pi" => Some(AgentSource::Pi),
            "claude" | "claude-code" => Some(AgentSource::ClaudeCode),
            "codex" => Some(AgentSource::Codex),
            other => {
                eprintln!("Unknown agent: {other}. Use: pi, claude, codex");
                std::process::exit(1);
            }
        };
        sources
            .into_iter()
            .filter(|s| target.is_none_or(|t| s.agent == t))
            .collect()
    } else {
        sources
    };

    if sources.is_empty() {
        println!("No other agent configurations found.");
        println!("Checked: ~/.pi/agent/, ~/.claude/, ~/.codex/");
        return;
    }

    // Display what was found
    println!("Found agent configurations:\n");
    let mut total_skills = 0;
    let mut total_agents_md = 0;

    for source in &sources {
        println!(
            "  {} ({})",
            source.agent.label(),
            match source.agent {
                AgentSource::Pi => "~/.pi/agent/",
                AgentSource::ClaudeCode => "~/.claude/",
                AgentSource::Codex => "~/.codex/",
            }
        );

        if !source.skills.is_empty() {
            println!("    {} skills:", source.skills.len());
            for skill in &source.skills {
                let desc = truncate_chars_with_suffix(&skill.description, 60, "…");
                println!("      - {} — {}", skill.name, desc);
            }
            total_skills += source.skills.len();
        }

        if !source.agents_md.is_empty() {
            for md in &source.agents_md {
                println!("    {} at {}", md.kind.label(), md.path.display());
            }
            total_agents_md += source.agents_md.len();
        }

        println!();
    }

    if dry_run {
        println!("Dry run — nothing was copied.");
        println!("Run without --dry-run to import.");
        return;
    }

    if total_skills == 0 && total_agents_md == 0 {
        println!("Nothing to import.");
        return;
    }

    // Confirm unless --yes
    if !auto_yes {
        print!(
            "Import {} skills and {} instruction files into imp? [y/N] ",
            total_skills, total_agents_md
        );
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return;
        }
    }

    let imp_config = Config::user_config_dir();
    let imp_skills = imp_config.join("skills");

    // Import skills
    for source in &sources {
        if source.skills.is_empty() {
            continue;
        }

        match import_skills(&source.skills, &imp_skills) {
            Ok(result) => {
                if !result.copied.is_empty() {
                    println!(
                        "  ✓ Imported {} skills from {}:",
                        result.copied.len(),
                        source.agent.label()
                    );
                    for name in &result.copied {
                        println!("      {name}");
                    }
                }
                for (name, reason) in &result.skipped {
                    match reason {
                        SkipReason::AlreadyExists => {
                            println!("    ⊘ {name} — already exists, skipped");
                        }
                        SkipReason::CopyFailed(err) => {
                            eprintln!("    ✗ {name} — copy failed: {err}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "  ✗ Failed to import skills from {}: {e}",
                    source.agent.label()
                );
            }
        }
    }

    // Import AGENTS.md (only the first one found, if imp doesn't have one yet)
    let mut imported_agents = false;
    for source in &sources {
        for md in &source.agents_md {
            if imported_agents {
                println!(
                    "    ⊘ {} from {} — already have AGENTS.md, skipped",
                    md.kind.label(),
                    source.agent.label()
                );
                continue;
            }
            match import_agents_md(md, &imp_config) {
                Ok(Some(dest)) => {
                    println!(
                        "  ✓ Imported {} from {} → {}",
                        md.kind.label(),
                        source.agent.label(),
                        dest.display()
                    );
                    imported_agents = true;
                }
                Ok(None) => {
                    println!("    ⊘ AGENTS.md already exists in imp config, skipped");
                    imported_agents = true;
                }
                Err(e) => {
                    eprintln!(
                        "  ✗ Failed to import {} from {}: {e}",
                        md.kind.label(),
                        source.agent.label()
                    );
                }
            }
        }
    }

    println!("\nDone. Skills are in {}", imp_skills.display());
}
