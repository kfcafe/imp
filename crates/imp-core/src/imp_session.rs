//! High-level session API for driving imp programmatically.
//!
//! `ImpSession` is the primary public interface for embedding imp in other
//! Rust programs, building custom UIs, or driving agents from orchestrators.
//! It wires together config, auth, model resolution, agent construction,
//! session persistence, and the event stream — eliminating the boilerplate
//! that each run mode (interactive, print, headless, RPC) otherwise
//! duplicates.
//!
//! # Example
//!
//! ```no_run
//! use imp_core::imp_session::{ImpSession, SessionOptions, SessionChoice};
//!
//! # async fn example() -> imp_core::Result<()> {
//! let mut session = ImpSession::create(SessionOptions {
//!     cwd: std::env::current_dir()?,
//!     ..Default::default()
//! }).await?;
//!
//! session.prompt("What files are in the current directory?").await?;
//!
//! while let Some(event) = session.recv_event().await {
//!     println!("{event:?}");
//! }
//! # Ok(())
//! # }
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use imp_llm::auth::{ApiKey, AuthStore};
use imp_llm::model::ModelRegistry;
use imp_llm::providers::create_provider;
use imp_llm::{Model, ThinkingLevel};

use crate::agent::{Agent, AgentCommand, AgentEvent, AgentHandle};
use crate::builder::AgentBuilder;
use crate::config::{AgentMode, Config};
use crate::error::{Error, Result};
use crate::session::SessionManager;
use crate::system_prompt::TaskContext;
use crate::ui::UserInterface;

// ── Options ─────────────────────────────────────────────────────

/// How to initialize the session file.
#[derive(Debug, Clone, Default)]
pub enum SessionChoice {
    /// Fresh session, persisted to disk.
    #[default]
    New,
    /// No persistence.
    InMemory,
    /// Continue the most recent session for the working directory.
    Continue,
    /// Open a specific session file.
    Open(PathBuf),
}

/// Configuration for creating an `ImpSession`.
///
/// All fields have sensible defaults — only `cwd` is typically required.
pub struct SessionOptions {
    /// Working directory. Tools resolve paths relative to this.
    pub cwd: PathBuf,

    /// Model hint — alias ("sonnet") or full ID. Resolved against the
    /// model registry. Falls back to config, then "sonnet".
    pub model: Option<String>,

    /// Provider override. Usually auto-detected from the model.
    pub provider: Option<String>,

    /// Runtime API key override (not persisted).
    pub api_key: Option<String>,

    /// Thinking level override.
    pub thinking: Option<ThinkingLevel>,

    /// Agent mode (full, worker, orchestrator, …).
    pub mode: Option<AgentMode>,

    /// Maximum turns before the agent stops.
    pub max_turns: Option<u32>,

    /// Replace the assembled system prompt entirely.
    pub system_prompt: Option<String>,

    /// Skip native tool registration.
    pub no_tools: bool,

    /// Session persistence strategy.
    pub session: SessionChoice,

    /// Task context for headless / unit mode.
    pub task: Option<TaskContext>,

    /// Lua extension loader. Called after native tools are registered.
    /// The binary crate typically provides this; library callers can
    /// pass `None` to skip Lua extensions.
    #[allow(clippy::type_complexity)]
    pub lua_loader: Option<Box<dyn FnOnce(&mut crate::tools::ToolRegistry) + Send>>,

    /// Custom UI implementation. Defaults to `NullInterface`.
    pub ui: Option<Arc<dyn UserInterface>>,

    /// Path to auth.json. Defaults to `~/.config/imp/auth.json`.
    pub auth_path: Option<PathBuf>,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            model: None,
            provider: None,
            api_key: None,
            thinking: None,
            mode: None,
            max_turns: None,
            system_prompt: None,
            no_tools: false,
            session: SessionChoice::default(),
            task: None,
            lua_loader: None,
            ui: None,
            auth_path: None,
        }
    }
}

// ── ImpSession ──────────────────────────────────────────────────

/// A fully wired agent session.
///
/// Manages the lifecycle of a single agent: config resolution, model
/// selection, session persistence, and the event/command channels.
pub struct ImpSession {
    agent: Option<Agent>,
    handle: AgentHandle,
    session_mgr: SessionManager,
    config: Config,
    model: Model,
    auth_store: AuthStore,
    model_registry: ModelRegistry,
    cwd: PathBuf,
    /// Task handle for the currently running agent loop, if any.
    agent_task: Option<JoinHandle<Result<()>>>,
}

