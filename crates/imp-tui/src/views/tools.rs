use imp_core::config::AnimationLevel;
use imp_llm::truncate_chars_with_suffix;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::animation::spinner_frame;
use crate::theme::Theme;

/// A tool call ready for display.
#[derive(Debug, Clone)]
pub struct DisplayToolCall {
    pub id: String,
    pub name: String,
    pub args_summary: String,
    pub output: Option<String>,
    pub details: serde_json::Value,
    pub is_error: bool,
    pub expanded: bool,
    /// Rolling buffer of streaming output lines (last 5)
    pub streaming_lines: Vec<String>,
}

impl DisplayToolCall {
    /// Build a compact one-line summary for the tool call header.
    pub fn header_line(&self, theme: &Theme) -> Line<'static> {
        self.header_line_animated(theme, 0, AnimationLevel::Minimal)
    }

    /// Header with animated spinner for running tools.
    pub fn header_line_animated(
        &self,
        theme: &Theme,
        tick: u64,
        animation_level: AnimationLevel,
    ) -> Line<'static> {
        self.header_line_animated_focused(theme, tick, false, animation_level)
    }

    /// Header with animated spinner and optional focus indicator.
    pub fn header_line_animated_focused(
        &self,
        theme: &Theme,
        tick: u64,
        focused: bool,
        animation_level: AnimationLevel,
    ) -> Line<'static> {
        let is_running = self.output.is_none() && !self.is_error;
        let icon = if self.is_error {
            "✗".to_string()
        } else if is_running {
            match animation_level {
                AnimationLevel::None => "•".to_string(),
                AnimationLevel::Spinner | AnimationLevel::Minimal => {
                    spinner_frame(tick).to_string()
                }
            }
        } else {
            "✓".to_string()
        };
        let icon_style = if self.is_error {
            theme.error_style()
        } else if is_running {
            Style::default().fg(theme.accent)
        } else {
            theme.success_style()
        };

        // Focus indicator prepended before the status icon
        let focus_span = if focused {
            Span::styled(
                "▸",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(" ")
        };

        let mut spans = vec![
            focus_span,
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
            spans.push(Span::styled(self.args_summary.clone(), theme.muted_style()));
        }

        // Result summary when collapsed — just line count (icon already shows status)
        if !self.expanded {
            if let Some(ref output) = self.output {
                if self.is_error {
                    spans.push(Span::styled(" error", theme.error_style()));
                } else {
                    let line_count = output.lines().count();
                    spans.push(Span::styled(
                        format!("  {line_count} lines"),
                        theme.muted_style(),
                    ));
                }
            }
        }

        Line::from(spans)
    }

    /// Build compact inline spans for multi-tool-per-line rendering: "✓ name args"
    pub fn compact_spans(&self, theme: &Theme) -> Vec<Span<'static>> {
        let icon_style = theme.success_style();
        let args_short = short_args(&self.args_summary);
        let mut spans = vec![
            Span::styled("✓ ", icon_style),
            Span::styled(
                self.name.clone(),
                Style::default()
                    .fg(theme.tool_name)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        if !args_short.is_empty() {
            spans.push(Span::styled(format!(" {args_short}"), theme.muted_style()));
        }
        spans
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
                let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let truncated = truncate_chars_with_suffix(cmd, 57, "…");
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
                    truncate_chars_with_suffix(&json, 57, "…")
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
                    self.theme.muted_style()
                };

                for (i, line_str) in output.lines().enumerate() {
                    let y = area.y + 1 + i as u16;
                    if y >= area.y + area.height {
                        break;
                    }
                    let line = Line::from(Span::styled(format!("    {line_str}"), output_style));
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

/// Check whether a tool call can be rendered in compact (inline) mode.
/// Compactable = completed successfully, not expanded, not an error.
pub fn is_compactable(tc: &DisplayToolCall) -> bool {
    tc.output.is_some() && !tc.is_error && !tc.expanded
}

/// Calculate the rendered height of a slice of tool calls using compact grouping.
/// Consecutive compactable calls share lines; others get their own full-height row.
pub fn tool_calls_compact_height(tcs: &[DisplayToolCall], width: u16) -> u16 {
    let mut h: u16 = 0;
    let mut i = 0;
    while i < tcs.len() {
        let tc = &tcs[i];
        if is_compactable(tc) {
            let group_start = i;
            while i < tcs.len() && is_compactable(&tcs[i]) {
                i += 1;
            }
            h += compact_group_line_count(&tcs[group_start..i], width);
        } else {
            h += tool_call_height(tc);
            i += 1;
        }
    }
    h
}

/// Calculate how many lines a group of compact tool calls takes.
/// Each call renders as "✓ name args" and we pack as many as fit per line.
fn compact_group_line_count(tcs: &[DisplayToolCall], width: u16) -> u16 {
    if tcs.is_empty() {
        return 0;
    }
    let usable = (width as usize).saturating_sub(4); // rail = 4 chars
    if usable == 0 {
        return tcs.len() as u16;
    }
    let mut lines: u16 = 1;
    let mut col: usize = 0;
    for tc in tcs {
        let span_len = compact_span_width(tc);
        if col > 0 && col + 2 + span_len > usable {
            lines += 1;
            col = span_len;
        } else if col > 0 {
            col += 2 + span_len; // 2 for "  " separator
        } else {
            col = span_len;
        }
    }
    lines
}

/// Width of a compact tool call span: "✓ name args" character count.
fn compact_span_width(tc: &DisplayToolCall) -> usize {
    let args_short = short_args(&tc.args_summary);
    let w = 2 + tc.name.len(); // "✓ name"
    if args_short.is_empty() {
        w
    } else {
        w + 1 + args_short.len()
    }
}

/// Shorten args_summary for compact display (just the filename or first word).
fn short_args(args: &str) -> String {
    if args.is_empty() {
        return String::new();
    }
    // For paths, show just the filename
    if args.contains('/') {
        if let Some(name) = args.rsplit('/').next() {
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    // For "$ command" bash summaries, take first 20 chars
    if let Some(cmd) = args.strip_prefix("$ ") {
        let short = if cmd.len() > 20 {
            format!("$ {}", truncate_chars_with_suffix(cmd, 17, "…"))
        } else {
            format!("$ {cmd}")
        };
        return short;
    }
    // For quoted grep patterns, keep as-is if short
    if args.len() <= 24 {
        return args.to_string();
    }
    truncate_chars_with_suffix(args, 21, "…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tc(name: &str, args: &str, output: Option<&str>, is_error: bool) -> DisplayToolCall {
        DisplayToolCall {
            id: "test".into(),
            name: name.into(),
            args_summary: args.into(),
            output: output.map(String::from),
            details: serde_json::Value::Null,
            is_error,
            expanded: false,
            streaming_lines: Vec::new(),
        }
    }

    #[test]
    fn compactable_completed_success() {
        let tc = make_tc("read", "file.rs", Some("contents"), false);
        assert!(is_compactable(&tc));
    }

    #[test]
    fn not_compactable_running() {
        let tc = make_tc("read", "file.rs", None, false);
        assert!(!is_compactable(&tc));
    }

    #[test]
    fn not_compactable_error() {
        let tc = make_tc("read", "file.rs", Some("err"), true);
        assert!(!is_compactable(&tc));
    }

    #[test]
    fn not_compactable_expanded() {
        let mut tc = make_tc("read", "file.rs", Some("data"), false);
        tc.expanded = true;
        assert!(!is_compactable(&tc));
    }

    #[test]
    fn short_args_path() {
        assert_eq!(short_args("src/views/tools.rs"), "tools.rs");
    }

    #[test]
    fn short_args_bash() {
        // "cargo test -p imp-tui" is 21 chars, > 20, so truncated to 17 + "…"
        assert_eq!(
            short_args("$ cargo test -p imp-tui"),
            "$ cargo test -p imp…"
        );
    }

    #[test]
    fn short_args_bash_short() {
        assert_eq!(short_args("$ ls -la"), "$ ls -la");
    }

    #[test]
    fn short_args_empty() {
        assert_eq!(short_args(""), "");
    }

    #[test]
    fn short_args_short_text() {
        assert_eq!(short_args("pattern"), "pattern");
    }

    #[test]
    fn compact_group_fits_one_line() {
        let tcs = vec![
            make_tc("read", "file.rs", Some("ok"), false),
            make_tc("grep", "pat", Some("ok"), false),
        ];
        assert_eq!(compact_group_line_count(&tcs, 80), 1);
    }

    #[test]
    fn compact_group_wraps() {
        let tcs: Vec<_> = (0..10)
            .map(|i| {
                make_tc(
                    "read",
                    &format!("long/path/to/file_{i}.rs"),
                    Some("ok"),
                    false,
                )
            })
            .collect();
        let lines = compact_group_line_count(&tcs, 80);
        assert!(lines > 1);
        assert!(lines < 10);
    }

    #[test]
    fn compact_height_mixed() {
        let tcs = vec![
            make_tc("read", "a.rs", Some("ok"), false),
            make_tc("read", "b.rs", Some("ok"), false),
            make_tc("bash", "$ cmd", None, false), // running
            make_tc("read", "c.rs", Some("ok"), false),
        ];
        let h = tool_calls_compact_height(&tcs, 80);
        // First 2 compact (1 line) + 1 running (1 line) + 1 compact (1 line) = 3
        assert_eq!(h, 3);
    }

    #[test]
    fn compact_height_all_compactable() {
        let tcs = vec![
            make_tc("read", "a.rs", Some("ok"), false),
            make_tc("grep", "pat", Some("ok"), false),
            make_tc("edit", "b.rs", Some("ok"), false),
        ];
        let h = tool_calls_compact_height(&tcs, 80);
        assert_eq!(h, 1);
    }
}
