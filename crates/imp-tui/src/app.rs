use std::collections::HashMap;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use imp_core::ui::WidgetContent;

use imp_lua::LuaRuntime;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use imp_core::agent::{AgentCommand, AgentEvent, AgentHandle};
use imp_core::builder::AgentBuilder;
use imp_core::compaction::{
    execute_compaction_with_retry, prepare_messages_for_compaction, select_compaction_strategy,
    CompactionCapabilities, CompactionStrategy, COMPACTION_SUMMARY_PREFIX,
    DEFAULT_KEEP_RECENT_GROUPS,
};
use imp_core::config::Config;
use imp_core::session::{SessionEntry, SessionManager};
use imp_core::Error as ImpCoreError;
use imp_llm::auth::AuthStore;
use imp_llm::model::{ModelMeta, ModelRegistry, ProviderRegistry};
use imp_llm::providers::create_provider;
use imp_llm::{
    truncate_chars_with_suffix, Cost, Message, Model, StreamEvent, ThinkingLevel, Usage,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::animation::AnimationState;
use crate::highlight::Highlighter;
use crate::keybindings::{self, Action};
use crate::selection::{
    extract_selected_text, SelectablePane, SelectionOverlay, SelectionState, TextSurface,
};
use crate::terminal::{set_window_title, InteractiveTerminal};
use crate::theme::Theme;
use crate::turn_tracker::TurnTracker;
use crate::views::ask_bar::AskState;
use crate::views::chat::{
    build_chat_render_data, build_text_surface_from_lines, clamped_scroll_offset_for_total_lines,
    DisplayMessage, MessageRole, RenderedChatView,
};
use crate::views::command_palette::{builtin_commands, CommandPaletteState, CommandPaletteView};
use crate::views::editor::{EditorState, EditorView};
use crate::views::file_finder::{collect_project_files, FileFinderState, FileFinderView};
use crate::views::login_picker::{login_providers, LoginPickerState, LoginPickerView};
use crate::views::model_selector::{ModelSelection, ModelSelectorState, ModelSelectorView};
use crate::views::personality::{PersonalityScope, PersonalityState, PersonalityView};
use crate::views::secrets_picker::{secret_providers, SecretsPickerState, SecretsPickerView};
use crate::views::session_picker::{SessionPickerState, SessionPickerView};
use crate::views::settings::{SettingsState, SettingsView};
use crate::views::sidebar::{
    build_detail_render_data, build_detail_text_surface_from_plain_lines, build_stream_lines,
    sidebar_sub_areas, Sidebar, SidebarDetailRenderData, SidebarView,
};
use crate::views::status::StatusInfo;
use crate::views::tools::DisplayToolCall;
use crate::views::top_bar::TopBar;
use crate::views::tree::{flatten_tree, TreeView, TreeViewState};
use crate::views::welcome::{needs_welcome, WelcomeState, WelcomeStep, WelcomeView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Chat,
    SidebarList,
    SidebarDetail,
}

#[derive(Debug)]
pub enum UiMode {
    Normal,
    ModelSelector(ModelSelectorState),
    CommandPalette(CommandPaletteState),
    FileFinder(FileFinderState),
    LoginPicker(LoginPickerState),
    SecretsPicker(SecretsPickerState),
    TreeView(TreeViewState),
    Settings(SettingsState),
    Personality(PersonalityState),
    SessionPicker(SessionPickerState),
    Welcome(WelcomeState),
}

#[derive(Debug, Clone)]
pub enum QueuedMessage {
    Steer(String),
    FollowUp(String),
}

pub enum AskReply {
    Select(tokio::sync::oneshot::Sender<Option<usize>>),
    Input(tokio::sync::oneshot::Sender<Option<String>>),
}

#[derive(Debug)]
enum LoginTaskExit {
    Success(String),
    Failed(String),
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn();
    }
}

fn search_provider_docs_url(provider: &str) -> &'static str {
    match provider {
        "tavily" => "https://app.tavily.com/home",
        "exa" => "https://dashboard.exa.ai/api-keys",
        "linkup" => "https://app.linkup.so/api-keys",
        "perplexity" => "https://www.perplexity.ai/settings/api",
        _ => "",
    }
}

fn prompt_text_for_secret_provider(provider: &str) -> String {
    let docs = search_provider_docs_url(provider);
    let mut lines = vec![format!("Configure secure credentials for {provider}")];
    if !docs.is_empty() {
        lines.push(String::new());
        lines.push(format!("Get credentials at: {docs}"));
    }
    lines.push(String::new());
    lines.push("First enter a comma-separated field list (default: api_key).".into());
    lines.push("Then imp will prompt for each field value.".into());
    lines.join("\n")
}

#[derive(Debug)]
enum SecretsFlowState {
    AwaitingFieldNames {
        provider: String,
    },
    AwaitingFieldValues {
        provider: String,
        fields: Vec<String>,
        current: usize,
        values: HashMap<String, String>,
    },
}

