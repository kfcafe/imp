use imp_core::session::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct SessionPickerState {
    pub sessions: Vec<SessionInfo>,
    pub selected: usize,
    pub scroll_offset: usize,
}

impl SessionPickerState {
    pub fn new(sessions: Vec<SessionInfo>) -> Self {
        Self {
            sessions,
            selected: 0,
            scroll_offset: 0,
        }
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
        if self.selected + 1 < self.sessions.len() {
            self.selected += 1;
        }
    }

    /// Adjust scroll_offset so the selected item is visible within `visible_height` rows.
    pub fn clamp_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected + 1 - visible_height;
        }
    }

    pub fn selected_session(&self) -> Option<&SessionInfo> {
        self.sessions.get(self.selected)
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
        if area.height < 5 || area.width < 20 {
            return;
        }

        Clear.render(area, buf);
        let block = Block::default()
            .title(" Resume Session ")
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.sessions.is_empty() {
            let line = Line::from(Span::styled(
                "  No sessions found",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        let visible_height = inner.height as usize;
        let scroll_offset = self.state.scroll_offset;
        let total = self.state.sessions.len();

        let visible_sessions = self
            .state
            .sessions
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height);

        for (row, (i, session)) in visible_sessions.enumerate() {
            let is_selected = i == self.state.selected;
            let style = if is_selected {
                self.theme.selected_style()
            } else {
                Style::default()
            };

            let preview = session
                .first_message
                .as_deref()
                .unwrap_or("(empty)")
                .chars()
                .take(40)
                .collect::<String>();

            let age = format_age(session.updated_at);
            let msgs = format!("{}msg", session.message_count);

            let line = Line::from(vec![
                Span::styled(
                    if is_selected { " ▸ " } else { "   " },
                    self.theme.accent_style(),
                ),
                Span::styled(preview, style),
                Span::raw("  "),
                Span::styled(msgs, self.theme.muted_style()),
                Span::raw("  "),
                Span::styled(age, self.theme.muted_style()),
            ]);

            buf.set_line(inner.x, inner.y + row as u16, &line, inner.width);
        }

        // Scroll indicators
        if scroll_offset > 0 {
            let indicator = Line::from(Span::styled("▲", self.theme.muted_style()));
            buf.set_line(
                inner.x + inner.width.saturating_sub(1),
                inner.y,
                &indicator,
                1,
            );
        }
        if scroll_offset + visible_height < total {
            let indicator = Line::from(Span::styled("▼", self.theme.muted_style()));
            buf.set_line(
                inner.x + inner.width.saturating_sub(1),
                inner.y + inner.height.saturating_sub(1),
                &indicator,
                1,
            );
        }
    }
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
