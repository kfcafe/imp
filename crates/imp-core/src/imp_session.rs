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

use std::collections::VecDeque;
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
use crate::session::{SessionEntry, SessionManager};
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

    /// Pre-assembled context messages injected before the first prompt.
    /// Built by `context_prefill::assemble_context()` at dispatch time.
    /// The agent starts with these files already in its cached prefix.
    pub context_prefill: Vec<imp_llm::Message>,
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
            context_prefill: Vec::new(),
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
    agent_task: Option<JoinHandle<(Agent, Result<()>)>>,
    completed_run_result: Option<Result<()>>,
    pending_persistence_errors: VecDeque<String>,
    /// Context prefill messages, injected once before the first prompt.
    context_prefill: Vec<imp_llm::Message>,
    context_prefill_injected: bool,
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
            context_prefill: options.context_prefill,
            context_prefill_injected: false,
            agent_task: None,
            completed_run_result: None,
            pending_persistence_errors: VecDeque::new(),
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

        self.completed_run_result = None;
        self.pending_persistence_errors.clear();

        // Persist user message to session
        let msg_id = uuid::Uuid::new_v4().to_string();
        let _ = self.session_mgr.append(SessionEntry::Message {
            id: msg_id,
            parent_id: None,
            message: imp_llm::Message::user(text),
        });

        // Load prior messages from session history into agent
        let mut agent = self
            .agent
            .take()
            .ok_or_else(|| Error::Config("Agent already consumed".into()))?;

        let mut history: Vec<imp_llm::Message> =
            self.session_mgr.get_active_messages();

        // The prompt was already appended to session history so resume/tree state
        // is correct, but Agent::run() will push the active prompt itself. Remove
        // the just-appended trailing user message to avoid duplicating it in the
        // model context for this run.
        if matches!(
            history.last(),
            Some(imp_llm::Message::User(user))
                if matches!(
                    user.content.as_slice(),
                    [imp_llm::ContentBlock::Text { text: last_text }] if last_text == text
                )
        ) {
            history.pop();
        }

        // Inject context prefill (once, before the first prompt). These messages
        // form the cached prefix: file contents the agent needs, assembled at
        // dispatch time by context_prefill::assemble_context(). Subsequent turns
        // get cache_read on this prefix instead of re-reading files.
        if !self.context_prefill_injected && !self.context_prefill.is_empty() {
            for msg in &self.context_prefill {
                history.push(msg.clone());
            }
            // Assistant acknowledgment to maintain user/assistant alternation
            history.push(imp_llm::Message::Assistant(imp_llm::AssistantMessage {
                content: vec![imp_llm::ContentBlock::Text {
                    text: "Context loaded. Ready to work.".into(),
                }],
                usage: None,
                stop_reason: imp_llm::StopReason::EndTurn,
                timestamp: imp_llm::now(),
            }));
            self.context_prefill_injected = true;
        }

        // Replace agent messages with session history. Agent::run() will append
        // the active prompt as the next user message.
        agent.messages = history;

        let prompt = text.to_string();
        let task = tokio::spawn(async move {
            let result = agent.run(prompt).await;
            (agent, result)
        });
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
            let (agent, result) = task
                .await
                .map_err(|e| Error::Config(format!("Agent task panicked: {e}")))?;
            self.agent = Some(agent);
            self.completed_run_result = Some(result);
            self.drain_pending_events_for_persistence();
        }

        if let Some(result) = self.completed_run_result.take() {
            return result;
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
        if let Some(error) = self.take_persistence_error() {
            return Some(AgentEvent::Error { error });
        }

        let event = self.handle.event_rx.recv().await?;
        let events = self.persist_event_entries(&event);

        if matches!(event, AgentEvent::AgentEnd { .. }) {
            if let Some(task) = self.agent_task.take() {
                match task.await {
                    Ok((agent, result)) => {
                        self.agent = Some(agent);
                        self.completed_run_result = Some(result);
                    }
                    Err(join_error) => {
                        self.push_persistence_error(
                            events,
                            format!("agent task panicked: {join_error}"),
                        );
                    }
                }
            }
        }

        Some(event)
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

    fn persist_event_entries(&mut self, event: &AgentEvent) -> Vec<&'static str> {
        match self
            .session_mgr
            .persist_agent_event_entries(&self.model, event)
        {
            Ok(persisted) => persisted,
            Err(error) => {
                self.push_persistence_error(
                    Vec::new(),
                    format!("failed to persist agent event entries: {error}"),
                );
                Vec::new()
            }
        }
    }

    fn drain_pending_events_for_persistence(&mut self) {
        while let Ok(event) = self.handle.event_rx.try_recv() {
            self.persist_event_entries(&event);
        }
    }

    fn push_persistence_error(&mut self, persisted: Vec<&'static str>, error: String) {
        let prefix = if persisted.is_empty() {
            "session persistence warning".to_string()
        } else {
            format!("session persistence warning after {}", persisted.join(", "))
        };
        self.pending_persistence_errors
            .push_back(format!("{prefix}: {error}"));
    }

    fn take_persistence_error(&mut self) -> Option<String> {
        self.pending_persistence_errors.pop_front()
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
    use imp_llm::{
        auth::{ApiKey, AuthStore},
        model::{Capabilities, ModelPricing},
        provider::{Context, Provider, RequestOptions},
        AssistantMessage, ContentBlock, ModelMeta, StopReason, StreamEvent, Usage,
    };
    use serde_json::json;
    use tempfile::TempDir;

    struct NoopProvider {
        models: Vec<ModelMeta>,
    }

    struct SingleResponseProvider {
        models: Vec<ModelMeta>,
        events: std::sync::Mutex<Option<Vec<imp_llm::Result<StreamEvent>>>>,
    }

    #[async_trait::async_trait]
    impl Provider for NoopProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> std::pin::Pin<Box<dyn futures_core::Stream<Item = imp_llm::Result<StreamEvent>> + Send>>
        {
            Box::pin(futures::stream::empty())
        }

        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
            Ok(String::new())
        }

        fn id(&self) -> &str {
            "noop"
        }

        fn models(&self) -> &[ModelMeta] {
            &self.models
        }
    }

    #[async_trait::async_trait]
    impl Provider for SingleResponseProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> std::pin::Pin<Box<dyn futures_core::Stream<Item = imp_llm::Result<StreamEvent>> + Send>>
        {
            let events = self
                .events
                .lock()
                .expect("single response provider lock")
                .take()
                .unwrap_or_default();
            Box::pin(futures::stream::iter(events))
        }

        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
            Ok(String::new())
        }

        fn id(&self) -> &str {
            "single-response"
        }

        fn models(&self) -> &[ModelMeta] {
            &self.models
        }
    }

    fn test_model() -> Model {
        let meta = ModelMeta {
            id: "test-model".into(),
            provider: "test-provider".into(),
            name: "Test Model".into(),
            context_window: 8192,
            max_output_tokens: 2048,
            pricing: ModelPricing {
                input_per_mtok: 2.0,
                output_per_mtok: 4.0,
                cache_read_per_mtok: 0.5,
                cache_write_per_mtok: 1.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        };
        Model {
            meta: meta.clone(),
            provider: Arc::new(NoopProvider { models: vec![meta] }),
        }
    }

    fn test_model_with_events(events: Vec<imp_llm::Result<StreamEvent>>) -> Model {
        let meta = ModelMeta {
            id: "test-model".into(),
            provider: "test-provider".into(),
            name: "Test Model".into(),
            context_window: 8192,
            max_output_tokens: 2048,
            pricing: ModelPricing {
                input_per_mtok: 2.0,
                output_per_mtok: 4.0,
                cache_read_per_mtok: 0.5,
                cache_write_per_mtok: 1.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        };
        Model {
            meta: meta.clone(),
            provider: Arc::new(SingleResponseProvider {
                models: vec![meta],
                events: std::sync::Mutex::new(Some(events)),
            }),
        }
    }

    fn test_assistant_message(timestamp: u64, usage: Option<Usage>) -> AssistantMessage {
        AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "done".into(),
            }],
            usage,
            stop_reason: StopReason::EndTurn,
            timestamp,
        }
    }

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

    #[tokio::test]
    async fn prompt_uses_session_history_without_duplicate_active_prompt() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let model = test_model_with_events(vec![Ok(StreamEvent::MessageEnd {
            message: AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "done".into(),
                }],
                usage: None,
                stop_reason: StopReason::EndTurn,
                timestamp: 42,
            },
        })]);
        let mut session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        session_mgr
            .append(SessionEntry::Message {
                id: "existing-user".into(),
                parent_id: None,
                message: imp_llm::Message::user("earlier"),
            })
            .unwrap();

        let (agent, handle) = Agent::new(clone_model(&model), cwd.clone());
        let mut session = ImpSession {
            agent: Some(agent),
            handle,
            session_mgr,
            config: Config::default(),
            model,
            auth_store: AuthStore::new(tmp.path().join("auth.json")),
            model_registry: ModelRegistry::with_builtins(),
            cwd,
            agent_task: None,
            completed_run_result: None,
            pending_persistence_errors: VecDeque::new(),
            context_prefill: Vec::new(),
            context_prefill_injected: false,
        };

        session.prompt("latest").await.unwrap();
        while let Some(event) = session.recv_event().await {
            if matches!(event, AgentEvent::AgentEnd { .. }) {
                break;
            }
        }
        session.wait().await.unwrap();

        let messages: Vec<_> = session.session_mgr.get_active_messages();
        assert_eq!(messages.len(), 3);
        match &messages[0] {
            imp_llm::Message::User(user) => match user.content.as_slice() {
                [ContentBlock::Text { text }] => assert_eq!(text, "earlier"),
                other => panic!("unexpected user content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
        match &messages[1] {
            imp_llm::Message::User(user) => match user.content.as_slice() {
                [ContentBlock::Text { text }] => assert_eq!(text, "latest"),
                other => panic!("unexpected user content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
        match &messages[2] {
            imp_llm::Message::Assistant(assistant) => match assistant.content.as_slice() {
                [ContentBlock::Text { text }] => assert_eq!(text, "done"),
                other => panic!("unexpected assistant content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn prompt_uses_compacted_active_history_for_follow_up_turns() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let model = test_model_with_events(vec![Ok(StreamEvent::MessageEnd {
            message: AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: "follow-up done".into(),
                }],
                usage: None,
                stop_reason: StopReason::EndTurn,
                timestamp: 99,
            },
        })]);
        let mut session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        session_mgr
            .append(SessionEntry::Message {
                id: "u1".into(),
                parent_id: None,
                message: imp_llm::Message::user("older request"),
            })
            .unwrap();
        session_mgr
            .append(SessionEntry::Message {
                id: "a1".into(),
                parent_id: None,
                message: imp_llm::Message::Assistant(AssistantMessage {
                    content: vec![ContentBlock::Text {
                        text: "older answer".into(),
                    }],
                    usage: None,
                    stop_reason: StopReason::EndTurn,
                    timestamp: 1,
                }),
            })
            .unwrap();
        session_mgr
            .append(SessionEntry::Message {
                id: "u2".into(),
                parent_id: None,
                message: imp_llm::Message::user("recent request"),
            })
            .unwrap();
        session_mgr
            .append(SessionEntry::Compaction {
                id: "c1".into(),
                parent_id: None,
                summary: "[CONTEXT COMPACTION] compacted summary".into(),
                first_kept_id: "u2".into(),
                tokens_before: 100,
                tokens_after: 40,
            })
            .unwrap();

        let (agent, handle) = Agent::new(clone_model(&model), cwd.clone());
        let mut session = ImpSession {
            agent: Some(agent),
            handle,
            session_mgr,
            config: Config::default(),
            model,
            auth_store: AuthStore::new(tmp.path().join("auth.json")),
            model_registry: ModelRegistry::with_builtins(),
            cwd,
            agent_task: None,
            completed_run_result: None,
            pending_persistence_errors: VecDeque::new(),
            context_prefill: Vec::new(),
            context_prefill_injected: false,
        };

        session.prompt("new follow-up").await.unwrap();
        while let Some(event) = session.recv_event().await {
            if matches!(event, AgentEvent::AgentEnd { .. }) {
                break;
            }
        }
        session.wait().await.unwrap();

        let messages = session.session_mgr.get_active_messages();
        assert_eq!(messages.len(), 4);
        match &messages[0] {
            imp_llm::Message::User(user) => match user.content.as_slice() {
                [ContentBlock::Text { text }] => assert!(text.contains("CONTEXT COMPACTION")),
                other => panic!("unexpected summary content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
        match &messages[1] {
            imp_llm::Message::User(user) => match user.content.as_slice() {
                [ContentBlock::Text { text }] => assert_eq!(text, "recent request"),
                other => panic!("unexpected recent user content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
        match &messages[2] {
            imp_llm::Message::User(user) => match user.content.as_slice() {
                [ContentBlock::Text { text }] => assert_eq!(text, "new follow-up"),
                other => panic!("unexpected follow-up content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn persist_event_entries_writes_assistant_and_canonical_usage() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let model = test_model();
        let session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        let (_agent, handle) = Agent::new(clone_model(&model), cwd.clone());

        let mut session = ImpSession {
            agent: None,
            handle,
            session_mgr,
            config: Config::default(),
            model,
            auth_store: AuthStore::new(tmp.path().join("auth.json")),
            model_registry: ModelRegistry::with_builtins(),
            cwd,
            agent_task: None,
            completed_run_result: None,
            pending_persistence_errors: VecDeque::new(),
            context_prefill: Vec::new(),
            context_prefill_injected: false,
        };

        let message = test_assistant_message(
            123,
            Some(Usage {
                input_tokens: 1_000,
                output_tokens: 250,
                cache_read_tokens: 100,
                cache_write_tokens: 50,
            }),
        );

        let persisted = session.persist_event_entries(&AgentEvent::TurnEnd {
            index: 2,
            message: message.clone(),
        });

        assert_eq!(persisted, vec!["assistant message", "canonical usage"]);

        let usage_records = session.session_mgr.usage_records();
        assert_eq!(usage_records.len(), 1);
        let record = &usage_records[0];
        assert_eq!(record.turn_index, Some(2));
        assert_eq!(record.provider.as_deref(), Some("test-provider"));
        assert_eq!(record.model.as_deref(), Some("test-model"));
        assert!(record.request_id.starts_with("assistant:"));
        assert!(record.assistant_message_id.is_some());
        let cost = record.cost.as_ref().unwrap();
        assert!((cost.input - 0.002).abs() < 1e-12);
        assert!((cost.output - 0.001).abs() < 1e-12);
        assert!((cost.cache_read - 0.00005).abs() < 1e-12);
        assert!((cost.cache_write - 0.00005).abs() < 1e-12);
        assert!((cost.total - 0.0031).abs() < 1e-12);
    }

    #[test]
    fn persist_event_entries_skips_usage_record_when_usage_missing() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let model = test_model();
        let session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        let (_agent, handle) = Agent::new(clone_model(&model), cwd.clone());

        let mut session = ImpSession {
            agent: None,
            handle,
            session_mgr,
            config: Config::default(),
            model,
            auth_store: AuthStore::new(tmp.path().join("auth.json")),
            model_registry: ModelRegistry::with_builtins(),
            cwd,
            agent_task: None,
            completed_run_result: None,
            pending_persistence_errors: VecDeque::new(),
            context_prefill: Vec::new(),
            context_prefill_injected: false,
        };

        let persisted = session.persist_event_entries(&AgentEvent::TurnEnd {
            index: 0,
            message: test_assistant_message(456, None),
        });

        assert_eq!(persisted, vec!["assistant message"]);
        assert!(session.session_mgr.usage_records().is_empty());
    }

    #[test]
    fn persist_event_entries_writes_tool_results() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let model = test_model();
        let session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        let (_agent, handle) = Agent::new(clone_model(&model), cwd.clone());

        let mut session = ImpSession {
            agent: None,
            handle,
            session_mgr,
            config: Config::default(),
            model,
            auth_store: AuthStore::new(tmp.path().join("auth.json")),
            model_registry: ModelRegistry::with_builtins(),
            cwd,
            agent_task: None,
            completed_run_result: None,
            pending_persistence_errors: VecDeque::new(),
            context_prefill: Vec::new(),
            context_prefill_injected: false,
        };

        let persisted = session.persist_event_entries(&AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".into(),
            result: imp_llm::ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "bash".into(),
                content: vec![ContentBlock::Text { text: "ok".into() }],
                is_error: false,
                details: json!({"exit_code": 0}),
                timestamp: 999,
            },
        });

        assert_eq!(persisted, vec!["tool result"]);
        assert!(session.session_mgr.entries().iter().any(|entry| matches!(
            entry,
            SessionEntry::Message {
                message: imp_llm::Message::ToolResult(_),
                ..
            }
        )));
    }
}