#[derive(Debug)]
enum RuntimeSignal {
    AgentEvent(AgentEvent),
    AgentTaskCompleted,
    AgentTaskFailed(String),
    LoginTaskSucceeded(String),
    LoginTaskFailed(String),
    UiRequest(crate::tui_interface::UiRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DragAutoScroll {
    pane: SelectablePane,
    direction: ScrollDirection,
    speed: usize,
    column: u16,
    row: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChatRenderCacheKey {
    width: u16,
    messages_epoch: u64,
    tick: u64,
    chat_tool_focus: Option<usize>,
    word_wrap: bool,
    chat_tool_display: imp_core::config::ChatToolDisplay,
    thinking_lines: usize,
    show_timestamps: bool,
    animation_level: imp_core::config::AnimationLevel,
    activity_state: AnimationState,
    theme_is_light: bool,
}

#[derive(Debug)]
struct ChatRenderCache {
    key: ChatRenderCacheKey,
    render: crate::views::chat::ChatRenderData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SidebarStreamCacheKey {
    width: u16,
    messages_epoch: u64,
    tick: u64,
    selected: Option<usize>,
    word_wrap: bool,
    tool_output: imp_core::config::ToolOutputDisplay,
    tool_output_lines: usize,
    animation_level: imp_core::config::AnimationLevel,
    theme_is_light: bool,
}

#[derive(Debug)]
struct SidebarStreamCache {
    key: SidebarStreamCacheKey,
    lines: Vec<Line<'static>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SidebarDetailCacheKey {
    width: u16,
    messages_epoch: u64,
    selected_tool_id_hash: u64,
    word_wrap: bool,
    tool_output_lines: usize,
    animation_level: imp_core::config::AnimationLevel,
    theme_is_light: bool,
}

#[derive(Debug)]
struct SidebarDetailCache {
    key: SidebarDetailCacheKey,
    render: SidebarDetailRenderData,
}

pub struct App {
    // Core
    pub running: bool,
    pub messages: Vec<DisplayMessage>,
    pub editor: EditorState,
    ask_editor_backup: Option<EditorState>,
    pub cwd: PathBuf,

    // Agent
    pub agent_handle: Option<AgentHandle>,
    agent_task: Option<tokio::task::JoinHandle<Result<(), ImpCoreError>>>,
    pub is_streaming: bool,
    pub message_queue: Vec<QueuedMessage>,

    // Session
    pub session: SessionManager,

    // Config
    pub config: Config,
    pub model_name: String,
    pub thinking_level: ThinkingLevel,
    pub context_window: u32,

    // UI state
    pub mode: UiMode,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub tools_expanded: bool,
    /// Index into the flattened tool call list — `None` means no tool focused.
    pub tool_focus: Option<usize>,

    pub ctrl_c_count: u8,
    pub needs_redraw: bool,
    pub last_esc: Option<Instant>,
    pub tick: u64,
    pub max_turns_override: Option<u32>,
    pub ui_rx: Option<tokio::sync::mpsc::Receiver<crate::tui_interface::UiRequest>>,
    pub ask_state: Option<crate::views::ask_bar::AskState>,
    pub ask_reply: Option<AskReply>,
    secrets_flow: Option<SecretsFlowState>,
    login_task: Option<tokio::task::JoinHandle<LoginTaskExit>>,

    // Accumulated stats
    pub accumulated_usage: Usage,
    pub accumulated_cost: Cost,
    /// Last turn's input tokens — best proxy for actual current context size.
    pub current_context_tokens: u32,
    chat_render_epoch: u64,

    // Extension state
    pub status_items: HashMap<String, String>,
    pub widgets: HashMap<String, WidgetContent>,

    /// Lua extension runtime (for command dispatch and hot-reload).
    pub lua_runtime: Option<Arc<Mutex<LuaRuntime>>>,

    // Sidebar
    pub sidebar: Sidebar,

    /// Which pane has focus for scroll routing.
    pub active_pane: Pane,
    /// Sidebar list area cached from last render (for click/scroll detection).
    pub sidebar_list_rect: Option<Rect>,
    /// Sidebar detail area cached from last render (for click/scroll detection).
    pub sidebar_detail_rect: Option<Rect>,
    /// Cached selectable chat surface from last render.
    pub chat_surface: Option<TextSurface>,
    /// Cached selectable sidebar detail surface from last render.
    pub sidebar_detail_surface: Option<TextSurface>,
    /// Current app-native text selection.
    pub selection: Option<SelectionState>,
    /// Selection anchor while dragging with the mouse.
    pub drag_selection: Option<SelectablePane>,
    /// Active edge-autoscroll while dragging a selection.
    drag_autoscroll: Option<DragAutoScroll>,
    /// Cached chat render data reused while only scroll offset changes.
    chat_render_cache: Option<ChatRenderCache>,
    sidebar_stream_cache: Option<SidebarStreamCache>,
    sidebar_detail_cache: Option<SidebarDetailCache>,

    // Turn activity tracking
    pub turn_tracker: TurnTracker,

    // Display helpers
    pub theme: Theme,
    pub highlighter: Highlighter,
    pub model_registry: ModelRegistry,
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
    auth_store: &AuthStore,
    registry: &ModelRegistry,
    meta: &ModelMeta,
) -> bool {
    meta.provider == "openai"
        && auth_store.resolve_api_key_only("openai").is_err()
        && (auth_store.get_oauth("openai").is_some()
            || auth_store.get_oauth("openai-codex").is_some())
        && model_supports_provider(registry, "openai-codex", &meta.id)
}

async fn resolve_provider_api_key(
    auth_store: &mut AuthStore,
    provider_name: &str,
) -> Result<String, imp_llm::Error> {
    match provider_name {
        "openai" => auth_store.resolve_api_key_only(provider_name),
        "openai-codex" => auth_store.resolve_chatgpt_oauth().await,
        _ => auth_store.resolve_with_refresh(provider_name).await,
    }
}

fn provider_logged_in(auth_store: &AuthStore, provider: &str) -> bool {
    match provider {
        "openai" => {
            auth_store.get_oauth("openai").is_some()
                || auth_store.get_oauth("openai-codex").is_some()
        }
        _ => auth_store.stored.contains_key(provider),
    }
}

fn oauth_provider(provider: &str) -> bool {
    matches!(provider, "anthropic" | "openai" | "openai-codex")
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

fn bump_epoch(epoch: &mut u64) {
    *epoch = epoch.wrapping_add(1);
}

fn stable_hash<T: std::hash::Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl App {
    pub fn new(
        config: Config,
        session: SessionManager,
        model_registry: ModelRegistry,
        cwd: PathBuf,
    ) -> Self {
        let model_name = config.model.clone().unwrap_or_else(|| "sonnet".into());
        let thinking_level = config.thinking.unwrap_or(ThinkingLevel::Medium);
        let theme = Theme::named(config.theme.as_deref().unwrap_or("default"));
        let context_window = model_registry
            .resolve_meta(&model_name, None)
            .map(|m| m.context_window)
            .unwrap_or(200_000);

        Self {
            running: true,
            messages: Vec::new(),
            editor: EditorState::new(),
            ask_editor_backup: None,
            cwd,
            agent_handle: None,
            agent_task: None,
            is_streaming: false,
            message_queue: Vec::new(),
            session,
            config,
            model_name,
            thinking_level,
            context_window,
            mode: UiMode::Normal,
            scroll_offset: 0,
            auto_scroll: true,
            tools_expanded: false,
            tool_focus: None,

            ctrl_c_count: 0,
            needs_redraw: true,
            last_esc: None,
            tick: 0,
            max_turns_override: None,
            ui_rx: None,
            ask_state: None,
            ask_reply: None,
            secrets_flow: None,
            login_task: None,
            accumulated_usage: Usage::default(),
            accumulated_cost: Cost::default(),
            current_context_tokens: 0,
            chat_render_epoch: 0,
            status_items: HashMap::new(),
            widgets: HashMap::new(),
            lua_runtime: None,
            sidebar: Sidebar::default(),
            active_pane: Pane::Chat,
            sidebar_list_rect: None,
            sidebar_detail_rect: None,
            chat_surface: None,
            sidebar_detail_surface: None,
            selection: None,
            drag_selection: None,
            drag_autoscroll: None,
            chat_render_cache: None,
            sidebar_stream_cache: None,
            sidebar_detail_cache: None,
            turn_tracker: TurnTracker::new(),
            theme,
            highlighter: Highlighter::new(),
            model_registry,
        }
    }

    /// Load messages from the current session branch into display messages.
    pub fn load_session_messages(&mut self) {
        self.messages.clear();
        self.invalidate_chat_render_cache();

        let mut branch_messages: Vec<Message> = self.session.get_active_messages();
        imp_core::session::sanitize_messages(&mut branch_messages);

        for msg in &branch_messages {
            match msg {
                // Attach tool results to their parent tool call display entry
                imp_llm::Message::ToolResult(tr) => {
                    let output_text = tr
                        .content
                        .iter()
                        .filter_map(|b| match b {
                            imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    let mut attached = false;
                    for display_msg in self.messages.iter_mut().rev() {
                        for tc in &mut display_msg.tool_calls {
                            if tc.id == tr.tool_call_id {
                                tc.output = Some(output_text.clone());
                                if tc.streaming_output.is_empty() {
                                    tc.streaming_output = output_text.clone();
                                }
                                tc.details = tr.details.clone();
                                tc.is_error = tr.is_error;
                                attached = true;
                                break;
                            }
                        }
                        if attached {
                            break;
                        }
                    }
                    // Only show as standalone if no matching tool call found
                    if !attached {
                        self.messages.push(DisplayMessage::from_message(msg));
                    }
                }
                _ => {
                    let mut display = DisplayMessage::from_message(msg);
                    if matches!(msg, imp_llm::Message::User(_))
                        && display.content.starts_with(COMPACTION_SUMMARY_PREFIX)
                    {
                        display.role = MessageRole::Compaction;
                    }
                    self.messages.push(display);
                }
            }
        }
    }
    pub async fn run(
        &mut self,
        terminal: &mut InteractiveTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.prepare_for_interactive()?;
        self.event_loop(terminal).await
    }

    pub fn terminal_title(&self) -> String {
        let title = self
            .session
            .name()
            .map(str::to_string)
            .or_else(|| self.session.title(48))
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "chat".to_string());
        format!("imp — {title}")
    }

    fn prepare_for_interactive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load Lua extensions (for slash commands and tool registration)
        self.reload_lua_extensions();

        // Check for first-run welcome flow
        let config_dir = Config::user_config_dir();
        let auth_path = config_dir.join("auth.json");
        if needs_welcome(&config_dir, &auth_path) {
            let all_models = self.model_registry.list().to_vec();
            self.mode = UiMode::Welcome(WelcomeState::new(&all_models));
        }

        Ok(())
    }

    async fn event_loop(
        &mut self,
        terminal: &mut InteractiveTerminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tick_rate = Duration::from_millis(16); // ~60fps

        loop {
            let _ = set_window_title(&self.terminal_title());
            // Render
            if self.needs_redraw {
                terminal.draw(|frame| self.render(frame))?;
                self.needs_redraw = false;
            }

            // Poll for terminal events with short timeout
            let timeout = tick_rate;
            if crossterm::event::poll(timeout)? {
                let event = crossterm::event::read()?;
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key(key)?;
                    }
                    Event::Paste(text) => {
                        self.handle_paste(text);
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    Event::Resize(_, _) => {
                        self.needs_redraw = true;
                    }
                    _ => {}
                }
            }

            // Drain agent events and UI requests (non-blocking)
            self.pump_runtime_signals();

            // Tick + periodic redraw for streaming/spinner
            self.tick = self.tick.wrapping_add(1);
            self.maybe_autoscroll_selection();
            if self.is_streaming {
                self.needs_redraw = true;
            }

            if !self.running {
                break;
            }
        }

        Ok(())
    }

    fn pump_runtime_signals(&mut self) {
        let signals = self.collect_runtime_signals();
        for signal in signals {
            self.handle_runtime_signal(signal);
        }
    }

    fn collect_runtime_signals(&mut self) -> Vec<RuntimeSignal> {
        let mut signals = Vec::new();

        if let Some(handle) = self.agent_handle.as_mut() {
            while let Ok(event) = handle.event_rx.try_recv() {
                signals.push(RuntimeSignal::AgentEvent(event));
            }
        }

        let agent_task_finished = self
            .agent_task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished);
        if agent_task_finished {
            if let Some(task) = self.agent_task.take() {
                let outcome = match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(task)
                }) {
                    Ok(Ok(())) | Ok(Err(ImpCoreError::Cancelled)) => Ok(()),
                    Ok(Err(error)) => Err(error.to_string()),
                    Err(error) => Err(format!("Internal agent task failure: {error}")),
                };

                // Drain one more time after join. The agent can finish with final
                // events already queued in event_rx; if we clear the handle first,
                // those late ToolExecutionEnd / TurnEnd / AgentEnd events are lost.
                if let Some(handle) = self.agent_handle.as_mut() {
                    while let Ok(event) = handle.event_rx.try_recv() {
                        signals.push(RuntimeSignal::AgentEvent(event));
                    }
                }

                match outcome {
                    Ok(()) => signals.push(RuntimeSignal::AgentTaskCompleted),
                    Err(error) => signals.push(RuntimeSignal::AgentTaskFailed(error)),
                }
            }
        }

        let login_task_finished = self
            .login_task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished);
        if login_task_finished {
            if let Some(task) = self.login_task.take() {
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(task)
                }) {
                    Ok(LoginTaskExit::Success(message)) => {
                        signals.push(RuntimeSignal::LoginTaskSucceeded(message));
                    }
                    Ok(LoginTaskExit::Failed(message)) => {
                        signals.push(RuntimeSignal::LoginTaskFailed(message));
                    }
                    Err(error) => signals.push(RuntimeSignal::LoginTaskFailed(format!(
                        "Login task failure: {error}"
                    ))),
                }
            }
        }

        if let Some(rx) = self.ui_rx.as_mut() {
            while let Ok(req) = rx.try_recv() {
                signals.push(RuntimeSignal::UiRequest(req));
            }
        }

        signals
    }

    fn handle_runtime_signal(&mut self, signal: RuntimeSignal) {
        match signal {
            RuntimeSignal::AgentEvent(event) => self.handle_agent_event(event),
            RuntimeSignal::AgentTaskCompleted => {
                // AgentEnd handling can synchronously spawn a replacement run via a
                // queued follow-up. Only clear the handle if no active task has
                // taken over by the time we process completion.
                let has_active_replacement = self
                    .agent_task
                    .as_ref()
                    .is_some_and(|task| !task.is_finished());
                if !has_active_replacement {
                    self.agent_handle = None;
                }
            }
            RuntimeSignal::AgentTaskFailed(error) => {
                let has_active_replacement = self
                    .agent_task
                    .as_ref()
                    .is_some_and(|task| !task.is_finished());
                if !has_active_replacement {
                    self.agent_handle = None;
                }
                self.present_agent_failure(error);
            }
            RuntimeSignal::LoginTaskSucceeded(message) => self.push_system_msg(&message),
            RuntimeSignal::LoginTaskFailed(message) => self.push_error_msg(&message),
            RuntimeSignal::UiRequest(req) => self.handle_ui_request(req),
        }
        self.needs_redraw = true;
    }

    fn present_agent_failure(&mut self, error: String) {
        self.is_streaming = false;
        if let Some(last) = self.messages.last_mut() {
            last.is_streaming = false;
        }
        self.push_error_msg(&parse_api_error(&error));
    }

    fn handle_ui_request(&mut self, req: crate::tui_interface::UiRequest) {
        use crate::tui_interface::UiRequest;
        use crate::views::ask_bar::{AskOption, AskState};

        match req {
            UiRequest::Select {
                title,
                options,
                reply,
            } => {
                let ask_options: Vec<AskOption> = options
                    .into_iter()
                    .map(|o| AskOption {
                        label: o.label,
                        description: o.description,
                        checked: false,
                    })
                    .collect();
                self.begin_ask(
                    AskState::with_placeholder(
                        title,
                        String::new(),
                        ask_options,
                        false,
                        "type to filter or answer freely…".into(),
                    ),
                    AskReply::Select(reply),
                );
            }
            UiRequest::Input {
                title,
                placeholder,
                reply,
            } => {
                self.begin_ask(
                    AskState::with_placeholder(
                        title,
                        String::new(),
                        vec![],
                        false,
                        placeholder,
                    ),
                    AskReply::Input(reply),
                );
            }
            UiRequest::Confirm {
                title,
                message,
                reply,
            } => {
                let options = vec![
                    AskOption {
                        label: "Yes".into(),
                        description: None,
                        checked: false,
                    },
                    AskOption {
                        label: "No".into(),
                        description: None,
                        checked: false,
                    },
                ];
                let (bool_tx, bool_rx) = tokio::sync::oneshot::channel();
                self.begin_ask(
                    AskState::with_placeholder(
                        title,
                        message,
                        options,
                        false,
                        String::new(),
                    ),
                    AskReply::Select(bool_tx),
                );
                let confirm_reply = reply;
                tokio::spawn(async move {
                    let result = bool_rx.await.ok().flatten();
                    let _ = confirm_reply.send(result.map(|idx| idx == 0));
                });
            }
            UiRequest::Notify { message, level: _ } => {
                self.push_system_msg(&message);
            }
            UiRequest::SetStatus { key, text } => {
                if let Some(t) = text {
                    self.status_items.insert(key, t);
                } else {
                    self.status_items.remove(&key);
                }
            }
            UiRequest::SetWidget { key, content } => {
                if let Some(content) = content {
                    self.widgets.insert(key, content);
                } else {
                    self.widgets.remove(&key);
                }
            }
            UiRequest::Custom { reply, .. } => {
                let _ = reply.send(None);
            }
        }
    }

    fn begin_ask(&mut self, mut state: AskState, reply: AskReply) {
        if self.ask_state.is_none() {
            self.ask_editor_backup = Some(self.editor.clone());
            self.editor.clear();
        }
        state.sync_from_editor(self.editor.content(), self.editor.cursor);
        self.ask_state = Some(state);
        self.ask_reply = Some(reply);
    }

    fn sync_ask_from_editor(&mut self) {
        if let Some(state) = self.ask_state.as_mut() {
            state.sync_from_editor(self.editor.content(), self.editor.cursor);
        }
    }

    fn restore_editor_after_ask(&mut self) {
        if let Some(saved) = self.ask_editor_backup.take() {
            self.editor = saved;
        } else {
            self.editor.clear();
        }
    }

    // ── Rendering ───────────────────────────────────────────────

    fn current_activity_state(&self) -> AnimationState {
        let active_tools = self
            .messages
            .iter()
            .flat_map(|m| m.tool_calls.iter())
            .filter(|tc| tc.output.is_none() && !tc.is_error)
            .count() as u32;

        let latest_streaming = self.messages.iter().rev().find(|m| m.is_streaming);
        let has_visible_content = latest_streaming
            .map(|m| !m.content.trim().is_empty())
            .unwrap_or(false);
        let has_tools_in_turn = latest_streaming
            .map(|m| !m.tool_calls.is_empty())
            .unwrap_or(active_tools > 0);

        AnimationState::from_streaming(
            self.is_streaming,
            has_visible_content,
            has_tools_in_turn,
            active_tools,
            !self.message_queue.is_empty(),
        )
    }

    fn chat_render_cache_key(
        &self,
        width: u16,
        chat_tool_focus: Option<usize>,
        chat_tool_display: imp_core::config::ChatToolDisplay,
        activity_state: AnimationState,
    ) -> ChatRenderCacheKey {
        ChatRenderCacheKey {
            width,
            messages_epoch: self.chat_render_epoch,
            tick: self.tick,
            chat_tool_focus,
            word_wrap: self.config.ui.word_wrap,
            chat_tool_display,
            thinking_lines: self.config.ui.thinking_lines,
            show_timestamps: self.config.ui.show_timestamps,
            animation_level: self.config.ui.animations,
            activity_state,
            theme_is_light: self.theme.bg == Theme::light().bg,
        }
    }

    fn cached_chat_render(
        &mut self,
        width: u16,
        chat_tool_focus: Option<usize>,
        chat_tool_display: imp_core::config::ChatToolDisplay,
        activity_state: AnimationState,
    ) -> &crate::views::chat::ChatRenderData {
        let key = self.chat_render_cache_key(
            width,
            chat_tool_focus,
            chat_tool_display,
            activity_state,
        );
        let cache_hit = self
            .chat_render_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key);
        if !cache_hit {
            let render = build_chat_render_data(
                &self.messages,
                &self.theme,
                &self.highlighter,
                width as usize,
                self.tick,
                chat_tool_focus,
                self.config.ui.word_wrap,
                chat_tool_display,
                self.config.ui.thinking_lines,
                self.config.ui.show_timestamps,
                self.config.ui.animations,
                activity_state,
            );
            self.chat_render_cache = Some(ChatRenderCache { key, render });
        }

        &self.chat_render_cache
            .as_ref()
            .expect("chat render cache set")
            .render
    }

    fn invalidate_chat_render_cache(&mut self) {
        self.chat_render_cache = None;
        bump_epoch(&mut self.chat_render_epoch);
        self.sidebar_stream_cache = None;
        self.sidebar_detail_cache = None;
    }

    fn sidebar_stream_cache_key(&self, width: u16) -> SidebarStreamCacheKey {
        SidebarStreamCacheKey {
            width,
            messages_epoch: self.chat_render_epoch,
            tick: self.tick,
            selected: self.tool_focus,
            word_wrap: self.config.ui.word_wrap,
            tool_output: self.config.ui.tool_output,
            tool_output_lines: self.config.ui.tool_output_lines,
            animation_level: self.config.ui.animations,
            theme_is_light: self.theme.bg == Theme::light().bg,
        }
    }

    fn cached_sidebar_stream_lines(&mut self, width: u16) -> &Vec<Line<'static>> {
        let key = self.sidebar_stream_cache_key(width);
        let cache_hit = self
            .sidebar_stream_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key);
        if !cache_hit {
            let all_tool_calls: Vec<&DisplayToolCall> = self
                .messages
                .iter()
                .flat_map(|m| m.tool_calls.iter())
                .collect();
            let lines = build_stream_lines(
                &all_tool_calls,
                self.tool_focus,
                &self.theme,
                &self.highlighter,
                self.tick,
                &self.config.ui,
                self.config.ui.animations,
                width as usize,
            );
            self.sidebar_stream_cache = Some(SidebarStreamCache { key, lines });
        }
        &self.sidebar_stream_cache.as_ref().expect("sidebar stream cache set").lines
    }

    fn sidebar_detail_cache_key(
        &self,
        width: u16,
        selected_tc: Option<&DisplayToolCall>,
    ) -> SidebarDetailCacheKey {
        SidebarDetailCacheKey {
            width,
            messages_epoch: self.chat_render_epoch,
            selected_tool_id_hash: stable_hash(&selected_tc.map(|tc| &tc.id)),
            word_wrap: self.config.ui.word_wrap,
            tool_output_lines: self.config.ui.tool_output_lines,
            animation_level: self.config.ui.animations,
            theme_is_light: self.theme.bg == Theme::light().bg,
        }
    }

    fn cached_sidebar_detail_render(
        &mut self,
        width: u16,
        selected_tc: Option<&DisplayToolCall>,
    ) -> &SidebarDetailRenderData {
        let key = self.sidebar_detail_cache_key(width, selected_tc);
        let cache_hit = self
            .sidebar_detail_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key);
        if !cache_hit {
            let render = build_detail_render_data(
                selected_tc,
                &self.config.ui,
                &self.highlighter,
                &self.theme,
                width as usize,
            );
            self.sidebar_detail_cache = Some(SidebarDetailCache { key, render });
        }
        &self.sidebar_detail_cache.as_ref().expect("sidebar detail cache set").render
    }

    fn render_widget_tray(&self, frame: &mut Frame, area: Rect) {
        if self.widgets.is_empty() || area.height == 0 {
            return;
        }

        let mut keys: Vec<_> = self.widgets.keys().cloned().collect();
        keys.sort();

        let mut sections = Vec::new();
        for key in keys {
            if let Some(widget) = self.widgets.get(&key) {
                match widget {
                    WidgetContent::Lines(lines) => {
                        if !lines.is_empty() {
                            sections.push(format!("{key}\n{}", lines.join("\n")));
                        }
                    }
                    WidgetContent::Component(component) => {
                        sections.push(format!("{key}\n[component: {}]", component.component_type));
                    }
                }
            }
        }

        if sections.is_empty() {
            return;
        }

        let text = sections.join("\n\n");
        let widget =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("widgets"));
        frame.render_widget(widget, area);
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Clear, area);

        let has_widgets = !self.widgets.is_empty();
        let widget_height = if has_widgets { 5 } else { 0 };

        // Editor/prompt height: while asking, the prompt box becomes the ask box.
        // Otherwise it grows to fit wrapped prompt text while preserving at least
        // 3 lines for the chat area and 1 for the top bar.
        let editor_inner_width = area.width.saturating_sub(2).max(1);
        let desired_editor_height = if let Some(state) = self.ask_state.as_ref() {
            state.prompt_height()
        } else {
            self.editor.visual_line_count(editor_inner_width) as u16 + 2
        };
        let max_editor_height = area.height.saturating_sub(1 + 3).max(3);
        let editor_height = desired_editor_height.clamp(3, max_editor_height);

        let constraints = vec![
            Constraint::Length(1),             // top bar
            Constraint::Length(widget_height), // widget tray
            Constraint::Min(3),                // messages area
            Constraint::Length(editor_height), // editor / ask prompt
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let (top_bar_area, widget_area, chat_area, editor_area) = (
            chunks[0],
            Some(chunks[1]),
            chunks[2],
            chunks[3],
        );

        // Split chat area for sidebar when open
        let (chat_area, sidebar_area) = if self.sidebar.open && chat_area.width >= 60 {
            let min_sidebar = 30u16;
            let pct = self.config.ui.sidebar_width.clamp(20, 80);
            let sidebar_w = (chat_area.width * pct / 100)
                .max(min_sidebar)
                .min(chat_area.width.saturating_sub(30));
            let chat_w = chat_area.width.saturating_sub(sidebar_w);
            let chat_rect = Rect {
                width: chat_w,
                ..chat_area
            };
            let sidebar_rect = Rect {
                x: chat_area.x + chat_w,
                width: sidebar_w,
                ..chat_area
            };
            (chat_rect, Some(sidebar_rect))
        } else {
            (chat_area, None)
        };

        // Top bar (header line)
        let status_info = self.build_status_info();
        let top_bar = TopBar::new(&status_info, &self.theme);
        frame.render_widget(top_bar, top_bar_area);

        if let Some(widget_area) = widget_area {
            self.render_widget_tray(frame, widget_area);
        }

        // Messages
        let chat_tool_display = self.config.ui.effective_chat_tool_display();
        let chat_tool_focus = if self.active_pane == Pane::Chat {
            self.tool_focus
        } else {
            None
        };
        let activity_state = self.current_activity_state();
        let total_chat_lines = {
            let chat_render = self.cached_chat_render(
                chat_area.width,
                chat_tool_focus,
                chat_tool_display,
                activity_state,
            );
            chat_render.lines.len()
        };
        self.scroll_offset = clamped_scroll_offset_for_total_lines(
            total_chat_lines,
            chat_area,
            self.scroll_offset,
        );
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }

        let chat_lines = {
            self.cached_chat_render(
                chat_area.width,
                chat_tool_focus,
                chat_tool_display,
                activity_state,
            )
            .lines
            .clone()
        };

        let chat = RenderedChatView::new(&chat_lines).scroll(self.scroll_offset);
        frame.render_widget(chat, chat_area);

        self.chat_surface = Some(build_text_surface_from_lines(
            &chat_lines,
            chat_area,
            self.scroll_offset,
        ));

        // Sidebar
        if let Some(sidebar_area) = sidebar_area {
            let tc_count = self.total_tool_calls();
            let sub = sidebar_sub_areas(sidebar_area, tc_count, self.config.ui.sidebar_style);
            let selected_tc_index = self.tool_focus;

            let stream_lines = if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Stream {
                Some(self.cached_sidebar_stream_lines(sub.0.width).clone())
            } else {
                None
            };
            let detail_render = if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Split {
                let selected_tc_owned = selected_tc_index.and_then(|i| {
                    self.messages
                        .iter()
                        .flat_map(|m| m.tool_calls.iter())
                        .nth(i)
                        .cloned()
                });
                Some(
                    self.cached_sidebar_detail_render(sub.1.width, selected_tc_owned.as_ref())
                        .clone(),
                )
            } else {
                None
            };

            let all_tool_calls: Vec<&DisplayToolCall> = self
                .messages
                .iter()
                .flat_map(|m| m.tool_calls.iter())
                .collect();
            let mut view = SidebarView::new(
                all_tool_calls,
                self.tool_focus,
                &self.theme,
                &self.highlighter,
                self.tick,
                self.sidebar.list_scroll,
                self.sidebar.detail_scroll,
                &self.config.ui,
            );

            match self.config.ui.sidebar_style {
                imp_core::config::SidebarStyle::Stream => {
                    let stream_lines = stream_lines.expect("stream cache lines");
                    view = view.precomputed_stream_lines(&stream_lines);
                    frame.render_widget(view, sidebar_area);
                }
                imp_core::config::SidebarStyle::Split => {
                    let detail_lines = detail_render.as_ref().expect("detail cache lines");
                    view = view.precomputed_detail_lines(&detail_lines.lines);
                    frame.render_widget(view, sidebar_area);
                }
            }

            self.sidebar_list_rect = Some(sub.0);
            self.sidebar_detail_rect = Some(sub.1);
            self.sidebar.list_height = sub.0.height;
            let detail_plain_lines = detail_render
                .as_ref()
                .map(|render| render.plain_lines.clone())
                .unwrap_or_default();
            self.sidebar_detail_surface = Some(build_detail_text_surface_from_plain_lines(
                &detail_plain_lines,
                sub.1,
                self.sidebar.detail_scroll,
            ));
        } else {
            self.sidebar_list_rect = None;
            self.sidebar_detail_rect = None;
            self.sidebar_detail_surface = None;
        }

        // Prompt area: reuse the normal editor box for asks.
        if let Some(ref state) = self.ask_state {
            use crate::views::ask_bar::AskBar;
            frame.render_widget(AskBar::new(state, &self.theme), editor_area);
        } else {
            let editor = EditorView::new(&self.editor, &self.theme, self.thinking_level)
                .model(&self.model_name)
                .streaming(self.is_streaming)
                .queued(!self.message_queue.is_empty())
                .context_usage(
                    self.current_context_tokens,
                    self.context_window,
                    self.config.ui.show_context_usage,
                )
                .tick(self.tick)
                .animation_level(self.config.ui.animations)
                .activity_state(activity_state);
            frame.render_widget(editor, editor_area);
        }

        frame.render_widget(
            SelectionOverlay::new(
                &self.theme,
                self.selection.as_ref(),
                self.chat_surface.as_ref(),
                self.sidebar_detail_surface.as_ref(),
            ),
            area,
        );

        // Pre-render: clamp session picker scroll so selected item is visible
        if let UiMode::SessionPicker(ref mut sp) = self.mode {
            let overlay_area = centered_rect(75, 70, area);
            let inner_h = overlay_area.height.saturating_sub(2) as usize;
            let visible_rows = (inner_h / 3).max(1);
            sp.clamp_scroll(visible_rows);
        }

        // Render overlays
        match &self.mode {
            UiMode::Normal => {}
            UiMode::ModelSelector(state) => {
                let overlay_area = centered_rect(60, 70, area);
                let view = ModelSelectorView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::CommandPalette(state) => {
                let palette_area = command_dropdown_area(editor_area, 12);
                let view = CommandPaletteView::new(state, &self.theme);
                frame.render_widget(view, palette_area);
            }
            UiMode::FileFinder(state) => {
                let finder_area = command_dropdown_area(editor_area, 12);
                let view = FileFinderView::new(state, &self.theme);
                frame.render_widget(view, finder_area);
            }
            UiMode::LoginPicker(state) => {
                let overlay_area = centered_rect(60, 40, area);
                let view = LoginPickerView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::SecretsPicker(state) => {
                let overlay_area = centered_rect(70, 50, area);
                let view = SecretsPickerView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::TreeView(state) => {
                let tree_area = centered_rect(80, 80, area);
                let view = TreeView::new(state, &self.theme);
                frame.render_widget(view, tree_area);
            }
            UiMode::Settings(state) => {
                let overlay_area = centered_rect(80, 90, area);
                let view = SettingsView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::Personality(state) => {
                let overlay_area = centered_rect(80, 80, area);
                let view = PersonalityView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::SessionPicker(state) => {
                let overlay_area = centered_rect(75, 70, area);
                let view = SessionPickerView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::Welcome(state) => {
                let overlay_area = centered_rect(70, 80, area);
                let view = WelcomeView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
        }

        // Set cursor position (only in normal mode)
        if matches!(self.mode, UiMode::Normal) {
            let (cx, cy) = if let Some(state) = self.ask_state.as_ref() {
                state.cursor_screen_position(editor_area)
            } else {
                self.editor.cursor_screen_position(editor_area)
            };
            frame.set_cursor_position((cx, cy));
        }
    }

    fn build_status_info(&self) -> StatusInfo {
        let cwd = self.cwd.to_string_lossy().to_string();
        let session_name = self
            .session
            .name()
            .map(str::to_string)
            .or_else(|| self.session.title(48))
            .unwrap_or_default();

        let total_input = self.accumulated_usage.input_tokens;
        let total_output = self.accumulated_usage.output_tokens;
        let current_context_tokens = self.current_context_tokens;
        // Use last turn's input_tokens as the actual context size rather than
        // accumulating across turns, which grows without bound and misrepresents
        // compacted conversations.
        let context_percent = if self.context_window > 0 {
            self.current_context_tokens as f64 / self.context_window as f64
        } else {
            0.0
        };
        let mut extension_items = self.status_items.clone();
        if let Some(info) = self.current_oauth_display_info() {
            extension_items.insert("oauth".into(), info.status_summary());
        }
        let active_tools = self
            .messages
            .iter()
            .flat_map(|m| m.tool_calls.iter())
            .filter(|tc| tc.output.is_none() && !tc.is_error)
            .count() as u32;

        StatusInfo {
            cwd,
            session_name,
            model: self.model_name.clone(),
            thinking: format!("{:?}", self.thinking_level),
            input_tokens: total_input,
            output_tokens: total_output,
            current_context_tokens,
            cost: self.accumulated_cost.total,
            context_percent,
            context_window: self.context_window,
            show_cost: self.config.ui.show_cost,
            show_context_usage: self.config.ui.show_context_usage,
            peek: self.tools_expanded,
            extension_items,
            is_streaming: self.is_streaming,
            active_tools,
            turn_elapsed: self.is_streaming.then(|| self.turn_tracker.elapsed()),
            tick: self.tick,
            animation_level: self.config.ui.animations,
            activity_state: self.current_activity_state(),
        }
    }

    fn current_oauth_display_info(&self) -> Option<imp_llm::auth::OAuthDisplayInfo> {
        let auth_path = Config::user_config_dir().join("auth.json");
        let auth_store = AuthStore::load(&auth_path).ok()?;
        let meta = self.model_registry.resolve_meta(&self.model_name, None)?;
        let mut provider_name = meta.provider.clone();
        if should_use_chatgpt_provider(&auth_store, &self.model_registry, &meta) {
            provider_name = "openai-codex".to_string();
        }
        auth_store.oauth_display_info(&provider_name)
    }

    fn current_model_meta_for_persistence(&self) -> Option<ModelMeta> {
        let auth_path = Config::user_config_dir().join("auth.json");
        let auth_store = AuthStore::load(&auth_path).ok();
        let mut meta = self.model_registry.resolve_meta(&self.model_name, None)?;

        if let Some(auth_store) = auth_store.as_ref() {
            if should_use_chatgpt_provider(auth_store, &self.model_registry, &meta) {
                meta = self
                    .model_registry
                    .resolve_meta(&self.model_name, Some("openai-codex"))?;
            }
        }

        Some(meta)
    }

    // ── Key handling ────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        self.needs_redraw = true;

        if self.ask_state.is_some() && self.is_paste_shortcut(key) {
            self.paste_from_clipboard();
            return Ok(());
        }

        // Reset ctrl+c counter on non-ctrl+c keypress
        if !(key.code == KeyCode::Char('c')
            && (key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::SUPER)))
        {
            self.ctrl_c_count = 0;
        }

        // Ask overlay intercepts all keys when active
        if self.ask_state.is_some() {
            self.handle_ask_key(key);
            return Ok(());
        }

        // Route based on current UI mode
        match &self.mode {
            UiMode::Normal => self.handle_normal_key(key)?,
            UiMode::ModelSelector(_)
            | UiMode::CommandPalette(_)
            | UiMode::FileFinder(_)
            | UiMode::LoginPicker(_)
            | UiMode::SecretsPicker(_) => self.handle_overlay_key(key),
            UiMode::Personality(_) => self.handle_personality_key(key),
            UiMode::TreeView(_) => self.handle_tree_key(key),
            UiMode::Settings(_) => self.handle_settings_key(key),
            UiMode::SessionPicker(_) => self.handle_session_picker_key(key),
            UiMode::Welcome(_) => self.handle_welcome_key(key),
        }

        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_copy_shortcut(key) {
            let _ = self.copy_selection();
            return Ok(());
        }
        if self.is_paste_shortcut(key) {
            self.paste_from_clipboard();
            return Ok(());
        }

        if key.modifiers.contains(KeyModifiers::SHIFT) {
            match key.code {
                KeyCode::Up => {
                    if self.extend_selection_lines(-1) {
                        return Ok(());
                    }
                }
                KeyCode::Down => {
                    if self.extend_selection_lines(1) {
                        return Ok(());
                    }
                }
                KeyCode::PageUp => {
                    if self.extend_selection_lines(-(self.config.ui.keyboard_scroll_lines as isize))
                    {
                        return Ok(());
                    }
                }
                KeyCode::PageDown => {
                    if self.extend_selection_lines(self.config.ui.keyboard_scroll_lines as isize) {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Esc && self.selection.is_some() {
            self.clear_selection();
            return Ok(());
        }

        let action = keybindings::resolve_normal(key);

        match action {
            Some(Action::Submit) => {
                if self.is_streaming {
                    // Queue steering message
                    let text = self.editor.content().to_string();
                    if !text.trim().is_empty() {
                        self.message_queue.push(QueuedMessage::Steer(text));
                        self.editor.clear();
                        // Send to agent
                        if let Some(ref handle) = self.agent_handle {
                            let _ = handle.command_tx.try_send(AgentCommand::Steer(
                                self.message_queue
                                    .last()
                                    .map(|m| match m {
                                        QueuedMessage::Steer(s) => s.clone(),
                                        QueuedMessage::FollowUp(s) => s.clone(),
                                    })
                                    .unwrap_or_default(),
                            ));
                        }
                    }
                } else {
                    self.send_message();
                }
            }
            Some(Action::FollowUp) => {
                if self.is_streaming {
                    let text = self.editor.content().to_string();
                    if !text.trim().is_empty() {
                        self.message_queue.push(QueuedMessage::FollowUp(text));
                        self.editor.clear();
                    }
                }
            }
            Some(Action::NewLine) => {
                self.editor.insert_newline();
            }
            Some(Action::Cancel) => {
                self.handle_cancel();
            }
            Some(Action::SelectModel) => {
                self.open_model_selector();
            }
            Some(Action::CycleModelForward) => {
                self.cycle_model(true);
            }
            Some(Action::CycleModelBackward) => {
                self.cycle_model(false);
            }
            Some(Action::CycleThinking) => {
                self.cycle_thinking_level();
            }
            Some(Action::SidebarToggle) => {
                self.toggle_sidebar();
            }
            Some(Action::Peek) => {
                // Legacy alias — behaves the same as ToolToggle with no focus
                self.tools_expanded = !self.tools_expanded;
                for msg in &mut self.messages {
                    for tc in &mut msg.tool_calls {
                        tc.expanded = self.tools_expanded;
                    }
                }
                self.invalidate_chat_render_cache();
            }
            Some(Action::ToolToggle) => {
                if let Some(idx) = self.tool_focus {
                    // Toggle just the focused tool call
                    if let Some(tc) = self.get_tool_call_mut(idx) {
                        tc.expanded = !tc.expanded;
                    }
                    self.invalidate_chat_render_cache();
                } else {
                    // No focus: toggle all (global expand/collapse)
                    self.tools_expanded = !self.tools_expanded;
                    for msg in &mut self.messages {
                        for tc in &mut msg.tool_calls {
                            tc.expanded = self.tools_expanded;
                        }
                    }
                    self.invalidate_chat_render_cache();
                }
            }
            Some(Action::ToolFocusNext) => {
                let total = self.total_tool_calls();
                if total > 0 {
                    if !self.sidebar.open {
                        self.sidebar.open = true;
                        self.focus_latest_tool();
                    } else {
                        let idx = match self.tool_focus {
                            None => 0,
                            Some(i) => (i + 1).min(total - 1),
                        };
                        self.focus_tool(idx);
                    }
                }
            }
            Some(Action::ToolFocusPrev) => {
                let total = self.total_tool_calls();
                if total > 0 {
                    if !self.sidebar.open {
                        self.sidebar.open = true;
                        self.focus_latest_tool();
                    } else {
                        let idx = match self.tool_focus {
                            None => total.saturating_sub(1),
                            Some(i) => i.saturating_sub(1),
                        };
                        self.focus_tool(idx);
                    }
                }
            }
            Some(Action::InsertChar('@')) => {
                self.editor.insert_char('@');
                self.open_file_finder();
            }
            Some(Action::InsertChar('/')) if self.editor.is_empty() && !self.is_streaming => {
                self.editor.insert_char('/');
                self.mode = UiMode::CommandPalette(CommandPaletteState::new(builtin_commands()));
            }
            Some(Action::InsertChar(c)) => {
                self.editor.insert_char(c);
            }
            Some(Action::Backspace) => {
                self.editor.delete_back();
            }
            Some(Action::Delete) => {
                self.editor.delete_forward();
            }
            Some(Action::CursorLeft) => {
                self.editor.move_left();
            }
            Some(Action::CursorRight) => {
                self.editor.move_right();
            }
            Some(Action::CursorUp) => {
                if self.sidebar.open && self.active_pane == Pane::SidebarList {
                    let total = self.total_tool_calls();
                    if total > 0 {
                        let idx = match self.tool_focus {
                            None => total.saturating_sub(1),
                            Some(i) => i.saturating_sub(1),
                        };
                        self.focus_tool(idx);
                    }
                } else if !self.editor.move_up() {
                    self.editor.history_prev();
                }
            }
            Some(Action::CursorDown) => {
                if self.sidebar.open && self.active_pane == Pane::SidebarList {
                    let total = self.total_tool_calls();
                    if total > 0 {
                        let idx = match self.tool_focus {
                            None => 0,
                            Some(i) => (i + 1).min(total - 1),
                        };
                        self.focus_tool(idx);
                    }
                } else if !self.editor.move_down() {
                    self.editor.history_next();
                }
            }
            Some(Action::CursorHome) => {
                self.editor.move_home();
            }
            Some(Action::CursorEnd) => {
                self.editor.move_end();
            }
            Some(Action::WordLeft) => {
                self.editor.move_word_left();
            }
            Some(Action::WordRight) => {
                self.editor.move_word_right();
            }
            Some(Action::DeleteWordBack) => {
                self.editor.delete_word_back();
            }
            Some(Action::DeleteToStart) => {
                self.editor.delete_to_start();
            }
            Some(Action::DeleteToEnd) => {
                self.editor.delete_to_end();
            }
            Some(Action::ScrollUp) | Some(Action::PageUp) => {
                self.scroll_active_pane_up(self.config.ui.keyboard_scroll_lines);
            }
            Some(Action::ScrollDown) | Some(Action::PageDown) => {
                self.scroll_active_pane_down(self.config.ui.keyboard_scroll_lines);
            }
            Some(Action::Quit) => {
                self.handle_cancel();
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) {
        let action = keybindings::resolve_overlay(key);

        match action {
            Some(Action::OverlayDismiss) => {
                // If dismissing command palette, clear the editor's slash prefix
                if matches!(self.mode, UiMode::CommandPalette(_)) {
                    self.editor.clear();
                }
                self.mode = UiMode::Normal;
            }
            Some(Action::OverlayUp) => match &mut self.mode {
                UiMode::ModelSelector(s) => s.move_up(),
                UiMode::CommandPalette(s) => s.move_up(),
                UiMode::FileFinder(s) => s.move_up(),
                UiMode::LoginPicker(s) => s.move_up(),
                UiMode::SecretsPicker(s) => s.move_up(),
                _ => {}
            },
            Some(Action::OverlayDown) => match &mut self.mode {
                UiMode::ModelSelector(s) => s.move_down(),
                UiMode::CommandPalette(s) => s.move_down(),
                UiMode::FileFinder(s) => s.move_down(),
                UiMode::LoginPicker(s) => s.move_down(),
                UiMode::SecretsPicker(s) => s.move_down(),
                _ => {}
            },
            Some(Action::OverlayFilter(c)) => match &mut self.mode {
                UiMode::ModelSelector(s) => s.push_filter(c),
                UiMode::CommandPalette(s) => {
                    s.push_filter(c);
                    self.editor.insert_char(c);
                }
                UiMode::FileFinder(s) => s.push_filter(c),
                _ => {}
            },
            Some(Action::OverlayBackspace) => match &mut self.mode {
                UiMode::ModelSelector(s) => s.pop_filter(),
                UiMode::CommandPalette(s) => {
                    s.pop_filter();
                    self.editor.delete_back();
                    // If editor is empty (backspaced past /), dismiss
                    if self.editor.is_empty() {
                        self.mode = UiMode::Normal;
                    }
                }
                UiMode::FileFinder(s) => s.pop_filter(),
                _ => {}
            },
            Some(Action::OverlaySelect) => {
                self.handle_overlay_select();
            }
            _ => {}
        }
    }

    fn handle_overlay_select(&mut self) {
        // Take ownership of mode to process selection
        let old_mode = std::mem::replace(&mut self.mode, UiMode::Normal);
        match old_mode {
            UiMode::ModelSelector(state) => {
                if let Some(selection) = state.selected_choice() {
                    match selection {
                        ModelSelection::Builtin(model) => {
                            self.model_name = model.id.clone();
                            self.context_window = model.context_window;
                        }
                        ModelSelection::Custom(model_id) => {
                            self.model_name = model_id;
                            if let Some(meta) =
                                self.model_registry.resolve_meta(&self.model_name, None)
                            {
                                self.context_window = meta.context_window;
                            }
                        }
                    }
                }
            }
            UiMode::CommandPalette(state) => {
                if let Some(cmd) = state.selected_command() {
                    self.editor.clear();
                    self.execute_command(&cmd.name.clone());
                }
            }
            UiMode::FileFinder(state) => {
                if let Some(file) = state.selected_file() {
                    self.editor.insert_char(' ');
                    for c in file.chars() {
                        self.editor.insert_char(c);
                    }
                }
            }
            UiMode::LoginPicker(state) => {
                if let Some(provider) = state.selected_provider() {
                    self.start_login(provider.id);
                }
            }
            UiMode::SecretsPicker(state) => {
                if let Some(provider) = state.selected_provider() {
                    self.start_secrets_flow(&provider.id);
                }
            }
            _ => {
                self.mode = old_mode;
            }
        }
    }

    fn handle_tree_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Tab => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let UiMode::TreeView(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let UiMode::TreeView(ref mut state) = self.mode {
                    state.move_down();
                }
            }
            KeyCode::Enter => {
                let selected_id = if let UiMode::TreeView(ref state) = self.mode {
                    state.selected_id().map(String::from)
                } else {
                    None
                };
                if let Some(id) = selected_id {
                    let _ = self.session.navigate(&id);
                    self.load_session_messages();
                    self.mode = UiMode::Normal;
                }
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let UiMode::TreeView(ref mut state) = self.mode {
                    state.cycle_filter();
                }
            }
            _ => {}
        }
    }

    // ── Tool focus helpers ───────────────────────────────────────

    /// Find a tool call's flat index by ID across all display messages.
    fn find_tool_call_index(&self, id: &str) -> Option<usize> {
        let mut index = 0;
        for msg in &self.messages {
            for tc in &msg.tool_calls {
                if tc.id == id {
                    return Some(index);
                }
                index += 1;
            }
        }
        None
    }

    /// Focus a tool call by flat index: update tool_focus and sync sidebar.
    fn focus_tool(&mut self, index: usize) {
        self.tool_focus = Some(index);
        self.active_pane = if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Split
        {
            Pane::SidebarList
        } else {
            Pane::SidebarDetail
        };
        if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Split {
            self.sidebar.reset_detail_scroll();
            self.sidebar.ensure_selected_visible(index);
        }
    }

    fn focus_latest_tool(&mut self) -> bool {
        let total = self.total_tool_calls();
        if total == 0 {
            return false;
        }
        self.focus_tool(total - 1);
        true
    }

    fn toggle_sidebar(&mut self) {
        if self.sidebar.open {
            self.sidebar.open = false;
            self.active_pane = Pane::Chat;
        } else {
            self.sidebar.open = true;
            if !self.focus_latest_tool() {
                self.active_pane = Pane::Chat;
            }
        }
    }

    /// Total number of tool calls across all display messages.
    fn total_tool_calls(&self) -> usize {
        self.messages.iter().map(|m| m.tool_calls.len()).sum()
    }

    /// Mutable access to a tool call by its flat index across all messages.
    fn get_tool_call_mut(
        &mut self,
        flat_idx: usize,
    ) -> Option<&mut crate::views::tools::DisplayToolCall> {
        let mut remaining = flat_idx;
        for msg in &mut self.messages {
            if remaining < msg.tool_calls.len() {
                return Some(&mut msg.tool_calls[remaining]);
            }
            remaining -= msg.tool_calls.len();
        }
        None
    }

    fn scroll_chat_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        self.auto_scroll = false;
    }

    fn scroll_chat_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    fn scroll_active_pane_up(&mut self, lines: usize) {
        match self.active_pane {
            Pane::SidebarList if self.sidebar.open => self.sidebar.scroll_list_up(lines),
            Pane::SidebarDetail if self.sidebar.open => self.sidebar.scroll_detail_up(lines),
            _ => self.scroll_chat_up(lines),
        }
    }

    fn scroll_active_pane_down(&mut self, lines: usize) {
        match self.active_pane {
            Pane::SidebarList if self.sidebar.open => self.sidebar.scroll_list_down(lines),
            Pane::SidebarDetail if self.sidebar.open => self.sidebar.scroll_detail_down(lines),
            _ => self.scroll_chat_down(lines),
        }
    }

    fn selection_surface(&self, pane: SelectablePane) -> Option<&TextSurface> {
        match pane {
            SelectablePane::Chat => self.chat_surface.as_ref(),
            SelectablePane::SidebarDetail => self.sidebar_detail_surface.as_ref(),
        }
    }

    fn clear_selection(&mut self) {
        self.selection = None;
        self.drag_selection = None;
        self.drag_autoscroll = None;
    }

    fn selection_text(&self) -> Option<String> {
        let selection = self.selection.as_ref()?;
        let surface = self.selection_surface(selection.pane)?;
        extract_selected_text(surface, selection).filter(|text| !text.is_empty())
    }

    fn copy_to_clipboard(&self, text: &str) {
        #[cfg(target_os = "macos")]
        {
            let _ = Self::write_to_clipboard_command("pbcopy", &[], text);
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Self::write_to_clipboard_linux(text);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn write_to_clipboard_command(program: &str, args: &[&str], text: &str) -> bool {
        use std::io::Write;

        let Ok(mut child) = std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            return false;
        };

        if let Some(mut stdin) = child.stdin.take() {
            if stdin.write_all(text.as_bytes()).is_err() {
                return false;
            }
        }

        child.wait().is_ok_and(|status| status.success())
    }

    #[cfg(target_os = "linux")]
    fn write_to_clipboard_linux(text: &str) -> bool {
        Self::write_to_clipboard_command("wl-copy", &[], text)
            || Self::write_to_clipboard_command("xclip", &["-selection", "clipboard"], text)
            || Self::write_to_clipboard_command("xsel", &["--clipboard", "--input"], text)
    }

    fn copy_selection(&mut self) -> bool {
        if let Some(text) = self.selection_text() {
            self.copy_to_clipboard(&text);
            self.push_system_msg("Copied selection to clipboard.");
            true
        } else {
            false
        }
    }

    fn is_copy_shortcut(&self, key: KeyEvent) -> bool {
        key.code == KeyCode::Char('c')
            && (key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::SUPER))
            && self.selection.is_some()
    }

    fn is_paste_shortcut(&self, key: KeyEvent) -> bool {
        key.code == KeyCode::Char('v')
            && (key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::SUPER))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn read_clipboard_command(program: &str, args: &[&str]) -> Option<String> {
        let output = std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    fn read_clipboard_text(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            return Self::read_clipboard_command("pbpaste", &[]);
        }
        #[cfg(target_os = "linux")]
        {
            return Self::read_clipboard_command("wl-paste", &["--no-newline"])
                .or_else(|| {
                    Self::read_clipboard_command("xclip", &["-selection", "clipboard", "-o"])
                })
                .or_else(|| Self::read_clipboard_command("xsel", &["--clipboard", "--output"]));
        }
        #[allow(unreachable_code)]
        None
    }

    fn paste_from_clipboard(&mut self) -> bool {
        let Some(text) = self.read_clipboard_text() else {
            return false;
        };

        self.handle_paste(text);
        true
    }

    fn handle_paste(&mut self, text: String) {
        for ch in text.chars() {
            match ch {
                '\n' => self.editor.insert_newline(),
                '\r' => {}
                c => self.editor.insert_char(c),
            }
        }
        if self.ask_state.is_some() {
            self.sync_ask_from_editor();
        }
        self.needs_redraw = true;
    }

    fn extend_selection_lines(&mut self, delta: isize) -> bool {
        let Some(mut selection) = self.selection.clone() else {
            return false;
        };
        let Some(surface) = self.selection_surface(selection.pane) else {
            return false;
        };

        selection.focus = surface.move_pos(selection.focus, delta, 0);
        match selection.pane {
            SelectablePane::Chat => {
                if selection.focus.line < surface.top_line {
                    self.scroll_chat_up(surface.top_line - selection.focus.line);
                } else {
                    let bottom = surface.top_line + surface.rect.height.saturating_sub(1) as usize;
                    if selection.focus.line > bottom {
                        self.scroll_chat_down(selection.focus.line - bottom);
                    }
                }
            }
            SelectablePane::SidebarDetail => {
                if selection.focus.line < surface.top_line {
                    self.sidebar
                        .scroll_detail_up(surface.top_line - selection.focus.line);
                } else {
                    let bottom = surface.top_line + surface.rect.height.saturating_sub(1) as usize;
                    if selection.focus.line > bottom {
                        self.sidebar
                            .scroll_detail_down(selection.focus.line - bottom);
                    }
                }
            }
        }

        self.selection = Some(selection);
        true
    }

    fn set_drag_autoscroll(
        &mut self,
        pane: SelectablePane,
        surface: &TextSurface,
        col: u16,
        row: u16,
    ) {
        let top_margin = surface.rect.y.saturating_add(1);
        let bottom_margin = surface
            .rect
            .y
            .saturating_add(surface.rect.height.saturating_sub(2));

        let next = if row <= top_margin {
            let speed = if row <= surface.rect.y { 3 } else { 1 };
            Some(DragAutoScroll {
                pane,
                direction: ScrollDirection::Up,
                speed,
                column: col,
                row,
            })
        } else if row >= bottom_margin {
            let lower_edge = surface.rect.y + surface.rect.height.saturating_sub(1);
            let speed = if row >= lower_edge { 3 } else { 1 };
            Some(DragAutoScroll {
                pane,
                direction: ScrollDirection::Down,
                speed,
                column: col,
                row,
            })
        } else {
            None
        };

        self.drag_autoscroll = next;
    }

    fn maybe_autoscroll_selection(&mut self) {
        let Some(auto) = self.drag_autoscroll else {
            return;
        };
        if self.drag_selection != Some(auto.pane) {
            self.drag_autoscroll = None;
            return;
        }

        let Some(surface) = self.selection_surface(auto.pane).cloned() else {
            self.drag_autoscroll = None;
            return;
        };

        let changed = match (auto.pane, auto.direction) {
            (SelectablePane::Chat, ScrollDirection::Up) => {
                let before = self.scroll_offset;
                self.scroll_chat_up(auto.speed);
                self.scroll_offset != before
            }
            (SelectablePane::Chat, ScrollDirection::Down) => {
                let before = self.scroll_offset;
                self.scroll_chat_down(auto.speed);
                self.scroll_offset != before
            }
            (SelectablePane::SidebarDetail, ScrollDirection::Up) => {
                let before = self.sidebar.detail_scroll;
                self.sidebar.scroll_detail_up(auto.speed);
                self.sidebar.detail_scroll != before
            }
            (SelectablePane::SidebarDetail, ScrollDirection::Down) => {
                let before = self.sidebar.detail_scroll;
                self.sidebar.scroll_detail_down(auto.speed);
                self.sidebar.detail_scroll != before
            }
        };

        if !changed {
            return;
        }

        if let Some(selection) = self.selection.as_mut() {
            if selection.pane == auto.pane {
                selection.focus = surface.pos_from_screen_clamped(auto.column, auto.row);
                self.needs_redraw = true;
            }
        }
    }

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        self.needs_redraw = true;

        // Session picker intercepts scroll events
        if matches!(self.mode, UiMode::SessionPicker(_)) {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if let UiMode::SessionPicker(ref mut state) = self.mode {
                        state.move_up();
                    }
                }
                MouseEventKind::ScrollDown => {
                    if let UiMode::SessionPicker(ref mut state) = self.mode {
                        state.move_down();
                    }
                }
                _ => {}
            }
            return;
        }

        let col = mouse.column;
        let row = mouse.row;

        let is_stream = self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Stream;
        let in_list = point_in_rect(col, row, self.sidebar_list_rect);
        let in_detail = point_in_rect(col, row, self.sidebar_detail_rect);
        let in_sidebar = in_list || in_detail;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if in_list {
                    self.active_pane = Pane::SidebarList;
                    self.sidebar
                        .scroll_list_up(self.config.ui.mouse_scroll_lines);
                } else if in_detail {
                    self.active_pane = Pane::SidebarDetail;
                    self.sidebar
                        .scroll_detail_up(self.config.ui.mouse_scroll_lines);
                } else if in_sidebar && is_stream {
                    self.active_pane = Pane::SidebarDetail;
                    self.sidebar
                        .scroll_detail_up(self.config.ui.mouse_scroll_lines);
                } else {
                    self.active_pane = Pane::Chat;
                    self.scroll_chat_up(self.config.ui.mouse_scroll_lines);
                }
            }
            MouseEventKind::ScrollDown => {
                if in_list {
                    self.active_pane = Pane::SidebarList;
                    self.sidebar
                        .scroll_list_down(self.config.ui.mouse_scroll_lines);
                } else if in_detail {
                    self.active_pane = Pane::SidebarDetail;
                    self.sidebar
                        .scroll_detail_down(self.config.ui.mouse_scroll_lines);
                } else if in_sidebar && is_stream {
                    self.active_pane = Pane::SidebarDetail;
                    self.sidebar
                        .scroll_detail_down(self.config.ui.mouse_scroll_lines);
                } else {
                    self.active_pane = Pane::Chat;
                    self.scroll_chat_down(self.config.ui.mouse_scroll_lines);
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                if in_list {
                    self.clear_selection();
                    self.active_pane = Pane::SidebarList;
                    if let Some(lr) = self.sidebar_list_rect {
                        let clicked_row = (row - lr.y) as usize;
                        let clicked_idx = self.sidebar.list_scroll + clicked_row;
                        let total = self.total_tool_calls();
                        if clicked_idx < total {
                            self.focus_tool(clicked_idx);
                        }
                    }
                    return;
                }

                if in_detail || (in_sidebar && is_stream) {
                    self.active_pane = Pane::SidebarDetail;
                    if let Some(surface) = self.sidebar_detail_surface.as_ref().cloned() {
                        if !surface.is_empty() {
                            let pos = surface.pos_from_screen_clamped(col, row);
                            self.selection =
                                Some(SelectionState::new(SelectablePane::SidebarDetail, pos, pos));
                            self.drag_selection = Some(SelectablePane::SidebarDetail);
                            self.set_drag_autoscroll(
                                SelectablePane::SidebarDetail,
                                &surface,
                                col,
                                row,
                            );
                        }
                    }
                    return;
                }

                self.active_pane = Pane::Chat;
                if let Some(surface) = self.chat_surface.as_ref().cloned() {
                    if !surface.is_empty() {
                        let pos = surface.pos_from_screen_clamped(col, row);
                        self.selection = Some(SelectionState::new(SelectablePane::Chat, pos, pos));
                        self.drag_selection = Some(SelectablePane::Chat);
                        self.set_drag_autoscroll(SelectablePane::Chat, &surface, col, row);
                    }
                }
            }
            MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                let Some(pane) = self.drag_selection else {
                    return;
                };
                let Some(surface) = self.selection_surface(pane).cloned() else {
                    return;
                };
                let pos = surface.pos_from_screen_clamped(col, row);
                if let Some(selection) = self.selection.as_mut() {
                    if selection.pane == pane {
                        selection.focus = pos;
                    }
                }
                self.set_drag_autoscroll(pane, &surface, col, row);
                match pane {
                    SelectablePane::Chat => {
                        self.active_pane = Pane::Chat;
                    }
                    SelectablePane::SidebarDetail => {
                        self.active_pane = Pane::SidebarDetail;
                    }
                }
            }
            MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                self.drag_selection = None;
                self.drag_autoscroll = None;
            }
            _ => {}
        }
    }

    fn handle_cancel(&mut self) {
        if !self.editor.is_empty() {
            // First Ctrl+C: clear editor
            self.editor.clear();
            self.ctrl_c_count = 0;
        } else if self.is_streaming {
            // Second: abort streaming
            if let Some(ref handle) = self.agent_handle {
                let _ = handle.command_tx.try_send(AgentCommand::Cancel);
            }
            self.is_streaming = false;
            self.ctrl_c_count = 0;
        } else {
            // Third: quit
            self.ctrl_c_count += 1;
            if self.ctrl_c_count >= 2 {
                self.running = false;
            }
        }
    }

    // ── Commands ────────────────────────────────────────────────

    fn spawn_agent_for_prompt(&mut self, prompt: &str) -> Result<(), String> {
        let auth_path = Config::user_config_dir().join("auth.json");
        let mut auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

        let mut meta = self
            .model_registry
            .resolve_meta(&self.model_name, None)
            .ok_or_else(|| format!("Unknown model: {}", self.model_name))?;

        let mut provider_name = meta.provider.clone();
        if should_use_chatgpt_provider(&auth_store, &self.model_registry, &meta) {
            provider_name = "openai-codex".to_string();
            meta = self
                .model_registry
                .resolve_meta(&self.model_name, Some(&provider_name))
                .ok_or_else(|| format!("Unknown model: {}", self.model_name))?;
        }

        let provider = create_provider(&provider_name)
            .ok_or_else(|| format!("Unknown provider: {provider_name}"))?;

        // Resolve API key with auto-refresh for expired OAuth tokens
        let api_key = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(resolve_provider_api_key(&mut auth_store, &provider_name))
        })
        .map_err(|e: imp_llm::Error| e.to_string())?;

        let model = Model {
            meta,
            provider: Arc::from(provider),
        };

        // Override thinking level from the TUI's current selection.
        let mut config = self.config.clone();
        config.thinking = Some(self.thinking_level);

        let lua_cwd = self.cwd.clone();
        let user_config_dir = imp_core::config::Config::user_config_dir();
        let (mut agent, handle) = AgentBuilder::new(config, self.cwd.clone(), model, api_key)
            .lua_tool_loader(move |tools| {
                imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools);
            })
            .build()
            .map_err(|e: imp_core::error::Error| e.to_string())?;

        // Wire TuiInterface so the ask tool works
        let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(16);
        agent.ui = crate::tui_interface::TuiInterface::new(ui_tx);
        self.ui_rx = Some(ui_rx);

        // Apply max_turns override from CLI
        if let Some(max_turns) = self.max_turns_override {
            agent.max_turns = max_turns;
        }

        let mut messages: Vec<Message> = self.session.get_active_messages();
        if matches!(
            messages.last(),
            Some(Message::User(user))
                if matches!(
                    user.content.as_slice(),
                    [imp_llm::ContentBlock::Text { text }] if text == prompt
                )
        ) {
            messages.pop();
        }
        // Collect tool_result IDs to know which tool_calls are paired (used by sanitize below)
        let _result_ids: std::collections::HashSet<String> = messages
            .iter()
            .filter_map(|m| match m {
                Message::ToolResult(tr) => Some(tr.tool_call_id.clone()),
                _ => None,
            })
            .collect();

        // Sanitize: strip unpaired tool_calls and orphaned tool_results
        imp_core::session::sanitize_messages(&mut messages);
        agent.messages = messages;

        let prompt = prompt.to_string();
        let task = tokio::spawn(async move { agent.run(prompt).await });

        self.agent_handle = Some(handle);
        self.agent_task = Some(task);
        Ok(())
    }

    fn send_message(&mut self) {
        let text = self.editor.content().to_string();
        if text.trim().is_empty() {
            return;
        }

        // Check for slash commands
        if let Some(cmd_text) = text.strip_prefix('/') {
            let typed = cmd_text.trim();
            // Resolve prefix: exact match first, then unique prefix match
            let commands = builtin_commands();
            let cmd = commands
                .iter()
                .find(|c| c.name == typed)
                .or_else(|| commands.iter().find(|c| c.name.starts_with(typed)))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| typed.to_string());
            self.execute_command(&cmd);
            self.editor.push_history();
            self.editor.clear();
            return;
        }

        // Add user message to display
        self.messages.push(DisplayMessage {
            role: MessageRole::User,
            content: text.clone(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });
        self.invalidate_chat_render_cache();

        // Persist to session
        let msg_id = uuid::Uuid::new_v4().to_string();
        let _ = self.session.append(SessionEntry::Message {
            id: msg_id,
            parent_id: None,
            message: imp_llm::Message::user(&text),
        });

        // Add streaming placeholder for assistant response
        self.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: true,
            timestamp: imp_llm::now(),
        });
        self.invalidate_chat_render_cache();

        self.is_streaming = true;
        self.auto_scroll = true;
        self.scroll_offset = 0;
        self.tool_focus = None;
        self.editor.push_history();
        self.editor.clear();

        if let Err(error) = self.spawn_agent_for_prompt(&text) {
            self.is_streaming = false;
            self.messages.pop();
            self.messages.push(DisplayMessage {
                role: MessageRole::Error,
                content: error,
                thinking: None,
                tool_calls: Vec::new(),
                assistant_blocks: Vec::new(),
                is_streaming: false,
                timestamp: imp_llm::now(),
            });
            self.invalidate_chat_render_cache();
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd.split_whitespace().next().unwrap_or("") {
            "quit" | "q" => {
                self.running = false;
            }
            "model" => {
                self.open_model_selector();
            }
            "tree" => {
                self.open_tree_view();
            }
            "new" => {
                self.messages.clear();
                self.invalidate_chat_render_cache();
                self.session = SessionManager::in_memory();
                self.tool_focus = None;
            }
            "compact" => {
                self.run_manual_compaction();
            }
            "hotkeys" => {
                self.push_system_msg(
                    "Keyboard shortcuts:\n\
  Enter         Send message\n\
  Shift+Enter   New line\n\
  Alt+Enter     Queue follow-up while streaming\n\
  Ctrl+C        Clear / Abort / Quit\n\
  Ctrl+C/Cmd+C  Copy selection\n\
  Ctrl+V/Cmd+V  Paste clipboard\n\
  Ctrl+L        Model selector\n\
  Ctrl+P        Next chosen model\n\
  Ctrl+Shift+P  Previous chosen model\n\
  Tab           Show/hide sidebar\n\
  Ctrl+O        Toggle tool output\n\
  Ctrl+Up/Down  Focus previous/next tool\n\
  Shift+Tab     Cycle thinking level\n\
  @             File finder\n\
  /command      Slash commands\n\
  PageUp/Down   Scroll",
                );
            }
            "settings" => {
                self.open_settings();
            }
            "personality" => {
                self.open_personality();
            }
            "resume" => {
                let session_dir = Config::session_dir();
                match SessionManager::list(&session_dir) {
                    Ok(sessions) if !sessions.is_empty() => {
                        let state = SessionPickerState::new(sessions, Some(&self.cwd));
                        if state.filtered_indices.is_empty() {
                            self.messages.push(DisplayMessage {
                                role: MessageRole::System,
                                content: "No saved sessions found.".into(),
                                thinking: None,
                                tool_calls: Vec::new(),
                                assistant_blocks: Vec::new(),
                                is_streaming: false,
                                timestamp: imp_llm::now(),
                            });
                        } else {
                            self.mode = UiMode::SessionPicker(state);
                        }
                    }
                    Ok(_) => {
                        self.messages.push(DisplayMessage {
                            role: MessageRole::System,
                            content: "No saved sessions found.".into(),
                            thinking: None,
                            tool_calls: Vec::new(),
                            assistant_blocks: Vec::new(),
                            is_streaming: false,
                            timestamp: imp_llm::now(),
                        });
                    }
                    Err(e) => {
                        self.messages.push(DisplayMessage {
                            role: MessageRole::Error,
                            content: format!("Failed to list sessions: {e}"),
                            thinking: None,
                            tool_calls: Vec::new(),
                            assistant_blocks: Vec::new(),
                            is_streaming: false,
                            timestamp: imp_llm::now(),
                        });
                    }
                }
            }
            "session" => {
                self.push_system_msg("/session is defunct. Use /resume to browse/search sessions.");
            }
            "name" => {
                let new_name = cmd.strip_prefix("name").unwrap_or("").trim();
                if new_name.is_empty() {
                    self.push_system_msg("Usage: /name <session name>");
                } else {
                    self.session.set_name(new_name);
                    self.push_system_msg(&format!("Session renamed to: {new_name}"));
                }
            }
            "export" => {
                let dest = cmd.strip_prefix("export").unwrap_or("").trim();
                let path = if dest.is_empty() {
                    let name = self.session.name().unwrap_or("conversation");
                    std::path::PathBuf::from(format!("{name}.md"))
                } else {
                    std::path::PathBuf::from(dest)
                };
                match self.export_conversation(&path) {
                    Ok(_) => self.push_system_msg(&format!("Exported to {}", path.display())),
                    Err(e) => self.push_system_msg(&format!("Export failed: {e}")),
                }
            }
            "reload" => {
                match imp_core::config::Config::resolve(
                    &imp_core::config::Config::user_config_dir(),
                    Some(&self.cwd),
                ) {
                    Ok(new_config) => {
                        self.config = new_config;
                        // Reload Lua extensions
                        self.reload_lua_extensions();
                        self.push_system_msg("Config and Lua extensions reloaded.");
                    }
                    Err(e) => self.push_system_msg(&format!("Reload failed: {e}")),
                }
            }
            "fork" => {
                let leaf = self.session.leaf_id().unwrap_or_default().to_string();
                let path = Config::session_dir().join(format!("{}.jsonl", uuid::Uuid::new_v4()));
                match self.session.fork(&leaf, &path) {
                    Ok(forked) => {
                        self.session = forked;
                        self.push_system_msg("Forked. You're on a new branch.");
                    }
                    Err(e) => self.push_system_msg(&format!("Fork failed: {e}")),
                }
            }
            "memory" | "mem" => {
                self.handle_memory_command(cmd);
            }
            "help" => {
                self.push_system_msg(concat!(
                    "Commands:\n",
                    "  /new        — start fresh session\n",
                    "  /model      — switch model\n",
                    "  /compact    — compress context\n",
                    "  /resume     — resume/search sessions\n",
                    "  /session    — legacy alias (defunct)\n",
                    "  /fork       — branch conversation\n",
                    "  /name <n>   — rename session\n",
                    "  /export [f] — export to markdown\n",
                    "  /copy       — copy selection or last response\n",
                    "  /memory     — view/edit agent memory\n",
                    "  /reload     — reload config\n",
                    "  /settings   — edit settings\n",
                    "  /personality — customize imp personality\n",
                    "  /login [provider]   — OAuth login (Anthropic/OpenAI)\n",
                    "  /secrets [provider] — save/list API keys & service secrets\n",
                    "  /help       — this message\n",
                    "  /quit       — exit",
                ));
            }
            "login" => {
                if let Some(provider) = cmd.split_whitespace().nth(1) {
                    self.start_login(provider);
                } else {
                    self.open_login_picker();
                }
            }
            "secrets" => {
                if let Some(provider) = cmd.split_whitespace().nth(1) {
                    self.start_secrets_flow(provider);
                } else {
                    self.open_secrets_picker();
                }
            }
            "welcome" | "setup" => {
                let all_models = self.model_registry.list().to_vec();
                self.mode = UiMode::Welcome(WelcomeState::new(&all_models));
            }
            "copy" => {
                if self.copy_selection() {
                    return;
                }
                // Copy last assistant message to clipboard
                if let Some(last) = self
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant || m.role == MessageRole::Error)
                {
                    let text = last.content.clone();
                    self.copy_to_clipboard(&text);
                    self.messages.push(DisplayMessage {
                        role: MessageRole::System,
                        content: "Copied to clipboard.".into(),
                        thinking: None,
                        tool_calls: Vec::new(),
                        assistant_blocks: Vec::new(),
                        is_streaming: false,
                        timestamp: imp_llm::now(),
                    });
                }
            }
            _ => {
                // Try Lua extension commands before reporting unknown
                if !self.try_lua_command(cmd) {
                    self.messages.push(DisplayMessage {
                        role: MessageRole::Error,
                        content: format!("Unknown command: /{cmd}"),
                        thinking: None,
                        tool_calls: Vec::new(),
                        assistant_blocks: Vec::new(),
                        is_streaming: false,
                        timestamp: imp_llm::now(),
                    });
                }
            }
        }
        self.editor.clear();
    }

    /// Handle `/memory` subcommands.
    ///
    /// - `/memory`           — show both stores
    /// - `/memory add <t>`   — add entry to memory.md
    /// - `/memory user <t>`  — add entry to user.md
    /// - `/memory remove <t>` — remove matching entry from memory.md
    /// - `/memory remove user <t>` — remove matching entry from user.md
    /// - `/memory clear`     — wipe memory.md
    /// - `/memory clear user` — wipe user.md
    fn handle_memory_command(&mut self, cmd: &str) {
        use imp_core::memory::MemoryStore;

        let config_dir = Config::user_config_dir();
        let mem_path = config_dir.join("memory.md");
        let user_path = config_dir.join("user.md");
        let mem_limit = self.config.learning.memory_char_limit;
        let user_limit = self.config.learning.user_char_limit;

        // Strip the command name prefix ("memory" or "mem") to get arguments
        let rest = cmd
            .strip_prefix("memory")
            .or_else(|| cmd.strip_prefix("mem"))
            .unwrap_or("")
            .trim();

        if rest.is_empty() {
            // Show both stores
            let mut output = String::new();

            match MemoryStore::load(&mem_path, mem_limit) {
                Ok(store) => {
                    let (used, limit) = store.usage();
                    output.push_str(&format!("Memory ({used}/{limit} chars):\n"));
                    if store.entries().is_empty() {
                        output.push_str("  (empty)\n");
                    } else {
                        for (i, entry) in store.entries().iter().enumerate() {
                            output.push_str(&format!("  {}. {}\n", i + 1, entry));
                        }
                    }
                }
                Err(e) => output.push_str(&format!("Error loading memory.md: {e}\n")),
            }

            output.push('\n');

            match MemoryStore::load(&user_path, user_limit) {
                Ok(store) => {
                    let (used, limit) = store.usage();
                    output.push_str(&format!("User profile ({used}/{limit} chars):\n"));
                    if store.entries().is_empty() {
                        output.push_str("  (empty)\n");
                    } else {
                        for (i, entry) in store.entries().iter().enumerate() {
                            output.push_str(&format!("  {}. {}\n", i + 1, entry));
                        }
                    }
                }
                Err(e) => output.push_str(&format!("Error loading user.md: {e}\n")),
            }

            if !self.config.learning.enabled {
                output.push_str("\n⚠ Learning is disabled in config. Memory won't be loaded into the system prompt.");
            }

            self.push_system_msg(output.trim_end());
            return;
        }

        let mut words = rest.splitn(2, char::is_whitespace);
        let sub = words.next().unwrap_or("");
        let arg = words.next().unwrap_or("").trim();

        match sub {
            "add" => {
                if arg.is_empty() {
                    self.push_system_msg("Usage: /memory add <text>");
                    return;
                }
                match MemoryStore::load(&mem_path, mem_limit) {
                    Ok(mut store) => match store.add(arg) {
                        Ok(result) => {
                            self.push_system_msg(&format!("{} [{}]", result.message, result.usage))
                        }
                        Err(e) => self.push_system_msg(&format!("Error: {e}")),
                    },
                    Err(e) => self.push_system_msg(&format!("Error: {e}")),
                }
            }
            "user" => {
                if arg.is_empty() {
                    self.push_system_msg("Usage: /memory user <text>");
                    return;
                }
                match MemoryStore::load(&user_path, user_limit) {
                    Ok(mut store) => match store.add(arg) {
                        Ok(result) => {
                            self.push_system_msg(&format!("{} [{}]", result.message, result.usage))
                        }
                        Err(e) => self.push_system_msg(&format!("Error: {e}")),
                    },
                    Err(e) => self.push_system_msg(&format!("Error: {e}")),
                }
            }
            "remove" | "rm" => {
                if arg.is_empty() {
                    self.push_system_msg("Usage: /memory remove <text>");
                    return;
                }
                // Check if removing from user store: "/memory remove user <text>"
                if let Some(user_arg) = arg.strip_prefix("user ").map(|s| s.trim()) {
                    if user_arg.is_empty() {
                        self.push_system_msg("Usage: /memory remove user <text>");
                        return;
                    }
                    match MemoryStore::load(&user_path, user_limit) {
                        Ok(mut store) => match store.remove(user_arg) {
                            Ok(result) => self
                                .push_system_msg(&format!("{} [{}]", result.message, result.usage)),
                            Err(e) => self.push_system_msg(&format!("Error: {e}")),
                        },
                        Err(e) => self.push_system_msg(&format!("Error: {e}")),
                    }
                } else {
                    match MemoryStore::load(&mem_path, mem_limit) {
                        Ok(mut store) => match store.remove(arg) {
                            Ok(result) => self
                                .push_system_msg(&format!("{} [{}]", result.message, result.usage)),
                            Err(e) => self.push_system_msg(&format!("Error: {e}")),
                        },
                        Err(e) => self.push_system_msg(&format!("Error: {e}")),
                    }
                }
            }
            "replace" => {
                // "/memory replace <old> -> <new>"
                if let Some((old, new)) = arg.split_once("->") {
                    let old = old.trim();
                    let new = new.trim();
                    if old.is_empty() || new.is_empty() {
                        self.push_system_msg("Usage: /memory replace <old text> -> <new text>");
                        return;
                    }
                    match MemoryStore::load(&mem_path, mem_limit) {
                        Ok(mut store) => match store.replace(old, new) {
                            Ok(result) => self
                                .push_system_msg(&format!("{} [{}]", result.message, result.usage)),
                            Err(e) => self.push_system_msg(&format!("Error: {e}")),
                        },
                        Err(e) => self.push_system_msg(&format!("Error: {e}")),
                    }
                } else {
                    self.push_system_msg("Usage: /memory replace <old text> -> <new text>");
                }
            }
            "clear" => {
                let target = arg;
                if target == "user" {
                    if user_path.exists() {
                        match std::fs::write(&user_path, "") {
                            Ok(_) => self.push_system_msg("User profile cleared."),
                            Err(e) => self.push_system_msg(&format!("Error: {e}")),
                        }
                    } else {
                        self.push_system_msg("User profile is already empty.");
                    }
                } else if target.is_empty() {
                    if mem_path.exists() {
                        match std::fs::write(&mem_path, "") {
                            Ok(_) => self.push_system_msg("Memory cleared."),
                            Err(e) => self.push_system_msg(&format!("Error: {e}")),
                        }
                    } else {
                        self.push_system_msg("Memory is already empty.");
                    }
                } else {
                    self.push_system_msg("Usage: /memory clear [user]");
                }
            }
            "help" => {
                self.push_system_msg(concat!(
                    "Memory commands:\n",
                    "  /memory              — show all entries\n",
                    "  /memory add <text>   — add to memory\n",
                    "  /memory user <text>  — add to user profile\n",
                    "  /memory remove <text>  — remove from memory\n",
                    "  /memory remove user <text> — remove from user profile\n",
                    "  /memory replace <old> -> <new> — replace entry\n",
                    "  /memory clear        — clear memory\n",
                    "  /memory clear user   — clear user profile",
                ));
            }
            _ => {
                self.push_system_msg(&format!(
                    "Unknown memory subcommand: {sub}\nUse /memory help for usage."
                ));
            }
        }
    }

    /// Reload Lua extensions: re-scan directories, re-create runtime, and update
    /// the stored runtime handle. Tools are not re-registered on the running
    /// agent (only new agents will pick them up), but commands become available
    /// immediately.
    fn reload_lua_extensions(&mut self) {
        let user_config_dir = Config::user_config_dir();
        match imp_lua::reload(&user_config_dir, Some(&self.cwd)) {
            Ok((rt, _exts)) => {
                self.lua_runtime = Some(Arc::new(Mutex::new(rt)));
            }
            Err(e) => {
                self.push_system_msg(&format!("Lua reload failed: {e}"));
                self.lua_runtime = None;
            }
        }
    }

    /// Try to dispatch a slash command to a Lua extension handler.
    /// Returns `true` if a matching Lua command was found and executed.
    fn try_lua_command(&mut self, cmd: &str) -> bool {
        let runtime = match &self.lua_runtime {
            Some(rt) => Arc::clone(rt),
            None => return false,
        };

        let guard = match runtime.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        // Find a command matching the typed name (first word)
        let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
        let args = cmd.strip_prefix(cmd_name).unwrap_or("").trim();

        if !guard.has_command(cmd_name) {
            return false;
        }

        // Execute via LuaRuntime's helper (keeps mlua types internal)
        let result = guard.execute_command(cmd_name, args);
        drop(guard);

        match result {
            Ok(Some(text)) => self.push_system_msg(&text),
            Ok(None) => {} // Command executed silently
            Err(e) => self.push_system_msg(&format!("Lua command error: {e}")),
        }
        true
    }

    fn start_secrets_flow(&mut self, provider: &str) {
        self.mode = UiMode::Normal;
        self.secrets_flow = Some(SecretsFlowState::AwaitingFieldNames {
            provider: provider.to_string(),
        });
        let (tx, _rx) = tokio::sync::oneshot::channel();
        self.begin_ask(
            crate::views::ask_bar::AskState::new(
                format!(
                    "{}\n\nField names (comma-separated) [api_key]:",
                    prompt_text_for_secret_provider(provider)
                ),
                String::new(),
                vec![],
                false,
            ),
            AskReply::Input(tx),
        );
    }

    fn start_login(&mut self, provider: &str) {
        if !oauth_provider(provider) {
            self.push_error_msg(&format!(
                "/login {provider} is OAuth-only. Use /secrets {provider} for API keys/secrets."
            ));
            return;
        }

        let status_message = match provider {
            "anthropic" => "Opening browser for Anthropic login...",
            "openai" | "openai-codex" => "Opening browser for OpenAI / ChatGPT login...",
            _ => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: format!(
                        "OAuth login for '{provider}' not supported. Use /secrets {provider} for API keys."
                    ),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
                return;
            }
        };

        self.mode = UiMode::Normal;
        self.push_system_msg(status_message);

        let auth_path = Config::user_config_dir().join("auth.json");
        let provider = provider.to_string();
        let task = tokio::spawn(async move {
            let login_result = match provider.as_str() {
                "anthropic" => {
                    imp_llm::oauth::anthropic::AnthropicOAuth::new()
                        .login(
                            |url| {
                                open_url(url);
                            },
                            || async { None },
                        )
                        .await
                }
                "openai" | "openai-codex" => {
                    imp_llm::oauth::chatgpt::ChatGptOAuth::new()
                        .login(
                            |url| {
                                open_url(url);
                            },
                            || async { None },
                        )
                        .await
                }
                _ => unreachable!(),
            };

            match login_result {
                Ok(credential) => {
                    let success_message = imp_llm::auth::oauth_display_info_for_credential(
                        provider.as_str(),
                        &credential,
                    )
                    .map(|info| info.login_message(provider.as_str()))
                    .unwrap_or_else(|| format!("Logged in to {} successfully.", provider));

                    let mut store = AuthStore::load(&auth_path)
                        .unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
                    match provider.as_str() {
                        "anthropic" => {
                            let _ = store.store(
                                "anthropic",
                                imp_llm::auth::StoredCredential::OAuth(credential),
                            );
                        }
                        "openai" | "openai-codex" => {
                            let _ = store.store(
                                "openai",
                                imp_llm::auth::StoredCredential::OAuth(credential.clone()),
                            );
                            let _ = store.store(
                                "openai-codex",
                                imp_llm::auth::StoredCredential::OAuth(credential),
                            );
                        }
                        _ => {}
                    }
                    LoginTaskExit::Success(success_message)
                }
                Err(e) => LoginTaskExit::Failed(format!("OAuth login failed: {e}")),
            }
        });
        self.login_task = Some(task);
    }

    fn open_secrets_picker(&mut self) {
        let auth_path = Config::user_config_dir().join("auth.json");
        let auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
        let providers = secret_providers(&ProviderRegistry::with_builtins())
            .into_iter()
            .map(|mut provider| {
                provider.configured = provider_logged_in(&auth_store, &provider.id);
                provider
            })
            .collect();
        self.mode = UiMode::SecretsPicker(SecretsPickerState::new(providers));
    }

    fn open_login_picker(&mut self) {
        let auth_path = Config::user_config_dir().join("auth.json");
        let auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
        let providers = login_providers(&ProviderRegistry::with_builtins())
            .into_iter()
            .filter(|provider| oauth_provider(provider.id))
            .map(|mut provider| {
                provider.logged_in = provider_logged_in(&auth_store, provider.id);
                provider
            })
            .collect();
        self.mode = UiMode::LoginPicker(LoginPickerState::new(providers));
    }

    fn open_settings(&mut self) {
        let models = self.filtered_models();
        let auth_path = Config::user_config_dir().join("auth.json");
        let auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
        let state = SettingsState::new(&self.config, &self.model_name, &models, &auth_store);
        self.mode = UiMode::Settings(state);
    }

    fn open_personality(&mut self) {
        let project_config = Config::load(&self.cwd.join(".imp").join("config.toml")).ok();
        let scope = if project_config.is_some() {
            PersonalityScope::Project
        } else {
            PersonalityScope::Global
        };
        let state = PersonalityState::new(
            &self.config.personality,
            project_config.as_ref().map(|c| &c.personality),
            scope,
        );
        self.mode = UiMode::Personality(state);
    }

    fn handle_session_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let UiMode::SessionPicker(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let UiMode::SessionPicker(ref mut state) = self.mode {
                    state.move_down();
                }
            }
            KeyCode::Backspace => {
                if let UiMode::SessionPicker(ref mut state) = self.mode {
                    state.pop_filter();
                }
            }
            KeyCode::Char(c) if !c.is_control() => {
                if let UiMode::SessionPicker(ref mut state) = self.mode {
                    state.push_filter(c);
                }
            }
            KeyCode::Enter => {
                let selected_path = if let UiMode::SessionPicker(ref state) = self.mode {
                    state.selected_session().map(|s| s.path.clone())
                } else {
                    None
                };
                self.mode = UiMode::Normal;
                if let Some(path) = selected_path {
                    match SessionManager::open(&path) {
                        Ok(session) => {
                            self.session = session;
                            self.load_session_messages();
                            if let Some(summary) = self.session.summary() {
                                self.messages.push(DisplayMessage {
                                    role: MessageRole::System,
                                    content: format!("Session resumed — {}", summary),
                                    thinking: None,
                                    tool_calls: Vec::new(),
                                    assistant_blocks: Vec::new(),
                                    is_streaming: false,
                                    timestamp: imp_llm::now(),
                                });
                            } else {
                                self.messages.push(DisplayMessage {
                                    role: MessageRole::System,
                                    content: "Session resumed.".into(),
                                    thinking: None,
                                    tool_calls: Vec::new(),
                                    assistant_blocks: Vec::new(),
                                    is_streaming: false,
                                    timestamp: imp_llm::now(),
                                });
                            }
                        }
                        Err(e) => {
                            self.messages.push(DisplayMessage {
                                role: MessageRole::Error,
                                content: format!("Failed to open session: {e}"),
                                thinking: None,
                                tool_calls: Vec::new(),
                                assistant_blocks: Vec::new(),
                                is_streaming: false,
                                timestamp: imp_llm::now(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_ask_key(&mut self, key: KeyEvent) {
        if self.is_paste_shortcut(key) {
            self.paste_from_clipboard();
            return;
        }

        let Some(state) = self.ask_state.as_ref() else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.cancel_ask();
            }
            KeyCode::Enter => {
                self.sync_ask_from_editor();
                self.finish_ask();
            }
            KeyCode::Tab => {
                let replacement = if !state.options.is_empty() && !state.input_active {
                    Some(state.options[state.cursor].label.clone())
                } else {
                    None
                };
                if let Some(text) = replacement {
                    self.editor.set_content(&text);
                    self.sync_ask_from_editor();
                }
            }
            KeyCode::Char(' ') if !state.input_active => {
                if let Some(state) = self.ask_state.as_mut() {
                    state.toggle_current();
                }
            }
            KeyCode::Char(c) if !state.input_active && c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                let quick_selected = if let Some(state) = self.ask_state.as_mut() {
                    state.quick_select(n)
                } else {
                    false
                };
                if quick_selected {
                    self.finish_ask();
                }
            }
            KeyCode::Up => {
                if let Some(state) = self.ask_state.as_mut() {
                    if state.input_active {
                        if !self.editor.move_up() {
                            self.editor.move_home();
                        }
                        self.sync_ask_from_editor();
                    } else {
                        state.cursor_up();
                    }
                }
            }
            KeyCode::Down => {
                if let Some(state) = self.ask_state.as_mut() {
                    if state.input_active {
                        if !self.editor.move_down() {
                            self.editor.move_end();
                        }
                        self.sync_ask_from_editor();
                    } else {
                        state.cursor_down();
                    }
                }
            }
            _ => {
                if let Some(action) = keybindings::resolve_normal(key) {
                    match action {
                        Action::InsertChar(c) => self.editor.insert_char(c),
                        Action::Backspace => self.editor.delete_back(),
                        Action::Delete => self.editor.delete_forward(),
                        Action::CursorLeft => self.editor.move_left(),
                        Action::CursorRight => self.editor.move_right(),
                        Action::CursorHome => self.editor.move_home(),
                        Action::CursorEnd => self.editor.move_end(),
                        Action::WordLeft => self.editor.move_word_left(),
                        Action::WordRight => self.editor.move_word_right(),
                        Action::DeleteWordBack => self.editor.delete_word_back(),
                        Action::DeleteToStart => self.editor.delete_to_start(),
                        Action::DeleteToEnd => self.editor.delete_to_end(),
                        Action::NewLine => self.editor.insert_newline(),
                        _ => {}
                    }
                    self.sync_ask_from_editor();
                }
            }
        }
    }

    fn finish_ask(&mut self) {
        use crate::views::ask_bar::AskResult;

        self.sync_ask_from_editor();
        let state = self.ask_state.take();
        let reply = self.ask_reply.take();

        let Some(state) = state else { return };
        let result = state.confirm();
        self.restore_editor_after_ask();

        // Show Q&A in chat
        self.push_system_msg(&format!("❯ {}", state.question));

        match (&result, reply) {
            (AskResult::Text(text), Some(AskReply::Input(tx))) => {
                self.push_system_msg(&format!("  {text}"));
                let _ = tx.send(Some(text.clone()));
                self.advance_secrets_flow(Some(text.clone()));
            }
            (AskResult::Selected(indices), Some(AskReply::Select(tx))) => {
                let labels: Vec<String> = indices
                    .iter()
                    .filter_map(|&i| state.options.get(i).map(|o| o.label.clone()))
                    .collect();
                self.push_system_msg(&format!("  {}", labels.join(", ")));
                // Send first selected index for single select
                let _ = tx.send(indices.first().copied());
            }
            (AskResult::Text(text), Some(AskReply::Select(tx))) => {
                // User typed custom text on a Select ask.
                // Find if the text matches an option label (case-insensitive).
                let match_idx = state
                    .options
                    .iter()
                    .position(|o| o.label.eq_ignore_ascii_case(text));
                if let Some(idx) = match_idx {
                    self.push_system_msg(&format!("  {}", state.options[idx].label));
                    let _ = tx.send(Some(idx));
                } else {
                    // No match — send None. The ask tool will get "User cancelled".
                    // The custom text is shown in chat so the user knows what happened.
                    self.push_system_msg(&format!("  {text}"));
                    let _ = tx.send(None);
                }
            }
            _ => {}
        }
    }

    fn advance_secrets_flow(&mut self, input: Option<String>) {
        let Some(flow) = self.secrets_flow.take() else {
            return;
        };

        match flow {
            SecretsFlowState::AwaitingFieldNames { provider } => {
                let field_names = parse_secret_field_names(input.as_deref().unwrap_or(""));
                let first_field = field_names
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "api_key".into());
                self.secrets_flow = Some(SecretsFlowState::AwaitingFieldValues {
                    provider,
                    fields: field_names,
                    current: 0,
                    values: HashMap::new(),
                });
                let (tx, _rx) = tokio::sync::oneshot::channel();
                self.begin_ask(
                    crate::views::ask_bar::AskState::new(
                        format!("Enter {first_field}:"),
                        String::new(),
                        vec![],
                        false,
                    ),
                    AskReply::Input(tx),
                );
            }
            SecretsFlowState::AwaitingFieldValues {
                provider,
                fields,
                current,
                mut values,
            } => {
                let Some(value) = input.filter(|value| !value.trim().is_empty()) else {
                    self.push_error_msg("Secret entry cancelled.");
                    return;
                };

                let field = fields.get(current).cloned().unwrap_or_else(|| "api_key".into());
                values.insert(field, value.trim().to_string());

                if current + 1 < fields.len() {
                    let next_field = fields[current + 1].clone();
                    self.secrets_flow = Some(SecretsFlowState::AwaitingFieldValues {
                        provider: provider.clone(),
                        fields: fields.clone(),
                        current: current + 1,
                        values,
                    });
                    let (tx, _rx) = tokio::sync::oneshot::channel();
                    self.begin_ask(
                        crate::views::ask_bar::AskState::new(
                            format!("Enter {next_field}:"),
                            String::new(),
                            vec![],
                            false,
                        ),
                        AskReply::Input(tx),
                    );
                    return;
                }

                let auth_path = Config::user_config_dir().join("auth.json");
                let mut auth_store =
                    AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
                match auth_store.store_secret_fields(&provider, values) {
                    Ok(()) => self.push_system_msg(&format!("Saved secure secrets for {provider}.")),
                    Err(e) => self.push_error_msg(&format!("Failed to save secrets for {provider}: {e}")),
                }
            }
        }
    }

    fn cancel_ask(&mut self) {
        self.secrets_flow = None;
        self.ask_state = None;
        self.restore_editor_after_ask();
        if let Some(reply) = self.ask_reply.take() {
            match reply {
                AskReply::Select(tx) => {
                    let _ = tx.send(None);
                }
                AskReply::Input(tx) => {
                    let _ = tx.send(None);
                }
            }
        }
        // Stop the agent — user wants control back
        if let Some(ref handle) = self.agent_handle {
            let _ = handle.command_tx.try_send(AgentCommand::Cancel);
        }
        self.is_streaming = false;
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        use crate::views::settings::SettingsField;
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                // Commit any pending edit, then dismiss
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.commit_edit();
                }
                self.mode = UiMode::Normal;
            }
            KeyCode::Up => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.move_down();
                }
            }
            KeyCode::Left => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.cycle_backward();
                }
            }
            KeyCode::Right => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.cycle_forward();
                }
            }
            KeyCode::Enter => {
                let is_save = matches!(
                    &self.mode,
                    UiMode::Settings(s) if s.current_field() == SettingsField::Save
                );
                if is_save {
                    self.save_settings();
                } else if let UiMode::Settings(ref mut state) = self.mode {
                    state.start_edit();
                }
            }
            KeyCode::Backspace => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.pop_char();
                }
            }
            KeyCode::Char(c) => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.push_char(c);
                }
            }
            _ => {}
        }
    }

    fn handle_personality_key(&mut self, key: KeyEvent) {
        use crate::views::personality::PersonalityField;

        match key.code {
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.move_down();
                }
            }
            KeyCode::Left => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.cycle_backward();
                }
            }
            KeyCode::Right => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.cycle_forward();
                }
            }
            KeyCode::Enter => {
                let (is_save, is_delete) = match &self.mode {
                    UiMode::Personality(s) => (
                        s.current_field() == PersonalityField::Save,
                        s.current_field() == PersonalityField::DeleteProfile,
                    ),
                    _ => (false, false),
                };
                if is_save {
                    self.save_personality();
                } else if is_delete {
                    self.delete_personality_profile();
                } else if let UiMode::Personality(ref mut state) = self.mode {
                    state.cycle_forward();
                }
            }
            KeyCode::Backspace => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.pop_char();
                }
            }
            KeyCode::Char(c) => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.push_char(c);
                }
            }
            _ => {}
        }
    }

    fn handle_welcome_key(&mut self, key: KeyEvent) {
        let step = match &self.mode {
            UiMode::Welcome(s) => s.current_step(),
            _ => return,
        };

        match step {
            WelcomeStep::Welcome => match key.code {
                KeyCode::Enter => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.advance();
                    }
                }
                KeyCode::Esc => {
                    self.mode = UiMode::Normal;
                }
                _ => {}
            },
            WelcomeStep::ProviderAuth => match key.code {
                KeyCode::Up => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.provider_up();
                        let all_models = self.model_registry.list().to_vec();
                        state.update_models(&all_models);
                    }
                }
                KeyCode::Down => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.provider_down();
                        let all_models = self.model_registry.list().to_vec();
                        state.update_models(&all_models);
                    }
                }
                KeyCode::Enter => {
                    let auth_result = if let UiMode::Welcome(ref mut state) = self.mode {
                        state.check_auth_resolved()
                    } else {
                        Ok(())
                    };
                    match auth_result {
                        Ok(()) => {
                            if let UiMode::Welcome(ref mut state) = self.mode {
                                state.advance();
                            }
                        }
                        Err(error) => {
                            self.messages.push(DisplayMessage {
                                role: MessageRole::Error,
                                content: error,
                                thinking: None,
                                tool_calls: Vec::new(),
                                assistant_blocks: Vec::new(),
                                is_streaming: false,
                                timestamp: imp_llm::now(),
                            });
                        }
                    }
                }
                KeyCode::Esc => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.go_back();
                    }
                }
                KeyCode::Backspace => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.pop_key_char();
                    }
                }
                KeyCode::Char(c) => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.push_key_char(c);
                    }
                }
                _ => {}
            },
            WelcomeStep::ModelThinking => match key.code {
                KeyCode::Up => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.model_up();
                    }
                }
                KeyCode::Down => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.model_down();
                    }
                }
                KeyCode::Right => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.cycle_thinking();
                    }
                }
                KeyCode::Left => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.cycle_thinking_back();
                    }
                }
                KeyCode::Enter => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.advance();
                    }
                }
                KeyCode::Esc => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.go_back();
                    }
                }
                _ => {}
            },
            WelcomeStep::WebSearch => match key.code {
                KeyCode::Up => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.web_provider_up();
                    }
                }
                KeyCode::Down => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.web_provider_down();
                    }
                }
                KeyCode::Enter => {
                    let web_result = if let UiMode::Welcome(ref mut state) = self.mode {
                        state.check_web_auth_resolved()
                    } else {
                        Ok(())
                    };
                    match web_result {
                        Ok(()) => {
                            self.finish_welcome();
                        }
                        Err(error) => {
                            self.messages.push(DisplayMessage {
                                role: MessageRole::Error,
                                content: error,
                                thinking: None,
                                tool_calls: Vec::new(),
                                assistant_blocks: Vec::new(),
                                is_streaming: false,
                                timestamp: imp_llm::now(),
                            });
                        }
                    }
                }
                KeyCode::Esc => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.go_back();
                    }
                }
                KeyCode::Backspace => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.pop_web_key_char();
                    }
                }
                KeyCode::Char(c) => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.push_web_key_char(c);
                    }
                }
                _ => {}
            },
            WelcomeStep::Done => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.mode = UiMode::Normal;
                }
                _ => {}
            },
        }
    }

    /// Persist welcome flow choices to config and auth, then advance to Done step.
    fn finish_welcome(&mut self) {
        let (
            model_id,
            thinking,
            provider_id,
            resolved_key,
            resolved_web_provider,
            resolved_web_key,
        ) = match &self.mode {
            UiMode::Welcome(state) => {
                let model_id = state
                    .selected_model()
                    .map(|m| m.id.clone())
                    .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
                let thinking = state.thinking_level;
                let provider_id = state.selected_provider_id().to_string();
                let resolved_key = state.resolved_key.clone();
                let resolved_web_provider = state.resolved_web_provider.clone();
                let resolved_web_key = state.resolved_web_key.clone();
                (
                    model_id,
                    thinking,
                    provider_id,
                    resolved_key,
                    resolved_web_provider,
                    resolved_web_key,
                )
            }
            _ => return,
        };

        // Update in-session config
        self.config.model = Some(model_id.clone());
        self.config.thinking = Some(thinking);
        self.model_name = model_id;
        self.thinking_level = thinking;

        if let Some(meta) = self.model_registry.resolve_meta(&self.model_name, None) {
            self.context_window = meta.context_window;
        }

        if let Some(web_provider) = resolved_web_provider
            .as_deref()
            .filter(|provider| *provider != "none")
        {
            self.config.web.search_provider = match web_provider {
                "tavily" => Some(imp_core::tools::web::types::SearchProvider::Tavily),
                "exa" => Some(imp_core::tools::web::types::SearchProvider::Exa),
                "linkup" => Some(imp_core::tools::web::types::SearchProvider::Linkup),
                "perplexity" => Some(imp_core::tools::web::types::SearchProvider::Perplexity),
                _ => self.config.web.search_provider,
            };
            std::env::set_var("IMP_WEB_PROVIDER", web_provider);
        }

        // Save config.toml
        let config_path = Config::user_config_path();
        if let Err(e) = self.config.save(&config_path) {
            self.messages.push(DisplayMessage {
                role: MessageRole::Error,
                content: format!("Failed to save config: {e}"),
                thinking: None,
                tool_calls: Vec::new(),
                assistant_blocks: Vec::new(),
                is_streaming: false,
                timestamp: imp_llm::now(),
            });
        }

        let auth_path = Config::user_config_dir().join("auth.json");
        let mut auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

        // Save API key if one was manually entered
        if let Some(key) = resolved_key {
            if let Err(e) = auth_store.store(
                &provider_id,
                imp_llm::auth::StoredCredential::ApiKey { key },
            ) {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: format!("Failed to save API key: {e}"),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
        }

        if let (Some(web_provider), Some(web_key)) = (
            resolved_web_provider
                .as_deref()
                .filter(|provider| *provider != "none"),
            resolved_web_key,
        ) {
            if let Err(e) = auth_store.store(
                web_provider,
                imp_llm::auth::StoredCredential::ApiKey { key: web_key },
            ) {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: format!("Failed to save web API key: {e}"),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
        }

        // Advance to Done screen
        if let UiMode::Welcome(ref mut state) = self.mode {
            state.advance();
        }
    }

    fn save_personality(&mut self) {
        let state = match &self.mode {
            UiMode::Personality(state) => state.clone(),
            _ => return,
        };

        match state.scope {
            PersonalityScope::Global => {
                state.save_to_config(&mut self.config.personality);
                let config_path = Config::user_config_path();
                match self.config.save(&config_path) {
                    Ok(()) => {
                        if let UiMode::Personality(ref mut current) = self.mode {
                            current.dirty = false;
                            current.saved_profiles =
                                self.config.personality.profiles.profile_names();
                            current.active_profile =
                                self.config.personality.profiles.active.clone();
                            current.profile_name = current
                                .active_profile
                                .clone()
                                .unwrap_or_else(|| "default".to_string());
                        }
                        self.push_system_msg(&format!(
                            "Personality saved to {}",
                            config_path.display()
                        ));
                    }
                    Err(e) => self.push_error_msg(&format!("Failed to save personality: {e}")),
                }
            }
            PersonalityScope::Project => {
                let path = self.cwd.join(".imp").join("config.toml");
                let mut project_config = Config::load(&path).unwrap_or_default();
                state.save_to_config(&mut project_config.personality);
                match project_config.save(&path) {
                    Ok(()) => {
                        if let UiMode::Personality(ref mut current) = self.mode {
                            current.dirty = false;
                            current.saved_profiles =
                                project_config.personality.profiles.profile_names();
                            current.active_profile =
                                project_config.personality.profiles.active.clone();
                            current.profile_name = current
                                .active_profile
                                .clone()
                                .unwrap_or_else(|| "default".to_string());
                        }
                        self.push_system_msg(&format!(
                            "Project personality saved to {}",
                            path.display()
                        ));
                    }
                    Err(e) => {
                        self.push_error_msg(&format!("Failed to save project personality: {e}"))
                    }
                }
            }
        }
    }

    fn delete_personality_profile(&mut self) {
        let state = match &self.mode {
            UiMode::Personality(state) => state.clone(),
            _ => return,
        };

        match state.scope {
            PersonalityScope::Global => {
                let deleted = if let UiMode::Personality(ref mut current) = self.mode {
                    current.delete_active_profile(&mut self.config.personality)
                } else {
                    false
                };
                if deleted {
                    let config_path = Config::user_config_path();
                    match self.config.save(&config_path) {
                        Ok(()) => self.push_system_msg("Deleted global personality profile."),
                        Err(e) => self.push_error_msg(&format!("Failed to save personality: {e}")),
                    }
                }
            }
            PersonalityScope::Project => {
                let path = self.cwd.join(".imp").join("config.toml");
                let mut project_config = Config::load(&path).unwrap_or_default();
                let deleted = if let UiMode::Personality(ref mut current) = self.mode {
                    current.delete_active_profile(&mut project_config.personality)
                } else {
                    false
                };
                if deleted {
                    match project_config.save(&path) {
                        Ok(()) => self.push_system_msg("Deleted project personality profile."),
                        Err(e) => {
                            self.push_error_msg(&format!("Failed to save project personality: {e}"))
                        }
                    }
                }
            }
        }
    }

    fn save_settings(&mut self) {
        // Extract state before mutating self
        let state = match &self.mode {
            UiMode::Settings(s) => s.clone(),
            _ => return,
        };

        // Apply to in-session config
        state.apply_to_config(&mut self.config);
        self.model_name = state.model.clone();
        self.thinking_level = state.thinking_level;
        self.theme = Theme::named(self.config.theme.as_deref().unwrap_or("default"));

        // Update context window from registry
        if let Some(meta) = self.model_registry.resolve_meta(&self.model_name, None) {
            self.context_window = meta.context_window;
        }

        let auth_path = Config::user_config_dir().join("auth.json");
        let mut auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
        let mut auth_notes = Vec::new();

        for (provider, value) in [
            ("tavily", state.tavily_api_key.trim()),
            ("exa", state.exa_api_key.trim()),
        ] {
            if value.is_empty() {
                continue;
            }

            match auth_store.store(
                provider,
                imp_llm::auth::StoredCredential::ApiKey {
                    key: value.to_string(),
                },
            ) {
                Ok(()) => auth_notes.push(format!("saved {provider} key")),
                Err(e) => {
                    self.messages.push(DisplayMessage {
                        role: MessageRole::Error,
                        content: format!("Failed to save {provider} API key: {e}"),
                        thinking: None,
                        tool_calls: Vec::new(),
                        assistant_blocks: Vec::new(),
                        is_streaming: false,
                        timestamp: imp_llm::now(),
                    });
                }
            }
        }

        // Persist to user config.toml
        let config_path = Config::user_config_path();
        match self.config.save(&config_path) {
            Ok(()) => {
                if let UiMode::Settings(ref mut s) = self.mode {
                    s.dirty = false;
                    s.tavily_api_key.clear();
                    s.exa_api_key.clear();
                    s.tavily_configured = provider_logged_in(&auth_store, "tavily");
                    s.exa_configured = provider_logged_in(&auth_store, "exa");
                }
                let mut message = format!("Settings saved to {}", config_path.display());
                if !auth_notes.is_empty() {
                    message.push_str(&format!(" ({})", auth_notes.join(", ")));
                }
                self.messages.push(DisplayMessage {
                    role: MessageRole::System,
                    content: message,
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
            Err(e) => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: format!("Failed to save settings: {e}"),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
        }
    }

    /// Return models filtered by `config.enabled_models` (if set) and by
    /// available credentials. Models whose provider has no auth configured
    /// are hidden unless explicitly listed in `enabled_models`.
    fn filtered_models(&self) -> Vec<ModelMeta> {
        let all = self.model_registry.list();

        // Load auth store to check which providers have credentials
        let auth_path = Config::user_config_dir().join("auth.json");
        let auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));

        match &self.config.enabled_models {
            Some(enabled) if !enabled.is_empty() => {
                let enabled_ids: Vec<&str> = enabled
                    .iter()
                    .filter_map(|name| {
                        self.model_registry
                            .find_by_alias(name)
                            .map(|m| m.id.as_str())
                    })
                    .collect();
                all.iter()
                    .filter(|m| enabled_ids.contains(&m.id.as_str()))
                    .cloned()
                    .collect()
            }
            _ => {
                // Auto-filter: only show models whose provider has credentials
                all.iter()
                    .filter(|m| auth_store.has_credentials(&m.provider))
                    .cloned()
                    .collect()
            }
        }
    }

    fn open_model_selector(&mut self) {
        let models = self.filtered_models();
        self.mode = UiMode::ModelSelector(ModelSelectorState::new(models, self.model_name.clone()));
    }

    fn open_file_finder(&mut self) {
        let files = collect_project_files(&self.cwd, 5000);
        self.mode = UiMode::FileFinder(FileFinderState::new(files));
    }

    fn open_tree_view(&mut self) {
        let tree = self.session.get_tree();
        let flat = flatten_tree(&tree, 0);
        if flat.is_empty() {
            self.push_system_msg("No session history yet.");
            return;
        }
        let current_id = self.session.leaf_id().map(String::from);
        self.mode = UiMode::TreeView(TreeViewState::new(flat, current_id));
    }

    fn cycle_model(&mut self, forward: bool) {
        let models = self.filtered_models();
        if models.is_empty() {
            return;
        }
        let current_idx = models.iter().position(|m| m.id == self.model_name);
        let next_idx = match current_idx {
            Some(idx) => {
                if forward {
                    (idx + 1) % models.len()
                } else {
                    (idx + models.len() - 1) % models.len()
                }
            }
            None => 0,
        };
        self.model_name = models[next_idx].id.clone();
        self.context_window = models[next_idx].context_window;
        self.invalidate_chat_render_cache();
        self.push_system_msg(&format!("Model: {}", self.model_name));
    }

    fn cycle_thinking_level(&mut self) {
        self.invalidate_chat_render_cache();
        self.thinking_level = match self.thinking_level {
            ThinkingLevel::Off => ThinkingLevel::Low,
            ThinkingLevel::Minimal => ThinkingLevel::Low,
            ThinkingLevel::Low => ThinkingLevel::Medium,
            ThinkingLevel::Medium => ThinkingLevel::High,
            ThinkingLevel::High => ThinkingLevel::XHigh,
            ThinkingLevel::XHigh => ThinkingLevel::Off,
        };
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn push_system_msg(&mut self, content: &str) {
        self.messages.push(DisplayMessage {
            role: MessageRole::System,
            content: content.to_string(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });
        self.invalidate_chat_render_cache();
    }

    fn push_error_msg(&mut self, content: &str) {
        self.messages.push(DisplayMessage {
            role: MessageRole::Error,
            content: content.to_string(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });
        self.invalidate_chat_render_cache();
    }

    fn run_manual_compaction(&mut self) {
        if self.is_streaming {
            self.push_error_msg("Cannot compact while the agent is actively streaming.");
            return;
        }

        let active_messages = self.session.get_active_messages();
        let prepared =
            prepare_messages_for_compaction(&active_messages, DEFAULT_KEEP_RECENT_GROUPS);
        if !prepared.should_compact() {
            self.push_system_msg("Not enough history to compact yet.");
            return;
        }

        let auth_path = Config::user_config_dir().join("auth.json");
        let mut auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

        let mut meta = match self.model_registry.resolve_meta(&self.model_name, None) {
            Some(meta) => meta,
            None => {
                self.push_error_msg(&format!("Unknown model: {}", self.model_name));
                return;
            }
        };

        let mut provider_name = meta.provider.clone();
        if should_use_chatgpt_provider(&auth_store, &self.model_registry, &meta) {
            provider_name = "openai-codex".to_string();
            if let Some(resolved) = self
                .model_registry
                .resolve_meta(&self.model_name, Some(&provider_name))
            {
                meta = resolved;
            }
        }

        let provider = match create_provider(&provider_name) {
            Some(provider) => provider,
            None => {
                self.push_error_msg(&format!("Unknown provider: {provider_name}"));
                return;
            }
        };

        let api_key = match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(resolve_provider_api_key(&mut auth_store, &provider_name))
        }) {
            Ok(key) => key,
            Err(e) => {
                self.push_error_msg(&format!("Failed to resolve auth for compaction: {e}"));
                return;
            }
        };

        let model = Model {
            meta,
            provider: Arc::from(provider),
        };
        let model_id = model.meta.id.clone();
        let model_meta = model.meta.clone();
        let model_provider = Arc::clone(&model.provider);

        let mut config = self.config.clone();
        config.thinking = Some(self.thinking_level);

        let lua_cwd = self.cwd.clone();
        let user_config_dir = imp_core::config::Config::user_config_dir();
        let (agent, _handle) = match AgentBuilder::new(config, self.cwd.clone(), model, api_key)
            .lua_tool_loader(move |tools| {
                imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools);
            })
            .build()
        {
            Ok(built) => built,
            Err(e) => {
                self.push_error_msg(&format!("Failed to build compaction agent: {e}"));
                return;
            }
        };

        let system_prompt = agent.system_prompt.clone();

        let strategy = select_compaction_strategy(&CompactionCapabilities {
            provider_id: &provider_name,
            model_id: &model_id,
            allow_provider_native: false,
        });
        if matches!(strategy, CompactionStrategy::ProviderNative) {
            self.push_system_msg(
                "Provider-native compaction is not enabled yet; falling back to local compaction.",
            );
        }

        let result = execute_compaction_with_retry(
            &mut self.session,
            DEFAULT_KEEP_RECENT_GROUPS,
            2,
            |prompt| {
                use futures::StreamExt;
                use imp_llm::provider::{CacheOptions, Context as LlmContext, RequestOptions};
                use imp_llm::StreamEvent;

                let model_meta = model_meta.clone();
                let model_provider = Arc::clone(&model_provider);
                let api_key = agent.api_key.clone();
                let system_prompt = system_prompt.clone();
                let prompt = prompt.to_string();
                let thinking_level = self.thinking_level;
                let retry_policy = agent.retry_policy.clone();

                tokio::task::block_in_place(|| {
                    let runtime = tokio::runtime::Handle::current();
                    runtime.block_on(async move {
                        let mut summary = String::new();
                        let mut message_end_text: Option<String> = None;

                        let model = Model {
                            meta: model_meta,
                            provider: model_provider,
                        };
                        let context = LlmContext {
                            messages: vec![Message::user(prompt)],
                        };
                        let options = RequestOptions {
                            thinking_level,
                            max_tokens: Some(2048),
                            temperature: Some(0.2),
                            system_prompt,
                            tools: Vec::new(),
                            cache_options: CacheOptions::default(),
                            effort: None,
                        };

                        let mut stream = imp_core::retry::stream_with_retry(
                            move || model.provider.stream(&model, context.clone(), options.clone(), &api_key),
                            retry_policy,
                        );

                        while let Some(item) = stream.next().await {
                            match item {
                                Ok(StreamEvent::TextDelta { text }) => summary.push_str(&text),
                                Ok(StreamEvent::MessageEnd { message }) => {
                                    let body = message
                                        .content
                                        .iter()
                                        .filter_map(|block| match block {
                                            imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join("");
                                    if !body.is_empty() {
                                        message_end_text = Some(body);
                                    }
                                }
                                Ok(_) => {}
                                Err(_) => return None,
                            }
                        }

                        let final_text = if !summary.trim().is_empty() {
                            summary
                        } else {
                            message_end_text.unwrap_or_default()
                        };
                        (!final_text.trim().is_empty()).then_some(final_text)
                    })
                })
            },
        );

        match result {
            Ok(Some(compaction)) => {
                self.load_session_messages();
                self.messages.push(DisplayMessage {
                    role: MessageRole::Compaction,
                    content: format!(
                        "Context compacted. Saved ~{} tokens. Preserved recent working context.",
                        compaction.tokens_before.saturating_sub(compaction.tokens_after)
                    ),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
                self.push_system_msg(&format!(
                    "Compaction summary stored. Active context now uses the compacted branch view."
                ));
            }
            Ok(None) => {
                self.push_system_msg("Not enough history to compact yet.");
            }
            Err(e) => {
                self.push_error_msg(&format!("Compaction failed: {e}"));
            }
        }
    }

    fn export_conversation(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        for msg in &self.messages {
            let role = match msg.role {
                MessageRole::User => "**You:**",
                MessageRole::Assistant => "**Assistant:**",
                MessageRole::System | MessageRole::Compaction => "*System:*",
                MessageRole::Error => "**Error:**",
            };
            writeln!(f, "{role}\n{}\n", msg.content)?;
            for tc in &msg.tool_calls {
                writeln!(f, "> `{}`: {}", tc.name, tc.args_summary)?;
                if let Some(ref output) = tc.output {
                    let preview = truncate_chars_with_suffix(output, 200, "");
                    writeln!(f, "> {preview}\n")?;
                }
            }
        }
        Ok(())
    }

    // ── Agent event handling ────────────────────────────────────

    pub fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::AgentStart { model, .. } => {
                self.model_name = model;
                self.is_streaming = true;
                self.tool_focus = None;
                self.invalidate_chat_render_cache();
                self.turn_tracker.reset();
            }
            AgentEvent::AgentEnd { cost, .. } => {
                self.accumulated_cost.total += cost.total;
                self.accumulated_cost.input += cost.input;
                self.accumulated_cost.output += cost.output;
                self.is_streaming = false;

                // Mark last streaming message as done
                if let Some(last) = self.messages.last_mut() {
                    last.is_streaming = false;
                }
                self.invalidate_chat_render_cache();

                // Process follow-up messages
                let follow_ups: Vec<_> = self
                    .message_queue
                    .drain(..)
                    .filter_map(|m| match m {
                        QueuedMessage::FollowUp(text) => Some(text),
                        _ => None,
                    })
                    .collect();
                for text in follow_ups {
                    self.editor.set_content(&text);
                    self.send_message();
                }
            }
            AgentEvent::MessageDelta { delta } => {
                if let Some(last) = self.messages.last_mut() {
                    match delta {
                        StreamEvent::TextDelta { text } => {
                            last.push_assistant_text_delta(&text);
                        }
                        StreamEvent::ThinkingDelta { text } => match &mut last.thinking {
                            Some(t) => t.push_str(&text),
                            None => last.thinking = Some(text),
                        },
                        StreamEvent::ToolCall {
                            id,
                            name,
                            arguments,
                        } => {
                            last.push_assistant_tool_call(DisplayToolCall {
                                id,
                                args_summary: DisplayToolCall::make_args_summary(&name, &arguments),
                                name,
                                output: None,
                                details: serde_json::Value::Null,
                                is_error: false,
                                expanded: self.tools_expanded,
                                streaming_lines: Vec::new(),
                                streaming_output: String::new(),
                            });
                        }
                        _ => {}
                    }
                }
                self.invalidate_chat_render_cache();
                // Auto-scroll to bottom
                if self.auto_scroll {
                    self.scroll_offset = 0;
                }
            }
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => {
                self.turn_tracker
                    .record_tool_start(&tool_call_id, &tool_name, &args);
                // Find the matching tool call and update it
                if let Some(last) = self.messages.last_mut() {
                    if let Some(tc) = last.tool_calls.last_mut() {
                        if tc.name == tool_name {
                            tc.args_summary = DisplayToolCall::make_args_summary(&tool_name, &args);
                        }
                    }
                }
                self.invalidate_chat_render_cache();
                // Sidebar: auto-follow the new tool call
                if let Some(idx) = self.find_tool_call_index(&tool_call_id) {
                    self.focus_tool(idx);
                    if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Stream {
                        self.sidebar.detail_scroll = usize::MAX;
                    }
                }
                // Auto-open on first tool if terminal is wide enough, or whenever
                // chat tool calls are hidden and the sidebar is their only surface.
                if !self.sidebar.first_tool_seen {
                    self.sidebar.first_tool_seen = true;
                    let (cols, _) = crossterm::terminal::size().unwrap_or((80, 24));
                    if self.config.ui.effective_chat_tool_display()
                        == imp_core::config::ChatToolDisplay::Hidden
                        || (self.config.ui.auto_open_sidebar
                            && cols >= self.config.ui.sidebar_auto_open_width)
                    {
                        self.sidebar.open = true;
                    }
                }
            }
            AgentEvent::ToolOutputDelta { tool_call_id, text } => {
                // Feed streaming output into the tool call's rolling buffer
                for msg in self.messages.iter_mut().rev() {
                    for tc in &mut msg.tool_calls {
                        if tc.id == tool_call_id && tc.output.is_none() {
                            // Append text to the full live transcript.
                            if !tc.streaming_output.is_empty() {
                                tc.streaming_output.push('\n');
                            }
                            tc.streaming_output.push_str(&text);
                            // Append text and keep configured rolling tail for chat.
                            for line in text.lines() {
                                tc.streaming_lines.push(line.to_string());
                            }
                            if tc.streaming_lines.len() > self.config.ui.streaming_lines {
                                let excess =
                                    tc.streaming_lines.len() - self.config.ui.streaming_lines;
                                tc.streaming_lines.drain(..excess);
                            }
                            break;
                        }
                    }
                }
                self.invalidate_chat_render_cache();
                if self.auto_scroll {
                    self.scroll_offset = 0;
                }
            }
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                result,
            } => {
                let is_error = result.is_error;
                self.turn_tracker.record_tool_end(&tool_call_id, is_error);
                // Build display text from result content
                let output_text = result
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                // Attach result to the matching display tool call
                for msg in self.messages.iter_mut().rev() {
                    for tc in &mut msg.tool_calls {
                        if tc.id == tool_call_id {
                            tc.output = Some(output_text.clone());
                            if tc.streaming_output.is_empty() {
                                tc.streaming_output = output_text.clone();
                            }
                            tc.details = result.details.clone();
                            tc.is_error = is_error;
                            // Auto-expand failed tool calls so the error is immediately visible
                            if is_error {
                                tc.expanded = true;
                            }
                            break;
                        }
                    }
                }

                self.invalidate_chat_render_cache();

                // Persist tool result to session so resume has full conversation
                let _ = self.session.append_tool_result_message(result);
            }
            AgentEvent::Timing { timing } => {
                self.status_items.insert(
                    "timing".to_string(),
                    format!(
                        "{} {}ms",
                        timing.stage.as_str(),
                        timing.since_llm_request_start_ms
                    ),
                );
            }
            AgentEvent::TurnEnd { index, message } => {
                // Update context tracking from this turn's usage
                if let Some(ref usage) = message.usage {
                    self.current_context_tokens = usage.input_tokens + usage.cache_read_tokens;
                    self.accumulated_usage.add(usage);
                }

                // Persist assistant message to session, plus canonical usage when possible.
                if let Some(model_meta) = self.current_model_meta_for_persistence() {
                    let _ = self.session.append_assistant_turn_with_model_meta(
                        &model_meta,
                        index,
                        message,
                    );
                } else {
                    let msg_id = uuid::Uuid::new_v4().to_string();
                    let _ = self.session.append(SessionEntry::Message {
                        id: msg_id,
                        parent_id: None,
                        message: imp_llm::Message::Assistant(message),
                    });
                }
            }
            AgentEvent::Error { error } => {
                // Stop streaming — errors can be terminal (no AgentEnd follows)
                self.is_streaming = false;
                if let Some(last) = self.messages.last_mut() {
                    last.is_streaming = false;
                }
                self.invalidate_chat_render_cache();

                // Parse the error for a cleaner display
                let display_error = parse_api_error(&error);

                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: display_error,
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
                self.invalidate_chat_render_cache();
            }
            _ => {}
        }
    }
}

// ── Error parsing ───────────────────────────────────────────────

/// Extract a human-readable message from API error strings.
/// Input: "Provider error: HTTP 401 Unauthorized: {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"OAuth token has expired...\"}}"
/// Output: "OAuth token has expired. Please obtain a new token or refresh your existing token. (use /login)"
fn parse_api_error(raw: &str) -> String {
    // Try to extract JSON from the error string.
    if let Some(json_start) = raw.find('{') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw[json_start..]) {
            // Anthropic error format: {"type":"error","error":{"type":"...","message":"..."}}
            if let Some(msg) = parsed
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                let hint = if msg.contains("expired") || msg.contains("token") {
                    " (use /login to refresh)"
                } else {
                    ""
                };
                return format!("{msg}{hint}");
            }
            // Simple {"message":"..."} format
            if let Some(msg) = parsed.get("message").and_then(|m| m.as_str()) {
                return msg.to_string();
            }
        }
    }

    // Some gateways / auth layers return full HTML error pages. Showing raw HTML
    // in the chat transcript is noisy and unhelpful, so collapse it to a short
    // summary instead.
    if looks_like_html_error(raw) {
        let status = extract_http_status(raw);
        let title = extract_html_title(raw);

        return match (status, title) {
            (Some(status), Some(title)) => format!(
                "Provider returned an HTML error page ({status}: {title}). This usually means an auth, gateway, proxy, or rate-limit issue."
            ),
            (Some(status), None) => format!(
                "Provider returned an HTML error page ({status}). This usually means an auth, gateway, proxy, or rate-limit issue."
            ),
            (None, Some(title)) => format!(
                "Provider returned an HTML error page ({title}). This usually means an auth, gateway, proxy, or rate-limit issue."
            ),
            (None, None) => "Provider returned an HTML error page. This usually means an auth, gateway, proxy, or rate-limit issue.".to_string(),
        };
    }

    raw.to_string()
}

