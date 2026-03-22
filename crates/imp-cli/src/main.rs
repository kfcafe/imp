use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use clap::{Parser, Subcommand};
use futures::StreamExt;
use imp_core::agent::{Agent, AgentCommand, AgentEvent, AgentHandle};
use imp_core::config::Config;
use imp_core::resources;
use imp_core::session::SessionManager;
use imp_core::system_prompt::{self, Attempt as TaskAttempt, TaskContext};
use imp_core::tools::ask::AskTool;
use imp_core::tools::bash::BashTool;
use imp_core::tools::diff::{DiffApplyTool, DiffShowTool};
use imp_core::tools::edit::EditTool;
use imp_core::tools::find::FindTool;
use imp_core::tools::grep::GrepTool;
use imp_core::tools::ls::LsTool;
use imp_core::tools::multi_edit::MultiEditTool;
use imp_core::tools::read::ReadTool;
use imp_core::tools::tree_sitter::{AstGrepTool, ProbeExtractTool, ProbeSearchTool, ScanTool};
use imp_core::tools::write::WriteTool;
use imp_core::ui::{ComponentSpec, NotifyLevel, SelectOption, UserInterface, WidgetContent};
use imp_llm::auth::AuthStore;
use imp_llm::model::ModelRegistry;
use imp_llm::oauth::anthropic::AnthropicOAuth;
use imp_llm::provider::{Context, RequestOptions, ThinkingLevel};
use imp_llm::providers::create_provider;
use imp_llm::{Message, Model, StreamEvent};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command as TokioCommand;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

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
    /// OAuth login flow
    Login {
        /// Provider to log in to (default: anthropic)
        provider: Option<String>,
    },
    /// Edit configuration
    Config,
    /// Run a mana unit headlessly
    Run {
        /// Unit ID to run
        unit_id: String,
    },
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
        }
    }

    // List models
    if cli.list_models {
        run_list_models();
        return;
    }

    // Print mode
    if let Some(ref prompt) = cli.print {
        if let Err(e) = run_print_mode(&cli, prompt).await {
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
            "{:<40} {:<12} {:>7}k ${:>8.2} ${:>8.2}",
            m.id,
            m.provider,
            m.context_window / 1000,
            m.pricing.input_per_mtok,
            m.pricing.output_per_mtok,
        );
    }
}

