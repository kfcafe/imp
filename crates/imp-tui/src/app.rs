use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use imp_core::format_error_for_display;
use imp_core::tools::mana::ManaTool;
use imp_core::tools::Tool;
use imp_core::ui::WidgetContent;
use imp_core::{mana_run_summary, stop_mana_run, ManaRunSummary, ManaUnitRef, TurnManaReview};
use mana_core::api;
use mana_core::api::TreeNode;
use mana_core::unit::Status;

use imp_lua::loader::discover_extensions;
use imp_lua::LuaRuntime;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind};
use imp_core::agent::{AgentCommand, AgentEvent, AgentHandle};
use imp_core::builder::AgentBuilder;
use imp_core::compaction::{
    execute_compaction_with_retry, execute_manual_compaction, prepare_messages_for_compaction,
    select_compaction_strategy, CompactionCapabilities, CompactionStrategy,
    COMPACTION_SUMMARY_PREFIX, DEFAULT_KEEP_RECENT_GROUPS,
};
use imp_core::config::Config;
use imp_core::personality::default_soul_markdown;
use imp_core::session::{SessionEntry, SessionManager};
use imp_core::Error as ImpCoreError;
use imp_llm::auth::AuthStore;
use imp_llm::model::{ModelMeta, ModelRegistry, ProviderRegistry};
use imp_llm::providers::create_provider;
use imp_llm::{
    truncate_chars_with_suffix, Cost, Message, Model, StreamEvent, ThinkingLevel, Usage,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Clear;
use ratatui::Frame;

use crate::animation::{title_breather_frame, title_working_glyph, AnimationState};
use crate::highlight::Highlighter;
use crate::keybindings::{self, Action};
use crate::selection::{
    extract_selected_text, SelectablePane, SelectionOverlay, SelectionState, TextSurface,
};
use crate::terminal::{ring_terminal_bell, set_window_title, InteractiveTerminal};
use crate::theme::Theme;
use crate::turn_tracker::TurnTracker;
use crate::views::ask_bar::AskState;
use crate::views::chat::{
    build_chat_render_data, build_click_map, build_text_surface_from_lines,
    clamped_scroll_offset_for_total_lines, scroll_offset_for_message_at_top, DisplayMessage,
    MessageRole, RenderedChatView,
};
use crate::views::command_palette::{
    builtin_commands, merge_extension_commands, merge_skill_commands, CommandPaletteState,
    CommandPaletteView,
};
use crate::views::editor::{EditorState, EditorView, WorkflowMode};
use crate::views::file_finder::{collect_project_files, FileFinderState, FileFinderView};
use crate::views::login_picker::{login_providers, LoginPickerState, LoginPickerView};
use crate::views::mana_navigator::{ManaNavigatorState, ManaNavigatorView};
use crate::views::model_selector::{ModelSelection, ModelSelectorState, ModelSelectorView};
use crate::views::personality::{PersonalityScope, PersonalityState, PersonalityView};
use crate::views::secrets_picker::{secret_providers, SecretsPickerState, SecretsPickerView};
use crate::views::session_picker::{SessionPickerState, SessionPickerView};
use crate::views::settings::{SettingsState, SettingsView};
use crate::views::sidebar::{
    build_detail_render_data, build_detail_text_surface_from_plain_lines, build_stream_lines,
    sidebar_sub_areas, thinking_detail_render_data, Sidebar, SidebarDetailRenderData, SidebarView,
};
use crate::views::startup::{
    action_block_height, summarize_inline, visible_section_count, StartupAction, StartupPanelData,
    StartupPanelView, StartupSection,
};
use crate::views::status::StatusInfo;
use crate::views::tools::DisplayToolCall;
use crate::views::tree::{flatten_tree, TreeView, TreeViewState};
use crate::views::welcome::{needs_welcome, WelcomeState, WelcomeStep, WelcomeView};

const LUA_RESTART_DIRECTIVE: &str = "__IMP_RESTART_AFTER_COMMAND__";

fn lua_result_requests_restart(result: Option<&str>) -> bool {
    result.is_some_and(|text| {
        text.lines()
            .any(|line| line.trim() == LUA_RESTART_DIRECTIVE)
    })
}

fn strip_lua_restart_directive(result: &str) -> String {
    result
        .lines()
        .filter(|line| line.trim() != LUA_RESTART_DIRECTIVE)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

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
    ManaNavigator(ManaNavigatorState),
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

impl QueuedMessage {
    fn text(&self) -> &str {
        match self {
            QueuedMessage::Steer(text) | QueuedMessage::FollowUp(text) => text,
        }
    }
}

pub enum AskReply {
    Select(tokio::sync::oneshot::Sender<Option<usize>>),
    MultiSelect(tokio::sync::oneshot::Sender<Option<Vec<usize>>>),
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

const MAX_RUNTIME_SIGNALS_PER_TICK: usize = 64;
const MAX_UI_REQUESTS_PER_TICK: usize = 16;

#[derive(Debug)]
enum RuntimeSignal {
    AgentEvent(AgentEvent),
    AgentTaskCompleted,
    AgentTaskFailed(String),
    CompactionTaskCompleted(String),
    CompactionTaskFailed(String),
    LuaCommandCompleted {
        command: String,
        result: Option<String>,
    },
    LuaCommandRestartRequested {
        command: String,
        result: Option<String>,
    },
    LuaCommandFailed {
        command: String,
        error: String,
    },
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
struct ThemeKind {
    is_light: bool,
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
    theme: ThemeKind,
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
    theme: ThemeKind,
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
    thinking_hash: u64,
    run_hash: u64,
    word_wrap: bool,
    tool_output_lines: usize,
    animation_level: imp_core::config::AnimationLevel,
    theme: ThemeKind,
}

#[derive(Debug)]
struct SidebarDetailCache {
    key: SidebarDetailCacheKey,
    render: SidebarDetailRenderData,
}

#[derive(Debug, Clone)]
struct StartupSurfaceData {
    panel: StartupPanelData,
}

#[derive(Debug, Clone, Copy)]
struct StartupSkillHit {
    index: usize,
    rect: Rect,
}

fn mana_run_summary_cache_key(run: &ManaRunSummary) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        run.run_id,
        run.scope,
        run.status,
        run.total_units,
        run.total_closed,
        run.total_failed,
        run.total_awaiting_verify,
        run.latest.as_deref().unwrap_or(""),
        run.logs.join("\n")
    )
}

fn mana_run_detail_render_data(run: &ManaRunSummary, theme: &Theme) -> SidebarDetailRenderData {
    let mut lines = vec![Line::from(vec![
        Span::styled("╭─", theme.muted_style()),
        Span::styled(
            " mana run ",
            theme.accent_style().add_modifier(Modifier::BOLD),
        ),
        Span::styled("─╮", theme.muted_style()),
    ])];
    let mut plain_lines = vec![
        format!("run: {}", run.run_id),
        format!("status: {}", run.status),
        format!("scope: {}", run.scope),
        format!(
            "units: {} closed / {} total",
            run.total_closed, run.total_units
        ),
        format!("failed: {}", run.total_failed),
        format!("awaiting verify: {}", run.total_awaiting_verify),
    ];
    if !run.agents.is_empty() {
        plain_lines.push("agents:".to_string());
        for agent in run.agents.iter().take(8) {
            plain_lines.push(format!(
                "  {} {} · {} · {}",
                agent.unit_id, agent.status, agent.action, agent.title
            ));
        }
    }
    if !run.agents.is_empty() {
        plain_lines.push("agents:".to_string());
        for agent in run.agents.iter().take(8) {
            plain_lines.push(format!(
                "  {} {} · {} · {}",
                agent.unit_id, agent.status, agent.action, agent.title
            ));
        }
    }
    if !run.agents.is_empty() {
        plain_lines.push("agents:".to_string());
        for agent in run.agents.iter().take(8) {
            plain_lines.push(format!(
                "  {} {} · {} · {}",
                agent.unit_id, agent.status, agent.action, agent.title
            ));
        }
    }
    let recent_logs = run.logs.iter().rev().take(12).collect::<Vec<_>>();
    if recent_logs.is_empty() {
        plain_lines.push("log: —".to_string());
    } else {
        plain_lines.push("log:".to_string());
        for log in recent_logs.into_iter().rev() {
            plain_lines.push(format!("  {log}"));
        }
    }
    for (index, line) in plain_lines.iter().enumerate() {
        let style = if index == 0 || index == 1 {
            theme.accent_style()
        } else if line == "log: —" || line.ends_with('—') {
            theme.muted_style()
        } else if line.starts_with("failed:") && !line.ends_with('0') {
            theme.warning_style()
        } else {
            theme.style()
        };
        lines.push(Line::from(Span::styled(line.clone(), style)));
    }
    SidebarDetailRenderData { lines, plain_lines }
}

fn startup_skill_detail_render_data(
    skill: &imp_core::resources::Skill,
    theme: &Theme,
) -> SidebarDetailRenderData {
    let mut plain_lines = vec![
        format!("skill: {}", skill.name),
        format!("path: {}", skill.path.display()),
    ];
    if !skill.description.trim().is_empty() {
        plain_lines.push(format!("description: {}", skill.description.trim()));
    }
    plain_lines.push(String::new());

    match std::fs::read_to_string(&skill.path) {
        Ok(content) => plain_lines.extend(content.lines().map(str::to_string)),
        Err(err) => plain_lines.push(format!("Failed to read skill: {err}")),
    }

    let lines = plain_lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                Line::from(Span::styled(
                    line.clone(),
                    theme.accent_style().add_modifier(Modifier::BOLD),
                ))
            } else if index <= 2 && !line.is_empty() {
                Line::from(Span::styled(line.clone(), theme.muted_style()))
            } else {
                Line::from(Span::raw(line.clone()))
            }
        })
        .collect();

    SidebarDetailRenderData { lines, plain_lines }
}

fn startup_skill_hits(area: Rect, panel: &StartupPanelData) -> Vec<StartupSkillHit> {
    if area.width < 24 || area.height < 8 {
        return Vec::new();
    }

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let sections_area = if inner.height < 12 {
        let action_height = 3.min(inner.height);
        Rect {
            y: inner.y + action_height,
            height: inner.height.saturating_sub(action_height),
            ..inner
        }
    } else {
        let action_height = action_block_height(inner.width, panel.actions.len());
        Rect {
            y: inner.y + action_height,
            height: inner.height.saturating_sub(action_height),
            ..inner
        }
    };

    startup_skill_hits_in_sections(sections_area, &panel.sections)
}

fn startup_skill_hits_in_sections(area: Rect, sections: &[StartupSection]) -> Vec<StartupSkillHit> {
    if sections.is_empty() || area.height == 0 || area.width == 0 {
        return Vec::new();
    }

    let visible_count = visible_section_count(area.width, area.height, sections.len());
    let visible_sections = &sections[..visible_count];

    if area.width >= 96 {
        let column_width = area.width / 4;
        let remainder = area.width % 4;
        return visible_sections
            .iter()
            .enumerate()
            .flat_map(|(index, section)| {
                let x_offset = column_width * index as u16 + remainder.min(index as u16);
                let width = column_width + u16::from((index as u16) < remainder);
                let rect = Rect {
                    x: area.x + x_offset,
                    width,
                    ..area
                };
                startup_skill_hits_in_section(rect, section)
            })
            .collect();
    }

    match visible_sections.len() {
        0 => Vec::new(),
        1 => startup_skill_hits_in_section(area, &visible_sections[0]),
        2 => {
            let rects = if area.width >= 90 {
                split_horizontal(area, &[50, 50])
            } else {
                split_vertical(area, &[50, 50])
            };
            visible_sections
                .iter()
                .zip(rects)
                .flat_map(|(section, rect)| startup_skill_hits_in_section(rect, section))
                .collect()
        }
        3 => {
            let rects = if area.width >= 120 {
                split_horizontal(area, &[33, 34, 33])
            } else if area.width >= 78 && area.height >= 12 {
                let rows = split_vertical(area, &[50, 50]);
                let top = split_horizontal(rows[0], &[50, 50]);
                vec![top[0], top[1], rows[1]]
            } else {
                split_vertical(area, &[34, 33, 33])
            };
            visible_sections
                .iter()
                .zip(rects)
                .flat_map(|(section, rect)| startup_skill_hits_in_section(rect, section))
                .collect()
        }
        _ => {
            let row_height = (area.height / visible_sections.len() as u16).max(3);
            visible_sections
                .iter()
                .enumerate()
                .flat_map(|(index, section)| {
                    let rect = Rect {
                        y: area.y + row_height * index as u16,
                        height: row_height,
                        ..area
                    };
                    startup_skill_hits_in_section(rect, section)
                })
                .collect()
        }
    }
}

fn startup_skill_hits_in_section(area: Rect, section: &StartupSection) -> Vec<StartupSkillHit> {
    if section.title != "skills" || area.height < 3 || area.width < 12 {
        return Vec::new();
    }

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    section
        .lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line.strip_prefix("• ")
                .is_some_and(|name| name != "none discovered")
        })
        .filter_map(|(index, _)| {
            let y = inner.y + index as u16;
            (y < inner.y + inner.height).then_some(StartupSkillHit {
                index,
                rect: Rect {
                    y,
                    height: 1,
                    ..inner
                },
            })
        })
        .collect()
}

fn split_horizontal(area: Rect, percentages: &[u16]) -> Vec<Rect> {
    let mut x = area.x;
    let mut used = 0u16;
    percentages
        .iter()
        .enumerate()
        .map(|(index, pct)| {
            let width = if index + 1 == percentages.len() {
                area.width.saturating_sub(used)
            } else {
                area.width * *pct / 100
            };
            let rect = Rect { x, width, ..area };
            x = x.saturating_add(width);
            used = used.saturating_add(width);
            rect
        })
        .collect()
}

fn split_vertical(area: Rect, percentages: &[u16]) -> Vec<Rect> {
    let mut y = area.y;
    let mut used = 0u16;
    percentages
        .iter()
        .enumerate()
        .map(|(index, pct)| {
            let height = if index + 1 == percentages.len() {
                area.height.saturating_sub(used)
            } else {
                area.height * *pct / 100
            };
            let rect = Rect { y, height, ..area };
            y = y.saturating_add(height);
            used = used.saturating_add(height);
            rect
        })
        .collect()
}

fn is_build_team_intent(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "use a team"
            | "build this with a team"
            | "parallelize this"
            | "run a team"
            | "use workers"
            | "run workers"
            | "team build"
            | "build with workers"
    )
}

fn is_build_continue_intent(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "continue"
            | "go"
            | "build"
            | "keep going"
            | "finish"
            | "finish this"
            | "finish this task"
            | "finish this epic"
            | "complete this"
            | "complete this epic"
    )
}

fn first_open_child(node: &TreeNode) -> Option<&TreeNode> {
    for child in &node.children {
        if child.status == Status::Open {
            return Some(child);
        }
        if let Some(descendant) = first_open_child(child) {
            return Some(descendant);
        }
    }
    None
}

#[derive(Debug, Clone)]
struct BuildModeBlockedTask {
    id: String,
    decisions: Vec<String>,
}

#[derive(Debug, Clone)]
enum BuildModeSelection {
    Task(BuildModeTask),
    Blocked(BuildModeBlockedTask),
}

#[derive(Debug, Clone)]
struct BuildModeTask {
    id: String,
    title: String,
    description: Option<String>,
    design: Option<String>,
    acceptance: Option<String>,
    notes: Option<String>,
    verify_fast: Option<String>,
    verify: Option<String>,
    verify_timeout: Option<u64>,
    paths: Vec<String>,
    dependencies: Vec<String>,
    requires: Vec<String>,
    produces: Vec<String>,
    decisions: Vec<String>,
}

impl BuildModeTask {
    fn prompt(&self, scope: &ManaUnitRef) -> String {
        let mut prompt = format!(
            "Build mode: work on mana task {} — {} under active scope {} — {}. Stay within this task and the active mana scope. Do not expand scope or add unrelated features.",
            self.id,
            self.title,
            scope.id,
            scope.title.trim()
        );
        if let Some(description) = self.description.as_deref().filter(|s| !s.trim().is_empty()) {
            push_prompt_section(&mut prompt, "Description", description.trim());
        }
        if let Some(design) = self.design.as_deref().filter(|s| !s.trim().is_empty()) {
            push_prompt_section(&mut prompt, "Design", design.trim());
        }
        if let Some(acceptance) = self.acceptance.as_deref().filter(|s| !s.trim().is_empty()) {
            push_prompt_section(&mut prompt, "Acceptance", acceptance.trim());
        }
        if !self.decisions.is_empty() {
            push_prompt_section(
                &mut prompt,
                "Blocking decisions",
                &self.decisions.join("\n"),
            );
        }
        if !self.paths.is_empty() {
            push_prompt_section(&mut prompt, "Relevant paths", &self.paths.join("\n"));
        }
        if !self.dependencies.is_empty() {
            push_prompt_section(&mut prompt, "Dependencies", &self.dependencies.join(", "));
        }
        if !self.requires.is_empty() {
            push_prompt_section(&mut prompt, "Requires", &self.requires.join("\n"));
        }
        if !self.produces.is_empty() {
            push_prompt_section(&mut prompt, "Produces", &self.produces.join("\n"));
        }
        if let Some(verify_fast) = self.verify_fast.as_deref().filter(|s| !s.trim().is_empty()) {
            push_prompt_section(&mut prompt, "Fast verify command", verify_fast.trim());
        }
        if let Some(verify) = self.verify.as_deref().filter(|s| !s.trim().is_empty()) {
            push_prompt_section(&mut prompt, "Verify command", verify.trim());
        }
        if let Some(timeout) = self.verify_timeout {
            push_prompt_section(&mut prompt, "Verify timeout", &format!("{timeout}s"));
        }
        if let Some(notes) = self.notes.as_deref().filter(|s| !s.trim().is_empty()) {
            push_prompt_section(&mut prompt, "Recent notes", notes.trim());
        }
        prompt.push_str("\n\nWhen done, verify with the narrowest relevant check and update/close the mana task with evidence if appropriate.");
        prompt
    }
}

const IMPROVE_CHANGELOG_PATH: &str = ".imp/improve-changelog.md";
const IMPROVE_SANDBOX_METADATA_PATH: &str = ".imp/improve-sandbox.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImproveSandboxMetadata {
    branch: String,
    base_branch: String,
    worktree: PathBuf,
    changelog_path: PathBuf,
    updated_at_unix_secs: u64,
}

impl From<&ImproveSandbox> for ImproveSandboxMetadata {
    fn from(sandbox: &ImproveSandbox) -> Self {
        Self {
            branch: sandbox.branch.clone(),
            base_branch: sandbox.base_branch.clone(),
            worktree: sandbox.worktree.clone(),
            changelog_path: sandbox.worktree.join(IMPROVE_CHANGELOG_PATH),
            updated_at_unix_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default(),
        }
    }
}

fn improve_safe_mode_prompt(scope: &ManaUnitRef, turn: u32, budget: u32) -> String {
    let title = scope.title.trim();
    let scope_label = if title.is_empty() {
        scope.id.clone()
    } else {
        format!("{} — {title}", scope.id)
    };
    format!(
        "Improve mode autoresearch turn {turn}/{budget} for active mana scope {scope_label}.\n\n\
Goal: independently improve the work graph and project understanding without surprising the user. Favor research, inspection, evaluation, critique, benchmarks, risk discovery, and actionable recommendations.\n\n\
Rules:\n\
- Stay within the active mana scope. Do not expand scope unless you create/propose an explicit follow-up under that scope.\n\
- Prefer read-only investigation and narrow verification commands. Do not make broad code changes, destructive changes, dependency additions, migrations, commits, or deployment changes.\n\
- If you find concrete follow-up work, create or update mana units with enough context for a later Build-mode worker.\n\
- If a consequential product/architecture decision is required, record a blocking mana decision or ask one concise question; otherwise keep researching.\n\
- At the end of this turn, summarize what you inspected, what you learned, and the next best improvement action."
    )
}

fn improve_code_mode_prompt(
    scope: &ManaUnitRef,
    turn: u32,
    budget: u32,
    sandbox: &ImproveSandbox,
) -> String {
    let title = scope.title.trim();
    let scope_label = if title.is_empty() {
        scope.id.clone()
    } else {
        format!("{} — {title}", scope.id)
    };
    format!(
        "Improve mode code-changing turn {turn}/{budget} for active mana scope {scope_label}.\n\n\
Sandbox:\n\
- Branch: {branch}\n\
- Worktree: {worktree}\n\
- Base: {base}\n\
- Changelog: {changelog}\n\n\
Goal: improve the project within the active mana scope. Research as needed, then make coherent code changes only inside the sandbox worktree.\n\n\
Rules:\n\
- Work only in the sandbox worktree path above. Do not edit files in the original checkout.\n\
- Maintain `{changelog}` in the sandbox. Keep it useful for the user to review before `/improve merge`: summary, changes made, verification, risks/concerns, files changed, and merge notes.\n\
- Stay within the active mana scope; create/update mana follow-ups for anything outside it.\n\
- Run the narrowest useful verification in the sandbox.\n\
- Do not merge, rebase, force-push, deploy, or change production resources.\n\
- Do not commit unless the user explicitly asks.\n\
- At the end of this turn, summarize changes, verification, and review commands such as `git -C {worktree} status` and `git -C {worktree} diff {base}...HEAD`." ,
        branch = sandbox.branch,
        worktree = sandbox.worktree.display(),
        base = sandbox.base_branch,
        changelog = IMPROVE_CHANGELOG_PATH,
    )
}

