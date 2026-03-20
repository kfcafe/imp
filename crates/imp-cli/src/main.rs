use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use futures::StreamExt;
use imp_core::config::Config;
use imp_core::session::SessionManager;
use imp_llm::auth::AuthStore;
use imp_llm::model::ModelRegistry;
use imp_llm::oauth::anthropic::AnthropicOAuth;
use imp_llm::provider::{Context, RequestOptions, ThinkingLevel};
use imp_llm::providers::create_provider;
use imp_llm::{Message, Model, StreamEvent};

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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Dispatch subcommands first
    if let Some(command) = cli.command {
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
            Commands::Run { unit_id } => {
                eprintln!("imp run {unit_id}: headless mode not yet implemented");
                std::process::exit(1);
            }
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

    // RPC / JSON modes
    match cli.mode.as_str() {
        "rpc" | "json" => {
            eprintln!("imp --mode {}: not yet implemented", cli.mode);
            std::process::exit(1);
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

    println!("{:<40} {:<12} {:>8} {:>10} {:>10}",
        "MODEL", "PROVIDER", "CONTEXT", "$/M IN", "$/M OUT");
    println!("{}", "-".repeat(84));

    for m in models {
        println!("{:<40} {:<12} {:>7}k ${:>8.2} ${:>8.2}",
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
            let mut auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

            eprintln!("Opening browser for Anthropic login...");
            eprintln!("If the browser doesn't open, visit the URL printed below.");

            let credential = oauth.login(
                |url| {
                    eprintln!("\n{url}\n");
                    let _ = open_url(url);
                },
                || async {
                    eprintln!("Paste the authorization code or redirect URL:");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).ok()?;
                    let trimmed = input.trim().to_string();
                    if trimmed.is_empty() { None } else { Some(trimmed) }
                },
            ).await?;

            auth_store.store("anthropic", imp_llm::auth::StoredCredential::OAuth(credential))?;
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
        std::process::Command::new("cmd").args(["/C", "start", url]).spawn()?;
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
    let model_hint = cli.model.as_deref()
        .or(config.model.as_deref())
        .unwrap_or("sonnet");

    let meta = registry.find_by_alias(model_hint)
        .ok_or_else(|| format!("Unknown model: {model_hint}"))?;

    let provider_name = cli.provider.as_deref()
        .unwrap_or(&meta.provider);

    Ok((meta.id.clone(), provider_name.to_string()))
}

async fn run_print_mode(cli: &Cli, prompt: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::resolve(
        &Config::user_config_dir(),
        Some(&std::env::current_dir()?),
    )?;

    let registry = ModelRegistry::with_builtins();
    let (model_id, provider_name) = resolve_model_and_provider(cli, &config, &registry)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let provider = create_provider(&provider_name)
        .ok_or_else(|| format!("Unknown provider: {provider_name}"))?;

    let meta = registry.find(&model_id)
        .ok_or_else(|| format!("Model not found: {model_id}"))?
        .clone();

    // Resolve API key
    let auth_path = Config::user_config_dir().join("auth.json");
    let mut auth_store = AuthStore::load(&auth_path)
        .unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

    if let Some(ref key) = cli.api_key {
        auth_store.set_runtime_key(&provider_name, key.clone());
    }

    let api_key = auth_store.resolve(&provider_name)?;

    let model = Model {
        meta,
        provider: Arc::from(provider),
    };

    let thinking = cli.thinking.as_deref()
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
    let config = Config::resolve(
        &Config::user_config_dir(),
        Some(&cwd),
    )?;

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
