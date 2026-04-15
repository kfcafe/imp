use std::collections::HashMap;
use std::time::Duration;

use crate::animation::{activity_label, format_elapsed, ActivitySurface, AnimationState};
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
    fn normalized_cursor(&self) -> usize {
        clamp_cursor_to_boundary(&self.content, self.cursor)
    }

    fn normalize_cursor(&mut self) {
        self.cursor = self.normalized_cursor();
    }

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
        self.normalize_cursor();
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.update_position();
    }

    pub fn insert_newline(&mut self) {
        self.normalize_cursor();
        self.content.insert(self.cursor, '\n');
        self.cursor += 1;
        self.update_position();
    }

    pub fn delete_back(&mut self) {
        self.normalize_cursor();
        if self.cursor > 0 {
            let prev = prev_char_boundary(&self.content, self.cursor);
            self.content.drain(prev..self.cursor);
            self.cursor = prev;
            self.update_position();
        }
    }

    pub fn delete_forward(&mut self) {
        self.normalize_cursor();
        if self.cursor < self.content.len() {
            let next = next_char_boundary(&self.content, self.cursor);
            self.content.drain(self.cursor..next);
            self.update_position();
        }
    }

    pub fn move_left(&mut self) {
        self.normalize_cursor();
        if self.cursor > 0 {
            self.cursor = prev_char_boundary(&self.content, self.cursor);
            self.update_position();
        }
    }

    pub fn move_right(&mut self) {
        self.normalize_cursor();
        if self.cursor < self.content.len() {
            self.cursor = next_char_boundary(&self.content, self.cursor);
            self.update_position();
        }
    }

    pub fn move_up(&mut self) -> bool {
        self.normalize_cursor();
        self.update_position();
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
        self.normalize_cursor();
        self.update_position();
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
        self.normalize_cursor();
        let before = &self.content[..self.cursor];
        self.cursor = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        self.update_position();
    }

    pub fn move_end(&mut self) {
        self.normalize_cursor();
        let after = &self.content[self.cursor..];
        self.cursor += after.find('\n').unwrap_or(after.len());
        self.update_position();
    }

    pub fn move_word_left(&mut self) {
        self.normalize_cursor();
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
        self.normalize_cursor();
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
        self.normalize_cursor();
        if self.cursor == 0 {
            return;
        }
        let start = self.cursor;
        self.move_word_left();
        self.content.drain(self.cursor..start);
        self.update_position();
    }

    pub fn delete_to_start(&mut self) {
        self.normalize_cursor();
        let line_start = {
            let before = &self.content[..self.cursor];
            before.rfind('\n').map(|p| p + 1).unwrap_or(0)
        };
        self.content.drain(line_start..self.cursor);
        self.cursor = line_start;
        self.update_position();
    }

    pub fn delete_to_end(&mut self) {
        self.normalize_cursor();
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

    pub fn visual_line_count_with_summary(&self, inner_width: u16, summarize_paste: bool) -> usize {
        editor_display_lines(&self.content, inner_width, summarize_paste)
            .len()
            .max(1)
    }

    pub fn visual_line_count(&self, inner_width: u16) -> usize {
        self.visual_line_count_with_summary(inner_width, false)
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
        if area.width == 0 || area.height == 0 {
            return (area.x, area.y);
        }

        let inner_x = area.x.saturating_add(1); // account for border
        let inner_y = area.y.saturating_add(1);
        let inner_width = area.width.saturating_sub(2).max(1);
        let cursor = self.normalized_cursor();
        let (visual_line, visual_col) =
            cursor_visual_position_for_text(&self.content, cursor, inner_width);
        let x = inner_x.saturating_add(visual_col as u16);
        let y =
            inner_y.saturating_add((visual_line as u16).saturating_sub(self.scroll_offset as u16));
        let max_x = area.x.saturating_add(area.width.saturating_sub(2));
        let max_y = area.y.saturating_add(area.height.saturating_sub(2));
        (x.min(max_x), y.min(max_y))
    }

    fn update_position(&mut self) {
        self.normalize_cursor();
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
    summarize_paste: bool,
    model_name: &'a str,
    cwd: &'a str,
    session_name: &'a str,
    is_streaming: bool,
    has_queued: bool,
    current_context_tokens: u32,
    context_window: u32,
    show_context_usage: bool,
    turn_elapsed: Option<Duration>,
    extension_items: Option<&'a HashMap<String, String>>,
    peek: bool,
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
            summarize_paste: false,
            model_name: "",
            cwd: "",
            session_name: "",
            is_streaming: false,
            has_queued: false,
            current_context_tokens: 0,
            context_window: 0,
            show_context_usage: true,
            turn_elapsed: None,
            extension_items: None,
            peek: false,
            tick: 0,
            animation_level: AnimationLevel::Minimal,
            activity_state: AnimationState::Idle,
        }
    }

    pub fn summarize_paste(mut self, summarize: bool) -> Self {
        self.summarize_paste = summarize;
        self
    }

    /// Set the model name shown in the editor border.
    pub fn model(mut self, name: &'a str) -> Self {
        self.model_name = name;
        self
    }

    pub fn identity(mut self, cwd: &'a str, session_name: &'a str) -> Self {
        self.cwd = cwd;
        self.session_name = session_name;
        self
    }

    pub fn turn_elapsed(mut self, elapsed: Option<Duration>) -> Self {
        self.turn_elapsed = elapsed;
        self
    }

    pub fn extension_items(mut self, items: &'a HashMap<String, String>, peek: bool) -> Self {
        self.extension_items = Some(items);
        self.peek = peek;
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

        let prompt_activity_state = if self.has_queued {
            AnimationState::Queued
        } else {
            self.activity_state
        };

        let border_style = superbar_border_style(
            self.theme,
            self.thinking_level,
            prompt_activity_state,
            self.tick,
            self.animation_level,
        );

        let top_left = build_identity_label(self.cwd, self.session_name, area.width);
        let top_right = build_top_right_label(self.turn_elapsed, self.theme);
        let bottom_left =
            build_bottom_left_label(prompt_activity_state, self.tick, self.animation_level);
        let activity =
            editor_activity_label(prompt_activity_state, self.tick, self.animation_level);

        // Build bottom-right metadata cluster
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
        let queue_label = None;
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
            .title(Line::from(top_left))
            .title(Line::from(top_right).alignment(Alignment::Right))
            .title_bottom(Line::from(bottom_left))
            .title_bottom(Line::from(bottom_spans).alignment(Alignment::Right))
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        // Render editor content using wrapped visual lines so auto-grow and cursor math stay aligned.
        let lines = editor_display_lines(&self.state.content, inner.width, self.summarize_paste)
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
            let placeholder = "Ask anything… ⇧↵ newline  @file attach context  / palette  : shell";
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

fn editor_display_lines(text: &str, inner_width: u16, summarize_paste: bool) -> Vec<String> {
    if summarize_paste {
        if let Some(summary) = crate::views::chat::pasted_block_summary(text) {
            return wrapped_lines_for_width(&summary, inner_width);
        }
    }

    wrapped_lines_for_width(text, inner_width)
}

fn build_identity_label(cwd: &str, session_name: &str, area_width: u16) -> Vec<Span<'static>> {
    let max_path = (area_width as usize / 3).clamp(12, 36);
    let cwd = abbreviate_home(cwd);
    let cwd = shorten_path(&cwd, max_path);
    let session_name = session_name.trim();

    let mut spans = vec![Span::raw(cwd)];
    if !session_name.is_empty() {
        spans.push(Span::raw(" · "));
        spans.push(Span::raw(session_name.to_string()));
    }
    spans
}