fn push_prompt_section(prompt: &mut String, heading: &str, body: &str) {
    if body.trim().is_empty() {
        return;
    }
    prompt.push_str("\n\n");
    prompt.push_str(heading);
    prompt.push_str(":\n");
    prompt.push_str(body.trim());
}

fn candidate_active_scope_from_review(review: &TurnManaReview) -> Option<ManaUnitRef> {
    if let Some(anchor) = review.anchor_unit.as_ref() {
        if is_scope_unit(&anchor.unit) {
            return Some(anchor.unit.clone());
        }
    }

    review
        .touched_units
        .iter()
        .rev()
        .find(|touched| is_scope_unit(&touched.unit))
        .map(|touched| touched.unit.clone())
}

fn is_scope_unit(unit: &ManaUnitRef) -> bool {
    unit.kind
        .as_deref()
        .is_some_and(|kind| matches!(kind.to_ascii_lowercase().as_str(), "epic"))
}

#[derive(Debug, Clone)]
struct ImproveSandbox {
    branch: String,
    base_branch: String,
    worktree: PathBuf,
}

#[derive(Debug, Clone)]
struct LoopState {
    message: String,
    completed_turns: u32,
    budget: u32,
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
    compaction_task: Option<tokio::task::JoinHandle<Result<String, String>>>,
    lua_command_task: Option<tokio::task::JoinHandle<(String, Result<Option<String>, String>)>>,
    pub is_streaming: bool,
    pub message_queue: Vec<QueuedMessage>,
    pending_agent_prompt: Option<String>,
    pending_agent_cwd: Option<PathBuf>,

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
    streaming_anchor_user_index: Option<usize>,
    pub auto_scroll: bool,
    pub tools_expanded: bool,
    /// Index into the flattened tool call list. `None` means inspector follows latest.
    pub tool_focus: Option<usize>,
    /// True once the user explicitly selects a tool; prevents new tools stealing focus.
    pub tool_focus_pinned: bool,
    /// True while inspector should keep live output pinned to the bottom.
    pub sidebar_auto_follow: bool,

    pub ctrl_c_count: u8,
    pub needs_redraw: bool,
    last_terminal_title: Option<String>,
    pub last_esc: Option<Instant>,
    pub tick: u64,
    pub max_turns_override: Option<u32>,
    completed_turns_in_run: u32,
    suppress_completion_notification: bool,
    pub ui_rx: Option<tokio::sync::mpsc::Receiver<crate::tui_interface::UiRequest>>,
    lua_command_ui: Option<Arc<dyn imp_core::ui::UserInterface>>,
    pub ask_state: Option<crate::views::ask_bar::AskState>,
    pub ask_reply: Option<AskReply>,
    pub workflow_mode: WorkflowMode,
    active_mana_scope: Option<ManaUnitRef>,
    active_mana_run: Option<ManaRunSummary>,
    build_auto_turns: u32,
    last_build_auto_task_id: Option<String>,
    improve_auto_turns: u32,
    improve_safe_mode: bool,
    improve_sandbox: Option<ImproveSandbox>,
    loop_state: Option<LoopState>,
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

    /// Startup skill selected for display in the inspector sidebar.
    selected_startup_skill: Option<imp_core::resources::Skill>,

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
    llm_thought_segment_started_at: Option<Instant>,
    pub turn_tracker: TurnTracker,

    // Display helpers
    pub theme: Theme,
    pub highlighter: Highlighter,
    pub model_registry: ModelRegistry,
}

fn slug_fragment(input: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in input.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
        if slug.len() >= 40 {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "scope".to_string()
    } else {
        slug
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(format!("git {} failed: {detail}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn create_improve_sandbox(cwd: &Path, scope: &ManaUnitRef) -> Result<ImproveSandbox, String> {
    let repo_root = run_git(cwd, &["rev-parse", "--show-toplevel"])?;
    let repo_root = PathBuf::from(repo_root);
    let base_branch = run_git(&repo_root, &["branch", "--show-current"]).map(|branch| {
        if branch.is_empty() {
            "HEAD".to_string()
        } else {
            branch
        }
    })?;
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let slug = slug_fragment(&format!("{}-{}", scope.id, scope.title));
    let branch = format!("imp/improve/{slug}");
    let mut worktree = repo_root
        .parent()
        .unwrap_or_else(|| repo_root.as_path())
        .join(format!("{repo_name}-improve-{slug}"));

    let existing_worktrees = run_git(&repo_root, &["worktree", "list", "--porcelain"])?;
    if existing_worktrees
        .lines()
        .any(|line| line == format!("branch refs/heads/{branch}"))
    {
        if let Some(path_line) = existing_worktrees
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|window| window[1] == format!("branch refs/heads/{branch}"))
            .and_then(|window| window[0].strip_prefix("worktree "))
        {
            return Ok(ImproveSandbox {
                branch,
                base_branch,
                worktree: PathBuf::from(path_line),
            });
        }
    }

    if worktree.exists() {
        for index in 2..100 {
            let candidate = repo_root
                .parent()
                .unwrap_or_else(|| repo_root.as_path())
                .join(format!("{repo_name}-improve-{slug}-{index}"));
            if !candidate.exists() {
                worktree = candidate;
                break;
            }
        }
    }

    let branch_exists = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(&repo_root)
        .status()
        .map_err(|err| format!("failed to check branch {branch}: {err}"))?
        .success();

    if branch_exists {
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                worktree
                    .to_str()
                    .ok_or_else(|| "worktree path is not valid UTF-8".to_string())?,
                &branch,
            ],
        )?;
    } else {
        run_git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                worktree
                    .to_str()
                    .ok_or_else(|| "worktree path is not valid UTF-8".to_string())?,
                "HEAD",
            ],
        )?;
    }

    Ok(ImproveSandbox {
        branch,
        base_branch,
        worktree,
    })
}

fn concise_git_status(cwd: &Path) -> Option<Vec<String>> {
    let branch = run_git(cwd, &["branch", "--show-current"]).ok()?;
    let branch = if branch.trim().is_empty() {
        run_git(cwd, &["rev-parse", "--short", "HEAD"]).ok()?
    } else {
        branch
    };
    let mut lines = vec![format!("git: {branch}")];
    if let Ok(upstream) = run_git(
        cwd,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    ) {
        if let Ok(counts) = run_git(cwd, &["rev-list", "--left-right", "--count", "HEAD...@{u}"]) {
            let mut parts = counts.split_whitespace();
            if let (Some(ahead), Some(behind)) = (parts.next(), parts.next()) {
                lines.push(format!(
                    "upstream: {upstream} (ahead {ahead}, behind {behind})"
                ));
            }
        }
    }
    let status = run_git(cwd, &["status", "--short"]).unwrap_or_default();
    if status.trim().is_empty() {
        lines.push("working tree: clean".to_string());
    } else {
        let entries: Vec<&str> = status.lines().collect();
        lines.push(format!("working tree: dirty ({} paths)", entries.len()));
        lines.extend(entries.iter().take(8).map(|line| format!("  {line}")));
        if entries.len() > 8 {
            lines.push(format!("  … {} more", entries.len() - 8));
        }
    }
    Some(lines)
}

fn improve_metadata_file(cwd: &Path) -> Option<PathBuf> {
    let repo_root = run_git(cwd, &["rev-parse", "--show-toplevel"]).ok()?;
    Some(PathBuf::from(repo_root).join(IMPROVE_SANDBOX_METADATA_PATH))
}

fn write_improve_sandbox_metadata(cwd: &Path, sandbox: &ImproveSandbox) -> Result<(), String> {
    let Some(path) = improve_metadata_file(cwd) else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create Improve metadata directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let metadata = ImproveSandboxMetadata::from(sandbox);
    let json = serde_json::to_string_pretty(&metadata)
        .map_err(|err| format!("failed to encode Improve metadata: {err}"))?;
    std::fs::write(&path, json)
        .map_err(|err| format!("failed to write Improve metadata {}: {err}", path.display()))
}

fn read_improve_sandbox_metadata(cwd: &Path) -> Result<Option<ImproveSandbox>, String> {
    let Some(metadata) = read_improve_sandbox_metadata_file(cwd)? else {
        return Ok(None);
    };
    validate_improve_sandbox_metadata(metadata)
}

fn read_improve_sandbox_metadata_file(
    cwd: &Path,
) -> Result<Option<ImproveSandboxMetadata>, String> {
    let Some(path) = improve_metadata_file(cwd) else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read Improve metadata {}: {err}", path.display()))?;
    let metadata: ImproveSandboxMetadata = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse Improve metadata {}: {err}", path.display()))?;
    Ok(Some(metadata))
}

fn validate_improve_sandbox_metadata(
    metadata: ImproveSandboxMetadata,
) -> Result<Option<ImproveSandbox>, String> {
    if !metadata.worktree.exists() {
        return Err(format!(
            "Improve metadata points to missing worktree {}",
            metadata.worktree.display()
        ));
    }
    if run_git(&metadata.worktree, &["rev-parse", "--is-inside-work-tree"]).is_err() {
        return Err(format!(
            "Improve metadata worktree is not a git worktree: {}",
            metadata.worktree.display()
        ));
    }
    Ok(Some(ImproveSandbox {
        branch: metadata.branch,
        base_branch: metadata.base_branch,
        worktree: metadata.worktree,
    }))
}

fn selected_read_file_path_from_tool(tc: Option<&DisplayToolCall>, cwd: &Path) -> Option<PathBuf> {
    let tc = tc?;
    if tc.name != "read" {
        return None;
    }

    let path = tc.details.get("path")?.as_str()?.trim();
    if path.is_empty() {
        return None;
    }

    let path = PathBuf::from(path);
    Some(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

fn open_path_in_editor(path: &Path) -> std::io::Result<()> {
    let editor = std::env::var_os("VISUAL").or_else(|| std::env::var_os("EDITOR"));
    if let Some(editor) = editor.filter(|value| !value.is_empty()) {
        return std::process::Command::new(editor)
            .arg(path)
            .spawn()
            .map(|_| ());
    }

    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ());
    }

    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
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
                || auth_store.has_credentials("openai")
        }
        _ => auth_store.has_credentials(provider),
    }
}

