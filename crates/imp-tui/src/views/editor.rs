use crate::animation::{activity_label, ActivitySurface, AnimationState};
use imp_core::config::AnimationLevel;
use imp_llm::ThinkingLevel;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};
use unicode_width::UnicodeWidthChar;

use crate::theme::Theme;

/// Multi-line editor state with cursor management.
#[derive(Debug, Clone)]
pub struct EditorState {
    pub content: String,
    pub cursor: usize,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub scroll_offset: usize,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            cursor_line: 0,
            cursor_col: 0,
            history: Vec::new(),
            history_idx: None,
            scroll_offset: 0,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.update_position();
    }

    pub fn insert_newline(&mut self) {
        self.content.insert(self.cursor, '\n');
        self.cursor += 1;
        self.update_position();
    }

    pub fn delete_back(&mut self) {
        if self.cursor > 0 {
            let prev = prev_char_boundary(&self.content, self.cursor);
            self.content.drain(prev..self.cursor);
            self.cursor = prev;
            self.update_position();
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor < self.content.len() {
            let next = next_char_boundary(&self.content, self.cursor);
            self.content.drain(self.cursor..next);
            self.update_position();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = prev_char_boundary(&self.content, self.cursor);
            self.update_position();
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor = next_char_boundary(&self.content, self.cursor);
            self.update_position();
        }
    }

    pub fn move_up(&mut self) -> bool {
        if self.cursor_line == 0 {
            return false; // signal: at top, caller may use for history
        }
        let lines: Vec<&str> = self.content.split('\n').collect();
        let target_line = self.cursor_line - 1;
        let target_col = self.cursor_col.min(lines[target_line].len());
        self.cursor = line_col_to_byte(&lines, target_line, target_col);
        self.update_position();
        true
    }

    pub fn move_down(&mut self) -> bool {
        let lines: Vec<&str> = self.content.split('\n').collect();
        if self.cursor_line >= lines.len() - 1 {
            return false; // signal: at bottom, caller may use for history
        }
        let target_line = self.cursor_line + 1;
        let target_col = self.cursor_col.min(lines[target_line].len());
        self.cursor = line_col_to_byte(&lines, target_line, target_col);
        self.update_position();
        true
    }

    pub fn move_home(&mut self) {
        let before = &self.content[..self.cursor];
        self.cursor = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        self.update_position();
    }

    pub fn move_end(&mut self) {
        let after = &self.content[self.cursor..];
        self.cursor += after.find('\n').unwrap_or(after.len());
        self.update_position();
    }

    pub fn move_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let bytes = self.content.as_bytes();
        let mut pos = self.cursor;
        // Skip whitespace
        while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        // Skip word chars
        while pos > 0 && !bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        self.cursor = pos;
        self.update_position();
    }

    pub fn move_word_right(&mut self) {
        let bytes = self.content.as_bytes();
        let len = bytes.len();
        let mut pos = self.cursor;
        // Skip current word
        while pos < len && !bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        // Skip whitespace
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.cursor = pos;
        self.update_position();
    }

    pub fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.cursor;
        self.move_word_left();
        self.content.drain(self.cursor..start);
        self.update_position();
    }

    pub fn delete_to_start(&mut self) {
        let line_start = {
            let before = &self.content[..self.cursor];
            before.rfind('\n').map(|p| p + 1).unwrap_or(0)
        };
        self.content.drain(line_start..self.cursor);
        self.cursor = line_start;
        self.update_position();
    }

    pub fn delete_to_end(&mut self) {
        let line_end = {
            let after = &self.content[self.cursor..];
            self.cursor + after.find('\n').unwrap_or(after.len())
        };
        self.content.drain(self.cursor..line_end);
        self.update_position();
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
        self.update_position();
    }

    pub fn set_content(&mut self, text: &str) {
        self.content = text.to_string();
        self.cursor = self.content.len();
        self.update_position();
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.content.split('\n').count().max(1)
    }

    pub fn visual_line_count(&self, inner_width: u16) -> usize {
        wrapped_lines_for_width(&self.content, inner_width).len().max(1)
    }

    pub fn push_history(&mut self) {
        if !self.content.trim().is_empty() {
            self.history.push(self.content.clone());
        }
        self.history_idx = None;
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            Some(i) if i > 0 => i - 1,
            Some(_) => return,
            None => {
                if !self.content.is_empty() {
                    self.history.push(self.content.clone());
                }
                self.history.len() - 1
            }
        };
        self.history_idx = Some(idx);
        self.content = self.history[idx].clone();
        self.cursor = self.content.len();
        self.update_position();
    }

    pub fn history_next(&mut self) {
        if let Some(i) = self.history_idx {
            if i + 1 < self.history.len() {
                self.history_idx = Some(i + 1);
                self.content = self.history[i + 1].clone();
            } else {
                self.history_idx = None;
                self.content.clear();
            }
            self.cursor = self.content.len();
            self.update_position();
        }
    }

    /// Calculate cursor position relative to a render area, accounting for soft wraps.
    pub fn cursor_screen_position(&self, area: Rect) -> (u16, u16) {
        let inner_x = area.x + 1; // account for border
        let inner_y = area.y + 1;
        let inner_width = area.width.saturating_sub(2).max(1);
        let (visual_line, visual_col) =
            cursor_visual_position_for_text(&self.content, self.cursor, inner_width);
        let x = inner_x + visual_col as u16;
        let y = inner_y + (visual_line as u16).saturating_sub(self.scroll_offset as u16);
        (
            x.min(area.x + area.width - 2),
            y.min(area.y + area.height - 2),
        )
    }

    fn update_position(&mut self) {
        let before = &self.content[..self.cursor];
        self.cursor_line = before.matches('\n').count();
        self.cursor_col = before
            .rfind('\n')
            .map(|p| self.cursor - p - 1)
            .unwrap_or(self.cursor);
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}

