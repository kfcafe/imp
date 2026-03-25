use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;
use crate::views::tools::DisplayToolCall;

// ── Sidebar state ───────────────────────────────────────────────

/// Sidebar state tracked in App.
#[derive(Default)]
pub struct Sidebar {
    /// Whether the sidebar pane is visible.
    pub open: bool,
    /// Scroll offset for the tool list pane (0 = top).
    pub list_scroll: usize,
    /// Scroll offset for the detail pane (0 = top).
    pub detail_scroll: usize,
    /// Whether the first tool has been seen (for auto-open logic).
    pub first_tool_seen: bool,
    /// Cached list pane height from last render (for scroll bounds).
    pub list_height: u16,
}

impl Sidebar {
    /// Reset detail scroll (call when selection changes).
    pub fn reset_detail_scroll(&mut self) {
        self.detail_scroll = 0;
    }

    /// Scroll the tool list up (toward earlier entries).
    pub fn scroll_list_up(&mut self, n: usize) {
        self.list_scroll = self.list_scroll.saturating_sub(n);
    }

    /// Scroll the tool list down (toward later entries).
    pub fn scroll_list_down(&mut self, n: usize) {
        self.list_scroll += n;
    }

    /// Scroll the detail pane up (toward earlier content).
    pub fn scroll_detail_up(&mut self, n: usize) {
        self.detail_scroll = self.detail_scroll.saturating_sub(n);
    }

    /// Scroll the detail pane down (toward later content).
    pub fn scroll_detail_down(&mut self, n: usize) {
        self.detail_scroll += n;
    }

    /// Ensure the selected tool call index is visible in the list.
    pub fn ensure_selected_visible(&mut self, selected: usize) {
        let visible = (self.list_height as usize).max(1);
        if selected < self.list_scroll {
            self.list_scroll = selected;
        } else if selected >= self.list_scroll + visible {
            self.list_scroll = selected.saturating_sub(visible.saturating_sub(1));
        }
    }
}

// ── Layout computation ──────────────────────────────────────────

/// Compute sidebar sub-areas for external hit-testing.
/// Takes the full sidebar `Rect` (including border) and the number of tool
/// calls.  Returns `(list_hit_rect, detail_hit_rect)` in screen coordinates,
/// spanning the full sidebar width so click detection is easy.
pub fn sidebar_sub_areas(sidebar_area: Rect, tool_count: usize) -> (Rect, Rect) {
    let content = Rect {
        x: sidebar_area.x + 2,
        y: sidebar_area.y,
        width: sidebar_area.width.saturating_sub(2),
        height: sidebar_area.height,
    };

    let (list_area, _, detail_area) = compute_split(content, tool_count);

    // Widen hit rects to cover the full sidebar width (including border)
    let list_hit = Rect {
        x: sidebar_area.x,
        width: sidebar_area.width,
        y: list_area.y,
        height: list_area.height,
    };
    let detail_hit = Rect {
        x: sidebar_area.x,
        width: sidebar_area.width,
        y: detail_area.y,
        height: detail_area.height,
    };

    (list_hit, detail_hit)
}

/// Internal layout: split the content area into list, separator, and detail.
/// Returns `(list_area, separator_y, detail_area)`.
fn compute_split(content: Rect, tool_count: usize) -> (Rect, Option<u16>, Rect) {
    let h = content.height as usize;
    let min_detail = 3; // header + separator + 1 content line
    let sep = 1;
    let min_total = 2 + sep + min_detail; // need at least 6 rows to split

    if h < min_total || tool_count == 0 {
        // Too small to split — give everything to list
        return (
            content,
            None,
            Rect {
                x: content.x,
                y: content.y + content.height,
                width: content.width,
                height: 0,
            },
        );
    }

    // Dynamic list height: grows with tool count, capped at 40%
    let max_list_pct = 40;
    let max_list = (h * max_list_pct / 100).max(2);
    let available_for_list = h.saturating_sub(sep + min_detail);
    let desired = tool_count.clamp(2, max_list);
    let list_h = desired.min(available_for_list).max(2);
    let detail_h = h.saturating_sub(list_h + sep);

    let list_area = Rect {
        height: list_h as u16,
        ..content
    };
    let sep_y = content.y + list_h as u16;
    let detail_area = Rect {
        y: sep_y + sep as u16,
        height: detail_h as u16,
        ..content
    };

    (list_area, Some(sep_y), detail_area)
}