impl ImpSession {
    /// Create a new session by resolving config, auth, model, and tools.
    ///
    /// This is the main factory — mirrors pi's `createAgentSession()`.
    pub async fn create(options: SessionOptions) -> Result<Self> {
        let cwd = options.cwd.clone();

        // 1. Load config (user + project, merged)
        let mut config = Config::resolve(&Config::user_config_dir(), Some(&cwd))?;

        // Apply option overrides
        if let Some(thinking) = options.thinking {
            config.thinking = Some(thinking);
        }
        if let Some(mode) = options.mode {
            config.mode = mode;
        }

        // 2. Resolve auth
        let auth_path = options
            .auth_path
            .clone()
            .unwrap_or_else(|| Config::user_config_dir().join("auth.json"));
        let mut auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));

        if let Some(ref key) = options.api_key {
            // We'll set this after we know the provider name
            // Store it temporarily
            let _ = key; // handled below
        }

        // 3. Resolve model + provider
        let model_registry = ModelRegistry::with_builtins();
        let model_hint = options
            .model
            .as_deref()
            .or(config.model.as_deref())
            .unwrap_or("sonnet");

        let meta = model_registry
            .resolve_meta(model_hint, options.provider.as_deref())
            .ok_or_else(|| Error::Config(format!("Unknown model: {model_hint}")))?;

        let mut provider_name = options
            .provider
            .as_deref()
            .unwrap_or(&meta.provider)
            .to_string();

        // ChatGPT/Codex provider selection
        if should_use_codex(
            &options,
            &auth_store,
            &model_registry,
            &meta.id,
            &provider_name,
        ) {
            provider_name = "openai-codex".to_string();
        }

        // Set runtime API key override now that we know the provider
        if let Some(ref key) = options.api_key {
            auth_store.set_runtime_key(&provider_name, key.clone());
        }

        let provider = create_provider(&provider_name)
            .ok_or_else(|| Error::Config(format!("Unknown provider: {provider_name}")))?;

        // 4. Resolve API key
        let api_key = resolve_api_key(&mut auth_store, &provider_name).await?;

        let model = Model {
            meta,
            provider: Arc::from(provider),
        };

        // 5. Build agent
        let (agent, handle) = if options.no_tools {
            let (mut a, h) = Agent::new(clone_model(&model), cwd.clone());
            a.api_key = api_key;
            a.thinking_level = config.thinking.unwrap_or(ThinkingLevel::Off);
            if let Some(max_turns) = options.max_turns.or(config.max_turns) {
                a.max_turns = max_turns;
            }
            a.system_prompt = options.system_prompt.clone().unwrap_or_default();
            if let Some(ui) = &options.ui {
                a.ui = Arc::clone(ui);
            }
            (a, h)
        } else {
            let mut builder =
                AgentBuilder::new(config.clone(), cwd.clone(), clone_model(&model), api_key);

            if let Some(task) = &options.task {
                builder = builder.task(task.clone());
            }
            if let Some(prompt) = &options.system_prompt {
                builder = builder.system_prompt(prompt.clone());
            }
            if let Some(lua_loader) = options.lua_loader {
                builder = builder.lua_tool_loader(lua_loader);
            }

            let (mut a, h) = builder.build()?;

            if let Some(max_turns) = options.max_turns {
                a.max_turns = max_turns;
            }
            if let Some(ui) = &options.ui {
                a.ui = Arc::clone(ui);
            }

            (a, h)
        };

        // 6. Set up session persistence
        let session_dir = Config::session_dir();
        let session_mgr = match options.session {
            SessionChoice::New => SessionManager::new(&cwd, &session_dir)?,
            SessionChoice::InMemory => SessionManager::in_memory(),
            SessionChoice::Continue => SessionManager::continue_recent(&cwd, &session_dir)?
                .unwrap_or_else(|| SessionManager::new(&cwd, &session_dir).unwrap()),
            SessionChoice::Open(ref path) => SessionManager::open(path)?,
        };

        Ok(Self {
            agent: Some(agent),
            handle,
            session_mgr,
            config,
            model,
            auth_store,
            model_registry,
            cwd,
            agent_task: None,
        })
    }

    // ── Prompting ───────────────────────────────────────────────

    /// Send a prompt and run the agent loop.
    ///
    /// The agent runs on a background task. Use [`recv_event`] to consume
    /// events, and [`steer`] / [`follow_up`] / [`cancel`] to control it.
    ///
    /// Returns an error if the agent is already running.
    pub async fn prompt(&mut self, text: &str) -> Result<()> {
        if self.agent_task.is_some() {
            return Err(Error::Config(
                "Agent is already running. Cancel or wait for it to finish.".into(),
            ));
        }

        // Persist user message to session
        let msg_id = uuid::Uuid::new_v4().to_string();
        let _ = self
            .session_mgr
            .append(crate::session::SessionEntry::Message {
                id: msg_id,
                parent_id: None,
                message: imp_llm::Message::user(text),
            });

        // Load prior messages from session history into agent
        let mut agent = self
            .agent
            .take()
            .ok_or_else(|| Error::Config("Agent already consumed".into()))?;

        let history: Vec<imp_llm::Message> = self
            .session_mgr
            .get_messages()
            .iter()
            .cloned()
            .cloned()
            .collect();

        // Replace agent messages with session history (which includes the
        // new user message we just appended).
        agent.messages = history;

        let prompt = text.to_string();
        let task = tokio::spawn(async move {
            agent.run(prompt).await?;
            Ok(())
        });
        // We can't get the agent back from the task easily, so we store
        // the task handle and reconstruct state from events when it ends.
        self.agent_task = Some(task);

        Ok(())
    }

    /// Send a prompt and block until the agent finishes.
    ///
    /// Events are still emitted via [`recv_event`], but this method
    /// does not return until the agent loop completes.
    pub async fn prompt_and_wait(&mut self, text: &str) -> Result<()> {
        self.prompt(text).await?;
        self.wait().await
    }

    /// Wait for the running agent to finish.
    pub async fn wait(&mut self) -> Result<()> {
        if let Some(task) = self.agent_task.take() {
            task.await
                .map_err(|e| Error::Config(format!("Agent task panicked: {e}")))??;
        }
        Ok(())
    }

    /// Interrupt the agent: delivered after the current tool finishes,
    /// remaining queued tools are skipped.
    pub async fn steer(&self, text: &str) -> Result<()> {
        self.handle
            .command_tx
            .send(AgentCommand::Steer(text.into()))
            .await
            .map_err(|_| Error::Config("Agent not running".into()))
    }

    /// Follow-up: delivered only after the agent finishes all current work.
    pub async fn follow_up(&self, text: &str) -> Result<()> {
        self.handle
            .command_tx
            .send(AgentCommand::FollowUp(text.into()))
            .await
            .map_err(|_| Error::Config("Agent not running".into()))
    }

    /// Cancel the current agent run.
    pub async fn cancel(&self) -> Result<()> {
        self.handle
            .command_tx
            .send(AgentCommand::Cancel)
            .await
            .map_err(|_| Error::Config("Agent not running".into()))
    }

    // ── Events ──────────────────────────────────────────────────

    /// Receive the next event from the agent.
    ///
    /// Returns `None` when the agent has finished and all events have
    /// been consumed.
    pub async fn recv_event(&mut self) -> Option<AgentEvent> {
        self.handle.event_rx.recv().await
    }

    /// Get mutable access to the raw event receiver.
    ///
    /// Use this when you need `select!` or other channel combinators.
    pub fn event_rx(&mut self) -> &mut mpsc::Receiver<AgentEvent> {
        &mut self.handle.event_rx
    }

    // ── Model ───────────────────────────────────────────────────

    /// Switch the model for subsequent prompts.
    ///
    /// The change takes effect on the next `prompt()` call.
    pub async fn set_model(&mut self, hint: &str) -> Result<()> {
        let meta = self
            .model_registry
            .resolve_meta(hint, None)
            .ok_or_else(|| Error::Config(format!("Unknown model: {hint}")))?;

        let provider_name = meta.provider.clone();
        let provider = create_provider(&provider_name)
            .ok_or_else(|| Error::Config(format!("Unknown provider: {provider_name}")))?;
        let api_key = resolve_api_key(&mut self.auth_store, &provider_name).await?;

        self.model = Model {
            meta,
            provider: Arc::from(provider),
        };

        // If we still have the agent (not currently running), update it
        if let Some(ref mut agent) = self.agent {
            agent.model = clone_model(&self.model);
            agent.api_key = api_key;
        }

        Ok(())
    }

    /// Set the thinking level for subsequent prompts.
    pub fn set_thinking(&mut self, level: ThinkingLevel) {
        self.config.thinking = Some(level);
        if let Some(ref mut agent) = self.agent {
            agent.thinking_level = level;
        }
    }

    // ── Accessors ───────────────────────────────────────────────

    /// The current model.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The resolved config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The session manager (tree, entries, persistence).
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_mgr
    }

    /// Mutable access to the session manager.
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_mgr
    }

    /// The working directory.
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// The auth store (for checking credentials, OAuth status, etc).
    pub fn auth_store(&self) -> &AuthStore {
        &self.auth_store
    }

    /// Mutable access to the auth store.
    pub fn auth_store_mut(&mut self) -> &mut AuthStore {
        &mut self.auth_store
    }

    /// The model registry.
    pub fn model_registry(&self) -> &ModelRegistry {
        &self.model_registry
    }

    /// Whether the agent is currently running a prompt.
    pub fn is_running(&self) -> bool {
        self.agent_task.is_some()
    }

    /// Get the raw command sender for advanced use cases.
    pub fn command_tx(&self) -> &mpsc::Sender<AgentCommand> {
        &self.handle.command_tx
    }
}

