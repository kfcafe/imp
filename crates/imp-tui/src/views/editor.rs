use imp_llm::ThinkingLevel;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

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

    /// Calculate cursor position relative to a render area.
    pub fn cursor_screen_position(&self, area: Rect) -> (u16, u16) {
        let inner_x = area.x + 1; // account for border
        let inner_y = area.y + 1;
        let x = inner_x + self.cursor_col as u16;
        let y = inner_y + (self.cursor_line as u16).saturating_sub(self.scroll_offset as u16);
        (x.min(area.x + area.width - 2), y.min(area.y + area.height - 2))
    }

    fn update_position(&mut self) {
        let before = &self.content[..self.cursor];
        self.cursor_line = before.matches('\n').count();
        self.cursor_col = before.rfind('\n').map(|p| self.cursor - p - 1).unwrap_or(self.cursor);
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
    is_streaming: bool,
    has_queued: bool,
}

impl<'a> EditorView<'a> {
    pub fn new(state: &'a EditorState, theme: &'a Theme, thinking_level: ThinkingLevel) -> Self {
        Self {
            state,
            theme,
            thinking_level,
            is_streaming: false,
            has_queued: false,
        }
    }

    pub fn streaming(mut self, streaming: bool) -> Self {
        self.is_streaming = streaming;
        self
    }

    pub fn queued(mut self, has_queued: bool) -> Self {
        self.has_queued = has_queued;
        self
    }
}

impl Widget for EditorView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width < 4 {
            return;
        }

        let border_color = self.theme.thinking_border_color(self.thinking_level);

        let title = if self.is_streaming {
            if self.has_queued {
                " streaming [queued] "
            } else {
                " streaming… "
            }
        } else {
            ""
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(area);
        block.render(area, buf);

        // Render editor content
        let lines: Vec<Line> = self
            .state
            .content
            .split('\n')
            .skip(self.state.scroll_offset)
            .map(|line| Line::raw(line.to_string()))
            .collect();

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        paragraph.render(inner, buf);
    }
}

// --- Helpers ---

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
