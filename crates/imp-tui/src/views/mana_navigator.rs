use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use imp_llm::truncate_chars_with_suffix;
use mana_core::api::{self, Unit};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct ManaTreeNode {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: u8,
    pub kind: String,
    pub parent: Option<String>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub has_verify: bool,
    pub child_count: usize,
    pub depth: usize,
    pub guides: Vec<bool>,
    pub is_last_child: bool,
}

#[derive(Debug, Clone)]
pub struct ManaNavigatorState {
    pub mana_dir: Option<PathBuf>,
    pub nodes: Vec<ManaTreeNode>,
    pub load_error: Option<String>,
    selected_id: Option<String>,
    expanded: HashSet<String>,
    detail: Option<Unit>,
    detail_error: Option<String>,
    filter: String,
    detail_scroll: usize,
    pub loading: bool,
}

impl ManaNavigatorState {
    pub fn load(cwd: &Path, initial_id: Option<&str>) -> Self {
        match Self::try_load(cwd, initial_id) {
            Ok(state) => state,
            Err((mana_dir, message)) => Self::error(mana_dir, message),
        }
    }

    pub fn loading(cwd: &Path) -> Self {
        Self {
            mana_dir: api::find_mana_dir(cwd).ok(),
            nodes: Vec::new(),
            load_error: None,
            selected_id: None,
            expanded: HashSet::new(),
            detail: None,
            detail_error: None,
            filter: String::new(),
            detail_scroll: 0,
            loading: true,
        }
    }

    pub fn try_load(
        cwd: &Path,
        initial_id: Option<&str>,
    ) -> Result<Self, (Option<PathBuf>, String)> {
        match api::find_mana_dir(cwd) {
            Ok(mana_dir) => match load_nodes(&mana_dir) {
                Ok(nodes) => {
                    let mut state = Self {
                        mana_dir: Some(mana_dir),
                        nodes,
                        load_error: None,
                        selected_id: None,
                        expanded: HashSet::new(),
                        detail: None,
                        detail_error: None,
                        filter: String::new(),
                        detail_scroll: 0,
                        loading: false,
                    };
                    if let Some(id) = initial_id {
                        state.expand_ancestors(id);
                    }
                    state.selected_id = initial_id
                        .and_then(|id| state.nodes.iter().find(|node| node.id == id))
                        .map(|node| node.id.clone())
                        .or_else(|| state.nodes.first().map(|node| node.id.clone()));
                    state.refresh_detail();
                    Ok(state)
                }
                Err(error) => Err((Some(mana_dir), format!("Failed to load mana tree: {error}"))),
            },
            Err(error) => Err((
                None,
                format!("No .mana directory found from {}: {error}", cwd.display()),
            )),
        }
    }

    pub fn error(mana_dir: Option<PathBuf>, message: String) -> Self {
        Self {
            mana_dir,
            nodes: Vec::new(),
            load_error: Some(message),
            selected_id: None,
            expanded: HashSet::new(),
            detail: None,
            detail_error: None,
            filter: String::new(),
            detail_scroll: 0,
            loading: false,
        }
    }

    pub fn move_up(&mut self) {
        let visible = self.visible_node_ids();
        if visible.is_empty() {
            self.selected_id = None;
            self.refresh_detail();
            return;
        }
        let idx = self.selected_visible_index_in(&visible).unwrap_or(0);
        self.selected_id = Some(visible[idx.saturating_sub(1)].clone());
        self.refresh_detail();
    }

    pub fn move_down(&mut self) {
        let visible = self.visible_node_ids();
        if visible.is_empty() {
            self.selected_id = None;
            self.refresh_detail();
            return;
        }
        let idx = self.selected_visible_index_in(&visible).unwrap_or(0);
        self.selected_id = Some(visible[(idx + 1).min(visible.len().saturating_sub(1))].clone());
        self.refresh_detail();
    }

    pub fn collapse_selected(&mut self) {
        if let Some(id) = self.selected_id.clone() {
            if self.expanded.remove(&id) {
                return;
            }
            if let Some(parent) = self.selected_node().and_then(|node| node.parent.clone()) {
                self.selected_id = Some(parent);
                self.refresh_detail();
            }
        }
    }