fn looks_like_html_error(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("<!doctype html")
        || lower.contains("<html")
        || lower.contains("<head")
        || lower.contains("<body")
        || lower.contains("<title")
}

fn extract_http_status(raw: &str) -> Option<String> {
    let start = raw.find("HTTP ")?;
    let rest = &raw[start..];
    let end = rest
        .find(|c: char| c == ':' || c == '\n' || c == '<')
        .unwrap_or(rest.len());
    let status = rest[..end].trim();
    (!status.is_empty()).then(|| status.to_string())
}

fn extract_html_title(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let title_start = lower.find("<title")?;
    let open_end = lower[title_start..].find('>')? + title_start + 1;
    let close_start = lower[open_end..].find("</title>")? + open_end;
    let title = raw[open_end..close_start].trim();
    (!title.is_empty()).then(|| title.to_string())
}

#[cfg(test)]
mod parse_api_error_tests {
    use super::parse_api_error;

    #[test]
    fn extracts_nested_json_error_message() {
        let raw = "Provider error: HTTP 401 Unauthorized: {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"OAuth token has expired\"}}";
        assert_eq!(
            parse_api_error(raw),
            "OAuth token has expired (use /login to refresh)"
        );
    }

    #[test]
    fn extracts_simple_json_message() {
        let raw = "Provider error: HTTP 429 Too Many Requests: {\"message\":\"Rate limited\"}";
        assert_eq!(parse_api_error(raw), "Rate limited");
    }

