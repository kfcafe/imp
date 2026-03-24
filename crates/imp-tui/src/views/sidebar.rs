use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;
use crate::views::tools::DisplayToolCall;

/// Sidebar state tracked in App.
#[derive(Default)]
pub struct Sidebar {
    /// Whether the sidebar pane is visible.
    pub open: bool,
    /// Which tool call is currently displayed.
    pub tool_id: Option<String>,
    /// Vertical scroll offset within the content.
    pub scroll: usize,
    /// Whether the first tool has been seen (for auto-open logic).
    pub first_tool_seen: bool,
}

impl Sidebar {
    /// Switch to displaying a new tool call, resetting scroll.
    pub fn follow(&mut self, tool_call_id: &str) {
        self.tool_id = Some(tool_call_id.to_string());
        self.scroll = 0;
    }

    /// Scroll up by `n` lines.
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll += n;
    }

    /// Scroll down by `n` lines, clamping to zero.
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }
}

/// Widget that renders the sidebar tool preview pane.
///
/// Shows a vertical separator on the left, a header line with tool name
/// and status, and a scrollable content area with the tool's output.
pub struct SidebarView<'a> {
    tool_call: Option<&'a DisplayToolCall>,
    theme: &'a Theme,
    tick: u64,
    scroll: usize,
}

impl<'a> SidebarView<'a> {
    pub fn new(
        tool_call: Option<&'a DisplayToolCall>,
        theme: &'a Theme,
        tick: u64,
        scroll: usize,
    ) -> Self {
        Self {
            tool_call,
            theme,
            tick,
            scroll,
        }
    }

    /// Whether the displayed tool is finished (idle).
    fn is_idle(&self) -> bool {
        self.tool_call.map(|tc| tc.output.is_some()).unwrap_or(true)
    }
}