    pub fn expand_selected(&mut self) {
        if let Some(id) = self.selected_id.clone() {
            self.expanded.insert(id);
        }
    }

    pub fn toggle_selected(&mut self) {
        if let Some(id) = self.selected_id.clone() {
            if self.expanded.contains(&id) {
                self.expanded.remove(&id);
            } else {
                self.expanded.insert(id);
            }
        }
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    pub fn selected_node(&self) -> Option<&ManaTreeNode> {
        let id = self.selected_id()?;
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn visible_nodes(&self) -> Vec<&ManaTreeNode> {
        if self.filter.trim().is_empty() {
            self.nodes
                .iter()
                .filter(|node| self.ancestors_expanded(node))
                .collect()
        } else {
            self.nodes
                .iter()
                .filter(|node| node.matches_filter(&self.filter))
                .collect()
        }
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn push_filter_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        self.filter.push(ch);
        self.select_first_visible();
    }

    pub fn pop_filter_char(&mut self) {
        self.filter.pop();
        self.select_first_visible();
    }

    pub fn clear_filter(&mut self) {
        if !self.filter.is_empty() {
            self.filter.clear();
            self.select_first_visible();
        }
    }

    pub fn scroll_detail_up(&mut self) {
        self.scroll_detail_up_by(1);
    }

    pub fn scroll_detail_down(&mut self) {
        self.scroll_detail_down_by(1);
    }

    pub fn scroll_detail_up_by(&mut self, lines: usize) {
        self.detail_scroll = self.detail_scroll.saturating_sub(lines);
    }

    pub fn scroll_detail_down_by(&mut self, lines: usize) {
        self.detail_scroll = self.detail_scroll.saturating_add(lines);
    }

    pub fn select_visible_index(&mut self, index: usize) {
        let visible = self.visible_node_ids();
        if let Some(id) = visible.get(index) {
            self.selected_id = Some(id.clone());
            self.refresh_detail();
        }
    }

    pub fn select_visible_row(&mut self, row: usize, height: usize) {
        let selected_idx = self.selected_visible_index().unwrap_or(0);
        let scroll = selected_idx.saturating_sub(height.saturating_sub(1));
        self.select_visible_index(scroll + row);
    }

    pub fn move_up_by(&mut self, lines: usize) {
        for _ in 0..lines.max(1) {
            self.move_up();
        }
    }

    pub fn move_down_by(&mut self, lines: usize) {
        for _ in 0..lines.max(1) {
            self.move_down();
        }
    }

    fn select_first_visible(&mut self) {
        let visible = self.visible_node_ids();
        self.selected_id = visible.first().cloned();
        self.refresh_detail();
    }

    pub fn selected_visible_index(&self) -> Option<usize> {
        let visible = self.visible_node_ids();
        self.selected_visible_index_in(&visible)
    }

    fn visible_node_ids(&self) -> Vec<String> {
        self.visible_nodes()
            .into_iter()
            .map(|node| node.id.clone())
            .collect()
    }

    fn selected_visible_index_in(&self, visible: &[String]) -> Option<usize> {
        let selected = self.selected_id.as_deref()?;
        visible.iter().position(|id| id == selected)
    }

    fn ancestors_expanded(&self, node: &ManaTreeNode) -> bool {
        let mut parent = node.parent.as_deref();
        while let Some(parent_id) = parent {
            if !self.expanded.contains(parent_id) {
                return false;
            }
            parent = self
                .nodes
                .iter()
                .find(|candidate| candidate.id == parent_id)
                .and_then(|candidate| candidate.parent.as_deref());
        }
        true
    }

    fn expand_ancestors(&mut self, id: &str) {
        let mut parent = self
            .nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| node.parent.clone());
        while let Some(parent_id) = parent {
            self.expanded.insert(parent_id.clone());
            parent = self
                .nodes
                .iter()
                .find(|node| node.id == parent_id)
                .and_then(|node| node.parent.clone());
        }
    }

    fn refresh_detail(&mut self) {
        self.detail_scroll = 0;
        self.detail = None;
        self.detail_error = None;
        let (Some(mana_dir), Some(id)) = (self.mana_dir.as_deref(), self.selected_id.as_deref())
        else {
            return;
        };
        match api::get_unit(mana_dir, id) {
            Ok(unit) => self.detail = Some(unit),
            Err(error) => self.detail_error = Some(format!("Failed to load unit {id}: {error}")),
        }
    }
}

fn load_nodes(mana_dir: &Path) -> Result<Vec<ManaTreeNode>, Box<dyn std::error::Error>> {
    let entries = api::list_units(
        mana_dir,
        &mana_core::ops::list::ListParams {
            include_closed: true,
            ..Default::default()
        },
    )?;
    let by_parent = entries.iter().fold(
        BTreeMap::<Option<String>, Vec<_>>::new(),
        |mut acc, entry| {
            acc.entry(entry.parent.clone()).or_default().push(entry);
            acc
        },
    );
    let mut out = Vec::new();
    flatten_entries(None, 0, Vec::new(), &by_parent, &mut out);
    Ok(out)
}

fn flatten_entries(
    parent: Option<&str>,
    depth: usize,
    guides: Vec<bool>,
    by_parent: &BTreeMap<Option<String>, Vec<&mana_core::api::IndexEntry>>,
    out: &mut Vec<ManaTreeNode>,
) {
    let key = parent.map(str::to_string);
    let Some(children) = by_parent.get(&key) else {
        return;
    };
    for (index, entry) in children.iter().enumerate() {
        let is_last_child = index + 1 == children.len();
        let child_count = by_parent.get(&Some(entry.id.clone())).map_or(0, Vec::len);
        out.push(ManaTreeNode {
            id: entry.id.clone(),
            title: entry.title.clone(),
            status: format!("{:?}", entry.status),
            priority: entry.priority,
            kind: format!("{:?}", entry.kind),
            parent: entry.parent.clone(),
            labels: entry.labels.clone(),
            assignee: entry.assignee.clone(),
            has_verify: entry.has_verify,
            child_count,
            depth,
            guides: guides.clone(),
            is_last_child,
        });
        let mut child_guides = guides.clone();
        child_guides.push(!is_last_child);
        flatten_entries(Some(&entry.id), depth + 1, child_guides, by_parent, out);
    }
}

impl ManaTreeNode {
    fn matches_filter(&self, query: &str) -> bool {
        let haystack = format!(
            "{} {} {} {} {} {}",
            self.id,
            self.title,
            self.status,
            self.kind,
            self.assignee.as_deref().unwrap_or(""),
            self.labels.join(" ")
        )
        .to_lowercase();
        fuzzy_match(&haystack, &query.to_lowercase())
    }
}

fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut chars = haystack.chars();
    needle
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .all(|needle_ch| chars.any(|hay_ch| hay_ch == needle_ch))
}

