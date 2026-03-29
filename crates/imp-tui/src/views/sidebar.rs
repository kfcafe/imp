use imp_core::config::{AnimationLevel, SidebarStyle, ToolOutputDisplay, UiConfig};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use serde_json::Value;

use crate::highlight::Highlighter;
use crate::selection::TextSurface;
use crate::theme::Theme;
use crate::views::tool_output::{styled_tool_output_lines, wrap_styled_lines};
use crate::views::tools::DisplayToolCall;

// ── Sidebar state ───────────────────────────────────────────────

/// Sidebar state tracked in App.
#[derive(Default)]
pub struct Sidebar {
    /// Whether the sidebar pane is visible.
    pub open: bool,
    /// Scroll offset for the tool list pane (split mode, 0 = top).
    pub list_scroll: usize,
    /// Scroll offset for the detail/stream pane (0 = top).
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

    /// Scroll the detail/stream pane up (toward earlier content).
    pub fn scroll_detail_up(&mut self, n: usize) {
        self.detail_scroll = self.detail_scroll.saturating_sub(n);
    }

    /// Scroll the detail/stream pane down (toward later content).
    pub fn scroll_detail_down(&mut self, n: usize) {
        self.detail_scroll += n;
    }

    /// Ensure the selected tool call index is visible in the list (split mode).
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
/// Returns `(top_hit_rect, bottom_hit_rect)` in screen coordinates.
/// In stream mode, top covers the full sidebar (bottom is zero-height).
/// In split mode, top = list area, bottom = detail area.
pub fn sidebar_sub_areas(
    sidebar_area: Rect,
    tool_count: usize,
    style: SidebarStyle,
) -> (Rect, Rect) {
    let content = Rect {
        x: sidebar_area.x + 2,
        y: sidebar_area.y,
        width: sidebar_area.width.saturating_sub(2),
        height: sidebar_area.height,
    };

    match style {
        SidebarStyle::Stream => {
            // Stream: single scrollable pane — top covers everything
            let full = Rect {
                x: sidebar_area.x,
                width: sidebar_area.width,
                ..content
            };
            let empty = Rect {
                x: sidebar_area.x,
                width: sidebar_area.width,
                y: sidebar_area.y + sidebar_area.height,
                height: 0,
            };
            (full, empty)
        }
        SidebarStyle::Split => {
            let (list_area, _, detail_area) = compute_split(content, tool_count);
            (
                Rect {
                    x: sidebar_area.x,
                    width: sidebar_area.width,
                    y: list_area.y,
                    height: list_area.height,
                },
                Rect {
                    x: sidebar_area.x,
                    width: sidebar_area.width,
                    y: detail_area.y,
                    height: detail_area.height,
                },
            )
        }
    }
}

/// Split-mode layout: list, separator, detail areas.
fn compute_split(content: Rect, tool_count: usize) -> (Rect, Option<u16>, Rect) {
    let h = content.height as usize;
    let min_detail = 3;
    let sep = 1;
    let min_total = 2 + sep + min_detail;

    if h < min_total || tool_count == 0 {
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

    let max_list = (h * 40 / 100).max(2);
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

/// Widget that renders the sidebar in either stream or split mode.
pub struct SidebarView<'a> {
    tool_calls: Vec<&'a DisplayToolCall>,
    selected: Option<usize>,
    theme: &'a Theme,
    highlighter: &'a Highlighter,
    tick: u64,
    list_scroll: usize,
    detail_scroll: usize,
    ui_config: &'a UiConfig,
}

impl<'a> SidebarView<'a> {
    pub fn new(
        tool_calls: Vec<&'a DisplayToolCall>,
        selected: Option<usize>,
        theme: &'a Theme,
        highlighter: &'a Highlighter,
        tick: u64,
        list_scroll: usize,
        detail_scroll: usize,
        ui_config: &'a UiConfig,
    ) -> Self {
        Self {
            tool_calls,
            selected,
            theme,
            highlighter,
            tick,
            list_scroll,
            detail_scroll,
            ui_config,
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

        match self.ui_config.sidebar_style {
            SidebarStyle::Stream => {
                render_stream(
                    &self.tool_calls,
                    self.selected,
                    self.theme,
                    self.highlighter,
                    self.tick,
                    self.detail_scroll,
                    self.ui_config,
                    content,
                    buf,
                    self.ui_config.animations,
                );
            }
            SidebarStyle::Split => {
                let (list_area, sep_y, detail_area) = compute_split(content, self.tool_calls.len());

                render_list(
                    &self.tool_calls,
                    self.selected,
                    self.theme,
                    self.tick,
                    self.list_scroll,
                    list_area,
                    buf,
                    self.ui_config.animations,
                );

                if let Some(sy) = sep_y {
                    let sep: String = "─".repeat(cw as usize);
                    buf.set_line(cx, sy, &Line::from(Span::styled(sep, border_style)), cw);
                }

                let selected_tc = self.selected.and_then(|i| self.tool_calls.get(i)).copied();
                render_detail(
                    selected_tc,
                    self.theme,
                    self.highlighter,
                    self.detail_scroll,
                    self.ui_config,
                    detail_area,
                    buf,
                );
            }
        }
    }
}

// ── Stream mode rendering ───────────────────────────────────────

/// Render the sidebar as a single chronological stream of tool calls
/// with their results shown inline underneath each header.
#[allow(clippy::too_many_arguments)]
fn render_stream(
    tool_calls: &[&DisplayToolCall],
    selected: Option<usize>,
    theme: &Theme,
    highlighter: &Highlighter,
    tick: u64,
    scroll: usize,
    ui_config: &UiConfig,
    area: Rect,
    buf: &mut Buffer,
    animation_level: AnimationLevel,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let width = area.width as usize;

    // Build all lines: for each tool call, header + output
    let mut all_lines: Vec<(Line<'_>, bool)> = Vec::new(); // (line, is_header)

    for (idx, tc) in tool_calls.iter().enumerate() {
        let focused = selected == Some(idx);
        let header = tc.header_line_animated_focused(theme, tick, focused, animation_level);
        all_lines.push((header, true));

        // Inline output below the header
        let output_lines = styled_output_lines(tc, ui_config, highlighter, theme, width);
        for line in output_lines {
            all_lines.push((indent_line(line), false));
        }

        // Blank line between tool calls (except after last)
        if idx + 1 < tool_calls.len() {
            all_lines.push((Line::raw(""), false));
        }
    }

    // Scrollable render
    let total = all_lines.len();
    let visible = area.height as usize;
    let start = scroll.min(total.saturating_sub(visible));

    for (i, (line, _)) in all_lines.iter().skip(start).take(visible).enumerate() {
        let row = area.y + i as u16;
        buf.set_line(area.x, row, line, area.width);
    }

    // Scroll indicator
    if total > visible && visible > 0 {
        let pct = ((start + visible).min(total) * 100) / total;
        let indicator = format!(" {pct}% ");
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

// ── Split mode: tool list ───────────────────────────────────────

fn render_list(
    tool_calls: &[&DisplayToolCall],
    selected: Option<usize>,
    theme: &Theme,
    tick: u64,
    scroll: usize,
    area: Rect,
    buf: &mut Buffer,
    animation_level: AnimationLevel,
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
        let header = tc.header_line_animated_focused(theme, tick, focused, animation_level);
        buf.set_line(area.x, row, &header, area.width);
    }

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

// ── Split mode: detail pane ─────────────────────────────────────

pub fn build_detail_text_surface(
    tc: Option<&DisplayToolCall>,
    area: Rect,
    scroll: usize,
    ui_config: &UiConfig,
    highlighter: &Highlighter,
    theme: &Theme,
) -> TextSurface {
    if area.height == 0 || area.width == 0 {
        return TextSurface::new(
            crate::selection::SelectablePane::SidebarDetail,
            area,
            Vec::new(),
            0,
        );
    }

    let rect = area;
    let lines = styled_detail_lines(tc, ui_config, highlighter, theme, area.width as usize)
        .into_iter()
        .map(line_to_plain_text)
        .collect::<Vec<_>>();
    let start = scroll.min(lines.len().saturating_sub(rect.height as usize));

    TextSurface::new(
        crate::selection::SelectablePane::SidebarDetail,
        rect,
        lines,
        start,
    )
}

fn render_detail(
    tc: Option<&DisplayToolCall>,
    theme: &Theme,
    highlighter: &Highlighter,
    scroll: usize,
    ui_config: &UiConfig,
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

    // Content fills the full detail pane; selection is already indicated in the list above.
    let lines = styled_detail_lines(Some(tc), ui_config, highlighter, theme, area.width as usize);
    let total = lines.len();
    let start = scroll.min(total.saturating_sub(area.height as usize));

    for (i, line) in lines
        .iter()
        .skip(start)
        .take(area.height as usize)
        .enumerate()
    {
        let row = area.y + i as u16;
        buf.set_line(area.x, row, line, area.width);
    }

    if total > area.height as usize && area.height > 0 {
        let visible = area.height as usize;
        let pct = ((start + visible).min(total) * 100) / total;
        let indicator = format!(" {pct}% ");
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

fn styled_detail_lines(
    tc: Option<&DisplayToolCall>,
    ui_config: &UiConfig,
    highlighter: &Highlighter,
    theme: &Theme,
    content_w: usize,
) -> Vec<Line<'static>> {
    let Some(tc) = tc else {
        return vec![Line::from(Span::styled(
            "Select a tool call",
            theme.muted_style(),
        ))];
    };

    let full_config = UiConfig {
        tool_output: ToolOutputDisplay::Full,
        word_wrap: ui_config.word_wrap,
        ..*ui_config
    };
    styled_output_lines(tc, &full_config, highlighter, theme, content_w)
}

fn styled_output_lines(
    tc: &DisplayToolCall,
    config: &UiConfig,
    highlighter: &Highlighter,
    theme: &Theme,
    width: usize,
) -> Vec<Line<'static>> {
    if matches!(config.tool_output, ToolOutputDisplay::Collapsed) {
        return Vec::new();
    }

    if tc.name == "mana" {
        let raw_lines = format_mana_output(tc);
        let limited = apply_tool_output_limit(raw_lines, config);
        return wrap_plain_lines(limited, width, config, theme, tc.is_error);
    }

    if tc.output.is_none() && !tc.streaming_output.is_empty() {
        let live_lines = tc
            .streaming_output
            .lines()
            .map(String::from)
            .collect::<Vec<_>>();
        let limited = apply_tool_output_limit(live_lines, config);
        return wrap_plain_lines(limited, width, config, theme, tc.is_error);
    }

    if tc.output.is_none() && !tc.streaming_lines.is_empty() {
        let limited = apply_tool_output_limit(tc.streaming_lines.clone(), config);
        return wrap_plain_lines(limited, width, config, theme, tc.is_error);
    }

    if tc.output.is_none() {
        return wrap_plain_lines(
            vec!["Running…".to_string()],
            width,
            config,
            theme,
            tc.is_error,
        );
    }

    let styled = styled_tool_output_lines(tc, highlighter, theme, tc.name == "read");
    let styled = apply_styled_tool_output_limit(styled, config, theme);
    if config.word_wrap && width > 0 {
        wrap_styled_lines(&styled, width.saturating_sub(2))
    } else {
        styled
    }
}

fn apply_tool_output_limit(raw_lines: Vec<String>, config: &UiConfig) -> Vec<String> {
    match config.tool_output {
        ToolOutputDisplay::Compact => {
            let max = config.tool_output_lines;
            if raw_lines.len() > max {
                let mut out: Vec<String> = raw_lines.into_iter().take(max).collect();
                out.push("…".to_string());
                out
            } else {
                raw_lines
            }
        }
        _ => raw_lines,
    }
}

fn apply_styled_tool_output_limit(
    lines: Vec<Line<'static>>,
    config: &UiConfig,
    theme: &Theme,
) -> Vec<Line<'static>> {
    match config.tool_output {
        ToolOutputDisplay::Compact => {
            let max = config.tool_output_lines;
            if lines.len() > max {
                let mut out: Vec<Line<'static>> = lines.into_iter().take(max).collect();
                out.push(Line::from(Span::styled("…", theme.muted_style())));
                out
            } else {
                lines
            }
        }
        _ => lines,
    }
}

fn wrap_plain_lines(
    lines: Vec<String>,
    width: usize,
    config: &UiConfig,
    theme: &Theme,
    is_error: bool,
) -> Vec<Line<'static>> {
    let style = if is_error {
        theme.error_style()
    } else {
        theme.muted_style()
    };

    let lines: Vec<Line<'static>> = lines
        .into_iter()
        .map(|line| Line::from(Span::styled(line, style)))
        .collect();

    if config.word_wrap && width > 0 {
        wrap_styled_lines(&lines, width.saturating_sub(2))
    } else {
        lines
    }
}

fn indent_line(line: Line<'static>) -> Line<'static> {
    let mut spans = vec![Span::raw("  ".to_string())];
    spans.extend(line.spans);
    Line::from(spans)
}

fn line_to_plain_text(line: Line<'static>) -> String {
    line.spans.into_iter().map(|span| span.content).collect()
}
fn format_mana_output(tc: &DisplayToolCall) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(view) = tc.details.get("view") {
        if let Some(summary) = view.get("summary") {
            lines.push(format!(
                "{} total · {} done · {} failed · {} awaiting verify · {} skipped",
                summary
                    .get("total_units")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                summary
                    .get("total_closed")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                summary
                    .get("total_failed")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                summary
                    .get("total_awaiting_verify")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                summary
                    .get("total_skipped")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ));
        }

        if let Some(units) = view.get("units").and_then(Value::as_array) {
            for unit in units {
                let status = unit
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("queued");
                let marker = match status {
                    "running" => "▶",
                    "done" => "✓",
                    "failed" => "✗",
                    "blocked" => "!",
                    _ => "…",
                };
                let id = unit.get("id").and_then(Value::as_str).unwrap_or("?");
                let title = unit.get("title").and_then(Value::as_str).unwrap_or("");
                let mut extras = Vec::new();
                if let Some(round) = unit.get("round").and_then(Value::as_u64) {
                    extras.push(format!("wave {round}"));
                }
                if let Some(agent) = unit.get("agent").and_then(Value::as_str) {
                    extras.push(agent.to_string());
                }
                if let Some(duration) = unit.get("duration_secs").and_then(Value::as_u64) {
                    extras.push(format!("{}s", duration));
                }
                let suffix = if extras.is_empty() {
                    String::new()
                } else {
                    format!("  {}", extras.join(" · "))
                };
                lines.push(format!("{marker} {id}  {title}  {status}{suffix}"));
            }
        }
    } else if !tc.streaming_output.is_empty() {
        lines.extend(tc.streaming_output.lines().map(String::from));
    } else if !tc.streaming_lines.is_empty() {
        lines.extend(tc.streaming_lines.clone());
    } else if let Some(ref output) = tc.output {
        lines.extend(output.lines().map(String::from));
    }

    if lines.is_empty() {
        vec!["Running…".to_string()]
    } else {
        lines
    }
}

#[cfg(test)]
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
        if end >= chars.len() || chars[end] == ' ' {
            let segment: String = chars[start..end].iter().collect();
            out.push(segment);
            start = if end < chars.len() { end + 1 } else { end };
            continue;
        }

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
            start = bp + 1;
        } else {
            let segment: String = chars[start..end].iter().collect();
            out.push(segment);
            start = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    // ── Sidebar state ───────────────────────────────────────────

    #[test]
    fn sidebar_default_state() {
        let sidebar = Sidebar::default();
        assert!(!sidebar.open);
        assert_eq!(sidebar.list_scroll, 0);
        assert_eq!(sidebar.detail_scroll, 0);
        assert!(!sidebar.first_tool_seen);
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
    fn sidebar_ensure_selected_visible_scrolls_down() {
        let mut sidebar = Sidebar {
            list_height: 5,
            ..Sidebar::default()
        };
        sidebar.ensure_selected_visible(7);
        assert!(sidebar.list_scroll + 5 > 7);
    }

    #[test]
    fn sidebar_ensure_selected_visible_scrolls_up() {
        let mut sidebar = Sidebar {
            list_height: 5,
            list_scroll: 10,
            ..Sidebar::default()
        };
        sidebar.ensure_selected_visible(3);
        assert_eq!(sidebar.list_scroll, 3);
    }

    // ── Layout ──────────────────────────────────────────────────

    #[test]
    fn compute_split_too_small() {
        let area = Rect::new(0, 0, 40, 4);
        let (list, sep, detail) = compute_split(area, 5);
        assert_eq!(list.height, 4);
        assert!(sep.is_none());
        assert_eq!(detail.height, 0);
    }

    #[test]
    fn compute_split_few_tools() {
        let area = Rect::new(0, 0, 40, 20);
        let (list, sep, detail) = compute_split(area, 3);
        assert!(sep.is_some());
        assert!(list.height >= 2);
        assert!(detail.height >= 3);
        assert_eq!(list.height as usize + 1 + detail.height as usize, 20);
    }

    #[test]
    fn sidebar_sub_areas_stream_covers_full() {
        let sidebar = Rect::new(50, 0, 30, 20);
        let (top, bottom) = sidebar_sub_areas(sidebar, 5, SidebarStyle::Stream);
        assert_eq!(top.height, 20);
        assert_eq!(bottom.height, 0);
    }

    #[test]
    fn sidebar_sub_areas_split_has_two_regions() {
        let sidebar = Rect::new(50, 0, 30, 20);
        let (top, bottom) = sidebar_sub_areas(sidebar, 5, SidebarStyle::Split);
        assert!(top.height > 0);
        assert!(bottom.height > 0);
    }

    #[test]
    fn format_mana_output_renders_summary_and_units() {
        let tc = DisplayToolCall {
            id: "1".into(),
            name: "mana".into(),
            args_summary: "run".into(),
            output: None,
            details: serde_json::json!({
                "view": {
                    "summary": {
                        "total_units": 3,
                        "total_closed": 2,
                        "total_failed": 1,
                        "total_awaiting_verify": 0,
                        "total_skipped": 0
                    },
                    "units": [
                        {"id": "1.1", "title": "First", "status": "done", "round": 1, "duration_secs": 8},
                        {"id": "1.2", "title": "Second", "status": "failed", "round": 1}
                    ]
                }
            }),
            is_error: false,
            expanded: false,
            streaming_lines: Vec::new(),
            streaming_output: String::new(),
        };

        let lines = format_mana_output(&tc);
        assert_eq!(
            lines[0],
            "3 total · 2 done · 1 failed · 0 awaiting verify · 0 skipped"
        );
        assert!(lines
            .iter()
            .any(|l| l.contains("✓ 1.1  First  done  wave 1 · 8s")));
        assert!(lines
            .iter()
            .any(|l| l.contains("✗ 1.2  Second  failed  wave 1")));
    }

    #[test]
    fn wrap_short_line_unchanged() {
        let mut out = Vec::new();
        wrap_into("hello", 10, &mut out);
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
    fn wrap_empty() {
        let mut out = Vec::new();
        wrap_into("", 10, &mut out);
        assert_eq!(out, vec![""]);
    }

    // ── Tool output lines ───────────────────────────────────────

    fn make_tc(name: &str, args: &str, output: Option<&str>, is_error: bool) -> DisplayToolCall {
        DisplayToolCall {
            id: format!("tc-{name}"),
            name: name.into(),
            args_summary: args.into(),
            output: output.map(String::from),
            details: serde_json::Value::Null,
            is_error,
            expanded: false,
            streaming_lines: Vec::new(),
            streaming_output: String::new(),
        }
    }

    #[test]
    fn styled_output_lines_read_include_numbered_source() {
        let mut tc = make_tc("read", "f.rs", Some("fn main() {}"), false);
        tc.details = serde_json::json!({"path": "src/main.rs", "lines": 1});
        let config = UiConfig {
            tool_output: ToolOutputDisplay::Full,
            word_wrap: false,
            ..Default::default()
        };
        let lines = styled_output_lines(
            &tc,
            &config,
            &crate::highlight::Highlighter::new(),
            &Theme::default(),
            80,
        );
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();
        assert!(plain[0].starts_with("   1 │ "));
        assert!(plain[0].contains("fn main()"));
    }

    #[test]
    fn styled_output_lines_use_live_streaming_output_in_sidebar() {
        let mut tc = make_tc("bash", "$ echo hi", None, false);
        tc.streaming_output = "line 1\nline 2".into();
        let config = UiConfig {
            tool_output: ToolOutputDisplay::Full,
            word_wrap: false,
            ..Default::default()
        };

        let lines = styled_output_lines(
            &tc,
            &config,
            &crate::highlight::Highlighter::new(),
            &Theme::default(),
            80,
        );
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();
        assert_eq!(plain, vec!["line 1".to_string(), "line 2".to_string()]);
    }

    #[test]
    fn styled_output_lines_write_show_file_content() {
        let mut tc = make_tc("write", "f.rs", Some("summary only"), false);
        tc.details = serde_json::json!({
            "path": "src/lib.rs",
            "summary": "src/lib.rs: 12 bytes created",
            "display_content": "pub fn hi() {}",
            "display_note": ""
        });
        let config = UiConfig {
            tool_output: ToolOutputDisplay::Full,
            word_wrap: false,
            ..Default::default()
        };
        let lines = styled_output_lines(
            &tc,
            &config,
            &crate::highlight::Highlighter::new(),
            &Theme::default(),
            80,
        );
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();
        assert!(plain.iter().any(|line| line.contains("pub fn hi")));
    }

    // ── Widget rendering ────────────────────────────────────────

    #[test]
    fn build_detail_text_surface_uses_full_area_without_header_offset() {
        let tc = make_tc("bash", "$ ls", Some("line1\nline2\nline3"), false);
        let config = UiConfig {
            sidebar_style: SidebarStyle::Split,
            word_wrap: false,
            ..Default::default()
        };
        let area = Rect::new(10, 5, 30, 6);

        let theme = Theme::default();
        let highlighter = crate::highlight::Highlighter::new();
        let surface = build_detail_text_surface(Some(&tc), area, 0, &config, &highlighter, &theme);

        assert_eq!(surface.rect, area);
    }

    #[test]
    fn sidebar_view_empty_no_panic() {
        let theme = Theme::default();
        let config = UiConfig::default();
        let highlighter = crate::highlight::Highlighter::new();
        let view = SidebarView::new(vec![], None, &theme, &highlighter, 0, 0, 0, &config);
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_stream_mode_no_panic() {
        let theme = Theme::default();
        let config = UiConfig {
            sidebar_style: SidebarStyle::Stream,
            ..Default::default()
        };
        let tc1 = make_tc("read", "file.rs", Some("fn main() {}"), false);
        let tc2 = make_tc("bash", "$ ls", Some("file1\nfile2"), false);
        let highlighter = crate::highlight::Highlighter::new();
        let view = SidebarView::new(
            vec![&tc1, &tc2],
            Some(0),
            &theme,
            &highlighter,
            0,
            0,
            0,
            &config,
        );
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_split_mode_no_panic() {
        let theme = Theme::default();
        let config = UiConfig {
            sidebar_style: SidebarStyle::Split,
            ..Default::default()
        };
        let tc1 = make_tc("read", "file.rs", Some("fn main() {}"), false);
        let tc2 = make_tc("bash", "$ ls", Some("file1\nfile2"), false);
        let highlighter = crate::highlight::Highlighter::new();
        let view = SidebarView::new(
            vec![&tc1, &tc2],
            Some(1),
            &theme,
            &highlighter,
            0,
            0,
            0,
            &config,
        );
        let area = Rect::new(0, 0, 50, 20);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }

    #[test]
    fn sidebar_view_tiny_no_panic() {
        let theme = Theme::default();
        let config = UiConfig::default();
        let tc = make_tc("read", "f.rs", Some("hello"), false);
        let highlighter = crate::highlight::Highlighter::new();
        let view = SidebarView::new(vec![&tc], Some(0), &theme, &highlighter, 0, 0, 0, &config);
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
    }
}
