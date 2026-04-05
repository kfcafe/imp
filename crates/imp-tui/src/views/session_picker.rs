use std::path::Path;

use imp_core::session::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

const ROW_HEIGHT: usize = 3;

#[derive(Debug, Clone)]
pub struct SessionPickerState {
    pub sessions: Vec<SessionInfo>,
    pub filtered_indices: Vec<usize>,
    pub filter: String,
    pub selected: usize,
    pub scroll_offset: usize,
    preferred_cwd: Option<String>,
}

impl SessionPickerState {
    pub fn new(sessions: Vec<SessionInfo>, preferred_cwd: Option<&Path>) -> Self {
        let mut state = Self {
            sessions,
            filtered_indices: Vec::new(),
            filter: String::new(),
            selected: 0,
            scroll_offset: 0,
            preferred_cwd: preferred_cwd.map(|path| path.to_string_lossy().to_string()),
        };
        state.refresh_filter();
        state
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }

    pub fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.refresh_filter();
    }

    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.refresh_filter();
    }

    /// Adjust scroll_offset so the selected item is visible within `visible_rows` entries.
    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected + 1 - visible_rows;
        }
    }

    pub fn selected_session(&self) -> Option<&SessionInfo> {
        let idx = *self.filtered_indices.get(self.selected)?;
        self.sessions.get(idx)
    }

    pub fn visible_sessions(&self) -> impl Iterator<Item = (usize, &SessionInfo)> {
        self.filtered_indices
            .iter()
            .copied()
            .enumerate()
            .map(|(visible_idx, session_idx)| (visible_idx, &self.sessions[session_idx]))
    }

    fn refresh_filter(&mut self) {
        let needle = self.filter.trim().to_lowercase();
        let mut ranked: Vec<(i64, usize)> = self
            .sessions
            .iter()
            .enumerate()
            .filter_map(|(idx, session)| {
                session_score(session, &needle, self.preferred_cwd.as_deref())
                    .map(|score| (score, idx))
            })
            .collect();

        ranked.sort_by(|(score_a, idx_a), (score_b, idx_b)| {
            score_b
                .cmp(score_a)
                .then_with(|| {
                    self.sessions[*idx_b]
                        .updated_at
                        .cmp(&self.sessions[*idx_a].updated_at)
                })
                .then_with(|| {
                    self.sessions[*idx_b]
                        .created_at
                        .cmp(&self.sessions[*idx_a].created_at)
                })
        });

        self.filtered_indices = ranked.into_iter().map(|(_, idx)| idx).collect();

        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
        self.scroll_offset = self.scroll_offset.min(self.selected);
    }
}

pub struct SessionPickerView<'a> {
    state: &'a SessionPickerState,
    theme: &'a Theme,
}

impl<'a> SessionPickerView<'a> {
    pub fn new(state: &'a SessionPickerState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for SessionPickerView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 8 || area.width < 24 {
            return;
        }

        Clear.render(area, buf);
        let title = if self.state.filter.is_empty() {
            " Resume Session ".to_string()
        } else {
            format!(" Resume Session / {} ", self.state.filter)
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let has_preview = inner.width >= 88;
        let columns = if has_preview {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100), Constraint::Percentage(0)])
                .split(inner)
        };
        let list_area = columns[0];
        let preview_area = columns[1];

        if self.state.filtered_indices.is_empty() {
            let msg = if self.state.filter.is_empty() {
                "  No sessions found"
            } else {
                "  No matching sessions"
            };
            let line = Line::from(Span::styled(msg, self.theme.muted_style()));
            buf.set_line(list_area.x, list_area.y, &line, list_area.width);
            if has_preview {
                render_preview_empty(preview_area, buf, self.theme);
            }
            return;
        }

        render_session_list(list_area, self.state, buf, self.theme);
        if has_preview {
            render_session_preview(preview_area, self.state.selected_session(), buf, self.theme);
        }
    }
}

