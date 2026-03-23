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
}

impl SessionPickerState {
    pub fn new(sessions: Vec<SessionInfo>) -> Self {
        Self {
            sessions,
            selected: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.sessions.len() {
            self.selected += 1;
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

        for (i, session) in self.state.sessions.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

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

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
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