// ── SidebarView widget ──────────────────────────────────────────

/// Widget that renders the split sidebar: tool list (top) + detail (bottom).
pub struct SidebarView<'a> {
    tool_calls: Vec<&'a DisplayToolCall>,
    selected: Option<usize>,
    theme: &'a Theme,
    tick: u64,
    list_scroll: usize,
    detail_scroll: usize,
}

impl<'a> SidebarView<'a> {
    pub fn new(
        tool_calls: Vec<&'a DisplayToolCall>,
        selected: Option<usize>,
        theme: &'a Theme,
        tick: u64,
        list_scroll: usize,
        detail_scroll: usize,
    ) -> Self {
        Self {
            tool_calls,
            selected,
            theme,
            tick,
            list_scroll,
            detail_scroll,
        }
    }
}

impl Widget for SidebarView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 3 || area.height < 2 {
            return;
        }

        // Left border separator
        let border_style = self.theme.border_style();
        for y in area.y..area.y + area.height {
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_symbol("│");
                cell.set_style(border_style);
            }
        }

        // Content area after the border + 1 padding column
        let cx = area.x + 2;
        let cw = area.width.saturating_sub(2);
        if cw == 0 {
            return;
        }
        let content = Rect {
            x: cx,
            y: area.y,
            width: cw,
            height: area.height,
        };

        if self.tool_calls.is_empty() {
            let line = Line::from(Span::styled("No tool calls", self.theme.muted_style()));
            buf.set_line(cx, area.y, &line, cw);
            return;
        }

        // Split into list / separator / detail
        let (list_area, sep_y, detail_area) = compute_split(content, self.tool_calls.len());

        // ── Tool list ───────────────────────────────────────────
        render_list(
            &self.tool_calls,
            self.selected,
            self.theme,
            self.tick,
            self.list_scroll,
            list_area,
            buf,
        );

        // ── Separator ───────────────────────────────────────────
        if let Some(sy) = sep_y {
            let sep: String = "─".repeat(cw as usize);
            buf.set_line(cx, sy, &Line::from(Span::styled(sep, border_style)), cw);
        }

        // ── Detail pane ─────────────────────────────────────────
        let selected_tc = self.selected.and_then(|i| self.tool_calls.get(i)).copied();
        render_detail(
            selected_tc,
            self.theme,
            self.tick,
            self.detail_scroll,
            detail_area,
            buf,
        );
    }
}

// ── Tool list rendering ─────────────────────────────────────────

fn render_list(
    tool_calls: &[&DisplayToolCall],
    selected: Option<usize>,
    theme: &Theme,
    tick: u64,
    scroll: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let visible = area.height as usize;
    let total = tool_calls.len();
    let start = scroll.min(total.saturating_sub(visible));

    for (i, tc) in tool_calls.iter().skip(start).take(visible).enumerate() {
        let idx = start + i;
        let focused = selected == Some(idx);
        let row = area.y + i as u16;
        let header = tc.header_line_animated_focused(theme, tick, focused);
        buf.set_line(area.x, row, &header, area.width);
    }

    // Scroll indicator (top-right)
    if total > visible && visible > 0 {
        let pct = ((start + visible).min(total) * 100) / total;
        let indicator = format!("{pct}%");
        let iw = indicator.len() as u16;
        if area.width > iw {
            let ix = area.x + area.width - iw;
            let iy = area.y + area.height.saturating_sub(1);
            buf.set_line(
                ix,
                iy,
                &Line::from(Span::styled(indicator, theme.muted_style())),
                iw,
            );
        }
    }
}

// ── Detail pane rendering ───────────────────────────────────────

