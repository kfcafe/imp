use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// A tool call ready for display.
#[derive(Debug, Clone)]
pub struct DisplayToolCall {
    pub id: String,
    pub name: String,
    pub args_summary: String,
    pub output: Option<String>,
    pub is_error: bool,
    pub expanded: bool,
}

impl DisplayToolCall {
    /// Build a compact one-line summary for the tool call header.
    pub fn header_line(&self, theme: &Theme) -> Line<'static> {
        let icon = if self.is_error { "✗" } else { "▶" };
        let icon_style = if self.is_error {
            theme.error_style()
        } else {
            Style::default().fg(theme.tool_name)
        };

        let mut spans = vec![
            Span::styled(format!(" {icon} "), icon_style),
            Span::styled(
                self.name.clone(),
                Style::default()
                    .fg(theme.tool_name)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        if !self.args_summary.is_empty() {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                self.args_summary.clone(),
                Style::default().fg(Color::DarkGray),
            ));
        }

        // Result summary when collapsed
        if !self.expanded {
            if let Some(ref output) = self.output {
                let line_count = output.lines().count();
                let status = if self.is_error {
                    " ✗ Error".to_string()
                } else {
                    format!(" ✓ {line_count} lines")
                };
                let status_style = if self.is_error {
                    theme.error_style()
                } else {
                    theme.success_style()
                };
                spans.push(Span::styled(status, status_style));
            }
        }

        Line::from(spans)
    }

    /// Build a compact args summary from tool name and arguments.
    pub fn make_args_summary(name: &str, args: &serde_json::Value) -> String {
        match name {
            "read" => args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            "bash" => {
                let cmd = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let truncated = if cmd.len() > 60 {
                    format!("{}…", &cmd[..57])
                } else {
                    cmd.to_string()
                };
                format!("$ {truncated}")
            }
            "edit" | "write" => args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            "grep" => {
                let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                format!("\"{pattern}\" {path}")
            }
            "find" => {
                let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
                pattern.to_string()
            }
            "ls" => args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or(".")
                .to_string(),
            _ => {
                let json = serde_json::to_string(args).unwrap_or_default();
                if json.len() > 60 {
                    format!("{}…", &json[..57])
                } else {
                    json
                }
            }
        }
    }
}

/// Renders a single tool call (header + optionally expanded output).
pub struct ToolCallView<'a> {
    tool_call: &'a DisplayToolCall,
    theme: &'a Theme,
}

impl<'a> ToolCallView<'a> {
    pub fn new(tool_call: &'a DisplayToolCall, theme: &'a Theme) -> Self {
        Self { tool_call, theme }
    }
}

impl Widget for ToolCallView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        // Render header line
        let header = self.tool_call.header_line(self.theme);
        buf.set_line(area.x, area.y, &header, area.width);

        // Render expanded output
        if self.tool_call.expanded {
            if let Some(ref output) = self.tool_call.output {
                let output_style = if self.tool_call.is_error {
                    self.theme.error_style()
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                for (i, line_str) in output.lines().enumerate() {
                    let y = area.y + 1 + i as u16;
                    if y >= area.y + area.height {
                        break;
                    }
                    let line = Line::from(Span::styled(
                        format!("    {line_str}"),
                        output_style,
                    ));
                    buf.set_line(area.x, y, &line, area.width);
                }
            }
        }
    }
}

/// Calculate the rendered height of a tool call.
pub fn tool_call_height(tc: &DisplayToolCall) -> u16 {
    let mut h: u16 = 1; // header
    if tc.expanded {
        if let Some(ref output) = tc.output {
            h += output.lines().count().min(50) as u16; // cap at 50 lines
        }
    }
    h
}