pub struct ManaNavigatorView<'a> {
    state: &'a ManaNavigatorState,
    theme: &'a Theme,
}

impl<'a> ManaNavigatorView<'a> {
    pub fn new(state: &'a ManaNavigatorState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for ManaNavigatorView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 20 {
            return;
        }
        Clear.render(area, buf);
        let title = self
            .state
            .mana_dir
            .as_ref()
            .map(|path| format!(" Mana {} ", path.display()))
            .unwrap_or_else(|| " Mana ".to_string());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.loading {
            buf.set_line(
                inner.x,
                inner.y,
                &Line::from(Span::styled("Loading mana…", self.theme.muted_style())),
                inner.width,
            );
            return;
        }
        if let Some(error) = &self.state.load_error {
            buf.set_line(
                inner.x,
                inner.y,
                &Line::from(Span::styled(error.clone(), self.theme.error_style())),
                inner.width,
            );
            return;
        }

        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        render_filter_bar(body[0], self.state, buf, self.theme);
        let columns = if body[1].width >= 90 {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
                .split(body[1])
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100), Constraint::Percentage(0)])
                .split(body[1])
        };
        render_mana_list(columns[0], self.state, buf, self.theme);
        if columns[1].width > 0 {
            render_mana_detail(columns[1], self.state, buf, self.theme);
        }
    }
}