    #[test]
    fn collapses_html_error_pages_to_summary() {
        let raw = "Provider error: HTTP 403 Forbidden: <!DOCTYPE html><html><head><title>Attention Required! | Cloudflare</title></head><body>blocked</body></html>";
        assert_eq!(
            parse_api_error(raw),
            "Provider returned an HTML error page (HTTP 403 Forbidden: Attention Required! | Cloudflare). This usually means an auth, gateway, proxy, or rate-limit issue."
        );
    }

    #[test]
    fn leaves_plain_text_errors_alone() {
        let raw = "Provider error: connection reset by peer";
        assert_eq!(parse_api_error(raw), raw);
    }
}

// ── Layout helpers ──────────────────────────────────────────────

/// Create a centered rect using percentage of the available area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Check if a point is inside an optional rect.
fn point_in_rect(col: u16, row: u16, rect: Option<Rect>) -> bool {
    match rect {
        Some(r) => col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height,
        None => false,
    }
}

/// Create an area above the editor for a dropdown.
fn command_dropdown_area(editor_area: Rect, max_height: u16) -> Rect {
    let height = max_height.min(editor_area.y);
    Rect {
        x: editor_area.x,
        y: editor_area.y.saturating_sub(height),
        width: editor_area.width.min(60),
        height,
    }
}