// ── Helpers ─────────────────────────────────────────────────────

/// Resolve the API key for a provider, handling OAuth refresh.
async fn resolve_api_key(auth_store: &mut AuthStore, provider: &str) -> Result<ApiKey> {
    let result = match provider {
        "openai" => auth_store.resolve_api_key_only(provider),
        "openai-codex" => auth_store.resolve_chatgpt_oauth().await,
        _ => auth_store.resolve_with_refresh(provider).await,
    };
    result.map_err(|e| Error::Config(format!("Auth failed for {provider}: {e}")))
}

/// Detect whether we should use the ChatGPT/Codex subscription provider
/// instead of the regular OpenAI API key provider.
fn should_use_codex(
    options: &SessionOptions,
    auth_store: &AuthStore,
    registry: &ModelRegistry,
    model_id: &str,
    provider_name: &str,
) -> bool {
    options.provider.is_none()
        && options.api_key.is_none()
        && provider_name == "openai"
        && auth_store.resolve_api_key_only("openai").is_err()
        && (auth_store.get_oauth("openai").is_some()
            || auth_store.get_oauth("openai-codex").is_some())
        && codex_supports_model(registry, model_id)
}

fn clone_model(model: &Model) -> Model {
    Model {
        meta: model.meta.clone(),
        provider: Arc::clone(&model.provider),
    }
}