fn render_detail(
    tc: Option<&DisplayToolCall>,
    theme: &Theme,
    tick: u64,
    scroll: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let Some(tc) = tc else {
        let line = Line::from(Span::styled("Select a tool call", theme.muted_style()));
        buf.set_line(area.x, area.y, &line, area.width);
        return;
    };

    let is_running = tc.output.is_none() && !tc.is_error;

    // Header line: icon + name + args
    render_detail_header(tc, is_running, theme, tick, area.x, area.y, area.width, buf);

    // Thin separator
    if area.height > 1 {
        let sep: String = "─".repeat(area.width as usize);
        buf.set_line(
            area.x,
            area.y + 1,
            &Line::from(Span::styled(sep, theme.border_style())),
            area.width,
        );
    }

    // Scrollable content
    if area.height > 2 {
        let content_y = area.y + 2;
        let content_h = (area.height - 2) as usize;
        let content_w = area.width as usize;

        let lines = detail_content_lines(tc, content_w);
        let total = lines.len();
        let start = scroll.min(total.saturating_sub(content_h).max(0));

        let style = if tc.is_error {
            theme.error_style()
        } else {
            Style::default().fg(theme.fg)
        };

        for (i, text) in lines.iter().skip(start).take(content_h).enumerate() {
            let row = content_y + i as u16;
            buf.set_line(
                area.x,
                row,
                &Line::from(Span::styled(text.clone(), style)),
                area.width,
            );
        }

        // Scroll percentage (bottom-right of detail)
        if total > content_h && content_h > 0 {
            let pct = ((start + content_h).min(total) * 100) / total;
            let indicator = format!(" {pct}% ");
            let iw = indicator.len() as u16;
            if area.width > iw {
                let ix = area.x + area.width - iw;
                let iy = content_y + content_h as u16 - 1;
                buf.set_line(
                    ix,
                    iy,
                    &Line::from(Span::styled(indicator, theme.muted_style())),
                    iw,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_detail_header(
    tc: &DisplayToolCall,
    is_running: bool,
    theme: &Theme,
    tick: u64,
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
) {
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
    } else {
        theme.success_style()
    };

    let name_style = Style::default()
        .fg(theme.tool_name)
        .add_modifier(Modifier::BOLD);

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

    buf.set_line(x, y, &Line::from(spans), width);
}

// ── Content formatting ──────────────────────────────────────────

/// Build word-wrapped display lines for the detail pane.
fn detail_content_lines(tc: &DisplayToolCall, width: usize) -> Vec<String> {
    if let Some(ref output) = tc.output {
        let formatted = format_output(&tc.name, output);
        let mut wrapped = Vec::new();
        for line in formatted {
            wrap_into(&line, width, &mut wrapped);
        }
        wrapped
    } else if !tc.streaming_lines.is_empty() {
        let mut wrapped = Vec::new();
        for line in &tc.streaming_lines {
            wrap_into(line, width, &mut wrapped);
        }
        wrapped
    } else {
        vec!["Running…".to_string()]
    }
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

// ── Word wrapping ───────────────────────────────────────────────

/// Word-wrap a single line into `out`, breaking at spaces when possible.
fn wrap_into(line: &str, width: usize, out: &mut Vec<String>) {
    if width == 0 {
        out.push(String::new());
        return;
    }

    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= width {
        out.push(line.to_string());
        return;
    }

    let mut start = 0;
    while start < chars.len() {
        let remaining = chars.len() - start;
        if remaining <= width {
            out.push(chars[start..].iter().collect());
            break;
        }

        let end = start + width;

        // If the char right after the chunk is a space (or end-of-string),
        // we can take the full chunk without splitting a word.
        if end >= chars.len() || chars[end] == ' ' {
            let segment: String = chars[start..end].iter().collect();
            out.push(segment);
            start = if end < chars.len() { end + 1 } else { end };
            continue;
        }

        // Search backward for a space to break at
        let mut break_at = None;
        for i in (start + 1..end).rev() {
            if chars[i] == ' ' {
                break_at = Some(i);
                break;
            }
        }

        if let Some(bp) = break_at {
            let segment: String = chars[start..bp].iter().collect();
            out.push(segment);
            start = bp + 1; // skip the space
        } else {
            // No space found — force break at width
            let segment: String = chars[start..end].iter().collect();
            out.push(segment);
            start = end;
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sidebar state tests ─────────────────────────────────────

    #[test]
    fn sidebar_default_state() {
        let sidebar = Sidebar::default();
        assert!(!sidebar.open);
        assert_eq!(sidebar.list_scroll, 0);
        assert_eq!(sidebar.detail_scroll, 0);
        assert!(!sidebar.first_tool_seen);
        assert_eq!(sidebar.list_height, 0);
    }

    #[test]
    fn sidebar_scroll_list() {
        let mut sidebar = Sidebar::default();
        sidebar.scroll_list_down(5);
        assert_eq!(sidebar.list_scroll, 5);
        sidebar.scroll_list_up(3);
        assert_eq!(sidebar.list_scroll, 2);
        sidebar.scroll_list_up(10);
        assert_eq!(sidebar.list_scroll, 0);
    }

    #[test]
    fn sidebar_scroll_detail() {
        let mut sidebar = Sidebar::default();
        sidebar.scroll_detail_down(5);
        assert_eq!(sidebar.detail_scroll, 5);
        sidebar.scroll_detail_up(3);
        assert_eq!(sidebar.detail_scroll, 2);
        sidebar.scroll_detail_up(10);
        assert_eq!(sidebar.detail_scroll, 0);
    }

    #[test]
    fn sidebar_reset_detail_scroll() {
        let mut sidebar = Sidebar::default();
        sidebar.detail_scroll = 42;
        sidebar.reset_detail_scroll();
        assert_eq!(sidebar.detail_scroll, 0);
    }

    #[test]
    fn sidebar_ensure_selected_visible_scrolls_down() {
        let mut sidebar = Sidebar::default();
        sidebar.list_height = 5;
        sidebar.list_scroll = 0;
        sidebar.ensure_selected_visible(7);
        assert!(sidebar.list_scroll > 0);
        assert!(sidebar.list_scroll + 5 > 7);
    }

    #[test]
    fn sidebar_ensure_selected_visible_scrolls_up() {
        let mut sidebar = Sidebar::default();
        sidebar.list_height = 5;
        sidebar.list_scroll = 10;
        sidebar.ensure_selected_visible(3);
        assert_eq!(sidebar.list_scroll, 3);
    }

    #[test]
    fn sidebar_ensure_selected_visible_already_visible() {
        let mut sidebar = Sidebar::default();
        sidebar.list_height = 5;
        sidebar.list_scroll = 2;
        sidebar.ensure_selected_visible(4);
        assert_eq!(sidebar.list_scroll, 2); // unchanged
    }

    // ── Layout tests ────────────────────────────────────────────

    #[test]
    fn compute_split_too_small() {
        let area = Rect::new(0, 0, 40, 4); // only 4 rows
        let (list, sep, detail) = compute_split(area, 5);
        assert_eq!(list.height, 4); // all to list
        assert!(sep.is_none());
        assert_eq!(detail.height, 0);
    }

    #[test]
    fn compute_split_no_tools() {
        let area = Rect::new(0, 0, 40, 20);
        let (list, sep, _detail) = compute_split(area, 0);
        assert_eq!(list.height, 20);
        assert!(sep.is_none());
    }

    #[test]
    fn compute_split_few_tools() {
        let area = Rect::new(0, 0, 40, 20);
        let (list, sep, detail) = compute_split(area, 3);
        assert!(sep.is_some());
        assert!(list.height >= 2);
        assert!(list.height <= 8); // 40% of 20
        assert!(detail.height >= 3);
        assert_eq!(list.height as usize + 1 + detail.height as usize, 20);
    }

    #[test]
    fn compute_split_many_tools() {
        let area = Rect::new(0, 0, 40, 30);
        let (list, sep, detail) = compute_split(area, 50);
        assert!(sep.is_some());
        // List should be capped at ~40%
        assert!(list.height <= 12); // 40% of 30
        assert!(detail.height >= 3);
    }

    #[test]
    fn sidebar_sub_areas_returns_full_width() {
        let sidebar = Rect::new(50, 0, 30, 20);
        let (list_hit, detail_hit) = sidebar_sub_areas(sidebar, 5);
        assert_eq!(list_hit.x, 50);
        assert_eq!(list_hit.width, 30);
        assert_eq!(detail_hit.x, 50);
        assert_eq!(detail_hit.width, 30);
    }

    // ── Word wrapping tests ─────────────────────────────────────

    #[test]
    fn wrap_short_line_unchanged() {
        let mut out = Vec::new();
        wrap_into("hello", 10, &mut out);
        assert_eq!(out, vec!["hello"]);
    }

    #[test]
    fn wrap_exact_width() {
        let mut out = Vec::new();
        wrap_into("hello", 5, &mut out);
        assert_eq!(out, vec!["hello"]);
    }

    #[test]
    fn wrap_at_space() {
        let mut out = Vec::new();
        wrap_into("hello world foo", 11, &mut out);
        assert_eq!(out, vec!["hello world", "foo"]);
    }

    #[test]
    fn wrap_long_word_force_break() {
        let mut out = Vec::new();
        wrap_into("abcdefghij", 4, &mut out);
        assert_eq!(out, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_mixed() {
        let mut out = Vec::new();
        wrap_into("short verylongwordthatexceedswidth end", 10, &mut out);
        assert_eq!(out[0], "short");
        assert_eq!(out[1], "verylongwo");
        assert_eq!(out[2], "rdthatexce");
        assert_eq!(out[3], "edswidth");
        assert_eq!(out[4], "end");
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn wrap_empty_line() {
        let mut out = Vec::new();
        wrap_into("", 10, &mut out);
        assert_eq!(out, vec![""]);
    }

    #[test]
    fn wrap_zero_width() {
        let mut out = Vec::new();
        wrap_into("hello", 0, &mut out);
        assert_eq!(out, vec![""]);
    }

    // ── Format output tests ─────────────────────────────────────

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
    fn format_output_edit_marks_diffs() {
        let lines = format_output("edit", "+added\n-removed\nunchanged");
        assert_eq!(lines[0], "+ added");
        assert_eq!(lines[1], "- removed");
        assert_eq!(lines[2], "unchanged");
    }

    // ── Detail content tests ────────────────────────────────────

    #[test]
    fn detail_content_running() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: None,
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
        };
        let lines = detail_content_lines(&tc, 40);
        assert_eq!(lines, vec!["Running…"]);
    }

    #[test]
    fn detail_content_streaming() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: None,
            is_error: false,
            expanded: false,
            streaming_lines: vec!["line1".into(), "line2".into()],
        };
        let lines = detail_content_lines(&tc, 40);
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn detail_content_completed() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: Some("done\nok".into()),
            is_error: false,
            expanded: false,
            streaming_lines: vec!["old".into()],
        };
        let lines = detail_content_lines(&tc, 40);
        assert_eq!(lines, vec!["done", "ok"]);
    }

    #[test]
    fn detail_content_wraps_long_output() {
        let tc = DisplayToolCall {
            id: "x".into(),
            name: "bash".into(),
            args_summary: String::new(),
            output: Some("this is a long line that should be wrapped at word boundaries".into()),
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
        };
        let lines = detail_content_lines(&tc, 20);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.chars().count() <= 20);
        }
    }

    // ── Widget rendering tests ──────────────────────────────────

    fn make_tc(name: &str, args: &str, output: Option<&str>, is_error: bool) -> DisplayToolCall {
        DisplayToolCall {
            id: format!("tc-{name}"),
            name: name.into(),
            args_summary: args.into(),
            output: output.map(String::from),
            is_error,
            expanded: false,
            streaming_lines: Vec::new(),
        }
    }

    #[test]
    fn sidebar_view_empty_no_panic() {
        let theme = Theme::default();
        let view = SidebarView::new(vec![], None, &theme, 0, 0, 0);
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_with_tools_no_panic() {
        let theme = Theme::default();
        let tc1 = make_tc("read", "file.rs", Some("fn main() {}"), false);
        let tc2 = make_tc("bash", "$ ls", Some("file1\nfile2"), false);
        let tc3 = make_tc("edit", "main.rs", Some("+new line"), false);
        let tool_calls: Vec<&DisplayToolCall> = vec![&tc1, &tc2, &tc3];
        let view = SidebarView::new(tool_calls, Some(1), &theme, 0, 0, 0);
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_tiny_area_no_panic() {
        let theme = Theme::default();
        let tc = make_tc("read", "f.rs", Some("hello"), false);
        let view = SidebarView::new(vec![&tc], Some(0), &theme, 0, 0, 0);
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_zero_area_no_panic() {
        let theme = Theme::default();
        let view = SidebarView::new(vec![], None, &theme, 0, 0, 0);
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_with_error_tool() {
        let theme = Theme::default();
        let tc = make_tc("bash", "$ false", Some("command failed"), true);
        let view = SidebarView::new(vec![&tc], Some(0), &theme, 0, 0, 0);
        let area = Rect::new(0, 0, 50, 15);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_with_running_tool() {
        let theme = Theme::default();
        let mut tc = make_tc("bash", "$ make", None, false);
        tc.streaming_lines = vec!["compiling...".into(), "linking...".into()];
        let view = SidebarView::new(vec![&tc], Some(0), &theme, 10, 0, 0);
        let area = Rect::new(0, 0, 50, 15);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }
}