/// The editor widget renders the input area with border and cursor.
pub struct EditorView<'a> {
    state: &'a EditorState,
    theme: &'a Theme,
    thinking_level: ThinkingLevel,
    model_name: &'a str,
    is_streaming: bool,
    has_queued: bool,
    current_context_tokens: u32,
    context_window: u32,
    show_context_usage: bool,
    tick: u64,
    animation_level: AnimationLevel,
    activity_state: AnimationState,
}

impl<'a> EditorView<'a> {
    pub fn new(state: &'a EditorState, theme: &'a Theme, thinking_level: ThinkingLevel) -> Self {
        Self {
            state,
            theme,
            thinking_level,
            model_name: "",
            is_streaming: false,
            has_queued: false,
            current_context_tokens: 0,
            context_window: 0,
            show_context_usage: true,
            tick: 0,
            animation_level: AnimationLevel::Minimal,
            activity_state: AnimationState::Idle,
        }
    }

    /// Set the model name shown in the editor border.
    pub fn model(mut self, name: &'a str) -> Self {
        self.model_name = name;
        self
    }

    pub fn streaming(mut self, streaming: bool) -> Self {
        self.is_streaming = streaming;
        self
    }

    pub fn queued(mut self, has_queued: bool) -> Self {
        self.has_queued = has_queued;
        self
    }

    pub fn context_usage(mut self, current_tokens: u32, context_window: u32, show: bool) -> Self {
        self.current_context_tokens = current_tokens;
        self.context_window = context_window;
        self.show_context_usage = show;
        self
    }

    pub fn tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    pub fn animation_level(mut self, level: AnimationLevel) -> Self {
        self.animation_level = level;
        self
    }

    pub fn activity_state(mut self, state: AnimationState) -> Self {
        self.activity_state = state;
        self
    }
}