#[cfg(test)]
mod session_lifecycle {
    use super::*;
    use imp_core::config::Config;
    use imp_core::session::{SessionEntry, SessionManager};
    use imp_llm::{AssistantMessage, ContentBlock, StopReason};
    use imp_llm::model::ModelRegistry;
    use imp_llm::ThinkingLevel;
    use tempfile::TempDir;

    /// Helper: build an App with defaults and an in-memory session.
    fn make_app() -> App {
        let config = Config::default();
        let session = SessionManager::in_memory();
        let registry = ModelRegistry::with_builtins();
        App::new(config, session, registry, PathBuf::from("/tmp/test"))
    }

    /// Helper: build an App backed by a persistent session in `dir`.
    fn make_persistent_app(tmp: &TempDir) -> App {
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let session = SessionManager::new(&cwd, &session_dir).unwrap();
        let config = Config {
            model: Some("sonnet".into()),
            ..Config::default()
        };
        let registry = ModelRegistry::with_builtins();
        App::new(config, session, registry, cwd)
    }

    #[test]
    fn terminal_title_uses_manual_session_name_when_present() {
        let mut app = make_app();
        app.session.set_name("my chat");
        assert_eq!(app.terminal_title(), "imp — my chat");
    }

    #[test]
    fn terminal_title_falls_back_to_summarized_first_prompt() {
        let mut app = make_app();
        app.session
            .append(SessionEntry::Message {
                id: "m1".into(),
                parent_id: None,
                message: Message::user(
                    "can we adjust the information that is displayed in the top bar",
                ),
            })
            .unwrap();
        assert_eq!(app.terminal_title(), "imp — adjust top bar layout");
    }