fn render_session_list(area: Rect, state: &SessionPickerState, buf: &mut Buffer, theme: &Theme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let visible_rows = (area.height as usize / ROW_HEIGHT).max(1);
    let scroll_offset = state.scroll_offset;
    let total = state.filtered_indices.len();

    let visible_sessions = state
        .visible_sessions()
        .skip(scroll_offset)
        .take(visible_rows);

    for (row, (visible_idx, session)) in visible_sessions.enumerate() {
        let is_selected = visible_idx == state.selected;
        let style = if is_selected {
            theme.selected_style()
        } else {
            Style::default()
        };

        let preview = session
            .summary
            .as_deref()
            .filter(|summary| !summary.trim().is_empty())
            .map(|summary| summary.trim().to_string())
            .or_else(|| {
                session
                    .first_message
                    .as_deref()
                    .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
            })
            .unwrap_or_else(|| "(empty)".to_string());

        let project = project_name(&session.cwd);
        let title = session
            .title(48)
            .unwrap_or_else(|| "(unnamed session)".to_string());
        let age = format_age(session.updated_at);
        let msgs = format!("{} msg", session.message_count);

        let title_width = area.width.saturating_sub(4) as usize;
        let meta_width = area.width.saturating_sub(4) as usize;
        let preview_width = area.width.saturating_sub(6) as usize;

        let title = truncate(&title, title_width);
        let meta = truncate(&format!("{project}  •  {msgs}  •  {age}"), meta_width);
        let preview = truncate(&preview, preview_width);

        let base_y = area.y + (row as u16 * ROW_HEIGHT as u16);

        let title_line = Line::from(vec![
            Span::styled(
                if is_selected { " ▸ " } else { "   " },
                theme.accent_style(),
            ),
            Span::styled(title, style),
        ]);
        buf.set_line(area.x, base_y, &title_line, area.width);

        if base_y + 1 < area.y + area.height {
            let meta_line = Line::from(vec![
                Span::raw("   "),
                Span::styled(meta, theme.muted_style()),
            ]);
            buf.set_line(area.x, base_y + 1, &meta_line, area.width);
        }

        if base_y + 2 < area.y + area.height {
            let preview_line = Line::from(vec![
                Span::raw("   "),
                Span::styled(preview, theme.muted_style()),
            ]);
            buf.set_line(area.x, base_y + 2, &preview_line, area.width);
        }
    }

    if scroll_offset > 0 {
        let indicator = Line::from(Span::styled("▲", theme.muted_style()));
        buf.set_line(area.x + area.width.saturating_sub(1), area.y, &indicator, 1);
    }
    if scroll_offset + visible_rows < total {
        let indicator = Line::from(Span::styled("▼", theme.muted_style()));
        buf.set_line(
            area.x + area.width.saturating_sub(1),
            area.y + area.height.saturating_sub(1),
            &indicator,
            1,
        );
    }
}

fn render_preview_empty(area: Rect, buf: &mut Buffer, theme: &Theme) {
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::LEFT)
        .border_style(theme.muted_style());
    let inner = block.inner(area);
    block.render(area, buf);
    let line = Line::from(Span::styled(
        "Type to fuzzy-search sessions.",
        theme.muted_style(),
    ));
    if inner.height > 0 {
        buf.set_line(inner.x, inner.y, &line, inner.width);
    }
}

fn render_session_preview(
    area: Rect,
    session: Option<&SessionInfo>,
    buf: &mut Buffer,
    theme: &Theme,
) {
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::LEFT)
        .border_style(theme.muted_style());
    let inner = block.inner(area);
    block.render(area, buf);

    let Some(session) = session else {
        return;
    };

    let title = session
        .title(80)
        .unwrap_or_else(|| "(unnamed session)".to_string());
    let summary = session
        .summary
        .as_deref()
        .filter(|summary| !summary.trim().is_empty())
        .unwrap_or("(no summary yet)");
    let prompt = session
        .first_message
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or("(no prompt captured)");

    let lines = [
        format!("Title: {title}"),
        format!("Project: {}", project_name(&session.cwd)),
        format!("Updated: {}", format_age(session.updated_at)),
        format!("Messages: {}", session.message_count),
        format!("ID: {}", session.id),
        String::new(),
        "Summary:".to_string(),
        summary.to_string(),
        String::new(),
        "First prompt:".to_string(),
        prompt.to_string(),
        String::new(),
        "Enter opens • type filters • Esc cancels".to_string(),
    ];

    let wrapped = wrap_lines(&lines, inner.width as usize, inner.height as usize);
    for (i, line) in wrapped.iter().enumerate() {
        if i >= inner.height as usize {
            break;
        }
        let style = if line.is_empty() {
            theme.muted_style()
        } else if matches!(line.as_str(), "Summary:" | "First prompt:") {
            theme.accent_style()
        } else {
            theme.muted_style()
        };
        let rendered = Line::from(Span::styled(line.clone(), style));
        buf.set_line(inner.x, inner.y + i as u16, &rendered, inner.width);
    }
}

fn session_score(session: &SessionInfo, needle: &str, preferred_cwd: Option<&str>) -> Option<i64> {
    let mut score = 0i64;

    if let Some(cwd) = preferred_cwd {
        if session.cwd == cwd {
            score += 20_000;
        } else if path_related(&session.cwd, cwd) {
            score += 5_000;
        } else if project_name(&session.cwd) == project_name(cwd) {
            score += 1_500;
        }
    }

    if needle.is_empty() {
        return Some(score);
    }

    let mut best_match = 0i64;
    best_match = best_match.max(text_match_score(session.name.as_deref(), needle, 1_200));
    best_match = best_match.max(text_match_score(session.summary.as_deref(), needle, 1_000));
    best_match = best_match.max(text_match_score(
        session.first_message.as_deref(),
        needle,
        700,
    ));
    best_match = best_match.max(text_match_score(Some(&session.cwd), needle, 500));
    best_match = best_match.max(text_match_score(Some(&session.id), needle, 300));

    if best_match == 0 {
        None
    } else {
        Some(score + best_match)
    }
}