fn build_top_right_label(turn_elapsed: Option<Duration>, theme: &Theme) -> Vec<Span<'static>> {
    turn_elapsed
        .map(|elapsed| vec![Span::styled(format_elapsed(elapsed), theme.muted_style())])
        .unwrap_or_default()
}

fn build_bottom_left_label(
    activity_state: AnimationState,
    tick: u64,
    animation_level: AnimationLevel,
) -> Vec<Span<'static>> {
    let label = editor_activity_label(activity_state, tick, animation_level);
    if label.is_empty() {
        Vec::new()
    } else {
        vec![Span::raw(label)]
    }
}

fn editor_activity_label(
    activity_state: AnimationState,
    tick: u64,
    animation_level: AnimationLevel,
) -> String {
    match activity_state {
        AnimationState::Thinking | AnimationState::WaitingForResponse => String::new(),
        _ => activity_label(
            activity_state,
            tick,
            animation_level,
            ActivitySurface::Editor,
        ),
    }
}

fn superbar_border_style(
    theme: &Theme,
    thinking_level: ThinkingLevel,
    activity_state: AnimationState,
    tick: u64,
    animation_level: AnimationLevel,
) -> Style {
    let color = superbar_border_color(theme, thinking_level, activity_state, tick, animation_level);
    let mut style = Style::default().fg(color);
    if superbar_border_is_animated(activity_state, animation_level) {
        style = style.add_modifier(ratatui::style::Modifier::BOLD);
    }
    style
}