fn codex_supports_model(_registry: &ModelRegistry, model_id: &str) -> bool {
    imp_llm::model::builtin_openai_codex_models()
        .iter()
        .any(|m| m.id == model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_options_default_is_sensible() {
        let opts = SessionOptions::default();
        assert!(opts.model.is_none());
        assert!(!opts.no_tools);
        assert!(matches!(opts.session, SessionChoice::New));
    }

    #[test]
    fn should_use_codex_returns_false_when_provider_set() {
        let auth_path = PathBuf::from("/tmp/nonexistent-auth.json");
        let auth_store = AuthStore::new(auth_path);
        let registry = ModelRegistry::with_builtins();
        let options = SessionOptions {
            provider: Some("openai".into()),
            ..Default::default()
        };
        assert!(!should_use_codex(
            &options,
            &auth_store,
            &registry,
            "gpt-4o",
            "openai"
        ));
    }

    #[test]
    fn should_use_codex_returns_false_when_api_key_set() {
        let auth_path = PathBuf::from("/tmp/nonexistent-auth.json");
        let auth_store = AuthStore::new(auth_path);
        let registry = ModelRegistry::with_builtins();
        let options = SessionOptions {
            api_key: Some("sk-test".into()),
            ..Default::default()
        };
        assert!(!should_use_codex(
            &options,
            &auth_store,
            &registry,
            "gpt-4o",
            "openai"
        ));
    }
}