    #[test]
    fn terminal_title_defaults_to_chat_when_empty() {
        let app = make_app();
        assert_eq!(app.terminal_title(), "imp — chat");
    }

    // ── 1. App::new creates with config + session ───────────────

    #[test]
    fn tui_integration_app_new_defaults() {
        let app = make_app();

        assert!(app.running);
        assert!(app.messages.is_empty());
        assert_eq!(app.model_name, "sonnet");
        assert_eq!(app.thinking_level, ThinkingLevel::Medium);
        assert_eq!(app.context_window, 1_000_000);
        assert!(!app.is_streaming);
        assert!(app.agent_handle.is_none());
        assert!(matches!(app.mode, UiMode::Normal));
    }

    #[test]
    fn tui_integration_app_new_with_custom_config() {
        let config = Config {
            model: Some("haiku".into()),
            thinking: Some(ThinkingLevel::High),
            ..Config::default()
        };
        let session = SessionManager::in_memory();
        let registry = ModelRegistry::with_builtins();
        let app = App::new(config, session, registry, PathBuf::from("/tmp"));

        assert_eq!(app.model_name, "haiku");
        assert_eq!(app.thinking_level, ThinkingLevel::High);
    }

    #[test]
    fn tui_integration_app_new_persistent_session() {
        let tmp = TempDir::new().unwrap();
        let app = make_persistent_app(&tmp);

        // Session is backed by a file on disk
        assert!(app.session.path().is_some());
        assert!(app.session.path().unwrap().exists());
    }

