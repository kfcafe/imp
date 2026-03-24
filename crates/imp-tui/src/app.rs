use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use imp_lua::LuaRuntime;

use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseEventKind,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use imp_core::agent::{AgentCommand, AgentEvent, AgentHandle};
use imp_core::builder::AgentBuilder;
use imp_core::config::Config;
use imp_core::session::{SessionEntry, SessionManager};
use imp_llm::auth::AuthStore;
use imp_llm::model::{ModelMeta, ModelRegistry};
use imp_llm::providers::create_provider;
use imp_llm::{Cost, Message, Model, StreamEvent, ThinkingLevel, Usage};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use ratatui::{Frame, Terminal};

use crate::highlight::Highlighter;
use crate::keybindings::{self, Action};
use crate::theme::Theme;
use crate::turn_tracker::TurnTracker;
use crate::views::chat::{ChatView, DisplayMessage, MessageRole};
use crate::views::command_palette::{builtin_commands, CommandPaletteState, CommandPaletteView};
use crate::views::editor::{EditorState, EditorView};
use crate::views::file_finder::{collect_project_files, FileFinderState, FileFinderView};
use crate::views::model_selector::{ModelSelectorState, ModelSelectorView};
use crate::views::session_picker::{SessionPickerState, SessionPickerView};
use crate::views::settings::{SettingsState, SettingsView};
use crate::views::sidebar::{Sidebar, SidebarView};
use crate::views::status::StatusInfo;
use crate::views::tools::DisplayToolCall;
use crate::views::top_bar::TopBar;
use crate::views::tree::{flatten_tree, TreeView, TreeViewState};
use crate::views::welcome::{needs_welcome, WelcomeState, WelcomeStep, WelcomeView};

type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Which pane currently has focus (for mouse-wheel routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Chat,
    Sidebar,
}

/// UI mode — determines what overlay is displayed.
#[derive(Debug)]
pub enum UiMode {
    Normal,
    ModelSelector(ModelSelectorState),
    CommandPalette(CommandPaletteState),
    FileFinder(FileFinderState),
    TreeView(TreeViewState),
    Settings(SettingsState),
    SessionPicker(SessionPickerState),
    Welcome(WelcomeState),
}

/// A queued message (steering or follow-up).
#[derive(Debug, Clone)]
pub enum QueuedMessage {
    Steer(String),
    FollowUp(String),
}

/// The TUI application state.
/// Holds the oneshot sender to reply to an ask tool request.
pub enum AskReply {
    Select(tokio::sync::oneshot::Sender<Option<usize>>),
    Input(tokio::sync::oneshot::Sender<Option<String>>),
}

pub struct App {
    // Core
    pub running: bool,
    pub messages: Vec<DisplayMessage>,
    pub editor: EditorState,
    pub cwd: PathBuf,

    // Agent
    pub agent_handle: Option<AgentHandle>,
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

    // Accumulated stats
    pub accumulated_usage: Usage,
    pub accumulated_cost: Cost,
    /// Last turn's input tokens — best proxy for actual current context size.
    pub current_context_tokens: u32,

    // Extension state
    pub status_items: HashMap<String, String>,

    /// Lua extension runtime (for command dispatch and hot-reload).
    pub lua_runtime: Option<Arc<Mutex<LuaRuntime>>>,

    // Sidebar
    pub sidebar: Sidebar,

    // Mouse click map: Y coordinate → tool_call_id (rebuilt each render)
    pub click_map: Vec<(u16, String)>,
    /// Which pane has focus for scroll routing.
    pub active_pane: Pane,
    /// Sidebar area cached from last render (for click detection).
    pub sidebar_rect: Option<Rect>,

    // Turn activity tracking
    pub turn_tracker: TurnTracker,

    // Display helpers
    pub theme: Theme,
    pub highlighter: Highlighter,
    pub model_registry: ModelRegistry,
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

