use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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
use imp_core::agent::{Agent, AgentCommand, AgentEvent, AgentHandle};
use imp_core::config::Config;
use imp_core::tools::web::types::SearchProvider;

use imp_core::imp_session::{ImpSession, SessionChoice, SessionOptions};
use imp_core::session::SessionManager;
use imp_core::system_prompt::{Attempt as TaskAttempt, TaskContext};
use imp_core::ui::{ComponentSpec, NotifyLevel, SelectOption, UserInterface, WidgetContent};
use imp_core::usage::{UsageCostBreakdown, UsageRecordSource, UsageTokens};
use imp_core::TimingEvent;
use imp_llm::auth::AuthStore;
use imp_llm::model::{ModelRegistry, ProviderRegistry};
use imp_llm::oauth::anthropic::AnthropicOAuth;
use imp_llm::oauth::chatgpt::ChatGptOAuth;
use imp_llm::provider::ThinkingLevel;
use imp_llm::providers::create_provider;
use imp_llm::{truncate_chars_with_suffix, Message, Model, StreamEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command as TokioCommand;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

mod usage_report;

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

#[derive(Subcommand)]
enum Commands {
    /// Log in to an LLM provider. Uses OAuth for Anthropic and OpenAI/ChatGPT; prompts for an API key for other providers.
    Login {
        /// Provider to log in to (e.g. anthropic, openai, deepseek, groq). Defaults to anthropic.
        provider: Option<String>,
    },
    /// Edit configuration
    Config,
    /// Run a mana unit headlessly
    Run {
        /// Unit ID to run
        unit_id: String,
    },
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
    /// Save a web search provider API key into imp auth storage
    WebLogin {
        /// Search provider to configure (tavily, exa, linkup, perplexity)
        provider: String,
    },
}

#[derive(Subcommand)]
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

#[derive(Debug, Deserialize)]
struct UnitFrontmatter {
    id: Option<String>,
    title: String,
    description: Option<String>,
    verify: Option<String>,
    notes: Option<String>,
    #[serde(default)]
    attempt_log: Vec<UnitAttempt>,
}

#[derive(Debug, Clone, Deserialize)]
struct UnitAttempt {
    num: Option<u32>,
    outcome: Option<String>,
    agent: Option<String>,
    started_at: Option<String>,
    summary: Option<String>,
}

#[derive(Debug, Clone)]
struct ManaUnit {
    id: Option<String>,
    title: String,
    description: String,
    verify: Option<String>,
    notes: Option<String>,
    attempts: Vec<UnitAttempt>,
    workspace_root: PathBuf,
}

impl ManaUnit {
    fn task_prompt(&self) -> String {
        let mut prompt = format!("Task: {}", self.title);

        if !self.description.trim().is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(self.description.trim());
        }

        if let Some(notes) = self
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|notes| !notes.is_empty())
        {
            prompt.push_str("\n\nNotes:\n");
            prompt.push_str(notes);
        }

        if !self.attempts.is_empty() {
            prompt.push_str("\n\nPrevious attempts:\n");
            for attempt in &self.attempts {
                prompt.push_str("- ");
                prompt.push_str(&format_attempt(attempt));
                prompt.push('\n');
            }
            while prompt.ends_with('\n') {
                prompt.pop();
            }
        }

        if let Some(verify) = self
            .verify
            .as_deref()
            .map(str::trim)
            .filter(|verify| !verify.is_empty())
        {
            prompt.push_str("\n\nVerify command: ");
            prompt.push_str(verify);
        }

        prompt
    }

    fn task_context(&self) -> TaskContext {
        let mut description = self.description.trim().to_string();

        if let Some(notes) = self
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|notes| !notes.is_empty())
        {
            if !description.is_empty() {
                description.push_str("\n\n");
            }
            description.push_str("Notes:\n");
            description.push_str(notes);
        }

        TaskContext {
            title: self.title.clone(),
            description,
            verify: self.verify.clone(),
            attempts: self
                .attempts
                .iter()
                .enumerate()
                .map(|(index, attempt)| TaskAttempt {
                    number: attempt.num.unwrap_or((index + 1) as u32),
                    outcome: attempt
                        .outcome
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    summary: attempt
                        .summary
                        .clone()
                        .unwrap_or_else(|| format_attempt(attempt)),
                })
                .collect(),
            dependencies: Vec::new(),
        }
    }
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Dispatch subcommands first
    if let Some(command) = &cli.command {
        match command {
            Commands::Login { provider } => {
                let provider_name = provider.as_deref().unwrap_or("anthropic");
                if let Err(e) = run_login(provider_name).await {
                    eprintln!("Login failed: {e}");
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
            Commands::Run { unit_id } => match run_headless_mode(&cli, unit_id).await {
                Ok(true) => return,
                Ok(false) => std::process::exit(1),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            },
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

    // Expand @file args into file content context
    let file_context = expand_file_args(&cli.args);

    // Read from stdin if piped
    let stdin_content = {
        use std::io::IsTerminal;
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

    // Interactive TUI mode
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
            eprintln!("Unknown mode: {other}. Use interactive, rpc, or json.");
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

async fn run_web_login(provider_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let provider = search_provider_from_name(provider_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Unknown web provider: {provider_name}. Use one of: tavily, exa, linkup, perplexity"
            ),
        )
    })?;

    let auth_path = Config::user_config_dir().join("auth.json");
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    let env_key = provider.env_key_name();
    eprintln!("Enter API key for {}:", provider.name());
    eprintln!("  Env var: {env_key}");
    eprintln!("  Get a key at: {}", search_provider_docs_url(provider));
    eprint!("> ");
    io::stdout().flush().ok();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let key = input.trim().to_string();

    if key.is_empty() {
        eprintln!("No key entered. Aborting.");
        std::process::exit(1);
    }

    auth_store.store(
        provider.name(),
        imp_llm::auth::StoredCredential::ApiKey { key },
    )?;
    eprintln!("API key saved for {} in {}.", provider.name(), auth_path.display());
    eprintln!("The web tool will now auto-detect {} without requiring an exported env var.", provider.name());

    Ok(())
}

async fn run_login(provider_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let auth_path = Config::user_config_dir().join("auth.json");
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    if provider_name == "anthropic" {
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
    } else if provider_name == "openai" || provider_name == "openai-codex" {
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
    } else {
        // For all other providers: prompt for an API key.
        let registry = ProviderRegistry::with_builtins();
        let provider_meta = registry.find(provider_name);
        let display_name = provider_meta.map(|p| p.name).unwrap_or(provider_name);
        let docs_hint = provider_meta.map(|p| p.docs_url).unwrap_or("");

        eprintln!("Enter API key for {display_name}:");
        if !docs_hint.is_empty() {
            eprintln!("  Get a key at: {docs_hint}");
        }
        eprint!("> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let key = input.trim().to_string();

        if key.is_empty() {
            eprintln!("No key entered. Aborting.");
            std::process::exit(1);
        }

        auth_store.store(
            provider_name,
            imp_llm::auth::StoredCredential::ApiKey { key },
        )?;
        eprintln!("API key saved for {display_name}.");
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

fn model_supports_provider(registry: &ModelRegistry, provider: &str, model_id: &str) -> bool {
    if provider == "openai-codex" {
        return imp_llm::model::builtin_openai_codex_models()
            .iter()
            .any(|model| model.id == model_id);
    }

    registry
        .list_by_provider(provider)
        .iter()
        .any(|model| model.id == model_id)
}

fn should_use_chatgpt_provider(
    cli: &Cli,
    auth_store: &AuthStore,
    registry: &ModelRegistry,
    model_id: &str,
    provider_name: &str,
) -> bool {
    cli.provider.is_none()
        && cli.api_key.is_none()
        && provider_name == "openai"
        && auth_store.resolve_api_key_only("openai").is_err()
        && (auth_store.get_oauth("openai").is_some()
            || auth_store.get_oauth("openai-codex").is_some())
        && model_supports_provider(registry, "openai-codex", model_id)
}

fn resolve_model_and_provider(
    cli: &Cli,
    config: &Config,
    registry: &ModelRegistry,
    auth_store: &AuthStore,
) -> Result<(String, String), String> {
    let model_hint = cli
        .model
        .as_deref()
        .or(config.model.as_deref())
        .unwrap_or("sonnet");

    let meta = registry
        .resolve_meta(model_hint, cli.provider.as_deref())
        .ok_or_else(|| format!("Unknown model: {model_hint}"))?;

    let mut provider_name = cli
        .provider
        .as_deref()
        .unwrap_or(&meta.provider)
        .to_string();
    if should_use_chatgpt_provider(cli, auth_store, registry, &meta.id, &provider_name) {
        provider_name = "openai-codex".to_string();
    }

    Ok((meta.id.clone(), provider_name))
}

async fn resolve_provider_api_key(
    auth_store: &mut AuthStore,
    provider_name: &str,
) -> Result<imp_llm::auth::ApiKey, imp_llm::Error> {
    match provider_name {
        "openai" => auth_store.resolve_api_key_only(provider_name),
        "openai-codex" => auth_store.resolve_chatgpt_oauth().await,
        _ => auth_store.resolve_with_refresh(provider_name).await,
    }
}

async fn run_headless_mode(cli: &Cli, unit_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let cwd = std::env::current_dir()?;
    emit_startup_timing(&mut startup_timer, StartupStage::CwdResolved);
    let unit = load_mana_unit(&cwd, unit_id)?;
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;
    emit_startup_timing(&mut startup_timer, StartupStage::ConfigResolved);
    emit_startup_timing(&mut startup_timer, StartupStage::ModelRegistryReady);
    emit_startup_timing(&mut startup_timer, StartupStage::AuthLoaded);
    emit_startup_timing(&mut startup_timer, StartupStage::ModelResolved);
    emit_startup_timing(&mut startup_timer, StartupStage::ProviderReady);
    emit_startup_timing(&mut startup_timer, StartupStage::ApiKeyResolved);

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
        system_prompt: cli.system_prompt.clone(),
        no_tools: cli.no_tools,
        session: SessionChoice::InMemory,
        task: Some(unit.task_context()),
        ..Default::default()
    };

    if !cli.no_tools {
        let lua_cwd = cwd.clone();
        options.lua_loader = Some(Box::new(move |tools| {
            let user_config_dir = Config::user_config_dir();
            imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools);
        }));
    }

    let mut session = ImpSession::create(options)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    emit_startup_timing(&mut startup_timer, StartupStage::AgentBuilt);

    let prompt = unit.task_prompt();
    emit_startup_timing(&mut startup_timer, StartupStage::PromptReady);
    session
        .prompt(&prompt)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    emit_startup_timing(&mut startup_timer, StartupStage::RunLoopStarted);

    while let Some(event) = session.recv_event().await {
        print_json_event(&event)?;
    }

    session
        .wait()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    // When MANA_BATCH_VERIFY is set the runner handles verification after all
    // agents complete.  Skip inline verify and exit 0 so the runner can batch
    // the shared verify commands once per unique command string.
    let batch_verify = std::env::var("MANA_BATCH_VERIFY").is_ok();
    if !batch_verify {
        if let Some(verify) = unit
            .verify
            .as_deref()
            .map(str::trim)
            .filter(|verify| !verify.is_empty())
        {
            let passed = run_verify_command(verify, &unit.workspace_root).await?;
            if passed {
                // Auto-close the unit on verify pass
                if let Some(ref id) = unit.id {
                    let close_result = std::process::Command::new("mana")
                        .args(["close", id])
                        .current_dir(&unit.workspace_root)
                        .output();
                    match close_result {
                        Ok(output) if output.status.success() => {
                            eprintln!("[imp] Unit {id} closed (verify passed)");
                        }
                        _ => {
                            eprintln!("[imp] Verify passed but failed to close unit {id}");
                        }
                    }
                }
            }
            return Ok(passed);
        }
    }

    Ok(true)
}

fn load_mana_unit(cwd: &Path, unit_id: &str) -> Result<ManaUnit, Box<dyn std::error::Error>> {
    let mana_dir = find_mana_dir(cwd).ok_or_else(|| {
        io::Error::other(format!(
            "Could not find .mana directory while walking up from {}",
            cwd.display()
        ))
    })?;

    let workspace_root = mana_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.to_path_buf());

    let mut candidates = Vec::new();

    for entry in fs::read_dir(&mana_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(file_name) => file_name,
            None => continue,
        };

        if !file_name.contains(unit_id) {
            continue;
        }

        candidates.push(parse_mana_unit(&path, workspace_root.clone())?);
    }

    if candidates.is_empty() {
        return Err(io::Error::other(format!(
            "Mana unit {unit_id} not found in {}",
            mana_dir.display()
        ))
        .into());
    }

    if let Some(unit) = candidates
        .iter()
        .position(|unit| unit.id.as_deref() == Some(unit_id))
        .map(|index| candidates.remove(index))
    {
        return Ok(unit);
    }

    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }

    let titles = candidates
        .into_iter()
        .map(|unit| unit.title)
        .collect::<Vec<_>>()
        .join(", ");

    Err(io::Error::other(format!(
        "Mana unit lookup for {unit_id} is ambiguous: {titles}"
    ))
    .into())
}