fn text_match_score(value: Option<&str>, needle: &str, weight: i64) -> i64 {
    let Some(value) = value else { return 0 };
    let haystack = value.to_lowercase();

    if haystack == needle {
        return weight + 900;
    }
    if haystack.starts_with(needle) {
        return weight + 600;
    }
    if let Some(pos) = haystack.find(needle) {
        return weight + 400 - pos as i64;
    }
    if fuzzy_match(&haystack, needle) {
        return weight + 150 + needle.len() as i64;
    }
    0
}

fn path_related(a: &str, b: &str) -> bool {
    let a = Path::new(a);
    let b = Path::new(b);
    a.starts_with(b) || b.starts_with(a)
}

fn project_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.contains(needle) {
        return true;
    }

    let mut chars = needle.chars();
    let Some(mut current) = chars.next() else {
        return true;
    };

    for ch in haystack.chars() {
        if ch == current {
            if let Some(next) = chars.next() {
                current = next;
            } else {
                return true;
            }
        }
    }

    false
}

fn truncate(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }

    if max_chars == 1 {
        return "…".to_string();
    }

    let take = max_chars.saturating_sub(1);
    let mut out = text.chars().take(take).collect::<String>();
    out.push('…');
    out
}

fn wrap_lines(lines: &[String], width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for line in lines {
        if out.len() >= max_lines {
            break;
        }
        if line.is_empty() {
            out.push(String::new());
            continue;
        }

        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            out.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in words {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };

            if candidate.chars().count() <= width {
                current = candidate;
            } else {
                if !current.is_empty() {
                    out.push(current);
                    if out.len() >= max_lines {
                        return out;
                    }
                }
                current = truncate(word, width);
            }
        }

        if !current.is_empty() {
            out.push(current);
        }
    }

    out.truncate(max_lines);
    out
}

fn format_age(updated_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let delta = now.saturating_sub(updated_at);
    if delta < 60 {
        "just now".into()
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_session(
        id: &str,
        title: Option<&str>,
        summary: Option<&str>,
        cwd: &str,
        first_message: &str,
        updated_at: u64,
    ) -> SessionInfo {
        SessionInfo {
            id: id.to_string(),
            path: PathBuf::from(format!("/tmp/{id}.jsonl")),
            cwd: cwd.to_string(),
            created_at: 0,
            updated_at,
            message_count: 3,
            first_message: Some(first_message.to_string()),
            name: title.map(str::to_string),
            summary: summary.map(str::to_string),
        }
    }

    #[test]
    fn picker_filter_matches_name_summary_and_path() {
        let sessions = vec![
            make_session(
                "one",
                Some("oauth debugging"),
                Some("Investigated OAuth refresh failures"),
                "/tmp/tower/imp",
                "first prompt about oauth login",
                10,
            ),
            make_session(
                "two",
                Some("render tweaks"),
                Some("Adjusted top bar display"),
                "/tmp/tower/wizard",
                "first prompt about top bar tweaks",
                20,
            ),
        ];

        let mut state = SessionPickerState::new(sessions, Some(Path::new("/tmp/tower/imp")));
        state.push_filter('o');
        state.push_filter('a');
        state.push_filter('u');
        state.push_filter('t');
        state.push_filter('h');

        assert_eq!(state.filtered_indices.len(), 1);
        assert_eq!(state.selected_session().unwrap().id, "one");

        state.pop_filter();
        assert_eq!(state.filter, "oaut");
    }

    #[test]
    fn fuzzy_match_supports_subsequence() {
        assert!(fuzzy_match("oauth debugging", "oad"));
        assert!(!fuzzy_match("render tweaks", "oz"));
    }

    #[test]
    fn preferred_cwd_sessions_rank_first() {
        let sessions = vec![
            make_session(
                "old-local",
                Some("local"),
                Some("older local session"),
                "/tmp/tower/imp",
                "prompt",
                10,
            ),
            make_session(
                "new-remote",
                Some("remote"),
                Some("newer remote session"),
                "/tmp/tower/wizard",
                "prompt",
                99,
            ),
        ];

        let state = SessionPickerState::new(sessions, Some(Path::new("/tmp/tower/imp")));
        assert_eq!(state.selected_session().unwrap().id, "old-local");
    }
}
