use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::highlight::Highlighter;
use crate::markdown;
use crate::theme::Theme;
use crate::views::tools::{tool_call_height, DisplayToolCall};

/// Role of a display message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Compaction,
    Error,
}

/// A message formatted for display in the chat view.
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: MessageRole,
    pub content: String,
    pub thinking: Option<String>,
    pub tool_calls: Vec<DisplayToolCall>,
    pub is_streaming: bool,
    pub timestamp: u64,
}

impl DisplayMessage {
    /// Construct from an imp_llm Message.
    pub fn from_message(msg: &imp_llm::Message) -> Self {
        match msg {
            imp_llm::Message::User(u) => {
                let text = u
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                Self {
                    role: MessageRole::User,
                    content: text,
                    thinking: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    timestamp: u.timestamp,
                }
            }
            imp_llm::Message::Assistant(a) => {
                let mut text = String::new();
                let mut thinking = None;
                let mut tool_calls = Vec::new();
                for block in &a.content {
                    match block {
                        imp_llm::ContentBlock::Text { text: t } => text.push_str(t),
                        imp_llm::ContentBlock::Thinking { text: t } => {
                            thinking = Some(t.clone());
                        }
                        imp_llm::ContentBlock::ToolCall {
                            id,
                            name,
                            arguments,
                        } => {
                            tool_calls.push(DisplayToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                args_summary: DisplayToolCall::make_args_summary(name, arguments),
                                output: None,
                                is_error: false,
                                expanded: false,
                                streaming_lines: Vec::new(),
                            });
                        }
                        _ => {}
                    }
                }
                Self {
                    role: MessageRole::Assistant,
                    content: text,
                    thinking,
                    tool_calls,
                    is_streaming: false,
                    timestamp: a.timestamp,
                }
            }
            imp_llm::Message::ToolResult(t) => {
                let text = t
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                Self {
                    role: if t.is_error {
                        MessageRole::Error
                    } else {
                        MessageRole::System
                    },
                    content: text,
                    thinking: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    timestamp: t.timestamp,
                }
            }
        }
    }

    /// Calculate the rendered line count for this message.
    pub fn line_count(&self, theme: &Theme, highlighter: &Highlighter) -> usize {
        let mut count = 0;

        // Prefix line
        count += 1;

        // Content lines (markdown renders to lines)
        if !self.content.is_empty() {
            match self.role {
                MessageRole::Assistant => {
                    count += markdown::render_markdown(&self.content, theme, highlighter).len();
                }
                _ => {
                    count += self.content.lines().count().max(1);
                }
            }
        }

        // Thinking block
        if self.thinking.is_some() {
            count += 1; // header
        }

        // Tool calls
        for tc in &self.tool_calls {
            count += tool_call_height(tc) as usize;
        }

        // Separator
        count += 1;
        count
    }
}

/// Chat view: displays conversation messages with streaming support.
pub struct ChatView<'a> {
    messages: &'a [DisplayMessage],
    theme: &'a Theme,
    highlighter: &'a Highlighter,
    scroll_offset: usize,
    tick: u64,
    /// Flat index of the focused tool call across all messages, if any.
    tool_focus: Option<usize>,
}

impl<'a> ChatView<'a> {
    pub fn new(
        messages: &'a [DisplayMessage],
        theme: &'a Theme,
        highlighter: &'a Highlighter,
    ) -> Self {
        Self {
            messages,
            theme,
            highlighter,
            scroll_offset: 0,
            tick: 0,
            tool_focus: None,
        }
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn tick(mut self, tick: u64) -> Self {
        self.tick = tick;
        self
    }

    pub fn tool_focus(mut self, focus: Option<usize>) -> Self {
        self.tool_focus = focus;
        self
    }
}

impl Widget for ChatView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Build all lines from all messages
        let mut all_lines: Vec<Line<'_>> = Vec::new();
        // Running counter of tool calls seen so far — used to map flat_idx → focused
        let mut tool_call_counter: usize = 0;