async fn run_login(provider_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    match provider_name {
        "anthropic" => {
            let oauth = AnthropicOAuth::new();
            let auth_path = Config::user_config_dir().join("auth.json");
            let mut auth_store =
                AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

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
            eprintln!("Logged in to Anthropic successfully.");
            Ok(())
        }
        other => {
            eprintln!("Login for '{other}' is not yet supported. Set the API key via environment variable instead.");
            eprintln!("  ANTHROPIC_API_KEY, OPENAI_API_KEY, or GOOGLE_API_KEY");
            std::process::exit(1);
        }
    }
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

fn resolve_model_and_provider(
    cli: &Cli,
    config: &Config,
    registry: &ModelRegistry,
) -> Result<(String, String), String> {
    let model_hint = cli
        .model
        .as_deref()
        .or(config.model.as_deref())
        .unwrap_or("sonnet");

    let meta = registry
        .find_by_alias(model_hint)
        .ok_or_else(|| format!("Unknown model: {model_hint}"))?;

    let provider_name = cli.provider.as_deref().unwrap_or(&meta.provider);

    Ok((meta.id.clone(), provider_name.to_string()))
}

async fn run_headless_mode(cli: &Cli, unit_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let unit = load_mana_unit(&cwd, unit_id)?;
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;
    let registry = ModelRegistry::with_builtins();
    let (model_id, provider_name) =
        resolve_model_and_provider(cli, &config, &registry).map_err(io::Error::other)?;

    let provider = create_provider(&provider_name)
        .ok_or_else(|| io::Error::other(format!("Unknown provider: {provider_name}")))?;

    let meta = registry
        .find(&model_id)
        .ok_or_else(|| io::Error::other(format!("Model not found: {model_id}")))?
        .clone();

    let auth_path = Config::user_config_dir().join("auth.json");
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    if let Some(ref key) = cli.api_key {
        auth_store.set_runtime_key(&provider_name, key.clone());
    }

    let api_key = auth_store.resolve(&provider_name)?;
    let model = Model {
        meta,
        provider: Arc::from(provider),
    };

    let (mut agent, mut handle) = Agent::new(model, cwd.clone());
    agent.thinking_level = cli
        .thinking
        .as_deref()
        .map(parse_thinking_level)
        .or(config.thinking)
        .unwrap_or(ThinkingLevel::Off);
    agent.api_key = api_key;

    if let Some(max_turns) = config.max_turns {
        agent.max_turns = max_turns;
    }

    if !cli.no_tools {
        register_headless_tools(&mut agent);
    }

    let task_context = unit.task_context();
    let user_config_dir = Config::user_config_dir();
    let agents_md = resources::discover_agents_md(&cwd, &user_config_dir);
    let skills = resources::discover_skills(&cwd, &user_config_dir);

    agent.system_prompt = cli.system_prompt.clone().unwrap_or_else(|| {
        system_prompt::assemble(
            &agent.tools,
            &agents_md,
            &skills,
            &[],
            Some(&task_context),
            None,
        )
        .text
    });

    let prompt = unit.task_prompt();
    let agent_task = tokio::spawn(async move { agent.run(prompt).await });

    while let Some(event) = handle.event_rx.recv().await {
        print_json_event(&event)?;
    }

    let agent_result = agent_task
        .await
        .map_err(|error| io::Error::other(format!("Agent task failed: {error}")))?;

    agent_result?;

    if let Some(verify) = unit
        .verify
        .as_deref()
        .map(str::trim)
        .filter(|verify| !verify.is_empty())
    {
        return run_verify_command(verify, &unit.workspace_root).await;
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

fn register_native_tools(agent: &mut Agent) {
    register_native_tools_with_ui(agent, true);
}

fn register_headless_tools(agent: &mut Agent) {
    register_native_tools_with_ui(agent, false);
}

fn register_native_tools_with_ui(agent: &mut Agent, include_ui_tools: bool) {
    if include_ui_tools {
        agent.tools.register(Arc::new(AskTool));
    }
    agent.tools.register(Arc::new(BashTool));
    agent.tools.register(Arc::new(DiffApplyTool));
    agent.tools.register(Arc::new(DiffShowTool));
    agent.tools.register(Arc::new(EditTool));
    agent.tools.register(Arc::new(FindTool));
    agent.tools.register(Arc::new(GrepTool));
    agent.tools.register(Arc::new(LsTool));
    agent.tools.register(Arc::new(MultiEditTool));
    agent.tools.register(Arc::new(ReadTool));
    agent.tools.register(Arc::new(WriteTool));
    agent.tools.register(Arc::new(ProbeSearchTool));
    agent.tools.register(Arc::new(ProbeExtractTool));
    agent.tools.register(Arc::new(ScanTool));
    agent.tools.register(Arc::new(AstGrepTool));

    // Mana integration
    agent
        .tools
        .register(Arc::new(imp_core::tools::mana::ManaTool));
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
        AgentEvent::CompactionStart => json!({ "type": "compaction_start" }),
        AgentEvent::CompactionEnd { summary } => {
            json!({ "type": "compaction_end", "summary": summary })
        }
        AgentEvent::Error { error } => json!({ "type": "error", "error": error }),
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
    let cwd = std::env::current_dir()?;
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;
    let registry = ModelRegistry::with_builtins();

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
    let (mut agent, handle) = create_rpc_agent(cli, cwd, config, registry, history, rpc_ui)?;
    let command_tx = handle.command_tx.clone();

    tokio::spawn(forward_rpc_events(handle, stdout_tx));

    let join_handle = tokio::spawn(async move {
        let result = agent.run(prompt).await;
        (agent, result)
    });

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
    let (model_id, provider_name) =
        resolve_model_and_provider(cli, config, registry).map_err(io::Error::other)?;

    let provider = create_provider(&provider_name)
        .ok_or_else(|| io::Error::other(format!("Unknown provider: {provider_name}")))?;

    let meta = registry
        .find(&model_id)
        .ok_or_else(|| io::Error::other(format!("Model not found: {model_id}")))?
        .clone();

    let auth_path = Config::user_config_dir().join("auth.json");
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    if let Some(ref key) = cli.api_key {
        auth_store.set_runtime_key(&provider_name, key.clone());
    }

    let api_key = auth_store.resolve(&provider_name)?;
    let model = Model {
        meta,
        provider: Arc::from(provider),
    };

    let (mut agent, handle) = Agent::new(model, cwd.to_path_buf());
    agent.thinking_level = cli
        .thinking
        .as_deref()
        .map(parse_thinking_level)
        .or(config.thinking)
        .unwrap_or(ThinkingLevel::Off);
    agent.api_key = api_key;
    agent.ui = rpc_ui as Arc<dyn UserInterface>;
    agent.messages = history;

    if let Some(max_turns) = config.max_turns {
        agent.max_turns = max_turns;
    }

    register_native_tools(&mut agent);

    let user_config_dir = Config::user_config_dir();
    let agents_md = resources::discover_agents_md(cwd, &user_config_dir);
    let skills = resources::discover_skills(cwd, &user_config_dir);

    agent.system_prompt = cli.system_prompt.clone().unwrap_or_else(|| {
        system_prompt::assemble(&agent.tools, &agents_md, &skills, &[], None, None).text
    });

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
        AgentEvent::CompactionStart => json!({ "type": "compaction_start" }),
        AgentEvent::CompactionEnd { summary } => {
            json!({ "type": "compaction_end", "summary": summary })
        }
        AgentEvent::Error { error } => json!({ "type": "error", "error": error }),
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
    let config = Config::resolve(&Config::user_config_dir(), Some(&std::env::current_dir()?))?;

    let registry = ModelRegistry::with_builtins();
    let (model_id, provider_name) = resolve_model_and_provider(cli, &config, &registry)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let provider = create_provider(&provider_name)
        .ok_or_else(|| format!("Unknown provider: {provider_name}"))?;

    let meta = registry
        .find(&model_id)
        .ok_or_else(|| format!("Model not found: {model_id}"))?
        .clone();

    // Resolve API key
    let auth_path = Config::user_config_dir().join("auth.json");
    let mut auth_store =
        AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    if let Some(ref key) = cli.api_key {
        auth_store.set_runtime_key(&provider_name, key.clone());
    }

    let api_key = auth_store.resolve(&provider_name)?;

    let model = Model {
        meta,
        provider: Arc::from(provider),
    };

    let thinking = cli
        .thinking
        .as_deref()
        .map(parse_thinking_level)
        .or(config.thinking)
        .unwrap_or(ThinkingLevel::Off);

    let system_prompt = cli.system_prompt.clone().unwrap_or_default();

    let context = Context {
        messages: vec![Message::user(prompt)],
    };

    let options = RequestOptions {
        thinking_level: thinking,
        max_tokens: Some(model.meta.max_output_tokens),
        system_prompt,
        ..Default::default()
    };

    let mut stream = model.provider.stream(&model, context, options, &api_key);

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(StreamEvent::TextDelta { text }) => {
                print!("{text}");
            }
            Ok(StreamEvent::ThinkingDelta { text }) => {
                // Thinking goes to stderr
                eprint!("{text}");
            }
            Ok(StreamEvent::ToolCall { name, .. }) => {
                eprintln!("[tool: {name}]");
            }
            Ok(StreamEvent::Error { error }) => {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
            Ok(StreamEvent::MessageEnd { .. }) => {
                println!();
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Stream error: {e}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

async fn run_interactive(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;

    let registry = ModelRegistry::with_builtins();
    let session = SessionManager::in_memory();

    let mut app = imp_tui::app::App::new(config, session, registry, cwd);

    // Apply CLI overrides
    if let Some(ref model) = cli.model {
        app.model_name = model.clone();
    }
    if let Some(ref thinking) = cli.thinking {
        app.thinking_level = parse_thinking_level(thinking);
    }

    app.run().await
}

#[cfg(test)]
mod tests {
    use super::*;
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
            verbose: false,
            list_models: false,
            args: Vec::new(),
            command: None,
        }
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
        let (model_id, provider) = resolve_model_and_provider(&cli, &config, &registry).unwrap();
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
        let (model_id, provider) = resolve_model_and_provider(&cli, &config, &registry).unwrap();
        assert!(model_id.contains("haiku"), "expected haiku, got {model_id}");
        assert_eq!(provider, "anthropic");
    }

    #[test]
    fn resolve_model_unknown_alias_errors() {
        let mut cli = default_cli();
        cli.model = Some("nonexistent-xyz".to_string());
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let result = resolve_model_and_provider(&cli, &config, &registry);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown model"));
    }

    #[test]
    fn resolve_model_cli_overrides_config() {
        let mut cli = default_cli();
        cli.model = Some("haiku".to_string());
        let mut config = Config::default();
        config.model = Some("sonnet".to_string());
        let registry = ModelRegistry::with_builtins();
        let (model_id, _) = resolve_model_and_provider(&cli, &config, &registry).unwrap();
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
        let (_, provider) = resolve_model_and_provider(&cli, &config, &registry).unwrap();
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
