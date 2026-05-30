use std::path::PathBuf;

use ratatui::layout::Rect;
use ratatui::text::Line;

use crate::animation::AnimationState;
use crate::selection::SelectablePane;
use crate::views::sidebar::SidebarDetailRenderData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DragAutoScroll {
    pub(super) pane: SelectablePane,
    pub(super) direction: ScrollDirection,
    pub(super) speed: usize,
    pub(super) column: u16,
    pub(super) row: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ThemeKind {
    pub(super) is_light: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChatRenderCacheKey {
    pub(super) width: u16,
    pub(super) messages_epoch: u64,
    pub(super) chat_tool_focus: Option<usize>,
    pub(super) word_wrap: bool,
    pub(super) chat_tool_display: imp_core::config::ChatToolDisplay,
    pub(super) thinking_lines: usize,
    pub(super) show_timestamps: bool,
    pub(super) animation_level: imp_core::config::AnimationLevel,
    pub(super) activity_state: AnimationState,
    pub(super) theme: ThemeKind,
    pub(super) tick: u64,
}

#[derive(Debug)]
pub(super) struct ChatRenderCache {
    pub(super) key: ChatRenderCacheKey,
    pub(super) render: crate::views::chat::ChatRenderData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SidebarStreamCacheKey {
    pub(super) width: u16,
    pub(super) messages_epoch: u64,
    pub(super) selected: Option<usize>,
    pub(super) word_wrap: bool,
    pub(super) tool_output: imp_core::config::ToolOutputDisplay,
    pub(super) tool_output_lines: usize,
    pub(super) animation_level: imp_core::config::AnimationLevel,
    pub(super) theme: ThemeKind,
}

#[derive(Debug)]
pub(super) struct SidebarStreamCache {
    pub(super) key: SidebarStreamCacheKey,
    pub(super) lines: Vec<Line<'static>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SidebarDetailCacheKey {
    pub(super) width: u16,
    pub(super) messages_epoch: u64,
    pub(super) selected_tool_id_hash: u64,
    pub(super) thinking_hash: u64,
    pub(super) run_hash: u64,
    pub(super) word_wrap: bool,
    pub(super) tool_output_lines: usize,
    pub(super) animation_level: imp_core::config::AnimationLevel,
    pub(super) theme: ThemeKind,
}

#[derive(Debug)]
pub(super) struct SidebarDetailCache {
    pub(super) key: SidebarDetailCacheKey,
    pub(super) render: SidebarDetailRenderData,
}

#[derive(Debug, Clone, Default)]
pub(super) struct StartupSurfaceMetadata {
    pub(super) skills: Vec<imp_core::resources::Skill>,
    pub(super) workflows: Vec<StartupWorkflowItem>,
    pub(super) repo_stats: Option<RepoStatsState>,
    pub(super) rule_files: Vec<PathBuf>,
    pub(super) provider_id: String,
    pub(super) web_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartupWorkflowItem {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) status: String,
    pub(super) kind: String,
    pub(super) path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RepoStatsState {
    Scanning,
    Ready(crate::repo_stats::RepoStats),
    HomeDirectory,
    NoRepo,
    Empty,
    Failed,
}

#[derive(Debug, Clone)]
pub(super) struct StartupSurfaceData {
    pub(super) panel: crate::views::startup::StartupPanelData,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct StartupSkillHit {
    pub(super) index: usize,
    pub(super) rect: Rect,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct StartupWorkflowHit {
    pub(super) index: usize,
    pub(super) rect: Rect,
}