    // ── 2. send_message persists to session ─────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tui_integration_send_message_persists() {
        let tmp = TempDir::new().unwrap();
        let mut app = make_persistent_app(&tmp);

        // Type a message and send
        app.editor.set_content("hello world");
        app.send_message();

        // User message persisted to session (even though agent spawn fails)
        let messages = app.session.get_messages();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].is_user());

        // Display should have user msg + error (agent spawn fails without auth)
        assert!(app.messages.len() >= 2);
        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(app.messages[0].content, "hello world");
    }

    #[test]
    fn tui_integration_send_message_empty_ignored() {
        let mut app = make_app();

        // Empty editor — send_message should be a no-op
        app.send_message();
        assert!(app.messages.is_empty());
        assert_eq!(app.session.get_messages().len(), 0);

        // Whitespace-only too
        app.editor.set_content("   ");
        app.send_message();
        assert!(app.messages.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tui_integration_send_message_persists_to_disk() {
        let tmp = TempDir::new().unwrap();
        let mut app = make_persistent_app(&tmp);
        let session_path = app.session.path().unwrap().to_path_buf();

        app.editor.set_content("persist me");
        app.send_message();

        // Reopen the file and verify the message is there
        let reopened = SessionManager::open(&session_path).unwrap();
        let msgs = reopened.get_messages();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].is_user());
    }

    // ── 3. Slash commands ───────────────────────────────────────

    #[test]
    fn tui_integration_slash_new_clears_session() {
        let mut app = make_app();

        // Add some messages first
        app.messages.push(DisplayMessage {
            role: MessageRole::User,
            content: "old message".into(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: 0,
        });
        assert_eq!(app.messages.len(), 1);

        // Execute /new
        app.execute_command("new");

        assert!(app.messages.is_empty());
        // Session replaced with in-memory
        assert!(app.session.path().is_none());
    }

    #[test]
    fn tui_integration_slash_compact_noops_with_short_history() {
        let mut app = make_app();

        app.execute_command("compact");

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::System);
        assert_eq!(app.messages[0].content, "Not enough history to compact yet.");
    }

    #[test]
    fn load_session_messages_uses_compacted_active_history() {
        let mut app = make_app();
        app.session
            .append(SessionEntry::Message {
                id: "u1".into(),
                parent_id: None,
                message: Message::user("older request"),
            })
            .unwrap();
        app.session
            .append(SessionEntry::Message {
                id: "a1".into(),
                parent_id: None,
                message: Message::Assistant(AssistantMessage {
                    content: vec![ContentBlock::Text {
                        text: "older answer".into(),
                    }],
                    usage: None,
                    stop_reason: StopReason::EndTurn,
                    timestamp: 0,
                }),
            })
            .unwrap();
        app.session
            .append(SessionEntry::Message {
                id: "u2".into(),
                parent_id: None,
                message: Message::user("recent request"),
            })
            .unwrap();
        app.session
            .append(SessionEntry::Compaction {
                id: "c1".into(),
                parent_id: None,
                summary: format!("{}summary body", COMPACTION_SUMMARY_PREFIX),
                first_kept_id: "u2".into(),
                tokens_before: 100,
                tokens_after: 40,
            })
            .unwrap();

        app.load_session_messages();

        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[0].role, MessageRole::Compaction);
        assert!(app.messages[0].content.contains("summary body"));
        assert_eq!(app.messages[1].role, MessageRole::User);
        assert_eq!(app.messages[1].content, "recent request");
    }

    #[test]
    fn tui_integration_slash_quit_stops_app() {
        let mut app = make_app();
        assert!(app.running);

        app.execute_command("quit");
        assert!(!app.running);
    }

    #[test]
    fn tui_integration_slash_mouse_command_is_removed() {
        let mut app = make_app();
        // /mouse is no longer a recognized command — it should fall through to unknown
        app.execute_command("mouse");
        assert!(app
            .messages
            .last()
            .unwrap()
            .content
            .contains("Unknown command"));
    }

    #[test]
    fn tui_integration_slash_unknown_shows_error() {
        let mut app = make_app();

        app.execute_command("nonexistent");

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::Error);
        assert!(app.messages[0].content.contains("nonexistent"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn agent_task_completion_preserves_active_replacement_handle() {
        let mut app = make_app();
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(4);
        drop(event_tx);

        app.agent_handle = Some(AgentHandle {
            event_rx,
            command_tx,
        });
        app.agent_task = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(())
        }));

        app.handle_runtime_signal(RuntimeSignal::AgentTaskCompleted);

        assert!(
            app.agent_handle.is_some(),
            "active replacement handle should survive stale completion"
        );

        if let Some(task) = app.agent_task.take() {
            task.abort();
        }
    }

    #[test]
    fn agent_task_completion_clears_handle_when_no_replacement_is_active() {
        let mut app = make_app();
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(4);
        drop(event_tx);

        app.agent_handle = Some(AgentHandle {
            event_rx,
            command_tx,
        });
        app.agent_task = None;

        app.handle_runtime_signal(RuntimeSignal::AgentTaskCompleted);

        assert!(
            app.agent_handle.is_none(),
            "completed task should release handle when no replacement exists"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn agent_task_failure_preserves_active_replacement_handle() {
        let mut app = make_app();
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(4);
        drop(event_tx);

        app.agent_handle = Some(AgentHandle {
            event_rx,
            command_tx,
        });
        app.agent_task = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(())
        }));

        app.handle_runtime_signal(RuntimeSignal::AgentTaskFailed("boom".into()));

        assert!(
            app.agent_handle.is_some(),
            "active replacement handle should survive stale failure"
        );
        assert_eq!(
            app.messages.last().map(|m| m.role.clone()),
            Some(MessageRole::Error)
        );

        if let Some(task) = app.agent_task.take() {
            task.abort();
        }
    }

    #[test]
    fn tui_integration_slash_personality_opens_overlay() {
        let mut app = make_app();
        app.execute_command("personality");
        assert!(matches!(app.mode, UiMode::Personality(_)));
    }

    #[test]
    fn tui_integration_slash_memory_shows_stores() {
        let mut app = make_app();

        app.execute_command("memory");

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::System);
        assert!(app.messages[0].content.contains("Memory ("));
        assert!(app.messages[0].content.contains("User profile ("));
    }

    #[test]
    fn tui_integration_slash_memory_add_and_show() {
        let tmp = TempDir::new().unwrap();
        // Point config dir to temp so we don't touch real memory
        std::env::set_var("XDG_CONFIG_HOME", tmp.path().to_str().unwrap());

        let mut app = make_app();

        app.execute_command("memory add Test entry from slash command");
        assert!(app.messages.last().unwrap().content.contains("Added"));

        // Show should list the entry
        app.execute_command("memory");
        let content = &app.messages.last().unwrap().content;
        assert!(content.contains("Test entry from slash command"));

        // Clean up env var
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn tui_integration_slash_memory_help() {
        let mut app = make_app();

        app.execute_command("memory help");

        let content = &app.messages.last().unwrap().content;
        assert!(content.contains("/memory add"));
        assert!(content.contains("/memory remove"));
        assert!(content.contains("/memory clear"));
    }

    #[test]
    fn tui_integration_slash_memory_unknown_subcommand() {
        let mut app = make_app();

        app.execute_command("memory frobnicate");

        let content = &app.messages.last().unwrap().content;
        assert!(content.contains("Unknown memory subcommand"));
        assert!(content.contains("frobnicate"));
    }

    #[test]
    fn personality_state_default_sentence_is_visible() {
        let global = imp_core::personality::PersonalityConfig::default();
        let state = crate::views::personality::PersonalityState::new(
            &global,
            None,
            crate::views::personality::PersonalityScope::Global,
        );
        assert_eq!(
            state.sentence(),
            "You are imp, a practical, concise, coding agent."
        );
    }

    #[test]
    fn tui_integration_slash_via_send_message() {
        let mut app = make_app();

        // Type /new into editor and "send" — should route to execute_command
        app.editor.set_content("/new");
        app.send_message();

        // /new clears messages, so display should be empty
        assert!(app.messages.is_empty());
        // Editor should be cleared
        assert!(app.editor.is_empty());
    }

    // ── 4. Session reload on restart ────────────────────────────

    #[test]
    fn tui_integration_session_reload_on_restart() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");

        // First "session": create and send messages
        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
        let session_path = session.path().unwrap().to_path_buf();
        session
            .append(SessionEntry::Message {
                id: "m1".into(),
                parent_id: None,
                message: imp_llm::Message::user("first message"),
            })
            .unwrap();
        session
            .append(SessionEntry::Message {
                id: "m2".into(),
                parent_id: None,
                message: imp_llm::Message::user("second message"),
            })
            .unwrap();

        // "Restart": open the session file and create a new App
        let reloaded_session = SessionManager::open(&session_path).unwrap();
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let mut app = App::new(config, reloaded_session, registry, cwd);

        // Load persisted messages into display
        app.load_session_messages();

        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(app.messages[0].content, "first message");
        assert_eq!(app.messages[1].content, "second message");
    }

    #[test]
    fn tui_integration_continue_recent_session() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");

        // Create a session for this cwd
        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
        session
            .append(SessionEntry::Message {
                id: "m1".into(),
                parent_id: None,
                message: imp_llm::Message::user("continued"),
            })
            .unwrap();
        drop(session);

        // Simulate --continue: find the most recent session for this cwd
        let continued = SessionManager::continue_recent(&cwd, &session_dir)
            .unwrap()
            .expect("should find a session");
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let mut app = App::new(config, continued, registry, cwd);
        app.load_session_messages();

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "continued");
    }

    // ── 5. Model switching ──────────────────────────────────────

    #[test]
    fn tui_integration_model_switch_via_cycle() {
        let mut app = make_app();

        // The default "sonnet" alias isn't a canonical ID, so cycle_model
        // starts from index 0.  After cycling forward, the model changes.
        let models = app.model_registry.list().to_vec();
        assert!(!models.is_empty());

        app.cycle_model(true);
        let after_first = app.model_name.clone();
        // Should now be a canonical model ID from the registry
        assert!(
            models.iter().any(|m| m.id == after_first),
            "model_name should be a registered model after cycling"
        );

        app.cycle_model(true);
        let after_second = app.model_name.clone();
        assert_ne!(
            after_first, after_second,
            "cycling again should pick a different model"
        );

        // Cycling back returns to previous
        app.cycle_model(false);
        assert_eq!(app.model_name, after_first);
    }

    #[test]
    fn tui_integration_model_switch_updates_context_window() {
        let mut app = make_app();
        let original_ctx = app.context_window;

        // Cycle to a different model and check context_window updated
        app.cycle_model(true);
        let new_model = app.model_name.clone();
        let new_ctx = app.context_window;

        let meta = app.model_registry.find_by_alias(&new_model).unwrap();
        assert_eq!(new_ctx, meta.context_window);

        // If the new model has a different context window, verify it changed
        if meta.context_window != original_ctx {
            assert_ne!(new_ctx, original_ctx);
        }
    }

    #[test]
    fn tui_integration_thinking_level_cycle() {
        let mut app = make_app();
        assert_eq!(app.thinking_level, ThinkingLevel::Medium);

        app.cycle_thinking_level();
        assert_eq!(app.thinking_level, ThinkingLevel::High);

        app.cycle_thinking_level();
        assert_eq!(app.thinking_level, ThinkingLevel::XHigh);

        app.cycle_thinking_level();
        assert_eq!(app.thinking_level, ThinkingLevel::Off);
    }

    // ── 6. Mouse click handling ─────────────────────────────────

    #[test]
    fn app_starts_without_selection_state() {
        let app = make_app();
        assert!(app.selection.is_none());
        assert!(app.chat_surface.is_none());
        assert!(app.sidebar_list_rect.is_none());
    }

    #[test]
    fn mouse_click_on_chat_area_starts_selection_instead_of_opening_sidebar() {
        let mut app = make_app();

        // Simulate a message with a tool call
        app.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: "checking...".into(),
            thinking: None,
            tool_calls: vec![crate::views::tools::DisplayToolCall {
                id: "tc-42".into(),
                name: "bash".into(),
                args_summary: "$ ls".into(),
                output: Some("file1\nfile2".into()),
                details: serde_json::Value::Null,
                is_error: false,
                expanded: false,
                streaming_lines: Vec::new(),
                streaming_output: String::new(),
            }],
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: 0,
        });

        // Pre-populate chat surface; chat clicks now start selection instead of opening sidebar
        app.chat_surface = Some(TextSurface::new(
            SelectablePane::Chat,
            Rect::new(0, 0, 40, 5),
            vec!["checking...".into()],
            0,
        ));

        // Simulate a mouse click at row 5
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);

        assert!(!app.sidebar.open);
        assert_eq!(app.active_pane, Pane::Chat);
        assert!(app.selection.is_some());
    }

    #[test]
    fn mouse_click_on_sidebar_sets_focus() {
        let mut app = make_app();
        app.sidebar.open = true;
        app.sidebar_detail_rect = Some(Rect::new(50, 10, 30, 10));

        app.sidebar_detail_surface = Some(TextSurface::new(
            SelectablePane::SidebarDetail,
            Rect::new(50, 12, 30, 8),
            vec!["detail".into()],
            0,
        ));

        // Click inside sidebar detail
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 60,
            row: 15,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);

        assert_eq!(app.active_pane, Pane::SidebarDetail);
    }

    #[test]
    fn mouse_click_on_chat_area_sets_chat_focus() {
        let mut app = make_app();
        app.active_pane = Pane::SidebarDetail;
        app.sidebar_list_rect = Some(Rect::new(50, 1, 30, 5));
        app.sidebar_detail_rect = Some(Rect::new(50, 7, 30, 13));

        // Click outside sidebar (in chat area)
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 10,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);

        assert_eq!(app.active_pane, Pane::Chat);
    }

    #[test]
    fn keyboard_page_scroll_targets_chat_or_sidebar_detail() {
        let mut app = make_app();
        let lines = app.config.ui.keyboard_scroll_lines;

        app.handle_normal_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.scroll_offset, lines);
        assert!(!app.auto_scroll);
        assert_eq!(app.sidebar.detail_scroll, 0);

        app.sidebar.open = true;
        app.active_pane = Pane::SidebarDetail;
        app.handle_normal_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.sidebar.detail_scroll, 0);
        assert_eq!(app.scroll_offset, lines);

        app.handle_normal_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.sidebar.detail_scroll, lines);
        assert_eq!(app.scroll_offset, lines);

        app.active_pane = Pane::Chat;
        app.handle_normal_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.scroll_offset, 0);
        assert!(app.auto_scroll);
    }

    #[test]
    fn ctrl_b_and_ctrl_f_map_to_page_scroll() {
        let mut app = make_app();
        let lines = app.config.ui.keyboard_scroll_lines;

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(app.scroll_offset, lines);

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn mouse_scroll_routes_by_position() {
        let mut app = make_app();
        // Use split mode so list and detail scroll independently
        app.config.ui.sidebar_style = imp_core::config::SidebarStyle::Split;

        // Scroll up in chat area (no sidebar rects set)
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 5,
            row: 5,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);
        assert_eq!(app.scroll_offset, 3);
        assert!(!app.auto_scroll);

        // Set up sidebar rects and scroll in detail area
        app.sidebar_detail_rect = Some(Rect::new(50, 5, 30, 15));
        app.sidebar.detail_scroll = 0;
        let mouse_detail = crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 60,
            row: 10,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse_detail);
        assert_eq!(app.sidebar.detail_scroll, 3);
        // Chat scroll should be unchanged
        assert_eq!(app.scroll_offset, 3);

        // Scroll in list area
        app.sidebar_list_rect = Some(Rect::new(50, 0, 30, 5));
        app.sidebar.list_scroll = 0;
        let mouse_list = crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 60,
            row: 2,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse_list);
        assert_eq!(app.sidebar.list_scroll, 3);
    }

    #[test]
    fn mouse_drag_in_chat_creates_selection() {
        let mut app = make_app();
        app.chat_surface = Some(TextSurface::new(
            SelectablePane::Chat,
            Rect::new(0, 0, 40, 5),
            vec!["hello world".into(), "second line".into()],
            0,
        ));

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 1,
            row: 0,
            modifiers: KeyModifiers::empty(),
        });
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Drag(crossterm::event::MouseButton::Left),
            column: 4,
            row: 0,
            modifiers: KeyModifiers::empty(),
        });

        let selection = app.selection.clone().expect("selection created");
        assert_eq!(selection.pane, SelectablePane::Chat);
        let text = app.selection_text().unwrap();
        assert_eq!(text, "ello");
        assert_eq!(app.active_pane, Pane::Chat);
    }

    #[test]
    fn mouse_click_on_sidebar_list_selects_tool_for_review() {
        let mut app = make_app();
        app.sidebar.open = true;
        app.config.ui.sidebar_style = imp_core::config::SidebarStyle::Split;
        app.sidebar_list_rect = Some(Rect::new(50, 1, 30, 5));
        app.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: "checking...".into(),
            thinking: None,
            tool_calls: vec![crate::views::tools::DisplayToolCall {
                id: "tc-42".into(),
                name: "bash".into(),
                args_summary: "$ ls".into(),
                output: Some("file1\nfile2".into()),
                details: serde_json::Value::Null,
                is_error: false,
                expanded: false,
                streaming_lines: Vec::new(),
                streaming_output: String::new(),
            }],
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: 0,
        });

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 60,
            row: 1,
            modifiers: KeyModifiers::empty(),
        });

        assert_eq!(app.tool_focus, Some(0));
        assert_eq!(app.active_pane, Pane::SidebarList);
    }

    #[test]
    fn shift_down_extends_selection_and_copy_shortcut_copies_it() {
        let mut app = make_app();
        app.selection = Some(SelectionState::new(
            SelectablePane::Chat,
            crate::selection::SelectionPos { line: 0, col: 0 },
            crate::selection::SelectionPos { line: 0, col: 0 },
        ));
        app.chat_surface = Some(TextSurface::new(
            SelectablePane::Chat,
            Rect::new(0, 0, 40, 5),
            vec!["one".into(), "two".into(), "three".into()],
            0,
        ));

        app.handle_normal_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT))
            .unwrap();
        let selection = app.selection.clone().unwrap();
        assert_eq!(selection.focus.line, 1);

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
            .unwrap();
        assert!(app
            .messages
            .last()
            .unwrap()
            .content
            .contains("Copied selection"));
    }

    #[test]
    fn cmd_c_shortcut_is_treated_as_copy_when_selection_exists() {
        let mut app = make_app();
        app.selection = Some(SelectionState::new(
            SelectablePane::Chat,
            crate::selection::SelectionPos { line: 0, col: 0 },
            crate::selection::SelectionPos { line: 0, col: 0 },
        ));
        app.chat_surface = Some(TextSurface::new(
            SelectablePane::Chat,
            Rect::new(0, 0, 40, 5),
            vec!["one".into(), "two".into()],
            0,
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SUPER))
            .unwrap();

        assert!(app
            .messages
            .last()
            .unwrap()
            .content
            .contains("Copied selection"));
        assert_eq!(app.ctrl_c_count, 0);
    }

    #[test]
    fn drag_near_chat_edge_enables_and_clears_autoscroll() {
        let mut app = make_app();
        app.chat_surface = Some(TextSurface::new(
            SelectablePane::Chat,
            Rect::new(0, 0, 40, 5),
            vec![
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
                "f".into(),
            ],
            0,
        ));

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::empty(),
        });
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Drag(crossterm::event::MouseButton::Left),
            column: 1,
            row: 4,
            modifiers: KeyModifiers::empty(),
        });
        assert!(app.drag_autoscroll.is_some());

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Up(crossterm::event::MouseButton::Left),
            column: 1,
            row: 4,
            modifiers: KeyModifiers::empty(),
        });
        assert!(app.drag_autoscroll.is_none());
    }

    #[test]
    fn build_click_map_with_tool_calls() {
        use crate::highlight::Highlighter;
        use crate::theme::Theme;

        let theme = Theme::default();
        let highlighter = Highlighter::new();

        let messages = vec![
            DisplayMessage {
                role: MessageRole::User,
                content: "do something".into(),
                thinking: None,
                tool_calls: Vec::new(),
                assistant_blocks: Vec::new(),
                is_streaming: false,
                timestamp: 0,
            },
            DisplayMessage {
                role: MessageRole::Assistant,
                content: "ok".into(),
                thinking: None,
                tool_calls: vec![
                    crate::views::tools::DisplayToolCall {
                        id: "tc-1".into(),
                        name: "read".into(),
                        args_summary: "file.rs".into(),
                        output: Some("contents".into()),
                        details: serde_json::Value::Null,
                        is_error: false,
                        expanded: false,
                        streaming_lines: Vec::new(),
                        streaming_output: String::new(),
                    },
                    crate::views::tools::DisplayToolCall {
                        id: "tc-2".into(),
                        name: "edit".into(),
                        args_summary: "file.rs".into(),
                        output: Some("done".into()),
                        details: serde_json::Value::Null,
                        is_error: false,
                        expanded: false,
                        streaming_lines: Vec::new(),
                        streaming_output: String::new(),
                    },
                ],
                assistant_blocks: Vec::new(),
                is_streaming: false,
                timestamp: 0,
            },
        ];

        // Large chat area so everything is visible
        let area = Rect::new(0, 0, 80, 50);
        let click_map = crate::views::chat::build_click_map(
            &messages,
            &theme,
            &highlighter,
            area,
            0,
            true,
            imp_core::config::ChatToolDisplay::Interleaved,
            5,
            false,
        );

        // Should have 2 entries (one per tool call)
        assert_eq!(click_map.len(), 2);
        assert_eq!(click_map[0].1, "tc-1");
        assert_eq!(click_map[1].1, "tc-2");
        assert_eq!(click_map[1].0, click_map[0].0 + 1);
    }

    #[test]
    fn resumed_session_attaches_tool_results_persisted_before_assistant() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");

        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
        let session_path = session.path().unwrap().to_path_buf();

        let tool_result = imp_llm::ToolResultMessage {
            tool_call_id: "tc-1".into(),
            tool_name: "mana".into(),
            content: vec![imp_llm::ContentBlock::Text {
                text: "Invalid priority: 5".into(),
            }],
            is_error: true,
            details: serde_json::Value::Null,
            timestamp: imp_llm::now(),
        };

        let assistant = imp_llm::AssistantMessage {
            content: vec![
                imp_llm::ContentBlock::Text {
                    text: "Trying mana create".into(),
                },
                imp_llm::ContentBlock::ToolCall {
                    id: "tc-1".into(),
                    name: "mana".into(),
                    arguments: serde_json::json!({"action": "create", "priority": 5}),
                },
            ],
            usage: None,
            stop_reason: imp_llm::StopReason::ToolUse,
            timestamp: imp_llm::now(),
        };

        // Persist in the same order the runtime can produce: tool_result before assistant turn end.
        session
            .append(SessionEntry::Message {
                id: "tr1".into(),
                parent_id: None,
                message: imp_llm::Message::ToolResult(tool_result),
            })
            .unwrap();
        session
            .append(SessionEntry::Message {
                id: "a1".into(),
                parent_id: None,
                message: imp_llm::Message::Assistant(assistant),
            })
            .unwrap();

        let reopened = SessionManager::open(&session_path).unwrap();
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        let mut app = App::new(config, reopened, registry, cwd);
        app.load_session_messages();

        let tool_calls: Vec<&crate::views::tools::DisplayToolCall> = app
            .messages
            .iter()
            .flat_map(|m| m.tool_calls.iter())
            .collect();

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "tc-1");
        assert_eq!(tool_calls[0].output.as_deref(), Some("Invalid priority: 5"));
        assert!(tool_calls[0].is_error);
    }

    #[test]
    fn agent_end_does_not_double_count_usage_or_overwrite_context() {
        let mut app = make_app();
        let turn_usage = Usage {
            input_tokens: 500_000,
            output_tokens: 25_000,
            cache_read_tokens: 10_000,
            ..Usage::default()
        };
        let assistant = imp_llm::AssistantMessage {
            content: vec![imp_llm::ContentBlock::Text {
                text: "done".into(),
            }],
            usage: Some(turn_usage.clone()),
            stop_reason: imp_llm::StopReason::EndTurn,
            timestamp: 0,
        };

        app.handle_agent_event(AgentEvent::TurnEnd {
            index: 0,
            message: assistant,
        });
        app.handle_agent_event(AgentEvent::AgentEnd {
            usage: Usage {
                input_tokens: 1_000_000,
                output_tokens: 50_000,
                ..Usage::default()
            },
            cost: Cost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 3.0,
            },
        });

        assert_eq!(app.current_context_tokens, 510_000);
        assert_eq!(app.accumulated_usage.input_tokens, 500_000);
        assert_eq!(app.accumulated_usage.output_tokens, 25_000);
        assert_eq!(app.accumulated_cost.total, 3.0);
    }

    #[test]
    fn handle_ui_request_stores_and_removes_widgets() {
        let mut app = make_app();

        app.handle_ui_request(crate::tui_interface::UiRequest::SetWidget {
            key: "mana".into(),
            content: Some(imp_core::ui::WidgetContent::Lines(vec![
                "running unit 1".into(),
                "inspect with mana agents".into(),
            ])),
        });

        assert!(app.widgets.contains_key("mana"));

        app.handle_ui_request(crate::tui_interface::UiRequest::SetWidget {
            key: "mana".into(),
            content: None,
        });

        assert!(!app.widgets.contains_key("mana"));
    }

    #[test]
    fn custom_ui_request_returns_none_without_panicking() {
        let mut app = make_app();
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        app.handle_ui_request(crate::tui_interface::UiRequest::Custom {
            component: imp_core::ui::ComponentSpec {
                component_type: "mana-widget".into(),
                props: serde_json::json!({"state": "running"}),
                children: Vec::new(),
            },
            reply: tx,
        });

        assert_eq!(rx.try_recv().ok().flatten(), None);
    }
}