fn superbar_border_is_animated(
    activity_state: AnimationState,
    animation_level: AnimationLevel,
) -> bool {
    if animation_level == AnimationLevel::None {
        return false;
    }

    !matches!(
        activity_state,
        AnimationState::Idle | AnimationState::Thinking | AnimationState::WaitingForResponse
    )
}

fn superbar_border_color(
    theme: &Theme,
    thinking_level: ThinkingLevel,
    activity_state: AnimationState,
    tick: u64,
    animation_level: AnimationLevel,
) -> Color {
    let base = theme.thinking_border_color(thinking_level);
    if !superbar_border_is_animated(activity_state, animation_level) {
        return base;
    }

    let target = match activity_state {
        AnimationState::Idle => base,
        AnimationState::WaitingForResponse => theme.muted,
        AnimationState::Thinking => theme.accent,
        AnimationState::ExecutingTools { .. } => theme.warning,
        AnimationState::Streaming => theme.success,
        AnimationState::Queued => theme.warning,
    };

    let pulse = match animation_level {
        AnimationLevel::None => 0.0,
        AnimationLevel::Minimal => {
            const PULSE: [f32; 4] = [0.10, 0.22, 0.34, 0.22];
            PULSE[(tick / 4) as usize % PULSE.len()]
        }
        AnimationLevel::Spinner => {
            const PULSE: [f32; 6] = [0.16, 0.32, 0.50, 0.68, 0.50, 0.32];
            PULSE[(tick / 2) as usize % PULSE.len()]
        }
    };

    mix_color(base, target, pulse)
}

fn mix_color(base: Color, target: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    match (base, target) {
        (Color::Rgb(br, bg, bb), Color::Rgb(tr, tg, tb)) => Color::Rgb(
            mix_channel(br, tr, amount),
            mix_channel(bg, tg, amount),
            mix_channel(bb, tb, amount),
        ),
        _ if amount >= 0.5 => target,
        _ => base,
    }
}

fn mix_channel(base: u8, target: u8, amount: f32) -> u8 {
    (base as f32 + (target as f32 - base as f32) * amount).round() as u8
}

fn abbreviate_home(path: &str) -> String {
    if path == "/Users/asher" {
        "~".to_string()
    } else if let Some(rest) = path.strip_prefix("/Users/asher/") {
        format!("~/{rest}")
    } else {
        path.to_string()
    }
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        let shortened = shorten_path(&format!("home/{rest}"), max_len.saturating_sub(1));
        return shortened.replacen("home/", "~/", 1);
    }

    let parts: Vec<&str> = path.split('/').collect();
    let mut result = String::new();
    for part in parts.iter().rev() {
        let candidate = if result.is_empty() {
            part.to_string()
        } else {
            format!("{part}/{result}")
        };
        if candidate.len() > max_len {
            break;
        }
        result = candidate;
    }

    if result.len() < path.len() {
        format!("…/{result}")
    } else {
        result
    }
}

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
    let mut p = pos.min(s.len());
    while p < s.len() {
        p += 1;
        if s.is_char_boundary(p) {
            return p;
        }
    }
    s.len()
}