impl Widget for SidebarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 3 || area.height < 2 {
            return;
        }

        // Draw left border separator
        let border_style = self.theme.border_style();
        for y in area.y..area.y + area.height {
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_symbol("│");
                cell.set_style(border_style);
            }
        }

        // Content area starts after the border + 1 space
        let content_x = area.x + 2;
        let content_width = area.width.saturating_sub(2);
        if content_width == 0 {
            return;
        }

        let Some(tc) = self.tool_call else {
            let msg = " No tool output";
            if area.height > 0 {
                let line = Line::from(Span::styled(msg, self.theme.muted_style()));
                buf.set_line(content_x, area.y, &line, content_width);
            }
            return;
        };

        let idle = self.is_idle();

        // Header: icon + tool name + status
        render_header(
            tc,
            idle,
            self.theme,
            self.tick,
            content_x,
            area.y,
            content_width,
            buf,
        );

        // Separator under header
        if area.height > 1 {
            let sep: String = "─".repeat(content_width as usize);
            let line = Line::from(Span::styled(sep, self.theme.border_style()));
            buf.set_line(content_x, area.y + 1, &line, content_width);
        }

        // Scrollable content below separator
        if area.height > 2 {
            let output_y = area.y + 2;
            let output_height = area.height - 2;
            render_content(
                tc,
                idle,
                self.theme,
                self.scroll,
                content_x,
                output_y,
                content_width,
                output_height,
                buf,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_header(
    tc: &DisplayToolCall,
    idle: bool,
    theme: &Theme,
    tick: u64,
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
) {
    let is_running = tc.output.is_none() && !tc.is_error;

    let icon = if tc.is_error {
        "✗"
    } else if is_running {
        const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        SPINNER[(tick / 2) as usize % SPINNER.len()]
    } else {
        "✓"
    };

    let icon_style = if tc.is_error {
        theme.error_style()
    } else if is_running {
        Style::default().fg(theme.accent)
    } else if idle {
        theme.muted_style()
    } else {
        theme.success_style()
    };

    let name_style = if idle {
        theme.muted_style()
    } else {
        Style::default().fg(theme.tool_name)
    };

    let mut spans = vec![
        Span::styled(format!("{icon} "), icon_style),
        Span::styled(tc.name.clone(), name_style),
    ];

    if !tc.args_summary.is_empty() {
        spans.push(Span::styled(
            format!(" {}", tc.args_summary),
            theme.muted_style(),
        ));
    }

    let line = Line::from(spans);
    buf.set_line(x, y, &line, width);
}

#[allow(clippy::too_many_arguments)]
fn render_content(
    tc: &DisplayToolCall,
    idle: bool,
    theme: &Theme,
    scroll: usize,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    buf: &mut Buffer,
) {
    let lines = content_lines(tc);
    let visible = height as usize;
    let total = lines.len();
    let start = scroll.min(total.saturating_sub(visible));

    let style = if idle {
        theme.muted_style()
    } else if tc.is_error {
        theme.error_style()
    } else {
        Style::default().fg(theme.fg)
    };

    for (i, line_text) in lines.iter().skip(start).take(visible).enumerate() {
        let row = y + i as u16;
        let truncated = truncate_line(line_text, width as usize);
        let line = Line::from(Span::styled(truncated, style));
        buf.set_line(x, row, &line, width);
    }

    // Scroll percentage at bottom-right
    if total > visible && height > 0 {
        let pct = ((start + visible) * 100) / total;
        let indicator = format!(" {pct}% ");
        let ind_width = indicator.len() as u16;
        if width > ind_width {
            let ind_x = x + width - ind_width;
            let ind_y = y + height - 1;
            let line = Line::from(Span::styled(indicator, theme.muted_style()));
            buf.set_line(ind_x, ind_y, &line, ind_width);
        }
    }
}

/// Build display lines from tool call state.
fn content_lines(tc: &DisplayToolCall) -> Vec<String> {
    if let Some(ref output) = tc.output {
        return format_output(&tc.name, output);
    }
    if !tc.streaming_lines.is_empty() {
        return tc.streaming_lines.clone();
    }
    vec!["Running…".to_string()]
}

/// Format tool output based on tool type.
fn format_output(tool_name: &str, output: &str) -> Vec<String> {
    match tool_name {
        "read" => output
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{:>4} │ {}", i + 1, line))
            .collect(),
        "edit" | "multi_edit" => output
            .lines()
            .map(|line| {
                if line.starts_with('+') {
                    format!("+ {}", line.get(1..).unwrap_or(""))
                } else if line.starts_with('-') {
                    format!("- {}", line.get(1..).unwrap_or(""))
                } else {
                    line.to_string()
                }
            })
            .collect(),
        _ => output.lines().map(String::from).collect(),
    }
}

/// Truncate a line to fit within `max_width` characters.
fn truncate_line(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    // Walk chars to find a safe truncation point
    let truncated: String = s.chars().take(max_width - 1).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_default_state() {
        let sidebar = Sidebar::default();
        assert!(!sidebar.open);
        assert!(sidebar.tool_id.is_none());
        assert_eq!(sidebar.scroll, 0);
        assert!(!sidebar.first_tool_seen);
    }

    #[test]
    fn sidebar_follow_sets_id_and_resets_scroll() {
        let mut sidebar = Sidebar::default();
        sidebar.scroll = 42;
        sidebar.follow("tc-123");
        assert_eq!(sidebar.tool_id.as_deref(), Some("tc-123"));
        assert_eq!(sidebar.scroll, 0);
    }

    #[test]
    fn sidebar_scroll_up_down() {
        let mut sidebar = Sidebar::default();
        sidebar.scroll_up(5);
        assert_eq!(sidebar.scroll, 5);
        sidebar.scroll_down(3);
        assert_eq!(sidebar.scroll, 2);
        sidebar.scroll_down(10);
        assert_eq!(sidebar.scroll, 0);
    }

    #[test]
    fn sidebar_view_no_tool_renders() {
        let theme = Theme::default();
        let view = SidebarView::new(None, &theme, 0, 0);
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_with_completed_tool() {
        let theme = Theme::default();
        let tc = DisplayToolCall {
            id: "tc1".into(),
            name: "bash".into(),
            args_summary: "$ ls -la".into(),
            output: Some("file1\nfile2\nfile3".into()),
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
        };
        let view = SidebarView::new(Some(&tc), &theme, 0, 0);
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_with_streaming_tool() {
        let theme = Theme::default();
        let tc = DisplayToolCall {
            id: "tc1".into(),
            name: "bash".into(),
            args_summary: "$ make".into(),
            output: None,
            is_error: false,
            expanded: false,
            streaming_lines: vec!["compiling...".into(), "linking...".into()],
        };
        let view = SidebarView::new(Some(&tc), &theme, 10, 0);
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_with_error_tool() {
        let theme = Theme::default();
        let tc = DisplayToolCall {
            id: "tc1".into(),
            name: "bash".into(),
            args_summary: "$ false".into(),
            output: Some("command failed".into()),
            is_error: true,
            expanded: false,
            streaming_lines: Vec::new(),
        };
        let view = SidebarView::new(Some(&tc), &theme, 0, 0);
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_tiny_area_no_panic() {
        let theme = Theme::default();
        let tc = DisplayToolCall {
            id: "tc1".into(),
            name: "read".into(),
            args_summary: "file.rs".into(),
            output: Some("hello\nworld".into()),
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
        };
        let view = SidebarView::new(Some(&tc), &theme, 0, 0);
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);

        let view2 = SidebarView::new(Some(&tc), &theme, 0, 0);
        let area2 = Rect::new(0, 0, 0, 0);
        let mut buf2 = Buffer::empty(area2);
        view2.render(area2, &mut buf2);
    }

    #[test]
    fn format_output_read_has_line_numbers() {
        let lines = format_output("read", "fn main() {\n    println!(\"hi\");\n}");
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("1 │"));
        assert!(lines[0].contains("fn main()"));
        assert!(lines[2].contains("3 │"));
    }

    #[test]
    fn format_output_bash_is_raw() {
        let lines = format_output("bash", "total 42\ndrwxr-xr-x dir");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "total 42");
    }

    #[test]
    fn format_output_grep_is_raw() {
        let lines = format_output("grep", "file.rs:10:match found");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "file.rs:10:match found");
    }

    #[test]
    fn format_output_edit_marks_diffs() {
        let lines = format_output("edit", "+added\n-removed\nunchanged");
        assert_eq!(lines[0], "+ added");
        assert_eq!(lines[1], "- removed");
        assert_eq!(lines[2], "unchanged");
    }

    #[test]
    fn content_lines_running() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: None,
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
        };
        let lines = content_lines(&tc);
        assert_eq!(lines, vec!["Running…"]);
    }

    #[test]
    fn content_lines_streaming() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: None,
            is_error: false,
            expanded: false,
            streaming_lines: vec!["line1".into(), "line2".into()],
        };
        let lines = content_lines(&tc);
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn content_lines_completed_ignores_streaming() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: Some("done\nok".into()),
            is_error: false,
            expanded: false,
            streaming_lines: vec!["old stream".into()],
        };
        let lines = content_lines(&tc);
        assert_eq!(lines, vec!["done", "ok"]);
    }

    #[test]
    fn truncate_line_short() {
        assert_eq!(truncate_line("hello", 10), "hello");
    }

    #[test]
    fn truncate_line_exact() {
        assert_eq!(truncate_line("hello", 5), "hello");
    }

    #[test]
    fn truncate_line_overflow() {
        assert_eq!(truncate_line("hello world", 6), "hello…");
    }

    #[test]
    fn truncate_line_width_one() {
        assert_eq!(truncate_line("hello", 1), "…");
    }

    #[test]
    fn truncate_line_multibyte_utf8() {
        // 日本語 = 3 chars, 9 bytes. Truncating at byte 5 would split a char.
        let result = truncate_line("日本語テスト", 4);
        assert!(result.ends_with('…'));
        // Should not panic
        assert!(!result.is_empty());
    }
}