fn find_mana_dir(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);

    while let Some(dir) = current {
        let candidate = dir.join(".mana");
        if candidate.is_dir() {
            return Some(candidate);
        }
        current = dir.parent();
    }

    None
}

fn parse_mana_unit(
    path: &Path,
    workspace_root: PathBuf,
) -> Result<ManaUnit, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let (frontmatter, body) = split_frontmatter(&content)?;
    let frontmatter: UnitFrontmatter = serde_yaml::from_str(&frontmatter).map_err(|error| {
        io::Error::other(format!("Failed to parse {}: {error}", path.display()))
    })?;

    let frontmatter_description = frontmatter
        .description
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    let body = body.trim();
    let description = if !frontmatter_description.is_empty() && !body.is_empty() {
        format!("{frontmatter_description}\n\n{body}")
    } else if !frontmatter_description.is_empty() {
        frontmatter_description.to_string()
    } else {
        body.to_string()
    };

    Ok(ManaUnit {
        id: frontmatter.id,
        title: frontmatter.title,
        description,
        verify: frontmatter.verify,
        notes: frontmatter.notes,
        attempts: frontmatter.attempt_log,
        workspace_root,
    })
}

fn split_frontmatter(content: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.first().copied() != Some("---") {
        return Err(io::Error::other("Mana unit is missing YAML frontmatter").into());
    }

    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (*line == "---").then_some(index))
        .ok_or_else(|| io::Error::other("Mana unit frontmatter is not closed"))?;

    let yaml = lines[1..end].join("\n");
    let body = lines[end + 1..].join("\n");
    Ok((yaml, body))
}