pub fn clamp_cursor_to_boundary(text: &str, cursor: usize) -> usize {
    let mut clamped = cursor.min(text.len());
    while clamped > 0 && !text.is_char_boundary(clamped) {
        clamped -= 1;
    }
    clamped
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
    fn visual_line_count_with_paste_summary_uses_summary_height() {
        let mut editor = EditorState::new();
        editor.set_content(
            &(1..=25)
                .map(|i| format!("fn example_{i}() {{}}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        assert_eq!(editor.visual_line_count_with_summary(80, true), 1);
    }

    #[test]
    fn cursor_screen_position_tracks_soft_wraps() {
        let mut editor = EditorState::new();
        editor.set_content("abcdefghij");

        let area = Rect::new(0, 0, 6, 5); // inner width = 4
        let (x, y) = editor.cursor_screen_position(area);

        assert_eq!((x, y), (3, 3));
    }

    #[test]
    fn editor_operations_clamp_cursor_past_end() {
        let mut editor = EditorState::new();
        editor.set_content("abc");
        editor.cursor = 99;

        editor.delete_back();

        assert_eq!(editor.content(), "ab");
        assert_eq!(editor.cursor, 2);
    }

    #[test]
    fn editor_operations_clamp_invalid_utf8_boundary() {
        let mut editor = EditorState::new();
        editor.set_content("éx");
        editor.cursor = 1; // inside 'é'

        editor.insert_char('!');

        assert_eq!(editor.content(), "!éx");
        assert!(editor.content().is_char_boundary(editor.cursor));
    }

    #[test]
    fn cursor_screen_position_handles_tiny_area_without_underflow() {
        let mut editor = EditorState::new();
        editor.set_content("abc");
        editor.cursor = usize::MAX;

        let (x, y) = editor.cursor_screen_position(Rect::new(5, 7, 0, 0));
        assert_eq!((x, y), (5, 7));

        let (x, y) = editor.cursor_screen_position(Rect::new(5, 7, 1, 1));
        assert_eq!((x, y), (5, 7));
    }

    #[test]
    fn abbreviate_home_prefers_tilde() {
        assert_eq!(abbreviate_home("/Users/asher/tower/imp"), "~/tower/imp");
        assert_eq!(abbreviate_home("/tmp/project"), "/tmp/project");
    }

    #[test]
    fn identity_label_prefers_tilde_path() {
        let rendered = build_identity_label("/Users/asher/tower/imp", "chat", 80);
        let text: String = rendered
            .into_iter()
            .map(|span| span.content.into_owned())
            .collect();
        assert!(text.contains("~/tower/imp"));
        assert!(text.contains("chat"));
    }

    #[test]
    fn bottom_left_label_uses_live_run_state() {
        let rendered = build_bottom_left_label(
            AnimationState::ExecutingTools { active_tools: 2 },
            0,
            AnimationLevel::Minimal,
        );
        let text: String = rendered
            .into_iter()
            .map(|span| span.content.into_owned())
            .collect();
        assert!(text.contains("working"));
    }

    #[test]
    fn top_right_label_renders_elapsed() {
        let theme = Theme::default();
        let rendered = build_top_right_label(Some(Duration::from_secs(75)), &theme);
        let text: String = rendered
            .into_iter()
            .map(|span| span.content.into_owned())
            .collect();
        assert!(text.contains("1m15s"));
    }

    #[test]
    fn bottom_left_label_hides_thinking_state() {
        let rendered =
            build_bottom_left_label(AnimationState::Thinking, 0, AnimationLevel::Minimal);
        assert!(rendered.is_empty());
    }

    #[test]
    fn superbar_border_color_stays_base_when_idle() {
        let theme = Theme::default();
        let color = superbar_border_color(
            &theme,
            ThinkingLevel::Medium,
            AnimationState::Idle,
            0,
            AnimationLevel::Spinner,
        );
        assert_eq!(color, theme.thinking_border_color(ThinkingLevel::Medium));
    }

    #[test]
    fn superbar_border_color_stays_base_when_thinking() {
        let theme = Theme::default();
        let color = superbar_border_color(
            &theme,
            ThinkingLevel::Medium,
            AnimationState::Thinking,
            6,
            AnimationLevel::Spinner,
        );
        assert_eq!(color, theme.thinking_border_color(ThinkingLevel::Medium));
    }

    #[test]
    fn superbar_border_color_pulses_when_active() {
        let theme = Theme::default();
        let early = superbar_border_color(
            &theme,
            ThinkingLevel::Medium,
            AnimationState::ExecutingTools { active_tools: 1 },
            0,
            AnimationLevel::Spinner,
        );
        let peak = superbar_border_color(
            &theme,
            ThinkingLevel::Medium,
            AnimationState::ExecutingTools { active_tools: 1 },
            6,
            AnimationLevel::Spinner,
        );
        assert_ne!(early, peak);
    }
}