fn render_filter_bar(area: Rect, state: &ManaNavigatorState, buf: &mut Buffer, theme: &Theme) {
    let filter = if state.filter().is_empty() {
        "type to fuzzy find".to_string()
    } else {
        format!("find: {}", state.filter())
    };
    let hint = "  ↑/↓ move • ←/→ fold • PgUp/PgDn detail • Backspace clear • Esc close";
    let available = area.width as usize;
    let filter_width = available.saturating_sub(hint.chars().count());
    let filter = truncate_chars_with_suffix(&filter, filter_width.max(8), "…");
    buf.set_line(
        area.x,
        area.y,
        &Line::from(vec![
            Span::styled(filter, theme.accent_style()),
            Span::styled(hint, theme.muted_style()),
        ]),
        area.width,
    );
}

fn render_mana_list(area: Rect, state: &ManaNavigatorState, buf: &mut Buffer, theme: &Theme) {
    let visible = state.visible_nodes();
    if visible.is_empty() {
        buf.set_line(
            area.x,
            area.y,
            &Line::from(Span::styled("No mana units", theme.muted_style())),
            area.width,
        );
        return;
    }
    let selected_idx = state.selected_visible_index().unwrap_or(0);
    let visible_height = area.height as usize;
    let scroll = selected_idx.saturating_sub(visible_height.saturating_sub(1));
    for (row, node) in visible.iter().skip(scroll).take(visible_height).enumerate() {
        let is_selected = scroll + row == selected_idx;
        let y = area.y + row as u16;
        let branch = if node.depth == 0 {
            ""
        } else if node.is_last_child {
            "└─ "
        } else {
            "├─ "
        };
        let icon = if node.child_count == 0 {
            "  "
        } else if state.expanded.contains(&node.id) {
            "▾ "
        } else {
            "▸ "
        };
        let guides = render_guides(&node.guides);
        let suffix = format!(" [{} P{}]", node.status, node.priority);
        let prefix_width = guides.chars().count()
            + branch.chars().count()
            + icon.chars().count()
            + node.id.chars().count()
            + 1;
        let title_width = area
            .width
            .saturating_sub((prefix_width + suffix.chars().count()) as u16)
            as usize;
        let title = truncate_chars_with_suffix(&node.title, title_width.max(8), "…");
        let style = if is_selected {
            theme.selected_style()
        } else if node.status.eq_ignore_ascii_case("closed") {
            theme.muted_style()
        } else {
            Style::default()
        };
        let line = Line::from(vec![
            Span::styled(guides, theme.muted_style()),
            Span::styled(branch.to_string(), theme.muted_style()),
            Span::styled(icon.to_string(), theme.accent_style()),
            Span::styled(format!("{} ", node.id), theme.accent_style()),
            Span::styled(title, style),
            Span::styled(suffix, theme.muted_style()),
        ]);
        buf.set_line(area.x, y, &line, area.width);
    }
}