fn format_attempt(attempt: &UnitAttempt) -> String {
    let number = attempt
        .num
        .map(|num| format!("Attempt {num}"))
        .unwrap_or_else(|| "Attempt".to_string());
    let outcome = attempt.outcome.as_deref().unwrap_or("unknown");

    if let Some(summary) = attempt
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
    {
        return format!("{number} ({outcome}): {summary}");
    }

    let mut details = Vec::new();

    if let Some(agent) = attempt
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|agent| !agent.is_empty())
    {
        details.push(format!("agent {agent}"));
    }

    if let Some(started_at) = attempt
        .started_at
        .as_deref()
        .map(str::trim)
        .filter(|started_at| !started_at.is_empty())
    {
        details.push(format!("started {started_at}"));
    }

    if details.is_empty() {
        format!("{number} ({outcome})")
    } else {
        format!("{number} ({outcome}): {}", details.join(", "))
    }
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

fn print_json_event(event: &AgentEvent) -> Result<(), Box<dyn std::error::Error>> {
    let value = match event {
        AgentEvent::AgentStart { model, timestamp } => {
            json!({ "type": "agent_start", "model": model, "timestamp": timestamp })
        }
        AgentEvent::AgentEnd { usage, cost } => {
            json!({ "type": "agent_end", "usage": usage, "cost": cost })
        }
        AgentEvent::TurnStart { index } => json!({ "type": "turn_start", "index": index }),
        AgentEvent::TurnEnd { index, message } => {
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

    async fn select(&self, title: &str, options: &[SelectOption]) -> Option<usize> {
        let result = self
            .request(
                "select",
                json!({
                    "title": title,
                    "options": serde_json::to_value(options).unwrap_or_else(|_| json!([])),
                }),
            )
            .await?;

        result.as_u64().map(|index| index as usize)
    }

    async fn input(&self, title: &str, placeholder: &str) -> Option<String> {
        self.request(
            "input",
            json!({
                "title": title,
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
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;
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
    let auth_path = Config::user_config_dir().join("auth.json");
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
            .lua_tool_loader(move |tools| {
                let user_config_dir = Config::user_config_dir();
                imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools);
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
        AgentEvent::TurnEnd { index, message } => {
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
        AgentEvent::Error { error } => json!({ "type": "error", "error": error }),
        AgentEvent::ToolOutputDelta { tool_call_id, text } => {
            json!({ "type": "tool_output_delta", "tool_call_id": tool_call_id, "text": text })
        }
    }
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

async fn run_verify_command(verify: &str, cwd: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let output = run_shell_command(verify, cwd).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !stderr.trim().is_empty() {
            eprintln!("{stderr}");
        } else if !stdout.trim().is_empty() {
            eprintln!("{stdout}");
        }
    }

    Ok(output.status.success())
}

fn run_shell_command(command: &str, cwd: &Path) -> TokioCommand {
    #[cfg(target_os = "windows")]
    let mut shell = {
        let mut shell = TokioCommand::new("cmd");
        shell.args(["/C", command]);
        shell
    };

    #[cfg(not(target_os = "windows"))]
    let mut shell = {
        let mut shell = TokioCommand::new("sh");
        shell.args(["-lc", command]);
        shell
    };

    shell.current_dir(cwd);
    shell
}

async fn run_print_mode(cli: &Cli, prompt: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut startup_timer = StartupTimer::new(cli.verbose);
    emit_startup_timing(&mut startup_timer, StartupStage::ProcessStart);
    let cwd = std::env::current_dir()?;
    emit_startup_timing(&mut startup_timer, StartupStage::CwdResolved);
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;
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
        system_prompt: cli.system_prompt.clone(),
        no_tools: cli.no_tools,
        session: session_choice,
        ..Default::default()
    };
    emit_startup_timing(&mut startup_timer, StartupStage::SessionReady);

    if !cli.no_tools {
        let lua_cwd = std::env::current_dir().unwrap_or_default();
        let user_config_dir = Config::user_config_dir();
        options.lua_loader = Some(Box::new(move |tools| {
            imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools);
        }));
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
                        .map(|c| {
                            truncate_chars_with_suffix(c, 60, "…")
                        })
                        .unwrap_or_default(),
                    "read" | "write" | "edit" => args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    "grep" => args
                        .get("pattern")
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
                        eprintln!(
                            "[error: {}]",
                            truncate_chars_with_suffix(&text, 100, "")
                        );
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
                eprintln!("Error: {error}");
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

async fn run_interactive(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;

    let registry = ModelRegistry::with_builtins();

    let session = if cli.no_session {
        SessionManager::in_memory()
    } else if cli.cont {
        // Continue most recent session
        SessionManager::continue_recent(&cwd, &Config::session_dir())?
            .unwrap_or_else(|| SessionManager::new(&cwd, &Config::session_dir()).unwrap())
    } else if let Some(ref path) = cli.session {
        SessionManager::open(path)?
    } else {
        // New persistent session
        SessionManager::new(&cwd, &Config::session_dir())?
    };

    let mut runner = imp_tui::interactive::InteractiveRunner::new(config, session, registry, cwd)?;

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

    runner.run().await
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
    use imp_llm::auth::{OAuthCredential, StoredCredential};
    use imp_llm::provider::ThinkingLevel;
    use serde_json::json;

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
            verbose: false,
            list_models: false,
            args: Vec::new(),
            command: None,
        }
    }

    fn empty_auth_store() -> AuthStore {
        AuthStore::new(std::path::PathBuf::from("auth.json"))
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

    // ── split_frontmatter ──────────────────────────────────────────

    #[test]
    fn split_frontmatter_valid() {
        let content = "---\ntitle: Test\nverify: echo ok\n---\n\nBody text here.";
        let (yaml, body) = split_frontmatter(content).unwrap();
        assert!(yaml.contains("title: Test"));
        assert!(yaml.contains("verify: echo ok"));
        assert!(body.trim() == "Body text here.");
    }

    #[test]
    fn split_frontmatter_missing_opener() {
        let content = "title: Test\n---\nBody";
        let result = split_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn split_frontmatter_missing_closer() {
        let content = "---\ntitle: Test\nno closing delimiter";
        let result = split_frontmatter(content);
        assert!(result.is_err());
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

    // ── format_attempt ─────────────────────────────────────────────

    #[test]
    fn format_attempt_with_summary() {
        let attempt = UnitAttempt {
            num: Some(1),
            outcome: Some("failed".to_string()),
            agent: Some("pi-agent".to_string()),
            started_at: None,
            summary: Some("ran out of context".to_string()),
        };
        let result = format_attempt(&attempt);
        assert!(result.contains("Attempt 1"));
        assert!(result.contains("failed"));
        assert!(result.contains("ran out of context"));
    }

    #[test]
    fn format_attempt_without_summary() {
        let attempt = UnitAttempt {
            num: Some(2),
            outcome: Some("abandoned".to_string()),
            agent: Some("pi-agent".to_string()),
            started_at: Some("2026-03-21T08:00:00Z".to_string()),
            summary: None,
        };
        let result = format_attempt(&attempt);
        assert!(result.contains("Attempt 2"));
        assert!(result.contains("abandoned"));
        assert!(result.contains("agent pi-agent"));
    }

    // ── ManaUnit::task_prompt ──────────────────────────────────────

    #[test]
    fn mana_unit_task_prompt_full() {
        let unit = ManaUnit {
            id: Some("42".to_string()),
            title: "Fix the widget".to_string(),
            description: "The widget is broken.\nPlease fix it.".to_string(),
            verify: Some("cargo test".to_string()),
            notes: Some("Check the edge case.".to_string()),
            attempts: vec![UnitAttempt {
                num: Some(1),
                outcome: Some("failed".to_string()),
                agent: None,
                started_at: None,
                summary: Some("timed out".to_string()),
            }],
            workspace_root: PathBuf::from("/tmp"),
        };
        let prompt = unit.task_prompt();
        assert!(prompt.starts_with("Task: Fix the widget"));
        assert!(prompt.contains("The widget is broken."));
        assert!(prompt.contains("Notes:\nCheck the edge case."));
        assert!(prompt.contains("Previous attempts:"));
        assert!(prompt.contains("timed out"));
        assert!(prompt.contains("Verify command: cargo test"));
    }

    #[test]
    fn mana_unit_task_prompt_minimal() {
        let unit = ManaUnit {
            id: None,
            title: "Simple task".to_string(),
            description: String::new(),
            verify: None,
            notes: None,
            attempts: Vec::new(),
            workspace_root: PathBuf::from("/tmp"),
        };
        let prompt = unit.task_prompt();
        assert_eq!(prompt, "Task: Simple task");
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

    // ── parse_mana_unit (integration with tempfile) ────────────────

    #[test]
    fn parse_mana_unit_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("42-fix-bug.md");
        fs::write(
            &path,
            "---\nid: '42'\ntitle: Fix the bug\nverify: cargo test\n---\n\nDescription body.\n",
        )
        .unwrap();
        let unit = parse_mana_unit(&path, dir.path().to_path_buf()).unwrap();
        assert_eq!(unit.id.as_deref(), Some("42"));
        assert_eq!(unit.title, "Fix the bug");
        assert_eq!(unit.verify.as_deref(), Some("cargo test"));
        assert!(unit.description.contains("Description body."));
    }

    #[test]
    fn parse_mana_unit_missing_verify() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("99-no-verify.md");
        fs::write(&path, "---\ntitle: No verify\n---\n\nJust do it.\n").unwrap();
        let unit = parse_mana_unit(&path, dir.path().to_path_buf()).unwrap();
        assert_eq!(unit.title, "No verify");
        assert!(unit.verify.is_none());
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