        for msg in self.messages {
            match msg.role {
                MessageRole::User => {
                    all_lines.push(Line::from(Span::styled(
                        "You:",
                        Style::default()
                            .fg(self.theme.user_prefix)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for content_line in msg.content.lines() {
                        all_lines.push(Line::from(Span::raw(format!("  {content_line}"))));
                    }
                    if msg.content.is_empty() {
                        all_lines.push(Line::raw(""));
                    }
                }
                MessageRole::Assistant => {
                    all_lines.push(Line::from(Span::styled(
                        "Assistant:",
                        Style::default()
                            .fg(self.theme.accent)
                            .add_modifier(Modifier::BOLD),
                    )));

                    // Thinking: rolling 5-line tail (live stream of latest thought)
                    if let Some(ref thinking) = msg.thinking {
                        if !thinking.is_empty() {
                            let lines: Vec<&str> = thinking.lines().collect();
                            let total = lines.len();
                            let tail = if total > 5 {
                                &lines[total - 5..]
                            } else {
                                &lines[..]
                            };
                            for (i, line) in tail.iter().enumerate() {
                                let prefix = if i == 0 && tail.len() == 5 {
                                    "💭"
                                } else {
                                    "  "
                                };
                                all_lines.push(Line::from(Span::styled(
                                    format!("  {prefix} {line}"),
                                    self.theme.muted_style(),
                                )));
                            }
                        }
                    }

                    // Content with markdown rendering
                    if !msg.content.is_empty() {
                        let rendered =
                            markdown::render_markdown(&msg.content, self.theme, self.highlighter);
                        for line in rendered {
                            // Indent assistant content
                            let mut spans = vec![Span::raw("  ")];
                            spans.extend(line.spans);
                            all_lines.push(Line::from(spans));
                        }
                    }

                    // Streaming indicator with phase context
                    if msg.is_streaming {
                        const SPINNER: &[&str] =
                            &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                        let frame = SPINNER[(self.tick / 2) as usize % SPINNER.len()];
                        let has_running_tool = msg
                            .tool_calls
                            .iter()
                            .any(|tc| tc.output.is_none() && !tc.is_error);
                        let label = if has_running_tool {
                            ""
                        } else if msg.content.is_empty() && !msg.tool_calls.is_empty() {
                            " thinking…"
                        } else {
                            ""
                        };
                        all_lines.push(Line::from(Span::styled(
                            format!("  {frame}{label}"),
                            self.theme.accent_style(),
                        )));
                    }
                }
                MessageRole::System => {
                    // Tool results are typically not shown as top-level messages
                    // (they are attached to tool calls). But if standalone:
                    for line in msg.content.lines().take(3) {
                        all_lines.push(Line::from(Span::styled(
                            format!("  {line}"),
                            self.theme.muted_style(),
                        )));
                    }
                }
                MessageRole::Compaction => {
                    all_lines.push(Line::from(Span::styled(
                        format!("  [{}]", msg.content),
                        self.theme.muted_style(),
                    )));
                }
                MessageRole::Error => {
                    all_lines.push(Line::from(Span::styled(
                        format!("Error: {}", msg.content),
                        self.theme.error_style(),
                    )));
                }
            }

            // Tool calls — rendered with a │ left rail for visual grouping
            for tc in &msg.tool_calls {
                let is_running = tc.output.is_none() && !tc.is_error;
                let focused = self.tool_focus == Some(tool_call_counter);
                tool_call_counter += 1;

                let rail = Span::styled("  │", self.theme.muted_style());

                // Build header with rail prepended; use focused variant for ▸ indicator
                let mut header = tc.header_line_animated_focused(self.theme, self.tick, focused);
                header.spans.insert(0, rail.clone());

                if is_running && !tc.streaming_lines.is_empty() {
                    // Running: show header + rolling streaming output tail
                    all_lines.push(header);
                    for line in &tc.streaming_lines {
                        all_lines.push(Line::from(vec![
                            rail.clone(),
                            Span::styled(format!("    {line}"), self.theme.muted_style()),
                        ]));
                    }
                } else {
                    // Running (no output yet) or done: just the header line
                    all_lines.push(header);
                }

                // Expanded output (Tab peek or auto-expanded errors) — also railed
                if tc.expanded && !is_running {
                    if let Some(ref output) = tc.output {
                        let output_style = if tc.is_error {
                            self.theme.error_style()
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        for output_line in output.lines().take(50) {
                            all_lines.push(Line::from(vec![
                                rail.clone(),
                                Span::styled(format!("    {output_line}"), output_style),
                            ]));
                        }
                    }
                }
            }

            // Separator
            all_lines.push(Line::raw(""));
        }

        // Apply scroll offset: skip lines from the top
        let total_lines = all_lines.len();
        let visible_height = area.height as usize;

        let start = if self.scroll_offset == 0 {
            // Auto-scroll: show the last N lines
            total_lines.saturating_sub(visible_height)
        } else {
            total_lines.saturating_sub(visible_height + self.scroll_offset)
        };

        let visible = &all_lines[start..total_lines.min(start + visible_height)];

        for (i, line) in visible.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            buf.set_line(area.x, y, line, area.width);
        }
    }
}

/// Calculate the total number of rendered lines across all messages.
pub fn total_rendered_lines(
    messages: &[DisplayMessage],
    theme: &Theme,
    highlighter: &Highlighter,
) -> usize {
    messages
        .iter()
        .map(|m| m.line_count(theme, highlighter))
        .sum()
}