fn render_mana_detail(area: Rect, state: &ManaNavigatorState, buf: &mut Buffer, theme: &Theme) {
    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::LEFT)
        .border_style(theme.muted_style());
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines = Vec::new();
    if let Some(error) = &state.detail_error {
        lines.push(error.clone());
    } else if let Some(unit) = &state.detail {
        lines.extend([
            format!("{} — {}", unit.id, unit.title),
            format!(
                "Status: {:?}  Kind: {:?}  P{}",
                unit.status, unit.kind, unit.priority
            ),
            format!("Parent: {}", unit.parent.as_deref().unwrap_or("(root)")),
            format!("Assignee: {}", unit.assignee.as_deref().unwrap_or("-")),
            format!(
                "Labels: {}",
                if unit.labels.is_empty() {
                    "-".to_string()
                } else {
                    unit.labels.join(", ")
                }
            ),
            String::new(),
        ]);
        push_section(&mut lines, "Description", unit.description.as_deref());
        push_section(&mut lines, "Acceptance", unit.acceptance.as_deref());
        push_section(&mut lines, "Verify", unit.verify.as_deref());
        push_section(&mut lines, "Notes", unit.notes.as_deref());
        if !unit.decisions.is_empty() {
            lines.push("Decisions".to_string());
            lines.extend(
                unit.decisions
                    .iter()
                    .map(|decision| format!("- {decision}")),
            );
            lines.push(String::new());
        }
        if !unit.dependencies.is_empty() {
            lines.push(format!("Dependencies: {}", unit.dependencies.join(", ")));
        }
        lines.push(
            "↑/↓ move • ←/→ collapse/expand • type find • PgUp/PgDn scroll detail • Esc close"
                .to_string(),
        );
    }

    let wrapped = wrap_lines(&lines, inner.width as usize);
    let max_scroll = wrapped.len().saturating_sub(inner.height as usize);
    let scroll = state.detail_scroll.min(max_scroll);
    for (i, line) in wrapped
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .enumerate()
    {
        let style = if ["Description", "Acceptance", "Verify", "Notes", "Decisions"]
            .contains(&line.as_str())
        {
            theme.accent_style().add_modifier(Modifier::BOLD)
        } else {
            theme.muted_style()
        };
        buf.set_line(
            inner.x,
            inner.y + i as u16,
            &Line::from(Span::styled(line.clone(), style)),
            inner.width,
        );
    }
}

fn push_section(lines: &mut Vec<String>, title: &str, value: Option<&str>) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    lines.push(title.to_string());
    lines.extend(value.lines().map(str::to_string));
    lines.push(String::new());
}

fn render_guides(guides: &[bool]) -> String {
    guides
        .iter()
        .map(|has_more| if *has_more { "│  " } else { "   " })
        .collect()
}

fn wrap_lines(lines: &[String], width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for line in lines {
        if line.is_empty() {
            out.push(String::new());
        } else {
            let mut rest = line.as_str();
            while !rest.is_empty() {
                let chunk = truncate_chars_with_suffix(rest, width, "");
                let used = chunk.len();
                out.push(chunk);
                rest = rest.get(used..).unwrap_or("").trim_start();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, parent: Option<&str>, child_count: usize, depth: usize) -> ManaTreeNode {
        ManaTreeNode {
            id: id.into(),
            title: format!("unit {id}"),
            status: "Open".into(),
            priority: 2,
            kind: "Task".into(),
            parent: parent.map(str::to_string),
            labels: Vec::new(),
            assignee: None,
            has_verify: false,
            child_count,
            depth,
            guides: Vec::new(),
            is_last_child: true,
        }
    }

    #[test]
    fn navigation_clamps_at_bounds() {
        let mut state = ManaNavigatorState {
            mana_dir: None,
            nodes: vec![node("1", None, 0, 0), node("2", None, 0, 0)],
            load_error: None,
            selected_id: Some("1".into()),
            expanded: HashSet::new(),
            detail: None,
            detail_error: None,
            filter: String::new(),
            detail_scroll: 0,
            loading: false,
        };
        state.move_up();
        assert_eq!(state.selected_id(), Some("1"));
        state.move_down();
        state.move_down();
        assert_eq!(state.selected_id(), Some("2"));
    }

    #[test]
    fn collapsed_parent_hides_descendants() {
        let mut state = ManaNavigatorState {
            mana_dir: None,
            nodes: vec![node("1", None, 1, 0), node("1.1", Some("1"), 0, 1)],
            load_error: None,
            selected_id: Some("1".into()),
            expanded: HashSet::from(["1".into()]),
            detail: None,
            detail_error: None,
            filter: String::new(),
            detail_scroll: 0,
            loading: false,
        };
        assert_eq!(state.visible_nodes().len(), 2);
        state.toggle_selected();
        assert_eq!(state.visible_nodes().len(), 1);
    }
}