        Self {
            running: true,
            messages: Vec::new(),
            editor: EditorState::new(),
            cwd,
            agent_handle: None,
            is_streaming: false,
            message_queue: Vec::new(),
            session,
            config,
            model_name,
            thinking_level,
            context_window: 200_000,
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
            accumulated_usage: Usage::default(),
            accumulated_cost: Cost::default(),
            current_context_tokens: 0,
            status_items: HashMap::new(),
            lua_runtime: None,
            sidebar: Sidebar::default(),
            click_map: Vec::new(),
            active_pane: Pane::Chat,
            sidebar_rect: None,
            turn_tracker: TurnTracker::new(),
            theme,
            highlighter: Highlighter::new(),
            model_registry,
        }
    }

    /// Load messages from the current session branch into display messages.
    pub fn load_session_messages(&mut self) {
        self.messages.clear();
        for msg in self.session.get_messages() {
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
                _ => self.messages.push(DisplayMessage::from_message(msg)),
            }
        }
    }

    /// Run the TUI event loop. Sets up terminal, runs the loop, restores terminal.
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load Lua extensions (for slash commands and tool registration)
        self.reload_lua_extensions();

        // Check for first-run welcome flow
        let config_dir = Config::user_config_dir();
        let auth_path = config_dir.join("auth.json");
        if needs_welcome(&config_dir, &auth_path) {
            let all_models = self.model_registry.list().to_vec();
            self.mode = UiMode::Welcome(WelcomeState::new(&all_models));
        }

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.event_loop(&mut terminal).await;

        // Restore terminal (always, even on error)
        disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn event_loop(&mut self, terminal: &mut Tui) -> Result<(), Box<dyn std::error::Error>> {
        let tick_rate = Duration::from_millis(16); // ~60fps

        loop {
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
            self.drain_agent_events();
            self.drain_ui_requests();

            // Tick + periodic redraw for streaming/spinner
            self.tick = self.tick.wrapping_add(1);
            if self.is_streaming {
                self.needs_redraw = true;
            }

            if !self.running {
                break;
            }
        }

        Ok(())
    }

    fn drain_agent_events(&mut self) {
        // Collect events first to avoid double-borrow of self
        let events: Vec<AgentEvent> = self
            .agent_handle
            .as_mut()
            .map(|h| {
                let mut evts = Vec::new();
                while let Ok(event) = h.event_rx.try_recv() {
                    evts.push(event);
                }
                evts
            })
            .unwrap_or_default();

        for event in events {
            self.handle_agent_event(event);
            self.needs_redraw = true;
        }
    }

    fn drain_ui_requests(&mut self) {
        use crate::tui_interface::UiRequest;
        use crate::views::ask_bar::{AskOption, AskState};

        let requests: Vec<UiRequest> = self
            .ui_rx
            .as_mut()
            .map(|rx| {
                let mut reqs = Vec::new();
                while let Ok(req) = rx.try_recv() {
                    reqs.push(req);
                }
                reqs
            })
            .unwrap_or_default();

        for req in requests {
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
                    self.ask_state = Some(AskState::new(title, String::new(), ask_options, false));
                    self.ask_reply = Some(AskReply::Select(reply));
                    self.needs_redraw = true;
                }
                UiRequest::Input {
                    title,
                    placeholder: _,
                    reply,
                } => {
                    self.ask_state = Some(AskState::new(title, String::new(), vec![], false));
                    self.ask_reply = Some(AskReply::Input(reply));
                    self.needs_redraw = true;
                }
                UiRequest::Confirm {
                    title,
                    message,
                    reply,
                } => {
                    // Render as a select with Yes/No
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
                    self.ask_state = Some(AskState::new(title, message, options, false));
                    // Wrap: convert selected index to bool
                    let (bool_tx, bool_rx) = tokio::sync::oneshot::channel();
                    self.ask_reply = Some(AskReply::Select(bool_tx));
                    // Spawn a task to convert index → bool
                    let confirm_reply = reply;
                    tokio::spawn(async move {
                        let result = bool_rx.await.ok().flatten();
                        let _ = confirm_reply.send(result.map(|idx| idx == 0)); // 0 = Yes
                    });
                    self.needs_redraw = true;
                }
                UiRequest::Notify { message, level: _ } => {
                    self.push_system_msg(&message);
                    self.needs_redraw = true;
                }
                UiRequest::SetStatus { key, text } => {
                    if let Some(t) = text {
                        self.status_items.insert(key, t);
                    } else {
                        self.status_items.remove(&key);
                    }
                    self.needs_redraw = true;
                }
                _ => {} // SetWidget, Custom — not yet handled
            }
        }
    }

    // ── Rendering ───────────────────────────────────────────────

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Editor height: at least 3 lines, up to 1/3 of screen
        let editor_height = (self.editor.line_count() as u16 + 2)
            .max(3)
            .min(area.height / 3);

        let ask_height = self.ask_state.as_ref().map(|s| s.height()).unwrap_or(0);

        let constraints = if ask_height > 0 {
            vec![
                Constraint::Length(1),             // top bar
                Constraint::Min(3),                // messages area
                Constraint::Length(ask_height),    // ask overlay
                Constraint::Length(editor_height), // editor (dimmed while ask active)
            ]
        } else {
            vec![
                Constraint::Length(1),             // top bar
                Constraint::Min(3),                // messages area
                Constraint::Length(editor_height), // editor
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let (top_bar_area, chat_area, ask_area, editor_area) = if ask_height > 0 {
            (chunks[0], chunks[1], Some(chunks[2]), chunks[3])
        } else {
            (chunks[0], chunks[1], None, chunks[2])
        };

        // Split chat area for sidebar when open
        let (chat_area, sidebar_area) = if self.sidebar.open && chat_area.width >= 60 {
            let min_sidebar = 30u16;
            let sidebar_w = (chat_area.width * 40 / 100)
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

        // Messages
        let chat = ChatView::new(&self.messages, &self.theme, &self.highlighter)
            .scroll(self.scroll_offset)
            .tick(self.tick)
            .tool_focus(self.tool_focus);
        frame.render_widget(chat, chat_area);

        // Build click_map: Y coordinate → tool_call_id for mouse click handling.
        // We mirror the ChatView line-building logic to know which rendered rows
        // correspond to tool call headers.
        self.click_map = build_click_map(
            &self.messages,
            &self.theme,
            &self.highlighter,
            chat_area,
            self.scroll_offset,
        );

        // Cache sidebar rect for mouse hit-testing
        self.sidebar_rect = sidebar_area;

        // Sidebar
        if let Some(sidebar_area) = sidebar_area {
            let tool_call = self
                .sidebar
                .tool_id
                .as_ref()
                .and_then(|id| self.find_tool_call(id));
            let view = SidebarView::new(tool_call, &self.theme, self.tick, self.sidebar.scroll);
            frame.render_widget(view, sidebar_area);
        }

        // Ask overlay (between chat and editor)
        if let (Some(ask_area), Some(ref state)) = (ask_area, &self.ask_state) {
            use crate::views::ask_bar::AskBar;
            frame.render_widget(AskBar::new(state), ask_area);
        }

        // Editor
        let editor = EditorView::new(&self.editor, &self.theme, self.thinking_level)
            .model(&self.model_name)
            .streaming(self.is_streaming)
            .queued(!self.message_queue.is_empty());
        frame.render_widget(editor, editor_area);

        // Top bar (header line)
        let status_info = self.build_status_info();
        let top_bar = TopBar::new(&status_info, &self.theme);
        frame.render_widget(top_bar, top_bar_area);

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
            UiMode::TreeView(state) => {
                let tree_area = centered_rect(80, 80, area);
                let view = TreeView::new(state, &self.theme);
                frame.render_widget(view, tree_area);
            }
            UiMode::Settings(state) => {
                let overlay_area = centered_rect(60, 60, area);
                let view = SettingsView::new(state, &self.theme);
                frame.render_widget(view, overlay_area);
            }
            UiMode::SessionPicker(state) => {
                let overlay_area = centered_rect(60, 50, area);
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
            let (cx, cy) = self.editor.cursor_screen_position(editor_area);
            frame.set_cursor_position((cx, cy));
        }
    }

    fn build_status_info(&self) -> StatusInfo {
        let cwd = self.cwd.to_string_lossy().to_string();
        let session_name = self
            .session
            .path()
            .and_then(|p| p.file_stem())
            .map(|s| {
                let name = s.to_string_lossy();
                if name.len() > 8 {
                    format!("{}…", &name[..7])
                } else {
                    name.to_string()
                }
            })
            .unwrap_or_default();

        let total_input = self.accumulated_usage.input_tokens;
        let total_output = self.accumulated_usage.output_tokens;
        // Use last turn's input_tokens as the actual context size rather than
        // accumulating across turns, which grows without bound and misrepresents
        // compacted conversations.
        let context_percent = if self.context_window > 0 {
            self.current_context_tokens as f64 / self.context_window as f64
        } else {
            0.0
        };

        StatusInfo {
            cwd,
            session_name,
            model: self.model_name.clone(),
            thinking: format!("{:?}", self.thinking_level),
            input_tokens: total_input,
            output_tokens: total_output,
            cost: self.accumulated_cost.total,
            context_percent,
            context_window: self.context_window,
            peek: self.tools_expanded,
            extension_items: self.status_items.clone(),
        }
    }

    // ── Key handling ────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        self.needs_redraw = true;

        // Reset ctrl+c counter on non-ctrl+c keypress
        if !(key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)) {
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
            UiMode::ModelSelector(_) | UiMode::CommandPalette(_) | UiMode::FileFinder(_) => {
                self.handle_overlay_key(key)
            }
            UiMode::TreeView(_) => self.handle_tree_key(key),
            UiMode::Settings(_) => self.handle_settings_key(key),
            UiMode::SessionPicker(_) => self.handle_session_picker_key(key),
            UiMode::Welcome(_) => self.handle_welcome_key(key),
        }

        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
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
                self.sidebar.open = !self.sidebar.open;
            }
            Some(Action::Peek) => {
                // Legacy alias — behaves the same as ToolToggle with no focus
                self.tools_expanded = !self.tools_expanded;
                for msg in &mut self.messages {
                    for tc in &mut msg.tool_calls {
                        tc.expanded = self.tools_expanded;
                    }
                }
            }
            Some(Action::ToolToggle) => {
                if let Some(idx) = self.tool_focus {
                    // Toggle just the focused tool call
                    if let Some(tc) = self.get_tool_call_mut(idx) {
                        tc.expanded = !tc.expanded;
                    }
                } else {
                    // No focus: toggle all (global expand/collapse)
                    self.tools_expanded = !self.tools_expanded;
                    for msg in &mut self.messages {
                        for tc in &mut msg.tool_calls {
                            tc.expanded = self.tools_expanded;
                        }
                    }
                }
            }
            Some(Action::ToolFocusNext) => {
                let total = self.total_tool_calls();
                if total > 0 {
                    self.tool_focus = Some(match self.tool_focus {
                        None => 0,
                        Some(idx) => (idx + 1).min(total - 1),
                    });
                }
            }
            Some(Action::ToolFocusPrev) => {
                let total = self.total_tool_calls();
                if total > 0 {
                    self.tool_focus = Some(match self.tool_focus {
                        None => total.saturating_sub(1),
                        Some(idx) => idx.saturating_sub(1),
                    });
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
                if !self.editor.move_up() {
                    self.editor.history_prev();
                }
            }
            Some(Action::CursorDown) => {
                if !self.editor.move_down() {
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
                self.scroll_offset += 20;
                self.auto_scroll = false;
            }
            Some(Action::ScrollDown) | Some(Action::PageDown) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(20);
                if self.scroll_offset == 0 {
                    self.auto_scroll = true;
                }
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
                _ => {}
            },
            Some(Action::OverlayDown) => match &mut self.mode {
                UiMode::ModelSelector(s) => s.move_down(),
                UiMode::CommandPalette(s) => s.move_down(),
                UiMode::FileFinder(s) => s.move_down(),
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
                if let Some(model) = state.selected_model() {
                    self.model_name = model.id.clone();
                    self.context_window = model.context_window;
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
            _ => {
                self.mode = old_mode;
            }
        }
    }

    fn handle_tree_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up => {
                if let UiMode::TreeView(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down => {
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

    /// Find a tool call by ID across all display messages.
    fn find_tool_call(&self, id: &str) -> Option<&DisplayToolCall> {
        for msg in self.messages.iter().rev() {
            for tc in &msg.tool_calls {
                if tc.id == id {
                    return Some(tc);
                }
            }
        }
        None
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

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        self.needs_redraw = true;
        match mouse.kind {
            MouseEventKind::ScrollUp => match self.active_pane {
                Pane::Chat => {
                    self.scroll_offset += 3;
                    self.auto_scroll = false;
                }
                Pane::Sidebar => {
                    self.sidebar.scroll_up(3);
                }
            },
            MouseEventKind::ScrollDown => match self.active_pane {
                Pane::Chat => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(3);
                    if self.scroll_offset == 0 {
                        self.auto_scroll = true;
                    }
                }
                Pane::Sidebar => {
                    self.sidebar.scroll_down(3);
                }
            },
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let col = mouse.column;
                let row = mouse.row;

                // Check if click is in the sidebar area
                if let Some(sr) = self.sidebar_rect {
                    if col >= sr.x && col < sr.x + sr.width && row >= sr.y && row < sr.y + sr.height
                    {
                        self.active_pane = Pane::Sidebar;
                        return;
                    }
                }

                // Click is in the chat area — set focus to chat
                self.active_pane = Pane::Chat;

                // Check click_map for tool call hits
                if let Some(tool_id) = self
                    .click_map
                    .iter()
                    .find(|(y, _)| *y == row)
                    .map(|(_, id)| id.clone())
                {
                    self.sidebar.follow(&tool_id);
                    self.sidebar.open = true;
                    self.active_pane = Pane::Sidebar;
                }
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
        let meta = self
            .model_registry
            .find_by_alias(&self.model_name)
            .cloned()
            .ok_or_else(|| format!("Unknown model: {}", self.model_name))?;

        let provider_name = meta.provider.clone();
        let provider = create_provider(&provider_name)
            .ok_or_else(|| format!("Unknown provider: {provider_name}"))?;

        let auth_path = Config::user_config_dir().join("auth.json");
        let mut auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));

        // Resolve API key with auto-refresh for expired OAuth tokens
        let api_key = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(auth_store.resolve_with_refresh(&provider_name))
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

        let mut messages: Vec<Message> = self.session.get_messages().into_iter().cloned().collect();
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
        tokio::spawn(async move {
            if let Err(e) = agent.run(prompt).await {
                eprintln!("[imp] agent error: {e}");
            }
        });

        self.agent_handle = Some(handle);
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
            is_streaming: false,
            timestamp: imp_llm::now(),
        });

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
            is_streaming: true,
            timestamp: imp_llm::now(),
        });

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
                is_streaming: false,
                timestamp: imp_llm::now(),
            });
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
                self.session = SessionManager::in_memory();
            }
            "compact" => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Compaction,
                    content: "Context compaction requested".into(),
                    thinking: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
            "hotkeys" => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::System,
                    content: [
                        "Keyboard shortcuts:",
                        "  Enter         Send message",
                        "  Shift+Enter   New line",
                        "  Ctrl+C        Clear / Abort / Quit",
                        "  Ctrl+L        Model selector",
                        "  Ctrl+O        Toggle tool output",
                        "  Ctrl+T        Toggle thinking",
                        "  Shift+Tab     Cycle thinking level",
                        "  @             File finder",
                        "  /command      Slash commands",
                        "  PageUp/Down   Scroll",
                    ]
                    .join("\n"),
                    thinking: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
            "settings" => {
                self.open_settings();
            }
            "resume" | "session" => {
                let session_dir = Config::session_dir();
                match SessionManager::list(&session_dir) {
                    Ok(sessions) if !sessions.is_empty() => {
                        self.mode = UiMode::SessionPicker(SessionPickerState::new(sessions));
                    }
                    Ok(_) => {
                        self.messages.push(DisplayMessage {
                            role: MessageRole::System,
                            content: "No saved sessions found.".into(),
                            thinking: None,
                            tool_calls: Vec::new(),
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
                            is_streaming: false,
                            timestamp: imp_llm::now(),
                        });
                    }
                }
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
            "help" => {
                self.push_system_msg(concat!(
                    "Commands:\n",
                    "  /new        — start fresh session\n",
                    "  /model      — switch model\n",
                    "  /compact    — compress context\n",
                    "  /resume     — resume a session\n",
                    "  /session    — browse sessions\n",
                    "  /fork       — branch conversation\n",
                    "  /name <n>   — rename session\n",
                    "  /export [f] — export to markdown\n",
                    "  /copy       — copy last response\n",
                    "  /reload     — reload config\n",
                    "  /settings   — edit settings\n",
                    "  /login      — OAuth login\n",
                    "  /help       — this message\n",
                    "  /quit       — exit",
                ));
            }
            "login" => {
                let provider = cmd.split_whitespace().nth(1).unwrap_or("anthropic");
                self.start_login(provider);
            }
            "welcome" | "setup" => {
                let all_models = self.model_registry.list().to_vec();
                self.mode = UiMode::Welcome(WelcomeState::new(&all_models));
            }
            "copy" => {
                // Copy last assistant message to clipboard
                if let Some(last) = self
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant || m.role == MessageRole::Error)
                {
                    let text = last.content.clone();
                    #[cfg(target_os = "macos")]
                    {
                        use std::io::Write;
                        if let Ok(mut child) = std::process::Command::new("pbcopy")
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                        {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(text.as_bytes());
                            }
                            let _ = child.wait();
                        }
                    }
                    #[cfg(target_os = "linux")]
                    {
                        use std::io::Write;
                        if let Ok(mut child) = std::process::Command::new("xclip")
                            .args(["-selection", "clipboard"])
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                        {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(text.as_bytes());
                            }
                            let _ = child.wait();
                        }
                    }
                    self.messages.push(DisplayMessage {
                        role: MessageRole::System,
                        content: "Copied to clipboard.".into(),
                        thinking: None,
                        tool_calls: Vec::new(),
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
                        is_streaming: false,
                        timestamp: imp_llm::now(),
                    });
                }
            }
        }
        self.editor.clear();
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

    fn start_login(&mut self, provider: &str) {
        if provider != "anthropic" {
            self.messages.push(DisplayMessage {
                role: MessageRole::Error,
                content: format!("Login for '{provider}' not supported. Set API key via env var."),
                thinking: None,
                tool_calls: Vec::new(),
                is_streaming: false,
                timestamp: imp_llm::now(),
            });
            return;
        }

        self.messages.push(DisplayMessage {
            role: MessageRole::System,
            content: "Opening browser for Anthropic login...".into(),
            thinking: None,
            tool_calls: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });

        // Run OAuth flow in background
        let auth_path = Config::user_config_dir().join("auth.json");
        tokio::spawn(async move {
            let oauth = imp_llm::oauth::anthropic::AnthropicOAuth::new();
            match oauth
                .login(
                    |url| {
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
                    },
                    || async { None }, // No manual fallback in TUI — browser only
                )
                .await
            {
                Ok(credential) => {
                    let mut store = AuthStore::load(&auth_path)
                        .unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
                    let _ = store.store(
                        "anthropic",
                        imp_llm::auth::StoredCredential::OAuth(credential),
                    );
                    // Note: can't push messages from here without a channel.
                    // The user will see it worked next time they send a message.
                }
                Err(e) => {
                    eprintln!("OAuth login failed: {e}");
                }
            }
        });
    }

    fn open_settings(&mut self) {
        let models = self.filtered_models();
        let state = SettingsState::new(&self.config, &self.model_name, &models);
        self.mode = UiMode::Settings(state);
    }

    fn handle_session_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up => {
                if let UiMode::SessionPicker(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down => {
                if let UiMode::SessionPicker(ref mut state) = self.mode {
                    state.move_down();
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
                            self.messages.push(DisplayMessage {
                                role: MessageRole::System,
                                content: "Session resumed.".into(),
                                thinking: None,
                                tool_calls: Vec::new(),
                                is_streaming: false,
                                timestamp: imp_llm::now(),
                            });
                        }
                        Err(e) => {
                            self.messages.push(DisplayMessage {
                                role: MessageRole::Error,
                                content: format!("Failed to open session: {e}"),
                                thinking: None,
                                tool_calls: Vec::new(),
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
        let Some(ref mut state) = self.ask_state else {
            return;
        };

        match key.code {
            KeyCode::Up => state.cursor_up(),
            KeyCode::Down => state.cursor_down(),
            KeyCode::Tab => state.tab_to_edit(),
            KeyCode::Char(' ') if !state.input_active => state.toggle_current(),
            KeyCode::Char(c) if !state.input_active && c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                if state.quick_select(n) {
                    // Quick-select + auto-confirm
                    self.finish_ask();
                }
            }
            KeyCode::Char(c) => state.type_char(c),
            KeyCode::Backspace => state.backspace(),
            KeyCode::Enter => {
                self.finish_ask();
            }
            KeyCode::Esc => {
                self.cancel_ask();
            }
            _ => {}
        }
    }

    fn finish_ask(&mut self) {
        use crate::views::ask_bar::AskResult;

        let state = self.ask_state.take();
        let reply = self.ask_reply.take();

        let Some(state) = state else { return };
        let result = state.confirm();

        // Show Q&A in chat
        self.push_system_msg(&format!("❯ {}", state.question));

        match (&result, reply) {
            (AskResult::Text(text), Some(AskReply::Input(tx))) => {
                self.push_system_msg(&format!("  {text}"));
                let _ = tx.send(Some(text.clone()));
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

    fn cancel_ask(&mut self) {
        self.ask_state = None;
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
                    if state.editing_number {
                        state.push_char(c);
                    }
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
                    let can_advance = if let UiMode::Welcome(ref mut state) = self.mode {
                        state.check_auth_resolved()
                    } else {
                        false
                    };
                    if can_advance {
                        if let UiMode::Welcome(ref mut state) = self.mode {
                            state.advance();
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
                    self.finish_welcome();
                }
                KeyCode::Esc => {
                    if let UiMode::Welcome(ref mut state) = self.mode {
                        state.go_back();
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
        let (model_id, thinking, provider_id, resolved_key) = match &self.mode {
            UiMode::Welcome(state) => {
                let model_id = state
                    .selected_model()
                    .map(|m| m.id.clone())
                    .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
                let thinking = state.thinking_level;
                let provider_id = state.selected_provider_id();
                let resolved_key = state.resolved_key.clone();
                (model_id, thinking, provider_id, resolved_key)
            }
            _ => return,
        };

        // Update in-session config
        self.config.model = Some(model_id.clone());
        self.config.thinking = Some(thinking);
        self.model_name = model_id;
        self.thinking_level = thinking;

        if let Some(meta) = self.model_registry.find_by_alias(&self.model_name) {
            self.context_window = meta.context_window;
        }

        // Save config.toml
        let config_path = Config::user_config_path();
        if let Err(e) = self.config.save(&config_path) {
            self.messages.push(DisplayMessage {
                role: MessageRole::Error,
                content: format!("Failed to save config: {e}"),
                thinking: None,
                tool_calls: Vec::new(),
                is_streaming: false,
                timestamp: imp_llm::now(),
            });
        }

        // Save API key if one was manually entered
        if let Some(key) = resolved_key {
            let auth_path = Config::user_config_dir().join("auth.json");
            let mut auth_store =
                AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
            if let Err(e) = auth_store.store(
                provider_id.provider_id(),
                imp_llm::auth::StoredCredential::ApiKey { key },
            ) {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: format!("Failed to save API key: {e}"),
                    thinking: None,
                    tool_calls: Vec::new(),
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

        // Update context window from registry
        if let Some(meta) = self.model_registry.find_by_alias(&self.model_name) {
            self.context_window = meta.context_window;
        }

        // Persist to user config.toml
        let config_path = Config::user_config_path();
        match self.config.save(&config_path) {
            Ok(()) => {
                if let UiMode::Settings(ref mut s) = self.mode {
                    s.dirty = false;
                }
                self.messages.push(DisplayMessage {
                    role: MessageRole::System,
                    content: format!("Settings saved to {}", config_path.display()),
                    thinking: None,
                    tool_calls: Vec::new(),
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
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
        }
    }

    /// Return models filtered by `config.enabled_models` (if set).
    /// Entries in the enabled list can be canonical IDs or short aliases —
    /// each is resolved through the registry before matching.
    fn filtered_models(&self) -> Vec<ModelMeta> {
        let all = self.model_registry.list();
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
            _ => all.to_vec(),
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
    }

    fn cycle_thinking_level(&mut self) {
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
            is_streaming: false,
            timestamp: imp_llm::now(),
        });
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
                    let preview = if output.len() > 200 {
                        &output[..200]
                    } else {
                        output
                    };
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
                self.turn_tracker.reset();
            }
            AgentEvent::AgentEnd { usage, cost } => {
                // Track actual context size: input_tokens on each API call reflects
                // the full conversation sent to the model, including any compaction.
                self.current_context_tokens = usage.input_tokens;
                self.accumulated_usage.add(&usage);
                self.accumulated_cost.total += cost.total;
                self.accumulated_cost.input += cost.input;
                self.accumulated_cost.output += cost.output;
                self.is_streaming = false;

                // Mark last streaming message as done
                if let Some(last) = self.messages.last_mut() {
                    last.is_streaming = false;
                }

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
                            last.content.push_str(&text);
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
                            last.tool_calls.push(DisplayToolCall {
                                id,
                                args_summary: DisplayToolCall::make_args_summary(&name, &arguments),
                                name,
                                output: None,
                                is_error: false,
                                expanded: self.tools_expanded,
                                streaming_lines: Vec::new(),
                            });
                        }
                        _ => {}
                    }
                }
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
                // Sidebar: auto-follow the new tool
                self.sidebar.follow(&tool_call_id);
                // Auto-open on first tool if terminal is wide enough
                if !self.sidebar.first_tool_seen {
                    self.sidebar.first_tool_seen = true;
                    let (cols, _) = crossterm::terminal::size().unwrap_or((80, 24));
                    if cols >= 120 {
                        self.sidebar.open = true;
                    }
                }
            }
            AgentEvent::ToolOutputDelta { tool_call_id, text } => {
                // Feed streaming output into the tool call's rolling buffer
                for msg in self.messages.iter_mut().rev() {
                    for tc in &mut msg.tool_calls {
                        if tc.id == tool_call_id && tc.output.is_none() {
                            // Append text and keep last 5 lines
                            for line in text.lines() {
                                tc.streaming_lines.push(line.to_string());
                            }
                            if tc.streaming_lines.len() > 5 {
                                let excess = tc.streaming_lines.len() - 5;
                                tc.streaming_lines.drain(..excess);
                            }
                            break;
                        }
                    }
                }
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
                            tc.is_error = is_error;
                            // Auto-expand failed tool calls so the error is immediately visible
                            if is_error {
                                tc.expanded = true;
                            }
                            break;
                        }
                    }
                }

                // Persist tool result to session so resume has full conversation
                let msg_id = uuid::Uuid::new_v4().to_string();
                let _ = self.session.append(SessionEntry::Message {
                    id: msg_id,
                    parent_id: None,
                    message: imp_llm::Message::ToolResult(result),
                });
            }
            AgentEvent::TurnEnd { message, .. } => {
                // Update context tracking from this turn's usage
                if let Some(ref usage) = message.usage {
                    self.current_context_tokens = usage.input_tokens + usage.cache_read_tokens;
                    self.accumulated_usage.add(usage);
                }

                // Persist assistant message to session
                let msg_id = uuid::Uuid::new_v4().to_string();
                let _ = self.session.append(SessionEntry::Message {
                    id: msg_id,
                    parent_id: None,
                    message: imp_llm::Message::Assistant(message),
                });
            }
            AgentEvent::CompactionEnd { summary } => {
                // Context shrunk — reset so % shows 0 until the next turn updates it.
                self.current_context_tokens = 0;
                self.messages.push(DisplayMessage {
                    role: MessageRole::Compaction,
                    content: summary,
                    thinking: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
            }
            AgentEvent::Error { error } => {
                // Stop streaming — errors can be terminal (no AgentEnd follows)
                self.is_streaming = false;
                if let Some(last) = self.messages.last_mut() {
                    last.is_streaming = false;
                }

                // Parse the error for a cleaner display
                let display_error = parse_api_error(&error);

                self.messages.push(DisplayMessage {
                    role: MessageRole::Error,
                    content: display_error,
                    thinking: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
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
    // Try to extract JSON from the error string
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
    raw.to_string()
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

/// Build a click map: Vec<(screen_y, tool_call_id)> for each tool call header
/// line that is visible in the chat area.  Mirrors the line-counting logic in
/// `ChatView::render` so mouse clicks land on the right tool call.
fn build_click_map(
    messages: &[DisplayMessage],
    theme: &Theme,
    highlighter: &Highlighter,
    chat_area: Rect,
    scroll_offset: usize,
) -> Vec<(u16, String)> {
    // Step 1: walk messages and record (line_index, tool_call_id) for each
    // tool call header line.  The line numbering matches ChatView's all_lines.
    let mut tool_line_indices: Vec<(usize, String)> = Vec::new();
    let mut line_idx: usize = 0;

    for msg in messages {
        match msg.role {
            MessageRole::User => {
                // Same counting as ChatView: 1 line for prefix+first line, then 1 per extra line
                let content_lines = msg.content.lines().count().max(1);
                line_idx += content_lines;
            }
            MessageRole::Assistant => {
                // Thinking tail (up to 5 lines)
                if let Some(ref thinking) = msg.thinking {
                    if !thinking.is_empty() {
                        let count = thinking.lines().count().min(5);
                        line_idx += count;
                    }
                }
                // Markdown-rendered content
                if !msg.content.is_empty() {
                    let rendered =
                        crate::markdown::render_markdown(&msg.content, theme, highlighter);
                    line_idx += rendered.len();
                }
                // Streaming indicator
                if msg.is_streaming {
                    line_idx += 1;
                }
            }
            MessageRole::System => {
                line_idx += msg.content.lines().take(3).count().max(0);
            }
            MessageRole::Compaction => {
                line_idx += 1;
            }
            MessageRole::Error => {
                line_idx += 1;
            }
        }

        // Tool calls
        for tc in &msg.tool_calls {
            let is_running = tc.output.is_none() && !tc.is_error;
            // Header line — this is the clickable line
            tool_line_indices.push((line_idx, tc.id.clone()));
            line_idx += 1;

            // Running with streaming output
            if is_running && !tc.streaming_lines.is_empty() {
                line_idx += tc.streaming_lines.len();
            }

            // Expanded output
            if tc.expanded && !is_running {
                if let Some(ref output) = tc.output {
                    line_idx += output.lines().count().min(50);
                }
            }
        }

        // Separator
        line_idx += 1;
    }

    // Step 2: determine which lines are visible (same scroll logic as ChatView)
    let total_lines = line_idx;
    let visible_height = chat_area.height as usize;

    let start = if scroll_offset == 0 {
        total_lines.saturating_sub(visible_height)
    } else {
        total_lines.saturating_sub(visible_height + scroll_offset)
    };

    let end = total_lines.min(start + visible_height);

    // Step 3: map visible tool-call line indices to screen Y coordinates
    let mut result = Vec::new();
    for (li, id) in &tool_line_indices {
        if *li >= start && *li < end {
            let screen_y = chat_area.y + (*li - start) as u16;
            result.push((screen_y, id.clone()));
        }
    }

    result
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

    // ── 1. App::new creates with config + session ───────────────

    #[test]
    fn tui_integration_app_new_defaults() {
        let app = make_app();

        assert!(app.running);
        assert!(app.messages.is_empty());
        assert_eq!(app.model_name, "sonnet");
        assert_eq!(app.thinking_level, ThinkingLevel::Medium);
        assert_eq!(app.context_window, 200_000);
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
    fn tui_integration_slash_compact_adds_marker() {
        let mut app = make_app();

        app.execute_command("compact");

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::Compaction);
        assert!(app.messages[0].content.contains("compaction"));
    }

    #[test]
    fn tui_integration_slash_quit_stops_app() {
        let mut app = make_app();
        assert!(app.running);

        app.execute_command("quit");
        assert!(!app.running);
    }

    #[test]
    fn tui_integration_slash_unknown_shows_error() {
        let mut app = make_app();

        app.execute_command("nonexistent");

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::Error);
        assert!(app.messages[0].content.contains("nonexistent"));
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
    fn click_map_defaults_empty() {
        let app = make_app();
        assert!(app.click_map.is_empty());
        assert_eq!(app.active_pane, Pane::Chat);
        assert!(app.sidebar_rect.is_none());
    }

    #[test]
    fn mouse_click_on_tool_call_opens_sidebar() {
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
                is_error: false,
                expanded: false,
                streaming_lines: Vec::new(),
            }],
            is_streaming: false,
            timestamp: 0,
        });

        // Pre-populate click_map as if render had run
        app.click_map = vec![(5, "tc-42".into())];

        // Simulate a mouse click at row 5
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);

        assert!(app.sidebar.open);
        assert_eq!(app.sidebar.tool_id.as_deref(), Some("tc-42"));
        assert_eq!(app.active_pane, Pane::Sidebar);
    }

    #[test]
    fn mouse_click_on_sidebar_sets_focus() {
        let mut app = make_app();
        app.sidebar.open = true;
        app.sidebar_rect = Some(Rect::new(50, 1, 30, 20));

        // Click inside sidebar
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 60,
            row: 10,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);

        assert_eq!(app.active_pane, Pane::Sidebar);
    }

    #[test]
    fn mouse_click_on_chat_area_sets_chat_focus() {
        let mut app = make_app();
        app.active_pane = Pane::Sidebar;
        app.sidebar_rect = Some(Rect::new(50, 1, 30, 20));

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
    fn mouse_scroll_routes_by_active_pane() {
        let mut app = make_app();

        // Scroll up in chat pane
        app.active_pane = Pane::Chat;
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse);
        assert_eq!(app.scroll_offset, 3);
        assert!(!app.auto_scroll);

        // Scroll down in sidebar pane
        app.active_pane = Pane::Sidebar;
        app.sidebar.scroll = 10;
        let mouse_down = crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        app.handle_mouse(mouse_down);
        assert_eq!(app.sidebar.scroll, 7);
        // Chat scroll should be unchanged
        assert_eq!(app.scroll_offset, 3);
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
                        is_error: false,
                        expanded: false,
                        streaming_lines: Vec::new(),
                    },
                    crate::views::tools::DisplayToolCall {
                        id: "tc-2".into(),
                        name: "edit".into(),
                        args_summary: "file.rs".into(),
                        output: Some("done".into()),
                        is_error: false,
                        expanded: false,
                        streaming_lines: Vec::new(),
                    },
                ],
                is_streaming: false,
                timestamp: 0,
            },
        ];

        // Large chat area so everything is visible
        let area = Rect::new(0, 0, 80, 50);
        let click_map = super::build_click_map(&messages, &theme, &highlighter, area, 0);

        // Should have 2 entries (one per tool call)
        assert_eq!(click_map.len(), 2);
        assert_eq!(click_map[0].1, "tc-1");
        assert_eq!(click_map[1].1, "tc-2");
        // tc-2 should be on the row after tc-1
        assert_eq!(click_map[1].0, click_map[0].0 + 1);
    }
}