fn oauth_provider(provider: &str) -> bool {
    matches!(
        provider,
        "anthropic" | "openai" | "openai-codex" | "kimi-code"
    )
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

fn model_picker_chatgpt_oauth_models(
    registry: &ModelRegistry,
    auth_store: &AuthStore,
) -> Vec<ModelMeta> {
    let has_chatgpt_oauth =
        auth_store.get_oauth("openai").is_some() || auth_store.get_oauth("openai-codex").is_some();
    if !has_chatgpt_oauth || auth_store.resolve_api_key_only("openai").is_ok() {
        return Vec::new();
    }

    imp_llm::model::builtin_openai_codex_models()
        .into_iter()
        .filter(|model| registry.find(&model.id).is_none())
        .map(|mut model| {
            model.provider = "openai".into();
            model
        })
        .collect()
}

fn merge_model_options_with_oauth_only_models(
    mut models: Vec<ModelMeta>,
    oauth_only_models: Vec<ModelMeta>,
) -> Vec<ModelMeta> {
    if oauth_only_models.is_empty() {
        return models;
    }

    let insert_at = models
        .iter()
        .rposition(|model| model.provider == "openai")
        .map_or(models.len(), |index| index + 1);
    models.splice(insert_at..insert_at, oauth_only_models);
    models
}

fn filtered_model_options(
    registry: &ModelRegistry,
    config: &Config,
    auth_store: &AuthStore,
) -> Vec<ModelMeta> {
    let oauth_only_models = model_picker_chatgpt_oauth_models(registry, auth_store);

    match &config.enabled_models {
        Some(enabled) if !enabled.is_empty() => {
            let available_models = merge_model_options_with_oauth_only_models(
                registry.list().to_vec(),
                oauth_only_models,
            );

            let available_ids: HashSet<&str> =
                available_models.iter().map(|m| m.id.as_str()).collect();
            let enabled_ids: HashSet<String> = enabled
                .iter()
                .filter_map(|name| registry.resolve_meta(name, None).map(|model| model.id))
                .filter(|id| available_ids.contains(id.as_str()))
                .collect();

            available_models
                .into_iter()
                .filter(|model| enabled_ids.contains(&model.id))
                .collect()
        }
        _ => {
            let visible_models: Vec<ModelMeta> = registry
                .list()
                .iter()
                .filter(|model| auth_store.has_credentials(&model.provider))
                .cloned()
                .collect();
            merge_model_options_with_oauth_only_models(visible_models, oauth_only_models)
        }
    }
}

fn include_current_model_option(
    mut models: Vec<ModelMeta>,
    registry: &ModelRegistry,
    current_model: &str,
) -> (Vec<ModelMeta>, String) {
    let Some(meta) = registry.resolve_meta(current_model, None) else {
        return (models, current_model.to_string());
    };

    let canonical_id = meta.id.clone();
    if !models.iter().any(|model| model.id == canonical_id) {
        models.insert(0, meta);
    }

    (models, canonical_id)
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
            compaction_task: None,
            lua_command_task: None,
            is_streaming: false,
            message_queue: Vec::new(),
            pending_agent_prompt: None,
            pending_agent_cwd: None,
            session,
            config,
            model_name,
            thinking_level,
            context_window,
            mode: UiMode::Normal,
            scroll_offset: 0,
            streaming_anchor_user_index: None,
            auto_scroll: true,
            tools_expanded: false,
            tool_focus: None,
            tool_focus_pinned: false,
            sidebar_auto_follow: true,

            ctrl_c_count: 0,
            needs_redraw: true,
            last_terminal_title: None,
            last_esc: None,
            tick: 0,
            max_turns_override: None,
            completed_turns_in_run: 0,
            suppress_completion_notification: false,
            ui_rx: None,
            lua_command_ui: None,
            ask_state: None,
            ask_reply: None,
            workflow_mode: WorkflowMode::Explore,
            active_mana_scope: None,
            active_mana_run: None,
            build_auto_turns: 0,
            last_build_auto_task_id: None,
            improve_auto_turns: 0,
            improve_safe_mode: false,
            improve_sandbox: None,
            loop_state: None,
            secrets_flow: None,
            login_task: None,
            accumulated_usage: Usage::default(),
            accumulated_cost: Cost::default(),
            current_context_tokens: 0,
            chat_render_epoch: 0,
            status_items: HashMap::new(),
            widgets: HashMap::new(),
            lua_runtime: None,
            selected_startup_skill: None,
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
            llm_thought_segment_started_at: None,
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
        let identity = if self.is_streaming || self.compaction_task.is_some() {
            if self.config.ui.animations == imp_core::config::AnimationLevel::None {
                title_working_glyph()
            } else {
                title_breather_frame(self.tick)
            }
        } else {
            "imp"
        };
        format!("{identity} — {title}")
    }

    fn prepare_for_interactive(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = imp_core::storage::reconcile_legacy_into_global_root();
        // Load Lua extensions (for slash commands and tool registration)
        self.reload_lua_extensions();

        // Check for first-run welcome flow
        let config_dir = Config::user_config_dir();
        let auth_path = imp_core::storage::global_auth_path();
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
        loop {
            self.sync_window_title();
            // Render
            if self.needs_redraw {
                terminal.draw(|frame| self.render(frame))?;
                self.needs_redraw = false;
            }

            self.start_pending_agent_after_redraw();

            let tick_rate = self.effective_tick_rate();

            // Poll for terminal events with adaptive timeout
            if crossterm::event::poll(tick_rate)? {
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
            self.pump_runtime_signals().await;

            // Tick + periodic redraw for streaming/spinner
            self.tick = self.tick.wrapping_add(1);
            self.maybe_autoscroll_selection();
            if self.is_streaming || self.compaction_task.is_some() {
                self.sync_window_title();
                self.needs_redraw = true;
            }

            if !self.running {
                break;
            }
        }

        Ok(())
    }

    fn sync_window_title(&mut self) {
        let title = self.terminal_title();
        if self.last_terminal_title.as_deref() == Some(title.as_str()) {
            return;
        }
        let _ = set_window_title(&title);
        self.last_terminal_title = Some(title);
    }

    async fn pump_runtime_signals(&mut self) {
        let signals = self.collect_runtime_signals().await;
        for signal in signals {
            self.handle_runtime_signal(signal);
        }
    }

    async fn collect_runtime_signals(&mut self) -> Vec<RuntimeSignal> {
        let mut signals = Vec::new();

        if let Some(handle) = self.agent_handle.as_mut() {
            while signals.len() < MAX_RUNTIME_SIGNALS_PER_TICK {
                match handle.event_rx.try_recv() {
                    Ok(event) => signals.push(RuntimeSignal::AgentEvent(event)),
                    Err(_) => break,
                }
            }
        }

        let agent_task_finished = self
            .agent_task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished);
        if agent_task_finished {
            if let Some(task) = self.agent_task.take() {
                let outcome = match task.await {
                    Ok(Ok(())) | Ok(Err(ImpCoreError::Cancelled)) => Ok(()),
                    Ok(Err(error)) => Err(error.to_string()),
                    Err(error) => Err(format!("Internal agent task failure: {error}")),
                };

                // Drain one more time after confirmed completion. The agent can finish with final
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

        let compaction_task_finished = self
            .compaction_task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished);
        if compaction_task_finished {
            if let Some(task) = self.compaction_task.take() {
                match task.await {
                    Ok(Ok(summary)) => {
                        signals.push(RuntimeSignal::CompactionTaskCompleted(summary))
                    }
                    Ok(Err(error)) => signals.push(RuntimeSignal::CompactionTaskFailed(error)),
                    Err(error) => signals.push(RuntimeSignal::CompactionTaskFailed(format!(
                        "Internal compaction task failure: {error}"
                    ))),
                }
            }
        }

        let lua_command_task_finished = self
            .lua_command_task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished);
        if lua_command_task_finished {
            if let Some(task) = self.lua_command_task.take() {
                match task.await {
                    Ok((command, Ok(result))) => {
                        if lua_result_requests_restart(result.as_deref()) {
                            signals
                                .push(RuntimeSignal::LuaCommandRestartRequested { command, result })
                        } else {
                            signals.push(RuntimeSignal::LuaCommandCompleted { command, result })
                        }
                    }
                    Ok((command, Err(error))) => {
                        signals.push(RuntimeSignal::LuaCommandFailed { command, error })
                    }
                    Err(error) => signals.push(RuntimeSignal::LuaCommandFailed {
                        command: "lua".to_string(),
                        error: format!("Lua command task failure: {error}"),
                    }),
                }
            }
        }

        let login_task_finished = self
            .login_task
            .as_ref()
            .is_some_and(tokio::task::JoinHandle::is_finished);
        if login_task_finished {
            if let Some(task) = self.login_task.take() {
                match task.await {
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
            let remaining_budget = MAX_RUNTIME_SIGNALS_PER_TICK.saturating_sub(signals.len());
            let ui_budget = remaining_budget.min(MAX_UI_REQUESTS_PER_TICK);
            for _ in 0..ui_budget {
                match rx.try_recv() {
                    Ok(req) => signals.push(RuntimeSignal::UiRequest(req)),
                    Err(_) => break,
                }
            }
        }

        signals
    }

    fn handle_runtime_signal(&mut self, signal: RuntimeSignal) {
        match signal {
            RuntimeSignal::AgentEvent(event) => self.handle_agent_event(event),
            RuntimeSignal::AgentTaskCompleted => {
                self.maybe_notify_agent_completion();
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
            RuntimeSignal::CompactionTaskCompleted(summary) => {
                self.finish_manual_compaction(summary)
            }
            RuntimeSignal::CompactionTaskFailed(error) => {
                self.finish_compaction_status_message("Compaction failed.");
                self.push_error_msg(&format!("Compaction failed: {error}"));
            }
            RuntimeSignal::LuaCommandCompleted { command, result } => {
                self.finish_lua_command_status_message(&format!("/{command} finished."));
                if let Some(text) = result {
                    self.push_system_msg(&text);
                }
            }
            RuntimeSignal::LuaCommandRestartRequested { command, result } => {
                self.finish_lua_command_status_message(&format!("/{command} finished."));
                if let Some(text) = result {
                    self.push_system_msg(&strip_lua_restart_directive(&text));
                }
                self.restart_after_lua_command();
            }
            RuntimeSignal::LuaCommandFailed { command, error } => {
                self.finish_lua_command_status_message(&format!("/{command} failed."));
                self.push_error_msg(&format!("Lua command error: {error}"));
            }
            RuntimeSignal::LoginTaskSucceeded(message) => self.push_system_msg(&message),
            RuntimeSignal::LoginTaskFailed(message) => self.push_error_msg(&message),
            RuntimeSignal::UiRequest(req) => self.handle_ui_request(req),
        }
        self.needs_redraw = true;
    }

    fn present_agent_failure(&mut self, error: String) {
        self.completed_turns_in_run = 0;
        self.is_streaming = false;
        self.streaming_anchor_user_index = None;
        if let Some(last) = self.latest_streaming_message_mut() {
            last.is_streaming = false;
        }
        self.push_error_msg(&format_error_for_display(&error));
    }

    fn maybe_notify_agent_completion(&mut self) {
        if self.is_streaming {
            return;
        }
        if self.completed_turns_in_run == 0 {
            return;
        }
        if self.suppress_completion_notification {
            self.completed_turns_in_run = 0;
            self.suppress_completion_notification = false;
            return;
        }
        if !self.config.ui.notify_on_agent_complete {
            self.completed_turns_in_run = 0;
            return;
        }

        let _ = ring_terminal_bell();
        self.completed_turns_in_run = 0;
    }

    fn handle_ui_request(&mut self, req: crate::tui_interface::UiRequest) {
        use crate::tui_interface::UiRequest;
        use crate::views::ask_bar::{AskOption, AskState};

        match req {
            UiRequest::Select {
                title,
                context,
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
                        context,
                        ask_options,
                        false,
                        "type to filter or answer freely…".into(),
                    ),
                    AskReply::Select(reply),
                );
            }
            UiRequest::MultiSelect {
                title,
                context,
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
                        context,
                        ask_options,
                        true,
                        "type to answer freely…".into(),
                    ),
                    AskReply::MultiSelect(reply),
                );
            }
            UiRequest::Input {
                title,
                context,
                placeholder,
                reply,
            } => {
                self.begin_ask(
                    AskState::with_placeholder(title, context, vec![], false, placeholder),
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
                    AskState::with_placeholder(title, message, options, false, String::new()),
                    AskReply::Select(bool_tx),
                );
                let confirm_reply = reply;
                tokio::spawn(async move {
                    let result = bool_rx.await.ok().flatten();
                    let _ = confirm_reply.send(result.map(|idx| idx == 0));
                });
            }
            UiRequest::Notify { message, level } => match level {
                imp_core::ui::NotifyLevel::Error => self.push_error_msg(&message),
                imp_core::ui::NotifyLevel::Warning => self.push_warning_msg(&message),
                imp_core::ui::NotifyLevel::Info => self.push_system_msg(&message),
            },
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

        if self.compaction_task.is_some() {
            return AnimationState::Thinking;
        }

        AnimationState::from_streaming(
            self.is_streaming,
            has_visible_content,
            has_tools_in_turn,
            active_tools,
            !self.message_queue.is_empty(),
        )
    }

    fn theme_kind(&self) -> ThemeKind {
        ThemeKind {
            is_light: self.theme.bg == Theme::light().bg,
        }
    }

    fn effective_tick_rate(&self) -> Duration {
        if self.is_streaming
            || self.compaction_task.is_some()
            || self.drag_autoscroll.is_some()
            || self.pending_agent_prompt.is_some()
        {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(100)
        }
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
            theme: self.theme_kind(),
        }
    }

    fn cached_chat_render(
        &mut self,
        width: u16,
        chat_tool_focus: Option<usize>,
        chat_tool_display: imp_core::config::ChatToolDisplay,
        activity_state: AnimationState,
    ) -> &crate::views::chat::ChatRenderData {
        let key =
            self.chat_render_cache_key(width, chat_tool_focus, chat_tool_display, activity_state);
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

        &self
            .chat_render_cache
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
            theme: self.theme_kind(),
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
        &self
            .sidebar_stream_cache
            .as_ref()
            .expect("sidebar stream cache set")
            .lines
    }

    fn sidebar_detail_cache_key(
        &self,
        width: u16,
        selected_tc: Option<&DisplayToolCall>,
        thinking: Option<&str>,
        run: Option<&ManaRunSummary>,
    ) -> SidebarDetailCacheKey {
        SidebarDetailCacheKey {
            width,
            messages_epoch: self.chat_render_epoch,
            selected_tool_id_hash: stable_hash(&selected_tc.map(|tc| &tc.id)),
            thinking_hash: stable_hash(&thinking),
            run_hash: stable_hash(&run.map(mana_run_summary_cache_key)),
            word_wrap: self.config.ui.word_wrap,
            tool_output_lines: self.config.ui.tool_output_lines,
            animation_level: self.config.ui.animations,
            theme: self.theme_kind(),
        }
    }

    fn begin_llm_thought_segment(&mut self) {
        self.llm_thought_segment_started_at = Some(Instant::now());
    }

    fn finalize_llm_thought_segment(&mut self) -> Option<u64> {
        self.llm_thought_segment_started_at
            .take()
            .map(|started_at| started_at.elapsed().as_secs().max(1))
    }

    fn selected_tool_call(&self) -> Option<DisplayToolCall> {
        let index = match self.tool_focus {
            Some(index) => index,
            None if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Inspector => {
                self.total_tool_calls().checked_sub(1)?
            }
            None => return None,
        };

        self.messages
            .iter()
            .flat_map(|message| message.tool_calls.iter())
            .nth(index)
            .cloned()
    }

    fn cached_sidebar_detail_render(
        &mut self,
        width: u16,
        selected_tc: Option<&DisplayToolCall>,
        thinking: Option<&str>,
        run: Option<&ManaRunSummary>,
    ) -> &SidebarDetailRenderData {
        let key = self.sidebar_detail_cache_key(width, selected_tc, thinking, run);
        let cache_hit = self
            .sidebar_detail_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key);
        if !cache_hit {
            let render = if let Some(run) = run {
                mana_run_detail_render_data(run, &self.theme)
            } else if let Some(thinking) = thinking {
                thinking_detail_render_data(
                    thinking,
                    &self.theme,
                    width as usize,
                    self.config.ui.word_wrap,
                )
            } else {
                build_detail_render_data(
                    selected_tc,
                    &self.config.ui,
                    &self.highlighter,
                    &self.theme,
                    width as usize,
                )
            };
            self.sidebar_detail_cache = Some(SidebarDetailCache { key, render });
        }
        &self
            .sidebar_detail_cache
            .as_ref()
            .expect("sidebar detail cache set")
            .render
    }

    fn latest_thinking_trace(&self) -> Option<String> {
        self.messages
            .iter()
            .rev()
            .find_map(|message| {
                message
                    .thinking
                    .as_deref()
                    .filter(|text| !text.trim().is_empty())
            })
            .map(str::to_owned)
    }

    fn startup_skills(&self) -> Vec<imp_core::resources::Skill> {
        let user_config_dir = imp_core::config::Config::user_config_dir();
        imp_core::resources::discover_skills(&self.cwd, &user_config_dir)
    }

    fn startup_skill_hits(&self, chat_area: Rect) -> Vec<StartupSkillHit> {
        let startup = self.build_startup_surface();
        startup_skill_hits(chat_area, &startup.panel)
    }

    fn select_startup_skill_at(&mut self, col: u16, row: u16) -> bool {
        if !matches!(self.mode, UiMode::Normal) || !self.messages.is_empty() {
            return false;
        }

        let Some(chat_area) = self.chat_surface.as_ref().map(|surface| surface.rect) else {
            return false;
        };

        let Some(hit) = self
            .startup_skill_hits(chat_area)
            .into_iter()
            .find(|hit| point_in_rect(col, row, Some(hit.rect)))
        else {
            return false;
        };

        let Some(skill) = self.startup_skills().into_iter().nth(hit.index) else {
            return false;
        };

        self.selected_startup_skill = Some(skill);
        self.sidebar.open = true;
        self.sidebar.reset_detail_scroll();
        self.sidebar_auto_follow = false;
        self.tool_focus = None;
        self.tool_focus_pinned = false;
        self.sidebar_detail_cache = None;
        true
    }

    fn build_startup_surface(&self) -> StartupSurfaceData {
        let user_config_dir = imp_core::config::Config::user_config_dir();
        let skills = self.startup_skills();
        let lua_extensions = discover_extensions(&user_config_dir, Some(&self.cwd));
        let repo_label = self
            .cwd
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("this project")
            .to_string();

        let lua_extension_summary = match &self.lua_runtime {
            Some(runtime) => match runtime.lock() {
                Ok(_) => summarize_inline(
                    lua_extensions.iter().map(|ext| ext.name.clone()).collect(),
                    3,
                ),
                Err(_) => "unavailable (runtime lock error)".to_string(),
            },
            None => summarize_inline(
                lua_extensions.iter().map(|ext| ext.name.clone()).collect(),
                3,
            ),
        };

        let auth_path = imp_core::storage::global_auth_path();
        let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
        let provider_meta = self.current_model_meta_for_persistence();
        let provider_id = provider_meta
            .as_ref()
            .map(|meta| meta.provider.as_str())
            .unwrap_or("unknown");
        let provider_auth = if auth_store.has_credentials(provider_id) {
            "ready"
        } else {
            "needs auth"
        };
        let web_summary = self
            .config
            .web
            .search_provider
            .map(|provider| {
                let status = if auth_store.has_credentials(provider.name()) {
                    "ready"
                } else {
                    "needs key"
                };
                format!("{} ({status})", provider.name())
            })
            .unwrap_or_else(|| "disabled".to_string());
        let mode = format!("{:?}", self.config.mode).to_lowercase();
        let session_name = self
            .session
            .name()
            .map(str::to_string)
            .or_else(|| self.session.title(48))
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "new chat".to_string());
        let session_lines = vec![
            format!("• project: {repo_label}"),
            format!("• session: {session_name}"),
            format!("• model: {}", self.model_name),
            format!("• provider: {provider_id} ({provider_auth})"),
            format!("• thinking: {:?}", self.thinking_level),
            format!("• web: {web_summary}"),
        ];

        let visible_prompt_tools = {
            let mut registry = imp_core::tools::ToolRegistry::new();
            imp_core::builder::register_native_tools(&mut registry);
            let mut names = registry
                .definitions_for_mode(&self.config.mode)
                .into_iter()
                .map(|def| def.name)
                .collect::<Vec<_>>();
            names.sort();
            names
        };

        let actions = vec![
            StartupAction {
                trigger: "type".to_string(),
                label: "start".to_string(),
                description: "question, goal, sketch, or task".to_string(),
            },
            StartupAction {
                trigger: "/resume".to_string(),
                label: "sessions".to_string(),
                description: "browse and search saved work".to_string(),
            },
            StartupAction {
                trigger: "/settings".to_string(),
                label: "runtime".to_string(),
                description: format!("{mode}; thinking {:?}", self.thinking_level),
            },
            StartupAction {
                trigger: "Ctrl+L".to_string(),
                label: "model".to_string(),
                description: format!("{}", self.model_name),
            },
        ];

        let tool_lines = visible_prompt_tools
            .iter()
            .map(|name| format!("• {name}"))
            .collect::<Vec<_>>();

        let skill_lines = if skills.is_empty() {
            vec!["• none discovered".to_string()]
        } else {
            skills
                .iter()
                .map(|skill| format!("• {}", skill.name))
                .collect::<Vec<_>>()
        };

        let extension_lines = vec![
            format!("• lua: {lua_extension_summary}"),
            "• commands: /command".to_string(),
            "• shell: /new, /model, /mana, /resume, /settings, /personality, /setup".to_string(),
            format!("• mode: {mode}"),
        ];

        let sections = vec![
            StartupSection {
                title: "session".to_string(),
                lines: session_lines,
            },
            StartupSection {
                title: "tools".to_string(),
                lines: tool_lines,
            },
            StartupSection {
                title: "skills".to_string(),
                lines: skill_lines,
            },
            StartupSection {
                title: "extensions".to_string(),
                lines: extension_lines,
            },
        ];

        StartupSurfaceData {
            panel: StartupPanelData { actions, sections },
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Clear, area);

        // Editor/prompt height: while asking, the prompt box becomes the ask box.
        // Otherwise it grows to fit wrapped prompt text while preserving at least
        // 3 lines for the chat area.
        let editor_inner_width = area.width.saturating_sub(2).max(1);
        let desired_editor_height = if let Some(state) = self.ask_state.as_ref() {
            state.prompt_height(editor_inner_width)
        } else {
            self.editor
                .visual_line_count_with_summary(editor_inner_width, true) as u16
                + 2
        };
        let max_editor_height = area.height.saturating_sub(3).max(3);
        let editor_height = desired_editor_height.clamp(3, max_editor_height);

        let constraints = vec![
            Constraint::Min(3),                // messages area
            Constraint::Length(editor_height), // editor / ask prompt
        ];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let (chat_area, editor_area) = (chunks[0], chunks[1]);

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
        let _ = self.theme_kind();

        // Messages
        let chat_tool_display = self.config.ui.effective_chat_tool_display();
        let chat_tool_focus = self.tool_focus;
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
        self.scroll_offset =
            clamped_scroll_offset_for_total_lines(total_chat_lines, chat_area, self.scroll_offset);
        if self.auto_scroll {
            if let Some(anchor_index) = self.streaming_anchor_user_index {
                self.scroll_offset = scroll_offset_for_message_at_top(
                    &self.messages,
                    &self.theme,
                    &self.highlighter,
                    chat_area,
                    anchor_index,
                    self.tick,
                    chat_tool_focus,
                    self.config.ui.word_wrap,
                    chat_tool_display,
                    self.config.ui.thinking_lines,
                    self.config.ui.show_timestamps,
                    self.config.ui.animations,
                    activity_state,
                );
            }
        }
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

        if matches!(self.mode, UiMode::Normal) && self.messages.is_empty() {
            let startup = self.build_startup_surface();
            frame.render_widget(
                StartupPanelView::new(&startup.panel, &self.theme),
                chat_area,
            );
            self.chat_surface = Some(TextSurface::new(
                SelectablePane::Chat,
                chat_area,
                Vec::new(),
                0,
            ));
        } else {
            let chat = RenderedChatView::new(&chat_lines).scroll(self.scroll_offset);
            frame.render_widget(chat, chat_area);

            self.chat_surface = Some(build_text_surface_from_lines(
                &chat_lines,
                chat_area,
                self.scroll_offset,
            ));
        }

        if !matches!(self.mode, UiMode::Normal) || !self.messages.is_empty() {
            self.selected_startup_skill = None;
        }

        // Sidebar
        if let Some(sidebar_area) = sidebar_area {
            let tc_count = self.total_tool_calls();
            let sub = sidebar_sub_areas(sidebar_area, tc_count, self.config.ui.sidebar_style);
            let stream_lines =
                if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Stream {
                    Some(self.cached_sidebar_stream_lines(sub.0.width).clone())
                } else {
                    None
                };
            let selected_index = if self.selected_startup_skill.is_some() {
                None
            } else {
                self.tool_focus.or_else(|| {
                    (self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Inspector)
                        .then(|| self.total_tool_calls().checked_sub(1))
                        .flatten()
                })
            };
            let detail_render = if let Some(skill) = self.selected_startup_skill.as_ref() {
                Some(startup_skill_detail_render_data(skill, &self.theme))
            } else if matches!(
                self.config.ui.sidebar_style,
                imp_core::config::SidebarStyle::Split | imp_core::config::SidebarStyle::Inspector
            ) {
                let selected_tc_owned = self.selected_tool_call();
                let run = if selected_tc_owned.is_none() {
                    self.active_mana_run.clone()
                } else {
                    None
                };
                let thinking = (selected_tc_owned.is_none() && run.is_none())
                    .then(|| self.latest_thinking_trace())
                    .flatten();
                Some(
                    self.cached_sidebar_detail_render(
                        sub.1.width,
                        selected_tc_owned.as_ref(),
                        thinking.as_deref(),
                        run.as_ref(),
                    )
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
                selected_index,
                &self.theme,
                &self.highlighter,
                self.tick,
                self.sidebar.list_scroll,
                self.sidebar.detail_scroll,
                &self.config.ui,
            );

            match self.config.ui.sidebar_style {
                imp_core::config::SidebarStyle::Inspector => {
                    let detail_lines = detail_render.as_ref().expect("detail cache lines");
                    view = view.precomputed_detail_lines(&detail_lines.lines);
                    frame.render_widget(view, sidebar_area);
                }
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
            let status_info = self.build_status_info();
            let editor = EditorView::new(&self.editor, &self.theme, self.thinking_level)
                .summarize_paste(true)
                .model(&self.model_name)
                .identity(&status_info.cwd, &status_info.session_name)
                .turn_elapsed(status_info.turn_elapsed)
                .extension_items(&status_info.extension_items, status_info.peek)
                .streaming(self.is_streaming)
                .queued(self.queued_message_preview(area.width))
                .context_usage(
                    self.current_context_tokens,
                    self.context_window,
                    self.config.ui.show_context_usage,
                )
                .tick(self.tick)
                .animation_level(self.config.ui.animations)
                .activity_state(activity_state)
                .workflow_mode(self.workflow_mode)
                .mana_scope_label(self.active_mana_scope_label())
                .mana_run_label(self.active_mana_run_label())
                .build_loop_label(self.build_loop_label())
                .improve_status_label(self.improve_status_label())
                .loop_label(self.loop_label());
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
            UiMode::ManaNavigator(state) => {
                let mana_area = centered_rect(88, 86, area);
                let view = ManaNavigatorView::new(state, &self.theme);
                frame.render_widget(view, mana_area);
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
        let auth_path = imp_core::storage::global_auth_path();
        let auth_store = AuthStore::load(&auth_path).ok()?;
        let meta = self.model_registry.resolve_meta(&self.model_name, None)?;
        let mut provider_name = meta.provider.clone();
        if should_use_chatgpt_provider(&auth_store, &self.model_registry, &meta) {
            provider_name = "openai-codex".to_string();
        }
        auth_store.oauth_display_info(&provider_name)
    }

    fn current_model_meta_for_persistence(&self) -> Option<ModelMeta> {
        let auth_path = imp_core::storage::global_auth_path();
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
            UiMode::ManaNavigator(_) => self.handle_mana_navigator_key(key),
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
                    let text = self.editor.content().to_string();
                    if !text.trim().is_empty() {
                        self.queue_streaming_message(QueuedMessage::Steer(text));
                    }
                } else {
                    self.send_message();
                }
            }
            Some(Action::FollowUp) => {
                if self.is_streaming {
                    let text = self.editor.content().to_string();
                    if !text.trim().is_empty() {
                        self.queue_streaming_message(QueuedMessage::FollowUp(text));
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
            Some(Action::OpenSelectedReadFile) => {
                self.open_selected_read_file();
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
                        self.focus_latest_tool_with_pin(false);
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
                        self.focus_latest_tool_with_pin(false);
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
                self.mode = UiMode::CommandPalette(CommandPaletteState::new(self.slash_commands()));
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

    fn handle_mana_navigator_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Tab => {
                self.mode = UiMode::Normal;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let UiMode::ManaNavigator(ref mut state) = self.mode {
                    state.move_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let UiMode::ManaNavigator(ref mut state) = self.mode {
                    state.move_down();
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let UiMode::ManaNavigator(ref mut state) = self.mode {
                    state.collapse_selected();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let UiMode::ManaNavigator(ref mut state) = self.mode {
                    state.expand_selected();
                }
            }
            KeyCode::Enter => {
                if let UiMode::ManaNavigator(ref mut state) = self.mode {
                    state.toggle_selected();
                }
            }
            _ => {}
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
            KeyCode::Char('f') => {
                let selected_id = if let UiMode::TreeView(ref state) = self.mode {
                    state.selected_id().map(String::from)
                } else {
                    None
                };
                if let Some(id) = selected_id {
                    let path = imp_core::storage::global_sessions_dir()
                        .join(format!("{}.jsonl", uuid::Uuid::new_v4()));
                    match self.session.fork(&id, &path) {
                        Ok(forked) => {
                            self.session = forked;
                            self.load_session_messages();
                            self.mode = UiMode::Normal;
                            self.push_system_msg(
                                "Forked from selected tree node. You're on a new branch.",
                            );
                        }
                        Err(e) => {
                            self.mode = UiMode::Normal;
                            self.push_system_msg(&format!("Fork failed: {e}"));
                        }
                    }
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
        self.focus_tool_with_pin(index, true);
    }

    fn focus_latest_tool_with_pin(&mut self, pinned: bool) -> bool {
        let total = self.total_tool_calls();
        if total == 0 {
            return false;
        }
        self.focus_tool_with_pin(total - 1, pinned);
        true
    }

    fn focus_tool_with_pin(&mut self, index: usize, pinned: bool) {
        self.tool_focus = Some(index);
        self.tool_focus_pinned = pinned;
        self.sidebar_auto_follow = !pinned;
        self.sidebar.open = true;
        self.sidebar.reset_detail_scroll();
        self.active_pane = match self.config.ui.sidebar_style {
            imp_core::config::SidebarStyle::Split => Pane::SidebarList,
            imp_core::config::SidebarStyle::Inspector | imp_core::config::SidebarStyle::Stream => {
                Pane::SidebarDetail
            }
        };
        if self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Split {
            self.sidebar.ensure_selected_visible(index);
        }
    }

    fn selected_read_file_path(&self) -> Option<PathBuf> {
        selected_read_file_path_from_tool(self.selected_tool_call().as_ref(), &self.cwd)
    }

    fn open_selected_read_file(&mut self) {
        let Some(path) = self.selected_read_file_path() else {
            self.push_system_msg("No read file selected to open.");
            return;
        };

        if !path.is_file() {
            self.push_error_msg(&format!(
                "Selected read file does not exist: {}",
                path.display()
            ));
            return;
        }

        match open_path_in_editor(&path) {
            Ok(()) => self.push_system_msg(&format!("Opened {}", path.display())),
            Err(error) => {
                self.push_error_msg(&format!("Failed to open {}: {error}", path.display()))
            }
        }
    }

    fn toggle_sidebar(&mut self) {
        if self.sidebar.open {
            self.sidebar.open = false;
            self.active_pane = Pane::Chat;
        } else {
            self.sidebar.open = true;
            if self.tool_focus.is_none() && !self.focus_latest_tool_with_pin(false) {
                self.active_pane = Pane::Chat;
            } else {
                self.active_pane = Pane::SidebarDetail;
            }
        }
    }

    fn tool_id_at_chat_row(&self, row: u16, chat_area: Rect) -> Option<String> {
        build_click_map(
            &self.messages,
            &self.theme,
            &self.highlighter,
            chat_area,
            self.scroll_offset,
            self.config.ui.word_wrap,
            self.config.ui.effective_chat_tool_display(),
            self.config.ui.thinking_lines,
            self.config.ui.show_timestamps,
        )
        .into_iter()
        .find_map(|(tool_row, tool_id)| (tool_row == row).then_some(tool_id))
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
            Pane::SidebarDetail if self.sidebar.open => {
                self.sidebar_auto_follow = false;
                self.sidebar.scroll_detail_up(lines);
            }
            _ => self.scroll_chat_up(lines),
        }
    }

    fn scroll_active_pane_down(&mut self, lines: usize) {
        match self.active_pane {
            Pane::SidebarList if self.sidebar.open => self.sidebar.scroll_list_down(lines),
            Pane::SidebarDetail if self.sidebar.open => {
                self.sidebar_auto_follow = false;
                self.sidebar.scroll_detail_down(lines);
            }
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
        let is_inspector =
            self.config.ui.sidebar_style == imp_core::config::SidebarStyle::Inspector;
        let in_list = point_in_rect(col, row, self.sidebar_list_rect);
        let in_detail = point_in_rect(col, row, self.sidebar_detail_rect);
        let in_sidebar = in_list || in_detail;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if in_list && !is_inspector {
                    self.active_pane = Pane::SidebarList;
                    self.sidebar
                        .scroll_list_up(self.config.ui.mouse_scroll_lines);
                } else if in_detail || (in_sidebar && (is_stream || is_inspector)) {
                    self.active_pane = Pane::SidebarDetail;
                    self.sidebar_auto_follow = false;
                    self.sidebar
                        .scroll_detail_up(self.config.ui.mouse_scroll_lines);
                } else {
                    self.active_pane = Pane::Chat;
                    self.scroll_chat_up(self.config.ui.mouse_scroll_lines);
                }
            }
            MouseEventKind::ScrollDown => {
                if in_list && !is_inspector {
                    self.active_pane = Pane::SidebarList;
                    self.sidebar
                        .scroll_list_down(self.config.ui.mouse_scroll_lines);
                } else if in_detail || (in_sidebar && (is_stream || is_inspector)) {
                    self.active_pane = Pane::SidebarDetail;
                    self.sidebar_auto_follow = false;
                    self.sidebar
                        .scroll_detail_down(self.config.ui.mouse_scroll_lines);
                } else {
                    self.active_pane = Pane::Chat;
                    self.scroll_chat_down(self.config.ui.mouse_scroll_lines);
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                if in_list && !is_inspector {
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

                if in_detail || (in_sidebar && (is_stream || is_inspector)) {
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
                if self.select_startup_skill_at(col, row) {
                    self.clear_selection();
                    return;
                }

                if let Some(chat_area) = self.chat_surface.as_ref().map(|surface| surface.rect) {
                    if let Some(tool_id) = self.tool_id_at_chat_row(row, chat_area) {
                        self.clear_selection();
                        if let Some(index) = self.find_tool_call_index(&tool_id) {
                            self.focus_tool(index);
                        }
                        return;
                    }
                }

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

    fn stop_active_work(&mut self) {
        if self.is_streaming || self.agent_task.is_some() {
            if let Some(ref handle) = self.agent_handle {
                let _ = handle.command_tx.try_send(AgentCommand::Cancel);
                handle
                    .cancel_token
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(task) = self.agent_task.take() {
                task.abort();
            }
            self.agent_handle = None;
            self.is_streaming = false;
            self.streaming_anchor_user_index = None;
            if let Some(last) = self.latest_streaming_message_mut() {
                last.is_streaming = false;
            }
        }

        self.pending_agent_prompt = None;
        self.pending_agent_cwd = None;
        self.loop_state = None;
        self.build_auto_turns = 0;
        self.last_build_auto_task_id = None;
        self.improve_auto_turns = 0;
        self.improve_sandbox = None;
        self.suppress_completion_notification = true;
        if let Some(run_id) = self.active_mana_run.as_ref().map(|run| run.run_id.clone()) {
            match stop_mana_run(&run_id) {
                Ok(Some(summary)) => {
                    self.active_mana_run = Some(summary);
                    self.push_system_msg(&format!(
                        "Stopped active mana run {run_id}. External workers may need manual cleanup."
                    ));
                }
                Ok(None) => {
                    self.push_system_msg(&format!("Active mana run {run_id} was not found."))
                }
                Err(err) => {
                    self.push_system_msg(&format!("Could not stop mana run {run_id}: {err}"))
                }
            }
        }

        self.push_system_msg("Stopped active imp work.");
    }

    fn handle_cancel(&mut self) {
        if !self.editor.is_empty() {
            // First Ctrl+C: clear editor
            self.editor.clear();
            self.ctrl_c_count = 0;
        } else if self.is_streaming || self.agent_task.is_some() {
            let already_cancelled = self.agent_handle.as_ref().is_some_and(|handle| {
                handle
                    .cancel_token
                    .load(std::sync::atomic::Ordering::Relaxed)
            });
            if already_cancelled {
                if let Some(task) = self.agent_task.take() {
                    task.abort();
                }
                self.agent_handle = None;
            } else if let Some(ref handle) = self.agent_handle {
                let _ = handle.command_tx.try_send(AgentCommand::Cancel);
                handle
                    .cancel_token
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            self.suppress_completion_notification = true;
            self.is_streaming = false;
            self.streaming_anchor_user_index = None;
            if let Some(last) = self.latest_streaming_message_mut() {
                last.is_streaming = false;
            }
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

    fn build_loop_label(&self) -> Option<String> {
        if self.workflow_mode != WorkflowMode::Build {
            return None;
        }
        match self.last_build_auto_task_id.as_deref() {
            Some(task_id) => Some(format!("task {task_id}")),
            None if self.active_mana_scope.is_some() => Some("ready".to_string()),
            None => None,
        }
    }

    fn improve_status_label(&self) -> Option<String> {
        if self.workflow_mode != WorkflowMode::Improve || self.improve_safe_mode {
            return None;
        }
        let sandbox = self.improve_sandbox.as_ref()?;
        let dir = sandbox
            .worktree
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| sandbox.worktree.to_str().unwrap_or("sandbox"));
        let budget = self.config.ui.improve_auto_turn_budget.max(1);
        Some(format!(
            "imp is improving {dir} · turn {}/{} · /improve-help for review",
            self.improve_auto_turns.min(budget),
            budget
        ))
    }

    fn loop_label(&self) -> Option<String> {
        let state = self.loop_state.as_ref()?;
        Some(format!(
            "↻ loop {}/{}",
            state.completed_turns.min(state.budget),
            state.budget
        ))
    }

    fn workflow_context_prompt(&self) -> Option<String> {
        let mode = self.workflow_mode.display_name();
        let mut context = format!("Workflow mode: {mode}.");
        if self.workflow_mode == WorkflowMode::Improve {
            if self.improve_safe_mode {
                context.push_str(" Improve safe mode is bounded autoresearch, evaluation, critique, and mana follow-up creation; avoid code edits.");
            } else if let Some(sandbox) = self.improve_sandbox.as_ref() {
                context.push_str(&format!(
                    " Improve mode may make code changes only in sandbox branch {} at {}. Do not edit the original checkout, commit, or merge without explicit approval.",
                    sandbox.branch,
                    sandbox.worktree.display()
                ));
            } else {
                context.push_str(" Improve mode may create a sandbox branch/worktree for code changes; do not edit the original checkout, commit, or merge without explicit approval.");
            }
        }
        if let Some(scope) = self.active_mana_scope.as_ref() {
            let title = scope.title.trim();
            if title.is_empty() {
                context.push_str(&format!(" Active mana scope: {}.", scope.id));
            } else {
                context.push_str(&format!(" Active mana scope: {} — {}.", scope.id, title));
            }
        }
        Some(context)
    }

    fn queue_build_mode_continuation_if_ready(&mut self) {
        if self.workflow_mode != WorkflowMode::Build
            || self.is_streaming
            || self.pending_agent_prompt.is_some()
        {
            return;
        }
        if self.active_mana_scope.is_none() {
            return;
        }
        let budget = self.config.ui.build_auto_turn_budget.max(1);
        if self.build_auto_turns >= budget {
            self.push_system_msg(&format!(
                "Build mode paused after {budget} automatic turns. Send a message or switch modes to continue."
            ));
            return;
        }
        if let Some((task_id, build_prompt)) = self.next_build_mode_prompt() {
            if self
                .last_build_auto_task_id
                .as_ref()
                .is_some_and(|last_task_id| last_task_id == &task_id)
            {
                self.push_system_msg(&format!(
                    "Build mode paused because mana task {task_id} is still open after the last automatic attempt. Close it, mark it blocked, or send a message to retry intentionally."
                ));
                return;
            }
            self.build_auto_turns += 1;
            self.last_build_auto_task_id = Some(task_id.clone());
            self.push_system_msg(&format!("Build mode: starting mana task {task_id}"));
            self.pending_agent_prompt = Some(build_prompt);
            self.pending_agent_cwd = None;
            self.needs_redraw = true;
        }
    }

    fn queue_improve_mode_continuation_if_ready(&mut self) {
        if self.workflow_mode != WorkflowMode::Improve
            || self.is_streaming
            || self.pending_agent_prompt.is_some()
        {
            return;
        }
        let Some(scope) = self.active_mana_scope.clone() else {
            self.push_system_msg("Improve mode needs an active mana scope. Use /scope <id> or read/create a mana epic first.");
            return;
        };
        let budget = self.config.ui.improve_auto_turn_budget.max(1);
        if self.improve_auto_turns >= budget {
            self.push_system_msg(&format!(
                "Improve mode paused after {budget} automatic turns. Send a message or switch modes to continue."
            ));
            return;
        }

        let prompt = if self.improve_safe_mode {
            improve_safe_mode_prompt(&scope, self.improve_auto_turns + 1, budget)
        } else {
            let Some(sandbox) = self.ensure_improve_sandbox(&scope) else {
                return;
            };
            improve_code_mode_prompt(&scope, self.improve_auto_turns + 1, budget, &sandbox)
        };

        self.improve_auto_turns += 1;
        if self.improve_safe_mode {
            self.push_system_msg(&format!(
                "Improve safe: research turn {}/{} for scope {}",
                self.improve_auto_turns, budget, scope.id
            ));
        } else if let Some(sandbox) = self.improve_sandbox.as_ref() {
            self.push_system_msg(&format!(
                "Improve mode: code turn {}/{} in branch {} at {}",
                self.improve_auto_turns,
                budget,
                sandbox.branch,
                sandbox.worktree.display()
            ));
        }
        self.pending_agent_prompt = Some(prompt);
        self.pending_agent_cwd = if self.improve_safe_mode {
            None
        } else {
            self.improve_sandbox
                .as_ref()
                .map(|sandbox| sandbox.worktree.clone())
        };
        self.needs_redraw = true;
    }

    fn queue_loop_continuation_if_ready(&mut self) {
        if self.is_streaming || self.pending_agent_prompt.is_some() {
            return;
        }
        let Some(state) = self.loop_state.as_mut() else {
            return;
        };
        if state.completed_turns >= state.budget {
            let budget = state.budget;
            self.loop_state = None;
            self.push_system_msg(&format!(
                "Loop paused after {budget} turns. Use /loop <message> to start again."
            ));
            return;
        }
        state.completed_turns += 1;
        let message = state.message.clone();
        let completed = state.completed_turns;
        let budget = state.budget;
        self.push_system_msg(&format!("Loop: turn {completed}/{budget}"));
        self.pending_agent_prompt = Some(message);
        self.pending_agent_cwd = None;
        self.needs_redraw = true;
    }

    fn stale_improve_metadata_message(&self) -> Option<String> {
        let metadata = match read_improve_sandbox_metadata_file(&self.cwd) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => return None,
            Err(err) => {
                return Some(format!(
                    "stale improve metadata: {err}\nnext: fix/remove {} or run /clean --force to forget stale metadata",
                    improve_metadata_file(&self.cwd)
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| IMPROVE_SANDBOX_METADATA_PATH.to_string())
                ));
            }
        };
        match validate_improve_sandbox_metadata(metadata.clone()) {
            Ok(Some(_)) => None,
            Ok(None) => None,
            Err(err) => Some(format!(
                "stale improve metadata: {err}\nmetadata: {}\nbranch: {}\nworktree: {}\nnext: run /clean --force to forget stale metadata; no branch/worktree will be deleted",
                improve_metadata_file(&self.cwd)
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| IMPROVE_SANDBOX_METADATA_PATH.to_string()),
                metadata.branch,
                metadata.worktree.display()
            )),
        }
    }

    fn current_improve_sandbox(&mut self) -> Option<ImproveSandbox> {
        if let Some(sandbox) = self.improve_sandbox.clone() {
            return Some(sandbox);
        }
        match read_improve_sandbox_metadata(&self.cwd) {
            Ok(Some(sandbox)) => {
                self.improve_sandbox = Some(sandbox.clone());
                Some(sandbox)
            }
            Ok(None) => None,
            Err(err) => {
                self.push_system_msg(&format!("Stale Improve sandbox metadata: {err}"));
                None
            }
        }
    }

    fn active_status_text(&mut self) -> String {
        let mut lines = Vec::new();
        lines.push("Status:".to_string());
        lines.push(format!("cwd: {}", self.cwd.display()));
        if let Some(git_lines) = concise_git_status(&self.cwd) {
            lines.extend(git_lines);
        }
        lines.push(format!("mode: {}", self.workflow_mode.display_name()));
        if self.is_streaming || self.agent_task.is_some() {
            lines.push("agent: running".to_string());
        } else if self.pending_agent_prompt.is_some() {
            lines.push("agent: queued".to_string());
        } else {
            lines.push("agent: idle".to_string());
        }
        if let Some(scope) = self.active_mana_scope.as_ref() {
            lines.push(format!("scope: {} — {}", scope.id, scope.title.trim()));
        }
        if let Some(run) = self.active_mana_run.as_ref() {
            lines.push(format!(
                "mana run: {} {} ({}/{}, failed {})",
                run.run_id, run.status, run.total_closed, run.total_units, run.total_failed
            ));
        }
        if self.workflow_mode == WorkflowMode::Build {
            let budget = self.config.ui.build_auto_turn_budget.max(1);
            lines.push(format!("build loop: {}/{}", self.build_auto_turns, budget));
            if let Some(task_id) = self.last_build_auto_task_id.as_ref() {
                lines.push(format!("last build task: {task_id}"));
            }
        }
        if self.workflow_mode == WorkflowMode::Improve {
            let budget = self.config.ui.improve_auto_turn_budget.max(1);
            lines.push(format!(
                "improve loop: {}/{}",
                self.improve_auto_turns, budget
            ));
            lines.push(format!(
                "improve mode: {}",
                if self.improve_safe_mode {
                    "safe"
                } else {
                    "sandbox"
                }
            ));
        }
        if let Some(sandbox) = self.current_improve_sandbox() {
            lines.push(format!("improve branch: {}", sandbox.branch));
            lines.push(format!("improve worktree: {}", sandbox.worktree.display()));
            lines.push(format!("improve base: {}", sandbox.base_branch));
            lines.push(format!(
                "improve changelog: {}",
                sandbox.worktree.join(IMPROVE_CHANGELOG_PATH).display()
            ));
            lines.push(format!(
                "next: review changelog, run /improve merge, then /improve merge --confirm (or /clean to discard)"
            ));
            if let Ok(status) = run_git(&sandbox.worktree, &["status", "--short"]) {
                lines.push(format!(
                    "worktree status: {}",
                    if status.trim().is_empty() {
                        "clean"
                    } else {
                        "dirty"
                    }
                ));
                if !status.trim().is_empty() {
                    lines.extend(status.lines().take(10).map(|line| format!("  {line}")));
                }
            }
        } else if let Some(message) = self.stale_improve_metadata_message() {
            lines.extend(message.lines().map(str::to_string));
        }
        if let Some(state) = self.loop_state.as_ref() {
            lines.push(format!("loop: {}/{}", state.completed_turns, state.budget));
            lines.push(format!(
                "loop message: {}",
                single_line_preview(&state.message)
            ));
        }
        lines.join("\n")
    }

    fn show_status_command(&mut self) {
        let status = self.active_status_text();
        self.push_system_msg(&status);
    }

    fn improve_merge_command(&mut self, args: &str) {
        let confirmed = args
            .split_whitespace()
            .any(|arg| arg == "--confirm" || arg == "confirm");
        let Some(sandbox) = self.current_improve_sandbox() else {
            self.push_system_msg("No active Improve sandbox to merge.");
            return;
        };
        let changelog = sandbox.worktree.join(IMPROVE_CHANGELOG_PATH);
        if !changelog.exists() {
            self.push_system_msg(&format!(
                "Refusing to merge: missing Improve changelog at {}. Review/complete the changelog first.",
                changelog.display()
            ));
            return;
        }
        match run_git(&self.cwd, &["status", "--short"]) {
            Ok(status) if !status.trim().is_empty() => {
                self.push_system_msg(&format!(
                    "Refusing to merge: current checkout is dirty. Commit/stash/revert first.\n{}",
                    status
                ));
                return;
            }
            Err(err) => {
                self.push_system_msg(&format!("Could not inspect current checkout: {err}"));
                return;
            }
            _ => {}
        }
        match run_git(&sandbox.worktree, &["status", "--short"]) {
            Ok(status) if !status.trim().is_empty() => {
                self.push_system_msg(&format!(
                    "Refusing to merge: Improve sandbox has uncommitted changes. Commit them in {} or clean/discard.\n{}",
                    sandbox.worktree.display(),
                    status
                ));
                return;
            }
            Err(err) => {
                self.push_system_msg(&format!("Could not inspect Improve sandbox: {err}"));
                return;
            }
            _ => {}
        }
        if !confirmed {
            self.push_system_msg(&format!(
                "Improve merge plan:\n- Branch: {}\n- Worktree: {}\n- Changelog: {}\n- Target checkout: {}\n- Operation: git merge --no-ff {}\n\nReview the changelog, then run `/improve merge --confirm` to merge. No merge has been performed.",
                sandbox.branch,
                sandbox.worktree.display(),
                changelog.display(),
                self.cwd.display(),
                sandbox.branch
            ));
            return;
        }
        match run_git(&self.cwd, &["merge", "--no-ff", &sandbox.branch]) {
            Ok(output) => {
                self.push_system_msg(&format!(
                    "Merged Improve branch {}. Changelog reviewed from {}.\n{}",
                    sandbox.branch,
                    changelog.display(),
                    output
                ));
            }
            Err(err) => self.push_system_msg(&format!("Improve merge failed: {err}")),
        }
    }

    fn clean_command(&mut self, args: &str) {
        let force = args
            .split_whitespace()
            .any(|arg| arg == "--force" || arg == "force");
        let Some(sandbox) = self.current_improve_sandbox() else {
            if force {
                if let Some(path) = improve_metadata_file(&self.cwd) {
                    if path.exists() {
                        match std::fs::remove_file(&path) {
                            Ok(()) => self.push_system_msg(&format!(
                                "Removed stale Improve metadata {}. No branch or worktree was deleted.",
                                path.display()
                            )),
                            Err(err) => self.push_system_msg(&format!(
                                "Failed to remove stale Improve metadata {}: {err}",
                                path.display()
                            )),
                        }
                    } else {
                        self.push_system_msg("Nothing to clean yet.");
                    }
                } else {
                    self.push_system_msg("Nothing to clean yet.");
                }
            } else if let Some(message) = self.stale_improve_metadata_message() {
                self.push_system_msg(&format!(
                    "{message}\nRun /clean --force to remove only the stale metadata file."
                ));
            } else {
                self.push_system_msg("Nothing to clean yet.");
            }
            return;
        };
        let status = run_git(&sandbox.worktree, &["status", "--short"]).unwrap_or_default();
        if !status.trim().is_empty() && !force {
            self.push_system_msg(&format!(
                "Improve sandbox is dirty; not cleaning without confirmation. Review `{}` then run `/clean --force` to remove worktree {}.\n{}",
                sandbox.branch,
                sandbox.worktree.display(),
                status
            ));
            return;
        }
        let mut command = Command::new("git");
        command.arg("worktree").arg("remove");
        if force {
            command.arg("--force");
        }
        command.arg(&sandbox.worktree).current_dir(&self.cwd);
        match command.output() {
            Ok(output) if output.status.success() => {
                self.improve_sandbox = None;
                if let Some(path) = improve_metadata_file(&self.cwd) {
                    let _ = std::fs::remove_file(path);
                }
                self.push_system_msg(&format!(
                    "Removed Improve worktree {}. Branch {} was kept.",
                    sandbox.worktree.display(),
                    sandbox.branch
                ));
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                self.push_system_msg(&format!("Clean failed: {}", err.trim()));
            }
            Err(err) => self.push_system_msg(&format!("Clean failed: {err}")),
        }
    }

    fn start_loop_command(&mut self, message: &str) {
        let message = message.trim();
        if message.is_empty() {
            self.push_system_msg("Usage: /loop <message>");
            return;
        }
        let budget = self.config.ui.loop_turn_budget.max(1);
        self.loop_state = Some(LoopState {
            message: message.to_string(),
            completed_turns: 0,
            budget,
        });
        self.push_system_msg(&format!("Loop started: {budget} turn budget."));
        self.queue_loop_continuation_if_ready();
    }

    fn ensure_improve_sandbox(&mut self, scope: &ManaUnitRef) -> Option<ImproveSandbox> {
        if let Some(sandbox) = self.improve_sandbox.clone() {
            return Some(sandbox);
        }
        match create_improve_sandbox(&self.cwd, scope) {
            Ok(sandbox) => {
                if let Err(err) = write_improve_sandbox_metadata(&self.cwd, &sandbox) {
                    self.push_system_msg(&format!("Improve sandbox metadata warning: {err}"));
                }
                self.push_system_msg(&format!(
                    "Improve sandbox ready: branch {} at {}. Review with `git -C {} diff {}...HEAD`.",
                    sandbox.branch,
                    sandbox.worktree.display(),
                    sandbox.worktree.display(),
                    sandbox.base_branch
                ));
                self.improve_sandbox = Some(sandbox.clone());
                Some(sandbox)
            }
            Err(err) => {
                self.push_system_msg(&format!("Could not create Improve sandbox: {err}"));
                None
            }
        }
    }

    fn try_launch_build_team(&mut self, text: &str) -> bool {
        if self.workflow_mode != WorkflowMode::Build || !is_build_team_intent(text) {
            return false;
        }

        let Some(scope) = self.active_mana_scope.clone() else {
            self.push_system_msg("Build team needs an active mana scope. Use /scope <id> or read/create a mana epic first.");
            return true;
        };

        match self.launch_mana_run_for_scope(&scope) {
            Ok(run_id) => {
                self.refresh_active_mana_run(&run_id);
                self.push_system_msg(&format!(
                    "Started mana team run {run_id} for scope {}.",
                    scope.id
                ));
            }
            Err(err) => {
                self.push_system_msg(&format!("Could not start mana team run: {err}"));
            }
        }
        true
    }

    fn launch_mana_run_for_scope(&self, scope: &ManaUnitRef) -> Result<String, String> {
        let (update_tx, _update_rx) = tokio::sync::mpsc::channel(16);
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(16);
        let ctx = imp_core::tools::ToolContext {
            cwd: self.cwd.clone(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx,
            command_tx,
            ui: Arc::new(imp_core::ui::NullInterface),
            file_cache: Arc::new(imp_core::tools::FileCache::new()),
            checkpoint_state: Arc::new(imp_core::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(
                imp_core::tools::FileTracker::default(),
            )),
            anchor_store: Arc::new(imp_core::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: imp_core::config::AgentMode::Full,
            read_max_lines: self.config.ui.read_max_lines,
            turn_mana_review: Arc::new(std::sync::Mutex::new(Default::default())),
            config: Arc::new(self.config.clone()),
            run_policy: Default::default(),
        };

        let output = futures::executor::block_on(ManaTool::default().execute(
            "build_team_run",
            serde_json::json!({ "action": "run", "id": scope.id }),
            ctx,
        ))
        .map_err(|err: imp_core::error::Error| err.to_string())?;

        if output.is_error {
            return Err(output
                .text_content()
                .unwrap_or("mana run failed")
                .to_string());
        }
        output.details["run_id"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "mana run did not return run_id".to_string())
    }

    fn build_mode_prompt_for_text(&mut self, text: &str) -> Option<String> {
        if self.workflow_mode != WorkflowMode::Build || !is_build_continue_intent(text) {
            return None;
        }

        self.next_build_mode_prompt().map(|(_, prompt)| prompt)
    }

    fn next_build_mode_prompt(&mut self) -> Option<(String, String)> {
        let Some(scope) = self.active_mana_scope.clone() else {
            self.push_system_msg("Build mode needs an active mana scope. Use /scope <id> or read/create a mana epic first.");
            return None;
        };

        match self.select_next_build_task(&scope.id) {
            Ok(Some(BuildModeSelection::Task(task))) => {
                let task_id = task.id.clone();
                Some((task_id, task.prompt(&scope)))
            }
            Ok(Some(BuildModeSelection::Blocked(blocked))) => {
                self.push_system_msg(&format!(
                    "Build mode paused: mana task {} has unresolved decision(s): {}",
                    blocked.id,
                    blocked.decisions.join("; ")
                ));
                None
            }
            Ok(None) => {
                self.push_system_msg(&format!(
                    "No open child tasks found under active mana scope {}.",
                    scope.id
                ));
                None
            }
            Err(err) => {
                self.push_system_msg(&format!("Could not select next build task: {err}"));
                None
            }
        }
    }

    fn select_next_build_task(
        &self,
        scope_id: &str,
    ) -> std::result::Result<Option<BuildModeSelection>, String> {
        let mana_dir = api::find_mana_dir(&self.cwd).map_err(|err| err.to_string())?;
        let graph = api::get_tree(&mana_dir, scope_id).map_err(|err| err.to_string())?;
        let Some(candidate) = first_open_child(&graph) else {
            return Ok(None);
        };
        let unit = api::get_unit(&mana_dir, &candidate.id).map_err(|err| err.to_string())?;
        if !unit.decisions.is_empty() {
            return Ok(Some(BuildModeSelection::Blocked(BuildModeBlockedTask {
                id: unit.id,
                decisions: unit.decisions,
            })));
        }
        Ok(Some(BuildModeSelection::Task(BuildModeTask {
            id: unit.id,
            title: unit.title,
            description: unit.description,
            design: unit.design,
            acceptance: unit.acceptance,
            notes: unit.notes,
            verify_fast: unit.verify_fast,
            verify: unit.verify,
            verify_timeout: unit.verify_timeout,
            paths: unit.paths,
            dependencies: unit.dependencies,
            requires: unit.requires,
            produces: unit.produces,
            decisions: unit.decisions,
        })))
    }

    fn spawn_agent_for_prompt_in_cwd(
        &mut self,
        prompt: &str,
        agent_cwd: PathBuf,
    ) -> Result<(), String> {
        let auth_path = imp_core::storage::global_auth_path();
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

        let requested_max_tokens = self.config.max_tokens;

        let lua_cwd = agent_cwd.clone();
        let user_config_dir = imp_core::config::Config::user_config_dir();
        let (mut agent, handle) = AgentBuilder::new(config, agent_cwd, model, api_key)
            .lua_tool_loader(move |policy, tools| {
                imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools, policy);
            })
            .build()
            .map_err(|e: imp_core::error::Error| e.to_string())?;

        // Wire TuiInterface so the ask tool works
        let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(16);
        let tui_ui = crate::tui_interface::TuiInterface::new(ui_tx);
        agent.ui = tui_ui.clone();
        self.lua_command_ui = Some(tui_ui);
        self.ui_rx = Some(ui_rx);

        // Apply max_turns override from CLI
        if let Some(max_turns) = self.max_turns_override {
            agent.max_turns = max_turns;
        }
        if let Some(max_tokens) = requested_max_tokens {
            agent.max_tokens = Some(max_tokens);
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

        if let Some(workflow_context) = self.workflow_context_prompt() {
            agent.messages.push(Message::user(workflow_context));
        }

        let prompt = prompt.to_string();
        let task = tokio::spawn(async move { agent.run(prompt).await });

        self.agent_handle = Some(handle);
        self.agent_task = Some(task);
        Ok(())
    }

    fn try_prompt_command(&mut self, text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        if let Some(cmd) = trimmed.strip_prefix("!!") {
            self.run_shell_command(cmd.trim());
            return true;
        }

        if let Some(cmd) = trimmed.strip_prefix('!') {
            self.run_shell_command(cmd.trim());
            return true;
        }

        if let Some(cmd) = trimmed.strip_prefix(':') {
            let cmd = cmd.trim();
            if cmd.is_empty() {
                self.push_system_msg("Usage: :cd <path>, :pwd, :! <command>, or : <command>");
                return true;
            }
            if let Some(path) = cmd.strip_prefix("cd").and_then(command_arg) {
                self.change_working_directory(path);
                return true;
            }
            if cmd == "pwd" {
                self.push_system_msg(&self.cwd.display().to_string());
                return true;
            }
            let shell_cmd = cmd.strip_prefix('!').map(str::trim).unwrap_or(cmd);
            self.run_shell_command(shell_cmd);
            return true;
        }

        false
    }

    fn change_working_directory(&mut self, path: &str) {
        if path.is_empty() {
            self.push_system_msg(&self.cwd.display().to_string());
            return;
        }
        let target = expand_prompt_path(path, &self.cwd);
        match target.canonicalize() {
            Ok(path) if path.is_dir() => {
                self.cwd = path;
                self.push_system_msg(&format!("cwd: {}", self.cwd.display()));
            }
            Ok(path) => self.push_error_msg(&format!("Not a directory: {}", path.display())),
            Err(error) => self.push_error_msg(&format!("cd failed: {error}")),
        }
    }

    fn run_shell_command(&mut self, command: &str) {
        if command.is_empty() {
            self.push_system_msg("Usage: ! <command> or !! <command>");
            return;
        }
        match Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.cwd)
            .output()
        {
            Ok(output) => {
                let mut text = format!("$ {command}\n");
                text.push_str(&String::from_utf8_lossy(&output.stdout));
                text.push_str(&String::from_utf8_lossy(&output.stderr));
                if !output.status.success() {
                    text.push_str(&format!("\n(exit {})", output.status));
                }
                self.push_system_msg(text.trim_end());
            }
            Err(error) => self.push_error_msg(&format!("Shell command failed: {error}")),
        }
    }

    fn queue_streaming_message(&mut self, message: QueuedMessage) {
        if let Some(previous) = self.message_queue.pop() {
            self.send_steering_message(previous.text().to_string());
        }
        self.message_queue.push(message);
        self.editor.clear();
        self.needs_redraw = true;
    }

    fn send_steering_message(&mut self, text: String) {
        if text.trim().is_empty() {
            return;
        }
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
        let _ = self.session.append(SessionEntry::Message {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            message: imp_llm::Message::user(&text),
        });
        if let Some(ref handle) = self.agent_handle {
            let _ = handle.command_tx.try_send(AgentCommand::Steer(text));
        }
    }

    fn queued_message_preview(&self, terminal_width: u16) -> Option<String> {
        let text = self.message_queue.first()?.text();
        let max_chars = (terminal_width as usize / 2).max(8);
        Some(truncate_chars_with_suffix(
            &single_line_preview(text),
            max_chars,
            "…",
        ))
    }

    fn send_message(&mut self) {
        let text = self.editor.content().to_string();
        if text.trim().is_empty() {
            return;
        }

        if self.try_prompt_command(&text) {
            self.editor.push_history();
            self.editor.clear();
            return;
        }
        // Check for slash commands. Only a single-line, slash-prefixed input is
        // treated as a command; pasted absolute paths or file contents can start
        // with `/` and must still be sent to the agent as normal text.
        if !text.contains('\n') {
            if let Some(cmd_text) = text.strip_prefix('/') {
                let typed = cmd_text.trim();
                let canonical_typed = if typed.eq_ignore_ascii_case("improve safe") {
                    "improve safe"
                } else {
                    typed
                };
                // Resolve prefix: exact match first, then unique prefix match.
                // Keep the original text for /skill:<name> so arguments survive.
                let commands = self.slash_commands();
                let cmd = if canonical_typed == "improve safe" {
                    canonical_typed.to_string()
                } else if canonical_typed.starts_with("skill:") {
                    canonical_typed.to_string()
                } else {
                    commands
                        .iter()
                        .find(|c| c.name == canonical_typed)
                        .or_else(|| {
                            commands
                                .iter()
                                .find(|c| c.name.starts_with(canonical_typed))
                        })
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| canonical_typed.to_string())
                };
                self.execute_command(&cmd);
                self.editor.push_history();
                self.editor.clear();
                return;
            }
        }

        // Add user message to display
        if self.compaction_task.is_some() || self.lua_command_task.is_some() {
            self.push_system_msg(
                "A background slash command is running; wait for it to finish before sending a new prompt.",
            );
            return;
        }

        let user_message_index = self.messages.len();
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
        self.streaming_anchor_user_index = Some(user_message_index);
        self.completed_turns_in_run = 0;
        self.suppress_completion_notification = false;
        self.auto_scroll = true;
        self.scroll_offset = 0;
        self.tool_focus = None;
        self.tool_focus_pinned = false;
        self.sidebar_auto_follow = true;
        self.editor.push_history();
        self.editor.clear();

        let mut agent_prompt = text.clone();
        if self.try_launch_build_team(&text) {
            return;
        }
        if let Some(build_prompt) = self.build_mode_prompt_for_text(&text) {
            agent_prompt = build_prompt;
        }
        self.pending_agent_prompt = Some(agent_prompt);
        self.pending_agent_cwd = None;
        self.needs_redraw = true;
    }

    fn start_pending_agent_after_redraw(&mut self) {
        let Some(text) = self.pending_agent_prompt.take() else {
            return;
        };
        let agent_cwd = self
            .pending_agent_cwd
            .take()
            .unwrap_or_else(|| self.cwd.clone());

        if let Err(error) = self.spawn_agent_for_prompt_in_cwd(&text, agent_cwd) {
            self.is_streaming = false;
            self.streaming_anchor_user_index = None;
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
            self.needs_redraw = true;
        }
    }

    fn restore_checkpoint_command(&mut self, needle: &str) {
        match self.session.find_checkpoint_record(needle) {
            None => self.push_system_msg(&format!("Checkpoint not found: {needle}")),
            Some(record) => {
                let mut lines = vec![format!(
                    "Checkpoint `{}` is recorded for this session, but TUI restore is not wired yet.",
                    record.checkpoint_id
                )];
                if let Some(label) = record.label {
                    lines.push(format!("Label: {label}"));
                }
                if !record.files.is_empty() {
                    lines.push("Files:".into());
                    for path in record.files {
                        lines.push(format!("- {path}"));
                    }
                }
                self.push_system_msg(&lines.join("\n"));
            }
        }
    }

    fn active_mana_run_label(&self) -> Option<String> {
        self.active_mana_run
            .as_ref()
            .map(|run| format!("run {} {}", run.run_id, run.status))
    }

    fn active_mana_scope_label(&self) -> Option<String> {
        self.active_mana_scope.as_ref().map(|scope| {
            let mut title = scope.title.trim().to_string();
            const MAX_TITLE_CHARS: usize = 42;
            if title.chars().count() > MAX_TITLE_CHARS {
                title = title.chars().take(MAX_TITLE_CHARS).collect::<String>();
                title.push('…');
            }
            if title.is_empty() {
                format!("mana {}", scope.id)
            } else {
                format!("mana {} {}", scope.id, title)
            }
        })
    }

    fn set_active_mana_run(&mut self, id: &str) {
        let id = id.trim();
        if id.is_empty() {
            let Some(active_id) = self.active_mana_run.as_ref().map(|run| run.run_id.clone())
            else {
                self.push_system_msg("Usage: /run <run-id> or /run clear");
                return;
            };
            self.refresh_active_mana_run(&active_id);
            return;
        }
        if id.eq_ignore_ascii_case("clear") || id.eq_ignore_ascii_case("none") {
            self.active_mana_run = None;
            self.push_system_msg("Active mana run cleared");
            return;
        }

        self.refresh_active_mana_run(id);
    }

    fn refresh_active_mana_run(&mut self, id: &str) {
        match mana_run_summary(id) {
            Ok(Some(summary)) => {
                self.push_system_msg(&format!(
                    "Active mana run: {} {} ({}/{}, failed {})",
                    summary.run_id,
                    summary.status,
                    summary.total_closed,
                    summary.total_units,
                    summary.total_failed
                ));
                self.active_mana_run = Some(summary);
            }
            Ok(None) => self.push_system_msg(&format!("Could not find mana run {id}")),
            Err(err) => self.push_system_msg(&format!("Could not read mana run {id}: {err}")),
        }
    }

    fn set_active_mana_scope(&mut self, id: &str) {
        let id = id.trim();
        if id.is_empty() {
            self.push_system_msg("Usage: /scope <mana-id> or /scope clear");
            return;
        }
        if id.eq_ignore_ascii_case("clear") || id.eq_ignore_ascii_case("none") {
            self.active_mana_scope = None;
            self.build_auto_turns = 0;
            self.last_build_auto_task_id = None;
            self.improve_auto_turns = 0;
            self.improve_sandbox = None;
            self.push_system_msg("Active mana scope cleared");
            return;
        }

        match self.resolve_mana_scope(id) {
            Ok(scope) => {
                let label = if scope.title.trim().is_empty() {
                    scope.id.clone()
                } else {
                    format!("{} {}", scope.id, scope.title.trim())
                };
                self.active_mana_scope = Some(scope);
                self.build_auto_turns = 0;
                self.last_build_auto_task_id = None;
                self.improve_auto_turns = 0;
                self.improve_sandbox = None;
                self.push_system_msg(&format!("Active mana scope: {label}"));
                self.queue_build_mode_continuation_if_ready();
                self.queue_improve_mode_continuation_if_ready();
            }
            Err(err) => {
                self.push_system_msg(&format!("Could not set mana scope {id}: {err}"));
            }
        }
    }

    fn resolve_mana_scope(&self, id: &str) -> std::result::Result<ManaUnitRef, String> {
        let mana_dir = api::find_mana_dir(&self.cwd).map_err(|err| err.to_string())?;
        let unit = api::get_unit(&mana_dir, id).map_err(|err| err.to_string())?;
        Ok(ManaUnitRef::new(
            &unit.id,
            &unit.title,
            Some(format!("{:?}", unit.kind)),
        ))
    }

    fn maybe_update_active_mana_scope_from_review(&mut self, review: &TurnManaReview) {
        let Some(scope) = candidate_active_scope_from_review(review) else {
            return;
        };

        if self
            .active_mana_scope
            .as_ref()
            .is_some_and(|active| active.id == scope.id)
        {
            return;
        }

        self.active_mana_scope = Some(scope);
        self.build_auto_turns = 0;
        self.last_build_auto_task_id = None;
        self.improve_auto_turns = 0;
        self.improve_sandbox = None;
    }

    fn set_workflow_mode(&mut self, mode: WorkflowMode) {
        self.workflow_mode = mode;
        self.build_auto_turns = 0;
        self.last_build_auto_task_id = None;
        self.improve_auto_turns = 0;
        self.improve_sandbox = None;
        self.improve_safe_mode = false;
        self.push_system_msg(&format!("Workflow mode: {}", mode.display_name()));
        self.queue_build_mode_continuation_if_ready();
        self.queue_improve_mode_continuation_if_ready();
    }

    fn set_improve_mode(&mut self, safe: bool) {
        self.workflow_mode = WorkflowMode::Improve;
        self.build_auto_turns = 0;
        self.last_build_auto_task_id = None;
        self.improve_auto_turns = 0;
        self.improve_sandbox = None;
        self.improve_safe_mode = safe;
        if safe {
            self.push_system_msg("Workflow mode: Improve safe (research-only)");
        } else {
            self.push_system_msg("Workflow mode: Improve (sandbox branch/worktree)");
        }
        self.queue_improve_mode_continuation_if_ready();
    }

    fn execute_command(&mut self, cmd: &str) {
        let mut parts = cmd.splitn(2, char::is_whitespace);
        let command = parts.next().unwrap_or("");
        let args = parts.next().unwrap_or("").trim();

        match command {
            "quit" | "q" => {
                self.running = false;
            }
            "model" => {
                self.open_model_selector();
            }
            "tree" => {
                self.open_tree_view();
            }
            "mana" => {
                self.open_mana_navigator(if args.is_empty() { None } else { Some(args) });
            }
            "new" => {
                self.messages.clear();
                self.invalidate_chat_render_cache();
                self.session = SessionManager::in_memory();
                self.tool_focus = None;
                self.tool_focus_pinned = false;
                self.sidebar_auto_follow = true;
                self.invalidate_chat_render_cache();
                self.accumulated_usage = Usage::default();
                self.accumulated_cost = Cost::default();
                self.current_context_tokens = 0;
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
  Ctrl+O        Open selected read file in editor\n\
  Ctrl+Up/Down  Focus previous/next tool\n\
  Shift+Tab     Cycle thinking level\n\
  @             File finder\n\
  /command      Slash commands\n\
  ! <cmd>       Run shell command in current cwd\n\
  !! <cmd>      Run shell command without adding output to agent context\n\
  :cd <path>    Change working directory\n\
  :pwd          Show working directory\n\
  : <cmd>       Run shell command\n\
  PageUp/Down   Scroll",
                );
            }
            "explore" => self.set_workflow_mode(WorkflowMode::Explore),
            "plan" => self.set_workflow_mode(WorkflowMode::Plan),
            "build" => self.set_workflow_mode(WorkflowMode::Build),
            "improve" => match args {
                arg if matches!(arg, "merge" | "adopt" | "approve") => {
                    self.improve_merge_command(args)
                }
                arg => self.set_improve_mode(arg.eq_ignore_ascii_case("safe")),
            },
            "improve-safe" => self.set_improve_mode(true),
            "improve-merge" => self.improve_merge_command("merge"),
            "improve-help" => self.push_system_msg(
                "Improve uses a new branch checked out in a separate worktree before making code changes. It never commits or merges without explicit approval. Use /improve safe for research-only evaluation and mana follow-ups.",
            ),
            "status" => self.show_status_command(),
            "clean" => self.clean_command(args),
            "loop" => self.start_loop_command(args),
            "scope" | "mana-scope" => self.set_active_mana_scope(args),
            "run" => self.set_active_mana_run(args),
            "stop" => self.stop_active_work(),
            "settings" => {
                self.open_settings();
            }
            "personality" => {
                self.open_personality();
            }
            "resume" => {
                let session_dir = imp_core::storage::global_sessions_dir();
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
                let path = imp_core::storage::global_sessions_dir()
                    .join(format!("{}.jsonl", uuid::Uuid::new_v4()));
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
            "checkpoints" => {
                let checkpoints = self.session.checkpoint_records();
                if checkpoints.is_empty() {
                    self.push_system_msg("No checkpoints recorded in this session.");
                } else {
                    let mut lines = vec![format!("{} checkpoint(s):", checkpoints.len())];
                    for checkpoint in checkpoints {
                        let label = checkpoint
                            .label
                            .as_deref()
                            .map(|label| format!(" — {label}"))
                            .unwrap_or_default();
                        lines.push(format!(
                            "- {}{} ({} file{})",
                            checkpoint.checkpoint_id,
                            label,
                            checkpoint.files.len(),
                            if checkpoint.files.len() == 1 { "" } else { "s" }
                        ));
                    }
                    self.push_system_msg(&lines.join("\n"));
                }
            }
            "restore-checkpoint" => {
                let needle = cmd.strip_prefix("restore-checkpoint").unwrap_or("").trim();
                if needle.is_empty() {
                    self.push_system_msg("Usage: /restore-checkpoint <checkpoint id or label>");
                } else {
                    self.restore_checkpoint_command(needle);
                }
            }
            "help" => {
                self.push_system_msg(concat!(
                    "Commands:\n",
                    "  /new        — start fresh session\n",
                    "  /model      — switch model\n",
                    "  /mana [id]  — browse mana work graph\n",
                    "  /scope <id> — set active mana scope\n",
                    "  /build      — switch to Build mode\n",
                    "  /improve    — improve in a sandbox branch/worktree\n",
                    "  /improve safe — research-only Improve mode\n",
                    "  /improve merge — show Improve merge plan\n",
                    "  /improve merge --confirm — merge active Improve branch\n",
                    "  /status    — show active work status\n",
                    "  /loop <msg> — repeat a prompt until stopped/budgeted\n",
                    "  /clean     — clean active sandbox/artifacts safely\n",
                    "  /stop       — stop active imp work\n",
                    "  /compact    — compress context\n",
                    "  /resume     — resume/search sessions\n",
                    "  /session    — legacy alias (defunct)\n",
                    "  /fork       — branch conversation\n",
                    "  /name <n>   — rename session\n",
                    "  /export [f] — export to markdown\n",
                    "  /copy       — copy selection or last response\n",
                    "  /memory     — view/edit agent memory\n",
                    "  /checkpoints — list recorded file checkpoints\n",
                    "  /restore-checkpoint <id> — inspect restore target for a checkpoint\n",
                    "  /reload     — reload config\n",
                    "  /settings   — edit settings\n",
                    "  /personality — customize imp personality\n",
                    "  /login [provider]   — OAuth login (Anthropic/OpenAI/Kimi Code)\n",
                    "  /secrets [provider] — save/list API keys & service secrets\n",
                    "  /help       — this message\n",
                    "  :cd <path>  — change working directory\n",
                    "  :pwd        — show working directory\n",
                    "  : <cmd>     — run shell command\n",
                    "  ! <cmd>     — run shell command\n",
                    "  !! <cmd>    — run shell command without adding output to agent context\n",
                    "\nTools: web.read supports web pages and public YouTube URLs (metadata + captions when available).\n",
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
                if let Some(last) = self.messages.iter().rev().find(|m| {
                    matches!(
                        m.role,
                        MessageRole::Assistant | MessageRole::Warning | MessageRole::Error
                    )
                }) {
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
                if !self.try_lua_command(cmd) && !self.try_skill_command(cmd) {
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

    fn slash_commands(&self) -> Vec<crate::views::command_palette::SlashCommand> {
        let extension_commands = self
            .lua_runtime
            .as_ref()
            .and_then(|runtime| runtime.lock().ok().map(|guard| guard.command_summaries()))
            .unwrap_or_default();
        let commands = merge_extension_commands(builtin_commands(), extension_commands);
        merge_skill_commands(commands, self.skill_summaries())
    }

    fn skill_summaries(&self) -> Vec<(String, String)> {
        let user_config_dir = Config::user_config_dir();
        imp_core::resources::discover_skills(&self.cwd, &user_config_dir)
            .into_iter()
            .map(|skill| (skill.name, skill.description))
            .collect()
    }

    fn try_skill_command(&mut self, cmd: &str) -> bool {
        let (skill_name, args) = if let Some(rest) = cmd.strip_prefix("skill:") {
            let skill_name = rest.split_whitespace().next().unwrap_or("");
            let args = rest.strip_prefix(skill_name).unwrap_or("").trim();
            (skill_name, args)
        } else {
            let skill_name = cmd.split_whitespace().next().unwrap_or("");
            let args = cmd.strip_prefix(skill_name).unwrap_or("").trim();
            (skill_name, args)
        };

        if skill_name.is_empty() {
            return false;
        }

        let user_config_dir = Config::user_config_dir();
        let Some(skill) = imp_core::resources::discover_skills(&self.cwd, &user_config_dir)
            .into_iter()
            .find(|skill| skill.name == skill_name)
        else {
            return false;
        };

        let content = match std::fs::read_to_string(&skill.path) {
            Ok(content) => content,
            Err(error) => {
                self.push_error_msg(&format!("Failed to load skill `{skill_name}`: {error}"));
                return true;
            }
        };

        let prompt = imp_core::resources::render_skill_invocation(skill_name, &content, args);
        self.editor.set_content(&prompt);
        self.send_message();
        true
    }

    /// Reload Lua extensions: re-scan directories, re-create runtime, and update
    /// the stored runtime handle. Tools are not re-registered on the running
    /// agent (only new agents will pick them up), but commands become available
    /// immediately.
    fn reload_lua_extensions(&mut self) {
        let user_config_dir = Config::user_config_dir();
        let policy = self
            .config
            .lua
            .resolve_policy(imp_core::config::AgentMode::Full);
        match imp_lua::reload(&user_config_dir, Some(&self.cwd), &policy) {
            Ok((rt, _exts)) => {
                self.lua_runtime = Some(Arc::new(Mutex::new(rt)));
            }
            Err(e) => {
                self.push_system_msg(&format!("Lua reload failed: {e}"));
                self.lua_runtime = None;
            }
        }
    }

    fn lua_command_call_context(&self) -> imp_lua::LuaCallContext {
        let (update_tx, _update_rx) = tokio::sync::mpsc::channel(16);
        let (command_tx, _command_rx) = tokio::sync::mpsc::channel(16);
        let ui: Arc<dyn imp_core::ui::UserInterface> = self
            .lua_command_ui
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| Arc::new(imp_core::ui::NullInterface));
        imp_lua::LuaCallContext {
            cwd: self.cwd.clone(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx,
            command_tx,
            ui,
            file_cache: Arc::new(imp_core::tools::FileCache::new()),
            checkpoint_state: Arc::new(imp_core::tools::CheckpointState::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(
                imp_core::tools::FileTracker::default(),
            )),
            anchor_store: Arc::new(imp_core::tools::AnchorStore::new()),
            lua_tool_loader: None,
            mode: imp_core::config::AgentMode::Full,
            read_max_lines: self.config.ui.read_max_lines,
            run_policy: Default::default(),
            config: Arc::new(self.config.clone()),
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
        drop(guard);

        // Execute via LuaRuntime's helper (keeps mlua types internal) on a
        // background task so extension commands share /compact's non-blocking
        // transcript animation and completion flow.
        if self.lua_command_task.is_some() {
            self.push_system_msg("A Lua command is already running.");
            return true;
        }

        let command_label = cmd_name.to_string();
        let args = args.to_string();
        let call_ctx = self.lua_command_call_context();
        self.messages.push(DisplayMessage {
            role: MessageRole::Compaction,
            content: format!("Running /{command_label}…"),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: true,
            timestamp: imp_llm::now(),
        });
        self.auto_scroll = true;
        self.scroll_offset = 0;
        self.invalidate_chat_render_cache();

        let task_command = command_label.clone();
        let run_lua_command = move || {
            let result = match runtime.lock() {
                Ok(guard) => guard
                    .execute_command_with_context(&task_command, &args, Some(call_ctx))
                    .map_err(|error| error.to_string()),
                Err(_) => Err("Lua runtime lock poisoned".to_string()),
            };
            (task_command, result)
        };

        if tokio::runtime::Handle::try_current().is_ok() {
            self.lua_command_task = Some(tokio::task::spawn_blocking(run_lua_command));
        } else {
            let (command, result) = run_lua_command();
            let signal = match result {
                Ok(result) if lua_result_requests_restart(result.as_deref()) => {
                    RuntimeSignal::LuaCommandRestartRequested { command, result }
                }
                Ok(result) => RuntimeSignal::LuaCommandCompleted { command, result },
                Err(error) => RuntimeSignal::LuaCommandFailed { command, error },
            };
            self.handle_runtime_signal(signal);
        }
        true
    }

    fn restart_after_lua_command(&mut self) {
        match std::env::current_exe() {
            Ok(exe) => match std::process::Command::new(&exe).spawn() {
                Ok(_) => {
                    self.push_system_msg("Restarting imp into the updated binary…");
                    self.running = false;
                }
                Err(error) => {
                    self.push_error_msg(&format!(
                        "Restart requested, but failed to launch {}: {error}",
                        exe.display()
                    ));
                }
            },
            Err(error) => {
                self.push_error_msg(&format!(
                    "Restart requested, but failed to resolve current imp executable: {error}"
                ));
            }
        }
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
            "kimi-code" => "Opening browser for Kimi Code login...",
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

        let auth_path = imp_core::storage::global_auth_path();
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
                "kimi-code" => {
                    imp_llm::oauth::kimi_code::KimiCodeOAuth::new()
                        .login(
                            |url| {
                                open_url(url);
                            },
                            |_msg| {
                                // Messages are silently dropped in the TUI background task;
                                // the browser URL is the primary signal.
                            },
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
                        "kimi-code" => {
                            let _ = store.store(
                                "kimi-code",
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
        let auth_path = imp_core::storage::global_auth_path();
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
        let auth_path = imp_core::storage::global_auth_path();
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
        let auth_path = imp_core::storage::global_auth_path();
        let auth_store =
            AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
        let state = SettingsState::new(&self.config, &self.model_name, &models, &auth_store);
        self.mode = UiMode::Settings(state);
    }

    fn open_personality(&mut self) {
        let user_config_dir = Config::user_config_dir();
        let global_path = user_config_dir.join("soul.md");
        let project_soul = imp_core::resources::discover_project_soul(&self.cwd);
        let project_path = project_soul
            .as_ref()
            .map(|soul| soul.path.clone())
            .unwrap_or_else(|| imp_core::resources::suggested_project_soul_path(&self.cwd));
        let scope = if project_soul.is_some() {
            PersonalityScope::Project
        } else {
            PersonalityScope::Global
        };
        let state = PersonalityState::from_paths(global_path, project_path, scope);
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
                    let cursor = state.cursor.min(state.options.len().saturating_sub(1));
                    state.options.get(cursor).map(|opt| opt.label.clone())
                } else {
                    None
                };
                if let Some(text) = replacement {
                    self.editor.set_content(&text);
                    self.editor.move_end();
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

        // Show Q&A in chat as user-style messages so they stay visually distinct
        // (System messages render muted/grey which makes them look faded.)
        self.messages.push(DisplayMessage {
            role: MessageRole::User,
            content: state.question.clone(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });

        match (&result, reply) {
            (AskResult::Text(text), Some(AskReply::Input(tx))) => {
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
                let _ = tx.send(Some(text.clone()));
                self.advance_secrets_flow(Some(text.clone()));
            }
            (AskResult::Selected(indices), Some(AskReply::Select(tx))) => {
                let labels: Vec<String> = indices
                    .iter()
                    .filter_map(|&i| state.options.get(i).map(|o| o.label.clone()))
                    .collect();
                self.messages.push(DisplayMessage {
                    role: MessageRole::User,
                    content: labels.join(", "),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
                self.invalidate_chat_render_cache();
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
                    self.messages.push(DisplayMessage {
                        role: MessageRole::User,
                        content: state.options[idx].label.clone(),
                        thinking: None,
                        tool_calls: Vec::new(),
                        assistant_blocks: Vec::new(),
                        is_streaming: false,
                        timestamp: imp_llm::now(),
                    });
                    self.invalidate_chat_render_cache();
                    let _ = tx.send(Some(idx));
                } else {
                    // No match — send None. The ask tool will get "User cancelled".
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
                    let _ = tx.send(None);
                }
            }
            (AskResult::Selected(indices), Some(AskReply::MultiSelect(tx))) => {
                let labels: Vec<String> = indices
                    .iter()
                    .filter_map(|&i| state.options.get(i).map(|o| o.label.clone()))
                    .collect();
                self.messages.push(DisplayMessage {
                    role: MessageRole::User,
                    content: labels.join(", "),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
                self.invalidate_chat_render_cache();
                let _ = tx.send(Some(indices.clone()));
            }
            (AskResult::Text(text), Some(AskReply::MultiSelect(tx))) => {
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
                let indices: Vec<usize> = state
                    .options
                    .iter()
                    .enumerate()
                    .filter_map(|(index, option)| {
                        option.label.eq_ignore_ascii_case(text).then_some(index)
                    })
                    .collect();
                let _ = tx.send((!indices.is_empty()).then_some(indices));
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

                let field = fields
                    .get(current)
                    .cloned()
                    .unwrap_or_else(|| "api_key".into());
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

                let auth_path = imp_core::storage::global_auth_path();
                let mut auth_store = AuthStore::load(&auth_path)
                    .unwrap_or_else(|_| AuthStore::new(auth_path.clone()));
                match auth_store.store_secret_fields(&provider, values) {
                    Ok(()) => {
                        self.push_system_msg(&format!("Saved secure secrets for {provider}."))
                    }
                    Err(e) => {
                        self.push_error_msg(&format!("Failed to save secrets for {provider}: {e}"))
                    }
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
                AskReply::MultiSelect(tx) => {
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
            KeyCode::Tab => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.switch_tab_forward();
                }
            }
            KeyCode::BackTab => {
                if let UiMode::Settings(ref mut state) = self.mode {
                    state.switch_tab_backward();
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
        match key.code {
            KeyCode::Esc => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    if state.pending_overwrite.is_some() {
                        state.cancel_overwrite();
                    } else {
                        self.mode = UiMode::Normal;
                    }
                }
            }
            KeyCode::Tab => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    state.switch_tab();
                }
            }
            KeyCode::Up => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    match state.tab {
                        crate::views::personality::PersonalityTab::Builder => state.move_up(),
                        crate::views::personality::PersonalityTab::Source => {
                            state.editor.move_up();
                        }
                    }
                }
            }
            KeyCode::Down => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    match state.tab {
                        crate::views::personality::PersonalityTab::Builder => state.move_down(),
                        crate::views::personality::PersonalityTab::Source => {
                            state.editor.move_down();
                        }
                    }
                }
            }
            KeyCode::Left => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    match state.tab {
                        crate::views::personality::PersonalityTab::Builder => {
                            state.cycle_backward()
                        }
                        crate::views::personality::PersonalityTab::Source => state.move_left(),
                    }
                }
            }
            KeyCode::Right => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    match state.tab {
                        crate::views::personality::PersonalityTab::Builder => state.cycle_forward(),
                        crate::views::personality::PersonalityTab::Source => state.move_right(),
                    }
                }
            }
            KeyCode::Enter => {
                let should_save = matches!(&self.mode, UiMode::Personality(s) if s.pending_overwrite.is_none() && matches!(s.tab, crate::views::personality::PersonalityTab::Builder) && matches!(s.current_field(), crate::views::personality::PersonalityField::Save));
                if should_save {
                    self.save_personality();
                } else if let UiMode::Personality(ref mut state) = self.mode {
                    if state.pending_overwrite.is_some() {
                        state.confirm_overwrite();
                    } else {
                        match state.tab {
                            crate::views::personality::PersonalityTab::Builder => {
                                state.cycle_forward()
                            }
                            crate::views::personality::PersonalityTab::Source => {
                                state.insert_newline()
                            }
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    if state.pending_overwrite.is_none()
                        && matches!(state.tab, crate::views::personality::PersonalityTab::Source)
                    {
                        state.pop_char();
                    }
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    if state.pending_overwrite.is_some() {
                        state.confirm_overwrite();
                    } else if matches!(state.tab, crate::views::personality::PersonalityTab::Source)
                    {
                        if let KeyCode::Char(c) = key.code {
                            state.insert_char(c);
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    if state.pending_overwrite.is_some() {
                        state.cancel_overwrite();
                    } else if matches!(state.tab, crate::views::personality::PersonalityTab::Source)
                    {
                        if let KeyCode::Char(c) = key.code {
                            state.insert_char(c);
                        }
                    }
                }
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save_personality();
            }
            KeyCode::Char(c) => {
                if let UiMode::Personality(ref mut state) = self.mode {
                    if state.pending_overwrite.is_none()
                        && matches!(state.tab, crate::views::personality::PersonalityTab::Source)
                    {
                        state.insert_char(c);
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
                let provider_id = state
                    .selected_provider_id()
                    .unwrap_or("anthropic")
                    .to_string();
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
        let config_path = imp_core::storage::global_config_path();
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

        let auth_path = imp_core::storage::global_auth_path();
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

        let path = state.current_path().clone();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.push_error_msg(&format!("Failed to create soul directory: {e}"));
                return;
            }
        }

        let content = if state.editor.is_empty() {
            default_soul_markdown()
        } else {
            state.editor.content().to_string()
        };

        match std::fs::write(&path, content) {
            Ok(()) => {
                if let UiMode::Personality(ref mut current) = self.mode {
                    current.save_success();
                }
                self.push_system_msg(&format!("Soul saved to {}", path.display()));
            }
            Err(e) => self.push_error_msg(&format!("Failed to save soul: {e}")),
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

        let auth_path = imp_core::storage::global_auth_path();
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
        let config_path = imp_core::storage::global_config_path();
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
        let auth_path = imp_core::storage::global_auth_path();
        let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
        filtered_model_options(&self.model_registry, &self.config, &auth_store)
    }

    fn open_model_selector(&mut self) {
        let models = self.filtered_models();
        let (models, current_model) =
            include_current_model_option(models, &self.model_registry, &self.model_name);
        self.mode = UiMode::ModelSelector(ModelSelectorState::new(models, current_model));
    }

    fn open_file_finder(&mut self) {
        let files = collect_project_files(&self.cwd, 5000);
        self.mode = UiMode::FileFinder(FileFinderState::new(files));
    }

    fn open_mana_navigator(&mut self, initial_id: Option<&str>) {
        self.mode = UiMode::ManaNavigator(ManaNavigatorState::load(&self.cwd, initial_id));
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
        self.push_message(MessageRole::System, content);
    }

    fn push_warning_msg(&mut self, content: &str) {
        self.push_message(MessageRole::Warning, content);
    }

    fn push_error_msg(&mut self, content: &str) {
        self.push_message(MessageRole::Error, content);
    }

    fn push_message(&mut self, role: MessageRole, content: &str) {
        self.messages.push(DisplayMessage {
            role,
            content: content.to_string(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });
        self.invalidate_chat_render_cache();
    }

    fn latest_streaming_message_mut(&mut self) -> Option<&mut DisplayMessage> {
        self.messages.iter_mut().rev().find(|msg| msg.is_streaming)
    }

    fn find_tool_call_mut(&mut self, tool_call_id: &str) -> Option<&mut DisplayToolCall> {
        for msg in self.messages.iter_mut().rev() {
            if let Some(tc) = msg.tool_calls.iter_mut().find(|tc| tc.id == tool_call_id) {
                return Some(tc);
            }
        }
        None
    }

    fn run_manual_compaction(&mut self) {
        if self.is_streaming {
            self.push_error_msg("Cannot compact while the agent is actively streaming.");
            return;
        }
        if self.compaction_task.is_some() {
            self.push_system_msg("Compaction is already running.");
            return;
        }

        let active_messages = self.session.get_active_messages();
        let prepared =
            prepare_messages_for_compaction(&active_messages, DEFAULT_KEEP_RECENT_GROUPS);
        if !prepared.should_compact() {
            self.push_system_msg("Not enough history to compact yet.");
            return;
        }

        let auth_path = imp_core::storage::global_auth_path();
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

        let model = Model {
            meta,
            provider: Arc::from(provider),
        };
        let model_id = model.meta.id.clone();
        let model_meta = model.meta.clone();
        let model_provider = Arc::clone(&model.provider);
        let requested_max_tokens = self.config.max_tokens;
        let thinking_level = self.thinking_level;

        let mut config = self.config.clone();
        config.thinking = Some(thinking_level);

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

        self.messages.push(DisplayMessage {
            role: MessageRole::Compaction,
            content: "Compacting context…".to_string(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: true,
            timestamp: imp_llm::now(),
        });
        self.auto_scroll = true;
        self.scroll_offset = 0;
        self.invalidate_chat_render_cache();

        let cwd = self.cwd.clone();
        let lua_cwd = self.cwd.clone();
        let user_config_dir = imp_core::config::Config::user_config_dir();
        let task = tokio::spawn(async move {
            let api_key = resolve_provider_api_key(&mut auth_store, &provider_name)
                .await
                .map_err(|e| format!("Failed to resolve auth for compaction: {e}"))?;

            let model = Model {
                meta: model_meta.clone(),
                provider: Arc::clone(&model_provider),
            };
            let (agent, _handle) = AgentBuilder::new(config, cwd, model, api_key)
                .lua_tool_loader(move |policy, tools| {
                    imp_lua::init_lua_extensions(&user_config_dir, Some(&lua_cwd), tools, policy);
                })
                .build()
                .map_err(|e| format!("Failed to build compaction agent: {e}"))?;

            let system_prompt = agent.system_prompt.clone();
            let retry_policy = agent.retry_policy.clone();
            execute_compaction_with_retry(
                &mut SessionManager::in_memory_with_messages(active_messages),
                DEFAULT_KEEP_RECENT_GROUPS,
                2,
                |prompt| {
                    use futures::StreamExt;
                    use imp_llm::provider::{CacheOptions, Context as LlmContext, RequestOptions};

                    let model_meta = model_meta.clone();
                    let model_provider = Arc::clone(&model_provider);
                    let api_key = agent.api_key.clone();
                    let system_prompt = system_prompt.clone();
                    let prompt = prompt.to_string();
                    let retry_policy = retry_policy.clone();

                    futures::executor::block_on(async move {
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
                            max_tokens: requested_max_tokens.or(Some(2048)),
                            temperature: Some(0.2),
                            system_prompt,
                            tools: Vec::new(),
                            cache_options: CacheOptions::default(),
                            effort: None,
                        };

                        let mut stream = imp_core::retry::stream_with_retry(
                            move || {
                                model.provider.stream(
                                    &model,
                                    context.clone(),
                                    options.clone(),
                                    &api_key,
                                )
                            },
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
                                            imp_llm::ContentBlock::Text { text } => {
                                                Some(text.as_str())
                                            }
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join("");
                                    if !body.is_empty() {
                                        message_end_text = Some(body);
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => return Err(error.to_string()),
                            }
                        }

                        let final_text = if !summary.trim().is_empty() {
                            summary
                        } else {
                            message_end_text.unwrap_or_default()
                        };
                        if final_text.trim().is_empty() {
                            Err("Compaction summary was empty".to_string())
                        } else {
                            Ok(final_text)
                        }
                    })
                    .ok()
                },
            )
            .map_err(|e| e.to_string())?
            .map(|result| {
                result
                    .summary
                    .trim_start_matches(COMPACTION_SUMMARY_PREFIX)
                    .to_string()
            })
            .ok_or_else(|| "Not enough history to compact yet.".to_string())
        });

        self.compaction_task = Some(task);
    }

    fn finish_compaction_status_message(&mut self, content: &str) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .rev()
            .find(|message| message.role == MessageRole::Compaction && message.is_streaming)
        {
            message.content = content.to_string();
            message.is_streaming = false;
            self.invalidate_chat_render_cache();
        }
    }

    fn finish_lua_command_status_message(&mut self, content: &str) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .rev()
            .find(|message| message.role == MessageRole::Compaction && message.is_streaming)
        {
            message.content = content.to_string();
            message.is_streaming = false;
            self.invalidate_chat_render_cache();
        }
    }

    fn finish_manual_compaction(&mut self, summary: String) {
        let result =
            execute_manual_compaction(&mut self.session, DEFAULT_KEEP_RECENT_GROUPS, |_| {
                Some(summary.clone())
            });

        match result {
            Ok(Some(compaction)) => {
                self.load_session_messages();
                self.messages.push(DisplayMessage {
                    role: MessageRole::Compaction,
                    content: format!(
                        "Context compacted. Saved ~{} tokens. Preserved recent working context.",
                        compaction
                            .tokens_before
                            .saturating_sub(compaction.tokens_after)
                    ),
                    thinking: None,
                    tool_calls: Vec::new(),
                    assistant_blocks: Vec::new(),
                    is_streaming: false,
                    timestamp: imp_llm::now(),
                });
                self.push_system_msg(
                    "Compaction summary stored. Active context now uses the compacted branch view.",
                );
            }
            Ok(None) => {
                self.finish_compaction_status_message("Not enough history to compact yet.");
            }
            Err(e) => {
                self.finish_compaction_status_message("Compaction failed.");
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
                MessageRole::Warning => "*Warning:*",
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
                self.tool_focus_pinned = false;
                self.sidebar_auto_follow = true;
                self.invalidate_chat_render_cache();
                self.begin_llm_thought_segment();
                self.turn_tracker.reset();
            }
            AgentEvent::AgentEnd { cost, .. } => {
                self.completed_turns_in_run = self.completed_turns_in_run.max(1);
                self.accumulated_cost.total += cost.total;
                self.accumulated_cost.input += cost.input;
                self.accumulated_cost.output += cost.output;
                self.is_streaming = false;
                self.streaming_anchor_user_index = None;

                // Mark last streaming message as done
                if let Some(last) = self.latest_streaming_message_mut() {
                    last.is_streaming = false;
                }
                self.invalidate_chat_render_cache();

                // Process queued messages. Follow-ups become visible user turns
                // and start the next agent run; steering messages that were still
                // queued at turn end are also surfaced and sent as the next prompt.
                let queued: Vec<_> = self.message_queue.drain(..).collect();
                for message in queued {
                    let text = message.text().to_string();
                    self.editor.set_content(&text);
                    self.send_message();
                }
                self.llm_thought_segment_started_at = None;
                self.queue_build_mode_continuation_if_ready();
                self.queue_improve_mode_continuation_if_ready();
                self.queue_loop_continuation_if_ready();
                self.maybe_notify_agent_completion();
            }
            AgentEvent::MessageDelta { delta } => {
                // Keep the current default compact: the main transcript shows
                // where the tool ran, and the sidebar inspector owns details.
                let tools_expanded = self.tools_expanded
                    && self.config.ui.effective_chat_tool_display()
                        == imp_core::config::ChatToolDisplay::Interleaved;
                let thought_duration = match &delta {
                    StreamEvent::TextDelta { text } if !text.trim().is_empty() => {
                        self.finalize_llm_thought_segment()
                    }
                    StreamEvent::ToolCall { .. } => self.finalize_llm_thought_segment(),
                    _ => None,
                };
                if let Some(last) = self.latest_streaming_message_mut() {
                    match delta {
                        StreamEvent::TextDelta { text } => {
                            if let Some(seconds) = thought_duration {
                                last.push_assistant_thought_duration(seconds);
                            }
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
                            if let Some(seconds) = thought_duration {
                                last.push_assistant_thought_duration(seconds);
                            }
                            last.push_assistant_tool_call(DisplayToolCall {
                                id,
                                args_summary: DisplayToolCall::make_args_summary(&name, &arguments),
                                name,
                                output: None,
                                details: arguments,
                                is_error: false,
                                expanded: tools_expanded,
                                streaming_lines: Vec::new(),
                                streaming_output: String::new(),
                            });
                        }
                        _ => {}
                    }
                }
                self.invalidate_chat_render_cache();
            }
            AgentEvent::ToolExecutionStart {
                tool_call_id,
                tool_name,
                args,
            } => {
                self.turn_tracker
                    .record_tool_start(&tool_call_id, &tool_name, &args);
                self.llm_thought_segment_started_at = None;
                // Find the matching tool call and update it
                if let Some(tc) = self.find_tool_call_mut(&tool_call_id) {
                    tc.args_summary = DisplayToolCall::make_args_summary(&tool_name, &args);
                    tc.details = args;
                }
                self.invalidate_chat_render_cache();
                // Sidebar: follow the new tool only until the user pins an older selection.
                if let Some(idx) = self.find_tool_call_index(&tool_call_id) {
                    if !self.tool_focus_pinned {
                        self.focus_tool_with_pin(idx, false);
                    }
                    if self.sidebar_auto_follow
                        && matches!(
                            self.config.ui.sidebar_style,
                            imp_core::config::SidebarStyle::Stream
                                | imp_core::config::SidebarStyle::Inspector
                        )
                    {
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
                let streaming_lines_limit = self.config.ui.streaming_lines;
                // Feed streaming output into the tool call's rolling buffer
                if let Some(tc) = self.find_tool_call_mut(&tool_call_id) {
                    // Append text to the full live transcript.
                    if !tc.streaming_output.is_empty() {
                        tc.streaming_output.push('\n');
                    }
                    tc.streaming_output.push_str(&text);
                    // Append text and keep configured rolling tail for chat.
                    for line in text.lines() {
                        tc.streaming_lines.push(line.to_string());
                    }
                    if tc.streaming_lines.len() > streaming_lines_limit {
                        let excess = tc.streaming_lines.len() - streaming_lines_limit;
                        tc.streaming_lines.drain(..excess);
                    }
                }
                self.invalidate_chat_render_cache();
            }
            AgentEvent::ToolExecutionEnd {
                tool_call_id,
                result,
            } => {
                let is_error = result.is_error;
                self.turn_tracker.record_tool_end(&tool_call_id, is_error);
                self.begin_llm_thought_segment();
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
                let inline_output_enabled = self.config.ui.effective_chat_tool_display()
                    == imp_core::config::ChatToolDisplay::Interleaved;
                // Attach result to the matching display tool call
                if let Some(tc) = self.find_tool_call_mut(&tool_call_id) {
                    tc.output = Some(output_text.clone());
                    if tc.streaming_output.is_empty() {
                        tc.streaming_output = output_text.clone();
                    }
                    tc.details = result.details.clone();
                    tc.is_error = is_error;
                    // Auto-expand failed tool calls so the error is immediately visible
                    // when inline tool output is enabled. In the default inspector flow,
                    // the selected sidebar owns full error details instead.
                    if is_error {
                        tc.expanded = inline_output_enabled;
                    }
                }

                self.invalidate_chat_render_cache();

                // Persist tool result to session so resume has full conversation
                let _ = self.session.append_tool_result_message(result);
            }
            AgentEvent::Warning { message } => {
                self.push_warning_msg(&message);
            }
            AgentEvent::RecoveryCheckpoint { .. } => {}
            AgentEvent::Timing { timing } => {
                self.status_items.insert("timing".to_string(), {
                    let label = timing
                        .label
                        .as_deref()
                        .map(|label| format!(" {label}"))
                        .unwrap_or_default();
                    let duration = timing
                        .duration_ms
                        .map(|ms| format!(" duration={ms}ms"))
                        .unwrap_or_default();
                    let elapsed = timing
                        .since_llm_request_start_ms
                        .map(|ms| format!(" llm={ms}ms"))
                        .unwrap_or_else(|| format!(" turn={}ms", timing.since_turn_start_ms));
                    format!("{}{}{}{}", timing.stage.as_str(), label, elapsed, duration)
                });
            }
            AgentEvent::TurnEnd {
                index,
                message,
                mana_review,
            } => {
                self.maybe_update_active_mana_scope_from_review(&mana_review);
                self.completed_turns_in_run += 1;
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
                self.completed_turns_in_run = 0;
                // Stop streaming — errors can be terminal (no AgentEnd follows)
                self.is_streaming = false;
                self.streaming_anchor_user_index = None;
                if let Some(last) = self.latest_streaming_message_mut() {
                    last.is_streaming = false;
                }
                self.invalidate_chat_render_cache();

                // Parse the error for a cleaner display
                let display_error = format_error_for_display(&error);

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

fn command_arg(rest: &str) -> Option<&str> {
    if rest.is_empty() {
        Some("")
    } else {
        rest.strip_prefix(char::is_whitespace).map(str::trim)
    }
}

fn expand_prompt_path(path: &str, cwd: &Path) -> PathBuf {
    let expanded = if path == "~" {
        std::env::var_os("HOME").map(PathBuf::from)
    } else if let Some(rest) = path.strip_prefix("~/") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(rest))
    } else {
        None
    };

    let path = expanded.unwrap_or_else(|| PathBuf::from(path));
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn single_line_preview(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod session_lifecycle {
    use super::*;
    use imp_core::config::Config;
    use imp_core::session::{SessionEntry, SessionManager};
    use imp_llm::auth::{AuthStore, OAuthCredential, StoredCredential};
    use imp_llm::model::ModelRegistry;
    use imp_llm::ThinkingLevel;
    use imp_llm::{AssistantMessage, ContentBlock, StopReason};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;
    use tempfile::TempDir;

    /// Helper: build an App with defaults and an in-memory session.
    fn make_app() -> App {
        let config = Config::default();
        let session = SessionManager::in_memory();
        let registry = ModelRegistry::with_builtins();
        App::new(config, session, registry, PathBuf::from("/tmp/test"))
    }

    /// Helper: build an App with defaults and a provided session.
    fn make_app_with_session(session: SessionManager, cwd: PathBuf) -> App {
        let config = Config::default();
        let registry = ModelRegistry::with_builtins();
        App::new(config, session, registry, cwd)
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

    fn render_status_to_string(info: &StatusInfo, width: u16) -> String {
        let theme = Theme::default();
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        crate::views::status::StatusBar::new(info, &theme).render(area, &mut buf);

        (0..area.width)
            .map(|x| {
                buf.cell((x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect()
    }

    #[test]
    fn filtered_model_options_includes_chatgpt_oauth_only_models() {
        let registry = ModelRegistry::with_builtins();
        let tmp = tempfile::tempdir().unwrap();
        let auth_path = tmp.path().join("auth.json");
        let mut auth_store = AuthStore::new(auth_path);
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

        let models = filtered_model_options(&registry, &Config::default(), &auth_store);
        let model = models
            .iter()
            .find(|model| model.id == "gpt-5.5")
            .expect("gpt-5.5 should be visible for ChatGPT OAuth users");
        assert_eq!(model.provider, "openai");

        let openai_model_index = models
            .iter()
            .position(|model| model.id == "gpt-5.3-codex-spark")
            .expect("built-in OpenAI model should be visible");
        let oauth_model_index = models
            .iter()
            .position(|model| model.id == "gpt-5.5")
            .expect("ChatGPT OAuth-only model should be visible");
        assert!(openai_model_index < oauth_model_index);
    }

    #[test]
    fn filtered_model_options_hides_chatgpt_oauth_only_models_when_openai_api_key_exists() {
        let registry = ModelRegistry::with_builtins();
        let tmp = tempfile::tempdir().unwrap();
        let auth_path = tmp.path().join("auth.json");
        let mut auth_store = AuthStore::new(auth_path);
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

        let models = filtered_model_options(&registry, &Config::default(), &auth_store);
        assert!(!models.iter().any(|model| model.id == "gpt-5.5"));
    }

    #[test]
    fn model_picker_includes_current_alias_even_without_auth() {
        let registry = ModelRegistry::with_builtins();
        let tmp = tempfile::tempdir().unwrap();
        let auth_store = AuthStore::new(tmp.path().join("auth.json"));
        let models = filtered_model_options(&registry, &Config::default(), &auth_store);
        assert!(models.is_empty());

        let (models, current_model) = include_current_model_option(models, &registry, "kimi");

        assert_eq!(current_model, "kimi-k2.6");
        assert!(models.iter().any(|model| model.id == "kimi-k2.6"));
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
        assert_eq!(app.terminal_title(), "imp — adjust top bar");
    }

    #[test]
    fn terminal_title_uses_breather_while_streaming() {
        let mut app = make_app();
        app.session.set_name("my chat");
        app.is_streaming = true;
        app.tick = 0;
        assert_eq!(app.terminal_title(), "· — my chat");
        app.tick = 36;
        assert_eq!(app.terminal_title(), "● — my chat");
    }

    #[test]
    fn terminal_title_uses_static_working_glyph_when_animations_are_off() {
        let mut app = make_app();
        app.config.ui.animations = imp_core::config::AnimationLevel::None;
        app.session.set_name("my chat");
        app.is_streaming = true;
        app.tick = 36;
        assert_eq!(app.terminal_title(), "• — my chat");
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
    fn ask_tab_replacement_moves_editor_and_ask_cursors_to_end() {
        use crate::views::ask_bar::{AskOption, AskState};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use tokio::sync::oneshot;

        let mut app = make_app();
        let (tx, _rx) = oneshot::channel();
        app.begin_ask(
            AskState::with_placeholder(
                "Choose".to_string(),
                String::new(),
                vec![AskOption {
                    label: "éclair".to_string(),
                    description: None,
                    checked: false,
                }],
                false,
                String::new(),
            ),
            AskReply::Select(tx),
        );
        app.editor.cursor = usize::MAX;
        if let Some(state) = app.ask_state.as_mut() {
            state.cursor = usize::MAX;
            state.input_active = false;
        }

        app.handle_ask_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));

        assert_eq!(app.editor.content(), "éclair");
        assert_eq!(app.editor.cursor, "éclair".len());
        assert!(app.editor.content().is_char_boundary(app.editor.cursor));
        let state = app.ask_state.as_ref().expect("ask still active");
        assert_eq!(state.input, "éclair");
        assert_eq!(state.input_cursor, "éclair".len());
        assert_eq!(state.editor_cursor, "éclair".len());
        assert!(state.input_active);
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

        // Display should have user msg + streaming placeholder; agent startup is deferred until
        // after the next redraw so the user's message can echo immediately.
        assert!(app.messages.len() >= 2);
        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(app.messages[0].content, "hello world");
        assert_eq!(app.messages[1].role, MessageRole::Assistant);
        assert!(app.messages[1].is_streaming);
    }

    #[test]
    fn send_message_defers_agent_start_until_after_echo_redraw() {
        let tmp = TempDir::new().unwrap();
        let mut app = make_persistent_app(&tmp);

        app.editor.set_content("echo first");
        app.send_message();

        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(app.messages[0].content, "echo first");
        assert_eq!(app.messages[1].role, MessageRole::Assistant);
        assert!(app.messages[1].is_streaming);
        assert!(app.agent_task.is_none());
        assert!(app.agent_handle.is_none());
        assert_eq!(app.pending_agent_prompt.as_deref(), Some("echo first"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pending_agent_start_reports_error_after_deferred_start() {
        let tmp = TempDir::new().unwrap();
        let mut app = make_persistent_app(&tmp);

        app.editor.set_content("start later");
        app.send_message();
        app.start_pending_agent_after_redraw();

        assert!(app.pending_agent_prompt.is_none());
        assert!(app
            .messages
            .iter()
            .any(|message| message.role == MessageRole::Error));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tui_integration_send_message_large_paste_displays_full_text() {
        let tmp = TempDir::new().unwrap();
        let mut app = make_persistent_app(&tmp);
        let pasted = (1..=25)
            .map(|i| format!("fn example_{i}() {{}}"))
            .collect::<Vec<_>>()
            .join("\n");

        app.editor.set_content(&pasted);
        app.send_message();

        assert!(app.messages.len() >= 2);
        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(app.messages[0].content, pasted);

        let persisted = app.session.get_messages();
        assert_eq!(persisted.len(), 1);
        let stored_text = match &persisted[0] {
            imp_llm::Message::User(user) => match user.content.as_slice() {
                [imp_llm::ContentBlock::Text { text }] => text.clone(),
                other => panic!("unexpected user content: {other:?}"),
            },
            other => panic!("expected user message, got {other:?}"),
        };
        assert_eq!(stored_text, pasted);
    }

    #[test]
    fn prompt_commands_change_cwd_and_run_shell_without_session_message() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let child = cwd.join("child");
        std::fs::create_dir_all(&child).unwrap();
        let mut app = make_app_with_session(SessionManager::in_memory(), cwd.clone());

        app.editor.set_content(":cd child");
        app.send_message();
        assert_eq!(app.cwd, child.canonicalize().unwrap());
        assert!(app.session.get_messages().is_empty());

        app.editor.set_content("!! pwd");
        app.send_message();
        assert!(app.session.get_messages().is_empty());
        assert!(app
            .messages
            .last()
            .map(|message| message.content.contains(child.to_string_lossy().as_ref()))
            .unwrap_or(false));
    }

    #[test]
    fn prompt_path_expansion_handles_relative_absolute_and_home_paths() {
        let cwd = PathBuf::from("/tmp/project");
        assert_eq!(expand_prompt_path("child", &cwd), cwd.join("child"));
        assert_eq!(
            expand_prompt_path("/var/tmp", &cwd),
            PathBuf::from("/var/tmp")
        );
        assert!(command_arg(" foo").is_some_and(|arg| arg == "foo"));
        assert!(command_arg("foo").is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn skill_command_injects_skill_prompt() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let skill_dir = cwd.join(".imp").join("skills").join("explain-code");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: explain-code\ndescription: Explain code clearly\n---\n\nExplain $ARGUMENTS with an analogy.",
        )
        .unwrap();
        let session_dir = tmp.path().join("sessions");
        let session = SessionManager::new(&cwd, &session_dir).unwrap();
        let mut app = make_app_with_session(session, cwd);

        assert!(app.try_skill_command("skill:explain-code src/main.rs"));

        assert!(app.messages.len() >= 2);
        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(
            app.messages[0].content,
            "Use the `explain-code` skill.\n\nExplain src/main.rs with an analogy."
        );
    }

    #[test]
    fn command_palette_includes_skill_commands() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let skill_dir = cwd.join(".imp").join("skills").join("explain-code");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: explain-code\ndescription: Explain code clearly\n---\n\nExplain code.",
        )
        .unwrap();
        let app = make_app_with_session(SessionManager::in_memory(), cwd);

        let commands = app.slash_commands();

        assert!(commands
            .iter()
            .any(|cmd| cmd.name == "explain-code" && cmd.description.contains("Skill:")));
    }

    #[test]
    fn render_skill_invocation_strips_frontmatter_and_appends_arguments() {
        let rendered = imp_core::resources::render_skill_invocation(
            "review",
            "---\nname: review\ndescription: Review things\n---\n\nReview carefully.",
            "src/lib.rs",
        );

        assert_eq!(
            rendered,
            "Use the `review` skill.\n\nReview carefully.\n\nARGUMENTS: src/lib.rs"
        );
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

    #[test]
    fn tui_integration_slash_mana_opens_navigator() {
        let mut app = make_app();
        app.execute_command("mana");
        assert!(matches!(app.mode, UiMode::ManaNavigator(_)));
    }

    #[test]
    fn command_palette_includes_mana_command() {
        let commands = builtin_commands();
        assert!(commands.iter().any(|cmd| cmd.name == "mana"));
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
        app.accumulated_usage = Usage {
            input_tokens: 12_345,
            output_tokens: 678,
            cache_read_tokens: 90,
            cache_write_tokens: 0,
        };
        app.accumulated_cost = Cost {
            input: 0.5,
            output: 0.25,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.75,
        };
        app.current_context_tokens = 12_435;
        assert_eq!(app.messages.len(), 1);

        // Execute /new
        app.execute_command("new");

        assert!(app.messages.is_empty());
        assert_eq!(app.accumulated_usage, Usage::default());
        assert_eq!(app.accumulated_cost, Cost::default());
        assert_eq!(app.current_context_tokens, 0);
        // Session replaced with in-memory
        assert!(app.session.path().is_none());
    }

    #[test]
    fn tui_integration_slash_new_resets_rendered_context_percent() {
        let mut app = make_app();
        app.context_window = 200_000;
        app.accumulated_usage = Usage {
            input_tokens: 12_345,
            output_tokens: 678,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
        };
        app.current_context_tokens = 50_000;

        let before = app.build_status_info();
        let before_render = render_status_to_string(&before, 120);
        assert!(before.context_percent > 0.0);
        assert!(before_render.contains("25%"));

        app.execute_command("new");

        let after = app.build_status_info();
        let after_render = render_status_to_string(&after, 120);
        assert_eq!(after.context_percent, 0.0);
        assert!(after_render.contains("0%"));
    }

    #[test]
    fn tui_integration_slash_compact_noops_with_short_history() {
        let mut app = make_app();

        app.execute_command("compact");

        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, MessageRole::System);
        assert_eq!(
            app.messages[0].content,
            "Not enough history to compact yet."
        );
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

    #[test]
    fn command_palette_includes_checkpoint_commands() {
        let commands = builtin_commands();
        assert!(commands.iter().any(|cmd| cmd.name == "checkpoints"));
        assert!(commands.iter().any(|cmd| cmd.name == "restore-checkpoint"));
    }

    #[test]
    fn command_palette_merges_lua_extension_commands() {
        let mut app = make_app();
        let runtime = LuaRuntime::new().unwrap();
        imp_lua::setup_host_api(&runtime).unwrap();
        runtime
            .exec(
                r#"
                imp.register_command("greet", {
                    description = "Say hello from Lua",
                    handler = function(args) return "Hello " .. args end
                })
                "#,
            )
            .unwrap();
        app.lua_runtime = Some(Arc::new(Mutex::new(runtime)));

        let commands = app.slash_commands();

        assert!(commands.iter().any(|cmd| cmd.name == "new"));
        assert!(commands
            .iter()
            .any(|cmd| cmd.name == "greet" && cmd.description == "Say hello from Lua"));
    }

    #[test]
    fn lua_extension_command_can_be_selected_from_palette() {
        let mut app = make_app();
        let runtime = LuaRuntime::new().unwrap();
        imp_lua::setup_host_api(&runtime).unwrap();
        runtime
            .exec(
                r#"
                imp.register_command("greet", {
                    description = "Say hello from Lua",
                    handler = function(args) return "Hello " .. args end
                })
                "#,
            )
            .unwrap();
        app.lua_runtime = Some(Arc::new(Mutex::new(runtime)));

        app.execute_command("greet world");

        let last = app.messages.last().expect("Lua command output");
        assert_eq!(last.role, MessageRole::System);
        assert_eq!(last.content, "Hello world");
    }

    #[test]
    fn execute_checkpoints_command_lists_recorded_checkpoints() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        std::fs::create_dir_all(&cwd).unwrap();
        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
        session
            .append_checkpoint_record(imp_core::session::SessionCheckpointRecord {
                version: imp_core::session::CHECKPOINT_RECORD_VERSION,
                checkpoint_id: "cp-1".into(),
                created_at: 123,
                label: Some("before edits".into()),
                files: vec!["src/main.rs".into()],
            })
            .unwrap();

        let mut app = make_app_with_session(session, cwd.clone());
        app.execute_command("checkpoints");
        let last = app.messages.last().expect("system message");
        assert!(last.content.contains("cp-1"));
        assert!(last.content.contains("before edits"));
    }

    #[test]
    fn execute_restore_checkpoint_command_reports_recorded_files() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        std::fs::create_dir_all(&cwd).unwrap();
        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
        session
            .append_checkpoint_record(imp_core::session::SessionCheckpointRecord {
                version: imp_core::session::CHECKPOINT_RECORD_VERSION,
                checkpoint_id: "cp-restore".into(),
                created_at: 123,
                label: Some("restore me".into()),
                files: vec!["src/main.rs".into(), "src/lib.rs".into()],
            })
            .unwrap();

        let mut app = make_app_with_session(session, cwd.clone());
        app.execute_command("restore-checkpoint restore me");
        let last = app.messages.last().expect("system message");
        assert!(last.content.contains("cp-restore"));
        assert!(last.content.contains("src/main.rs"));
        assert!(last.content.contains("not wired yet"));
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
            cancel_token: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            cancel_token: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            cancel_token: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

    #[tokio::test(flavor = "current_thread")]
    async fn esc_cancel_first_requests_cancel_second_aborts_stuck_agent_task() {
        let mut app = make_app();
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(4);
        let cancel_token = Arc::new(std::sync::atomic::AtomicBool::new(false));

        app.agent_handle = Some(AgentHandle {
            event_rx,
            command_tx,
            cancel_token: Arc::clone(&cancel_token),
        });
        app.agent_task = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
            Ok(())
        }));
        app.is_streaming = true;
        app.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: true,
            timestamp: imp_llm::now(),
        });

        app.handle_cancel();

        assert!(cancel_token.load(std::sync::atomic::Ordering::Relaxed));
        assert!(matches!(command_rx.try_recv(), Ok(AgentCommand::Cancel)));
        assert!(
            app.agent_task.is_some(),
            "first Esc should allow graceful cancellation"
        );
        assert!(!app.is_streaming);
        assert!(!app.messages.last().unwrap().is_streaming);

        app.handle_cancel();

        assert!(
            app.agent_task.is_none(),
            "second Esc should abort a stuck task"
        );
        assert!(app.agent_handle.is_none());
    }

    #[test]
    fn warning_notify_uses_system_role_not_error_role() {
        let mut app = make_app();
        app.handle_ui_request(crate::tui_interface::UiRequest::Notify {
            message: "Heads up".into(),
            level: imp_core::ui::NotifyLevel::Warning,
        });

        let last = app.messages.last().expect("warning message");
        assert_eq!(last.role, MessageRole::Warning);
        assert_eq!(last.content, "Heads up");
    }

    #[test]
    fn tool_updates_target_streaming_assistant_not_latest_message() {
        let mut app = make_app();
        app.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            tool_calls: vec![DisplayToolCall {
                id: "tool-1".into(),
                name: "ask".into(),
                args_summary: "question=Pick one".into(),
                output: None,
                details: serde_json::Value::Null,
                is_error: false,
                expanded: false,
                streaming_lines: Vec::new(),
                streaming_output: String::new(),
            }],
            assistant_blocks: Vec::new(),
            is_streaming: true,
            timestamp: imp_llm::now(),
        });
        app.messages.push(DisplayMessage {
            role: MessageRole::System,
            content: "transient note".into(),
            thinking: None,
            tool_calls: Vec::new(),
            assistant_blocks: Vec::new(),
            is_streaming: false,
            timestamp: imp_llm::now(),
        });

        app.handle_agent_event(AgentEvent::ToolExecutionStart {
            tool_call_id: "tool-1".into(),
            tool_name: "ask".into(),
            args: serde_json::json!({"question": "Pick one"}),
        });
        app.handle_agent_event(AgentEvent::ToolOutputDelta {
            tool_call_id: "tool-1".into(),
            text: "selected option".into(),
        });
        app.handle_agent_event(AgentEvent::ToolExecutionEnd {
            tool_call_id: "tool-1".into(),
            result: imp_llm::ToolResultMessage {
                tool_call_id: "tool-1".into(),
                tool_name: "ask".into(),
                content: vec![ContentBlock::Text {
                    text: "selected option".into(),
                }],
                is_error: false,
                details: serde_json::json!({}),
                timestamp: imp_llm::now(),
            },
        });

        let assistant = app
            .messages
            .iter()
            .find(|msg| msg.role == MessageRole::Assistant)
            .expect("assistant message");
        assert_eq!(assistant.tool_calls.len(), 1);
        assert_eq!(
            assistant.tool_calls[0].output.as_deref(),
            Some("selected option")
        );
        assert!(!assistant.tool_calls[0].is_error);

        let system = app.messages.last().expect("system message remains");
        assert_eq!(system.role, MessageRole::System);
        assert_eq!(system.content, "transient note");
    }
    #[test]
    fn tui_integration_slash_personality_opens_overlay() {
        let mut app = make_app();
        app.execute_command("personality");
        assert!(matches!(app.mode, UiMode::Personality(_)));
    }

    #[test]
    fn tui_personality_prefers_ancestor_project_soul_when_opening() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("project");
        let nested = project.join("src").join("deep");
        let session_dir = tmp.path().join("sessions");
        std::fs::create_dir_all(project.join(".imp")).unwrap();
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            project.join(".imp").join("soul.md"),
            "# Soul\n\nproject soul\n",
        )
        .unwrap();

        let session = SessionManager::new(&nested, &session_dir).unwrap();
        let mut app = make_app_with_session(session, nested.clone());
        app.execute_command("personality");

        match &app.mode {
            UiMode::Personality(state) => {
                assert_eq!(state.current_path(), &project.join(".imp").join("soul.md"));
                assert!(matches!(state.scope, PersonalityScope::Project));
            }
            _ => panic!("expected personality mode"),
        }
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
        // Point global config dir to temp so we don't touch real memory.
        // Config::user_config_dir uses HOME/.imp, not XDG_CONFIG_HOME.
        let previous_home = std::env::var_os("HOME");
        let previous_userprofile = std::env::var_os("USERPROFILE");
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("USERPROFILE");

        let mut app = make_app();

        app.execute_command("memory add Test entry from slash command");
        assert!(app.messages.last().unwrap().content.contains("Added"));

        // Show should list the entry
        app.execute_command("memory");
        let content = &app.messages.last().unwrap().content;
        assert!(content.contains("Test entry from slash command"));

        // Clean up env vars
        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(previous_userprofile) = previous_userprofile {
            std::env::set_var("USERPROFILE", previous_userprofile);
        } else {
            std::env::remove_var("USERPROFILE");
        }
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
        let tmp = TempDir::new().unwrap();
        let state = crate::views::personality::PersonalityState::new(
            tmp.path().to_path_buf(),
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

    #[tokio::test(flavor = "multi_thread")]
    async fn tui_integration_multiline_slash_paste_is_sent_as_prompt() {
        let mut app = make_app();
        let pasted = "/Users/asher/example.rs\nfn main() {}";

        app.editor.set_content(pasted);
        app.send_message();

        assert_eq!(app.messages[0].role, MessageRole::User);
        assert_eq!(app.messages[0].content, pasted);
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
        app.config.enabled_models = Some(
            app.model_registry
                .list()
                .iter()
                .take(3)
                .map(|m| m.id.clone())
                .collect(),
        );

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
        app.config.enabled_models = Some(
            app.model_registry
                .list()
                .iter()
                .take(2)
                .map(|m| m.id.clone())
                .collect(),
        );
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
    fn mouse_click_on_homepage_skill_opens_skill_in_inspector() {
        let tmp = TempDir::new().unwrap();
        let previous_home = std::env::var_os("HOME");
        let previous_userprofile = std::env::var_os("USERPROFILE");
        std::env::set_var("HOME", tmp.path());
        std::env::remove_var("USERPROFILE");
        let cwd = tmp.path().join("project");
        std::fs::create_dir_all(cwd.join(".imp/skills/rust")).unwrap();
        std::fs::write(
            cwd.join(".imp/skills/rust/SKILL.md"),
            "---\ndescription: Rust conventions\n---\n\n# Rust\n\nUse result types.",
        )
        .unwrap();
        let mut app = make_app_with_session(SessionManager::in_memory(), cwd);
        app.config.ui.sidebar_style = imp_core::config::SidebarStyle::Inspector;
        app.chat_surface = Some(TextSurface::new(
            SelectablePane::Chat,
            Rect::new(0, 0, 160, 30),
            Vec::new(),
            0,
        ));

        let rust_index = app
            .startup_skills()
            .iter()
            .position(|skill| skill.name == "rust")
            .expect("rust skill discovered");
        let hit = app
            .startup_skill_hits(Rect::new(0, 0, 160, 30))
            .into_iter()
            .find(|hit| hit.index == rust_index)
            .expect("rust skill visible");
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: hit.rect.x,
            row: hit.rect.y,
            modifiers: KeyModifiers::empty(),
        });

        assert!(app.sidebar.open);
        let detail = startup_skill_detail_render_data(
            app.selected_startup_skill.as_ref().expect("skill selected"),
            &app.theme,
        );
        assert!(detail.plain_lines.iter().any(|line| line == "# Rust"));
        assert!(detail
            .plain_lines
            .iter()
            .any(|line| line == "Use result types."));

        if let Some(previous_home) = previous_home {
            std::env::set_var("HOME", previous_home);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(previous_userprofile) = previous_userprofile {
            std::env::set_var("USERPROFILE", previous_userprofile);
        } else {
            std::env::remove_var("USERPROFILE");
        }
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
    fn selected_read_file_path_resolves_relative_path() {
        let cwd = PathBuf::from("/tmp/project");
        let tc = crate::views::tools::DisplayToolCall {
            id: "tc-read".into(),
            name: "read".into(),
            args_summary: "src/lib.rs".into(),
            output: Some("content".into()),
            details: serde_json::json!({ "path": "src/lib.rs" }),
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
            streaming_output: String::new(),
        };

        let path = selected_read_file_path_from_tool(Some(&tc), &cwd).unwrap();

        assert_eq!(path, cwd.join("src/lib.rs"));
    }

    #[test]
    fn selected_read_file_path_ignores_non_read_tools() {
        let tc = crate::views::tools::DisplayToolCall {
            id: "tc-shell".into(),
            name: "shell".into(),
            args_summary: "cat src/lib.rs".into(),
            output: None,
            details: serde_json::json!({ "path": "src/lib.rs" }),
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
            streaming_output: String::new(),
        };

        assert!(selected_read_file_path_from_tool(Some(&tc), Path::new("/tmp/project")).is_none());
    }

    #[test]
    fn ctrl_o_without_read_selection_reports_no_file() {
        let mut app = make_app();

        app.handle_normal_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL))
            .unwrap();

        assert!(app
            .messages
            .last()
            .unwrap()
            .content
            .contains("No read file selected"));
    }

    #[test]
    fn inspector_defaults_to_latest_tool_when_no_focus() {
        let mut app = make_app();
        app.config.ui.sidebar_style = imp_core::config::SidebarStyle::Inspector;
        app.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            tool_calls: vec![crate::views::tools::DisplayToolCall {
                id: "tc-latest".into(),
                name: "bash".into(),
                args_summary: "$ pwd".into(),
                output: Some("/tmp/test".into()),
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

        let selected = app.selected_tool_call().expect("latest tool selected");

        assert_eq!(selected.id, "tc-latest");
    }

    #[test]
    fn focusing_tool_resets_inspector_scroll() {
        let mut app = make_app();
        app.config.ui.sidebar_style = imp_core::config::SidebarStyle::Inspector;
        app.sidebar.detail_scroll = 12;

        app.focus_tool(0);

        assert_eq!(app.tool_focus, Some(0));
        assert_eq!(app.active_pane, Pane::SidebarDetail);
        assert_eq!(app.sidebar.detail_scroll, 0);
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
            mana_review: imp_core::mana_review::TurnManaReview::no_change(0),
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
    fn improve_mode_prompt_sets_research_guardrails() {
        let scope = ManaUnitRef::new("364", "Improve imp", Some("epic".into()));

        let prompt = improve_safe_mode_prompt(&scope, 2, 5);

        assert!(prompt.contains("Improve mode autoresearch turn 2/5"));
        assert!(prompt.contains("active mana scope 364"));
        assert!(prompt.contains("Prefer read-only investigation"));
        assert!(prompt.contains("create or update mana units"));
        assert!(prompt.contains("Do not make broad code changes"));
    }

    #[test]
    fn improve_mode_queues_bounded_autoresearch_turns() {
        let mut app = make_app();
        app.config.ui.improve_auto_turn_budget = 1;
        app.workflow_mode = WorkflowMode::Improve;
        app.improve_safe_mode = true;
        app.active_mana_scope = Some(ManaUnitRef::new("364", "Improve imp", Some("epic".into())));

        app.queue_improve_mode_continuation_if_ready();

        assert_eq!(app.improve_auto_turns, 1);
        let prompt = app.pending_agent_prompt.as_deref().unwrap();
        assert!(prompt.contains("Improve mode autoresearch turn 1/1"));

        app.pending_agent_prompt = None;
        app.pending_agent_cwd = None;
        app.queue_improve_mode_continuation_if_ready();

        assert_eq!(app.improve_auto_turns, 1);
        assert!(app.pending_agent_prompt.is_none());
        assert!(app
            .messages
            .iter()
            .any(|message| message.content.contains("Improve mode paused after 1")));
    }

    #[test]
    fn improve_mode_queues_sandbox_cwd_for_code_turns() {
        let mut app = make_app();
        app.config.ui.improve_auto_turn_budget = 1;
        app.workflow_mode = WorkflowMode::Improve;
        app.active_mana_scope = Some(ManaUnitRef::new("364", "Improve imp", Some("epic".into())));
        app.improve_sandbox = Some(ImproveSandbox {
            branch: "imp/improve/364-improve-imp".into(),
            base_branch: "nightly".into(),
            worktree: PathBuf::from("/tmp/imp-improve-364"),
        });

        app.queue_improve_mode_continuation_if_ready();

        assert_eq!(
            app.pending_agent_cwd.as_deref(),
            Some(Path::new("/tmp/imp-improve-364"))
        );
        assert!(app
            .pending_agent_prompt
            .as_deref()
            .unwrap()
            .contains("Improve mode code-changing turn 1/1"));
    }

    #[test]
    fn improve_safe_mode_keeps_original_cwd_for_agent_turns() {
        let mut app = make_app();
        app.config.ui.improve_auto_turn_budget = 1;
        app.workflow_mode = WorkflowMode::Improve;
        app.improve_safe_mode = true;
        app.active_mana_scope = Some(ManaUnitRef::new("364", "Improve imp", Some("epic".into())));

        app.queue_improve_mode_continuation_if_ready();

        assert!(app.pending_agent_cwd.is_none());
        assert!(app
            .pending_agent_prompt
            .as_deref()
            .unwrap()
            .contains("Improve mode autoresearch turn 1/1"));
    }

    #[test]
    fn loop_command_queues_prompt_and_shows_label() {
        let mut app = make_app();
        app.config.ui.loop_turn_budget = 3;

        app.start_loop_command("keep going");

        assert_eq!(app.pending_agent_prompt.as_deref(), Some("keep going"));
        assert_eq!(app.loop_label().as_deref(), Some("↻ loop 1/3"));
    }

    #[test]
    fn status_text_includes_active_loop() {
        let mut app = make_app();
        app.loop_state = Some(LoopState {
            message: "keep going".into(),
            completed_turns: 2,
            budget: 3,
        });

        let status = app.active_status_text();

        assert!(status.contains("loop: 2/3"));
        assert!(status.contains("loop message: keep going"));
    }

    #[test]
    fn improve_status_label_shows_sandbox_without_safe_mode() {
        let mut app = make_app();
        app.workflow_mode = WorkflowMode::Improve;
        app.config.ui.improve_auto_turn_budget = 5;
        app.improve_auto_turns = 2;
        app.improve_sandbox = Some(ImproveSandbox {
            branch: "imp/improve/364-improve-imp".into(),
            base_branch: "nightly".into(),
            worktree: PathBuf::from("/tmp/imp-improve-364"),
        });

        let label = app.improve_status_label().unwrap();

        assert!(label.contains("imp is improving imp-improve-364"));
        assert!(label.contains("turn 2/5"));
        assert!(label.contains("/improve-help"));

        app.improve_safe_mode = true;
        assert!(app.improve_status_label().is_none());
    }

    #[test]
    fn completion_bell_requires_completed_turn_and_resets_latch() {
        let mut app = make_app();
        app.config.ui.notify_on_agent_complete = true;

        app.maybe_notify_agent_completion();
        assert_eq!(app.completed_turns_in_run, 0);

        app.completed_turns_in_run = 2;
        app.maybe_notify_agent_completion();
        assert_eq!(app.completed_turns_in_run, 0);
    }

    #[test]
    fn completion_bell_toggle_still_resets_latch() {
        let mut app = make_app();
        app.config.ui.notify_on_agent_complete = false;
        app.completed_turns_in_run = 1;

        app.maybe_notify_agent_completion();

        assert_eq!(app.completed_turns_in_run, 0);
    }

    #[test]
    fn completion_bell_cancel_suppresses_notification_once() {
        let mut app = make_app();
        app.config.ui.notify_on_agent_complete = true;
        app.completed_turns_in_run = 1;
        app.suppress_completion_notification = true;

        app.maybe_notify_agent_completion();

        assert_eq!(app.completed_turns_in_run, 0);
        assert!(!app.suppress_completion_notification);
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