impl Widget for EditorView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width < 4 {
            return;
        }

        let base_border_color = self.theme.thinking_border_color(self.thinking_level);
        let border_color = base_border_color;

        let top_title = String::new();
        let activity = activity_label(
            self.activity_state,
            self.tick,
            self.animation_level,
            ActivitySurface::Editor,
        );

        // Build bottom-right model + thinking indicator
        let thinking_label = match self.thinking_level {
            ThinkingLevel::Off => "",
            ThinkingLevel::Minimal => "min",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "med",
            ThinkingLevel::High => "high",
            ThinkingLevel::XHigh => "xhigh",
        };
        let model_label = if self.model_name.is_empty() {
            None
        } else {
            Some(self.model_name.to_string())
        };
        let queue_label = if self.has_queued {
            Some("queued".to_string())
        } else {
            None
        };
        let context_ratio = if self.context_window > 0 {
            self.current_context_tokens as f64 / self.context_window as f64
        } else {
            0.0
        };
        let context_style = if context_ratio >= 0.75 {
            self.theme.error_style()
        } else if context_ratio >= 0.50 {
            self.theme.warning_style()
        } else {
            self.theme.muted_style()
        };
        let mut bottom_spans = Vec::new();
        let mut push_part = |text: String, style: Style| {
            if !bottom_spans.is_empty() {
                bottom_spans.push(Span::styled(" • ".to_string(), self.theme.muted_style()));
            }
            bottom_spans.push(Span::styled(text, style));
        };
        if let Some(model) = model_label {
            push_part(model, self.theme.accent_style());
        }
        if !thinking_label.is_empty() {
            push_part(
                thinking_label.to_string(),
                Style::default().fg(self.theme.thinking_border_color(self.thinking_level)),
            );
        }
        if self.show_context_usage && self.context_window > 0 {
            push_part(
                format_context_usage(self.current_context_tokens, self.context_window),
                context_style,
            );
        }
        if let Some(queue) = queue_label {
            push_part(queue, self.theme.warning_style());
        }
        if !activity.is_empty() {
            push_part(activity, self.theme.muted_style());
        }

        let block = Block::default()
            .title(top_title)
            .title_bottom(Line::from(bottom_spans).alignment(Alignment::Right))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        // Render editor content using wrapped visual lines so auto-grow and cursor math stay aligned.
        let lines = wrapped_lines_for_width(&self.state.content, inner.width)
            .into_iter()
            .skip(self.state.scroll_offset)
            .take(inner.height as usize)
            .collect::<Vec<_>>();

        for (idx, line) in lines.iter().enumerate() {
            if idx >= inner.height as usize {
                break;
            }
            buf.set_line(
                inner.x,
                inner.y + idx as u16,
                &Line::raw(line.clone()),
                inner.width,
            );
        }

        // Placeholder text when empty and not streaming
        if self.state.content.is_empty() && !self.is_streaming {
            let placeholder = "Ask anything… ⇧↵ newline  @file  /commands";
            buf.set_string(
                inner.x,
                inner.y,
                placeholder,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

// --- Helpers ---

fn format_context_usage(current_tokens: u32, context_window: u32) -> String {
    if context_window == 0 {
        return format_compact_tokens(current_tokens);
    }
    let percent = ((current_tokens as f64 / context_window as f64) * 100.0).round();
    format!("{percent:.0}%/{}", format_compact_tokens(context_window))
}

fn format_compact_tokens(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        let value = tokens as f64 / 1_000.0;
        if value >= 100.0 {
            format!("{:.0}k", value)
        } else if value >= 10.0 {
            format!("{:.1}k", value)
        } else {
            format!("{:.2}k", value)
        }
    } else {
        tokens.to_string()
    }
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos;
    while p > 0 {
        p -= 1;
        if s.is_char_boundary(p) {
            return p;
        }
    }
    0
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos;
    while p < s.len() {
        p += 1;
        if s.is_char_boundary(p) {
            return p;
        }
    }
    s.len()
}

fn line_col_to_byte(lines: &[&str], line: usize, col: usize) -> usize {
    let mut byte = 0;
    for (i, l) in lines.iter().enumerate() {
        if i == line {
            return byte + col.min(l.len());
        }
        byte += l.len() + 1; // +1 for \n
    }
    byte
}

pub fn wrapped_lines_for_width(text: &str, inner_width: u16) -> Vec<String> {
    let width = inner_width.max(1) as usize;
    let mut out = Vec::new();

    for logical in text.split('\n') {
        if logical.is_empty() {
            out.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0usize;

        for ch in logical.chars() {
            let ch_width = char_display_width(ch);

            if !current.is_empty() && current_width + ch_width > width {
                out.push(current);
                current = String::new();
                current_width = 0;
            }

            if current.is_empty() && ch_width > width {
                out.push(ch.to_string());
                continue;
            }

            current.push(ch);
            current_width += ch_width;

            if current_width == width {
                out.push(current);
                current = String::new();
                current_width = 0;
            }
        }

        if !current.is_empty() {
            out.push(current);
        }
    }

    if out.is_empty() {
        out.push(String::new());
    }

    out
}

pub fn cursor_visual_position_for_text(
    text: &str,
    cursor: usize,
    inner_width: u16,
) -> (usize, usize) {
    let width = inner_width.max(1) as usize;
    let mut row = 0usize;
    let mut col = 0usize;
    let mut byte = 0usize;

    for ch in text.chars() {
        if byte >= cursor {
            break;
        }

        if ch == '\n' {
            row += 1;
            col = 0;
            byte += ch.len_utf8();
            continue;
        }

        let ch_width = char_display_width(ch);

        if col > 0 && col + ch_width > width {
            row += 1;
            col = 0;
        }

        if col == 0 && ch_width > width {
            row += 1;
            col = 0;
            byte += ch.len_utf8();
            continue;
        }

        col += ch_width;
        byte += ch.len_utf8();

        if col == width {
            row += 1;
            col = 0;
        }
    }

    (row, col)
}

fn char_display_width(ch: char) -> usize {
    match ch {
        '\t' => 4,
        _ => ch.width().unwrap_or(1).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn format_context_usage_prefers_percent_over_current_tokens() {
        assert_eq!(format_context_usage(82_400, 1_000_000), "8%/1.0M");
        assert_eq!(format_context_usage(500_000, 1_000_000), "50%/1.0M");
    }

    #[test]
    fn format_compact_tokens_handles_millions() {
        assert_eq!(format_compact_tokens(1_000_000), "1.0M");
        assert_eq!(format_compact_tokens(1_250_000), "1.2M");
    }

    #[test]
    fn format_compact_tokens_handles_thousands() {
        assert_eq!(format_compact_tokens(9_500), "9.50k");
        assert_eq!(format_compact_tokens(12_300), "12.3k");
        assert_eq!(format_compact_tokens(234_000), "234k");
    }

    #[test]
    fn visual_line_count_includes_soft_wraps() {
        let mut editor = EditorState::new();
        editor.set_content("abcdefghij");

        assert_eq!(editor.visual_line_count(4), 3);
    }

    #[test]
    fn cursor_screen_position_tracks_soft_wraps() {
        let mut editor = EditorState::new();
        editor.set_content("abcdefghij");

        let area = Rect::new(0, 0, 6, 5); // inner width = 4
        let (x, y) = editor.cursor_screen_position(area);

        assert_eq!((x, y), (3, 3));
    }
}
