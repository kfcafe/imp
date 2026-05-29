use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct StartupAction {
    pub trigger: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct StartupSection {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StartupPanelData {
    pub actions: Vec<StartupAction>,
    pub sections: Vec<StartupSection>,
}

pub struct StartupPanelView<'a> {
    data: &'a StartupPanelData,
    theme: &'a Theme,
}

impl<'a> StartupPanelView<'a> {
    pub fn new(data: &'a StartupPanelData, theme: &'a Theme) -> Self {
        Self { data, theme }
    }
}

impl Widget for StartupPanelView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 24 || area.height < 8 {
            return;
        }

        let outer = Block::default()
            .title(Line::from(Span::styled(
                format!(" imp · {} ", env!("CARGO_PKG_VERSION")),
                self.theme.accent_style(),
            )))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        let inner = outer.inner(area);
        outer.render(area, buf);

        if inner.height < 12 {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3)])
                .split(inner);
            render_actions(chunks[0], buf, self.theme, &self.data.actions);
            render_sections(chunks[1], buf, self.theme, &self.data.sections);
            return;
        }

        let actions_height = action_block_height(inner.width, self.data.actions.len());

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(actions_height), Constraint::Min(6)])
            .split(inner);

        render_actions(chunks[0], buf, self.theme, &self.data.actions);
        render_sections(chunks[1], buf, self.theme, &self.data.sections);
    }
}

fn render_actions(area: Rect, buf: &mut Buffer, theme: &Theme, actions: &[StartupAction]) {
    if area.height < 3 || area.width < 18 || actions.is_empty() {
        return;
    }

    let block = Block::default()
        .title(Line::from(Span::styled(
            " welcome to imp. ",
            theme.header_style(),
        )))
        .borders(Borders::ALL)
        .border_style(theme.accent_style());
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if inner.width >= 96 && actions.len() >= 4 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);
        let mid = actions.len().div_ceil(2);
        render_action_lines(columns[0], buf, theme, &actions[..mid]);
        render_action_lines(columns[1], buf, theme, &actions[mid..]);
        return;
    }

    render_action_lines(inner, buf, theme, actions);
}

fn render_action_lines(area: Rect, buf: &mut Buffer, theme: &Theme, actions: &[StartupAction]) {
    let lines = actions
        .iter()
        .map(|action| {
            Line::from(vec![
                Span::styled(
                    format!(" {:<11}", action.trigger),
                    theme.accent_style().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    action.label.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", action.description), theme.muted_style()),
            ])
        })
        .collect::<Vec<_>>();

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn render_sections(area: Rect, buf: &mut Buffer, theme: &Theme, sections: &[StartupSection]) {
    if sections.is_empty() || area.height == 0 || area.width == 0 {
        return;
    }

    let visible_count = visible_section_count(area.width, area.height, sections.len());
    let visible_sections = &sections[..visible_count];

    if area.width >= 96 {
        let constraints =
            vec![Constraint::Ratio(1, visible_sections.len() as u32); visible_sections.len()];
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);
        for (section, rect) in visible_sections.iter().zip(columns.iter().copied()) {
            render_section(rect, buf, theme, section);
        }
        return;
    }

    match visible_sections.len() {
        0 => {}
        1 => render_section(area, buf, theme, &visible_sections[0]),
        2 => {
            let chunks = if area.width >= 90 {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area)
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area)
            };
            render_section(chunks[0], buf, theme, &visible_sections[0]);
            render_section(chunks[1], buf, theme, &visible_sections[1]);
        }
        3 => {
            if area.width >= 120 {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ])
                    .split(area);
                for (section, rect) in visible_sections.iter().zip(chunks.iter().copied()) {
                    render_section(rect, buf, theme, section);
                }
            } else if area.width >= 78 && area.height >= 12 {
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);
                let top = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rows[0]);
                render_section(top[0], buf, theme, &visible_sections[0]);
                render_section(top[1], buf, theme, &visible_sections[1]);
                render_section(rows[1], buf, theme, &visible_sections[2]);
            } else {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                    ])
                    .split(area);
                for (section, rect) in visible_sections.iter().zip(chunks.iter().copied()) {
                    render_section(rect, buf, theme, section);
                }
            }
        }
        _ => {
            let constraints =
                vec![
                    Constraint::Length((area.height / visible_sections.len() as u16).max(3));
                    visible_sections.len()
                ];
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(area);
            for (section, rect) in visible_sections.iter().zip(rows.iter().copied()) {
                render_section(rect, buf, theme, section);
            }
        }
    }
}

fn render_section(area: Rect, buf: &mut Buffer, theme: &Theme, section: &StartupSection) {
    if area.height < 3 || area.width < 12 {
        return;
    }

    let block = Block::default()
        .title(Line::from(Span::styled(
            format!(" {} ", section.title),
            theme.header_style(),
        )))
        .borders(Borders::ALL)
        .border_style(theme.border_style());
    let inner = block.inner(area);
    block.render(area, buf);

    let lines = if section.lines.is_empty() {
        vec![Line::from(Span::styled("none", theme.muted_style()))]
    } else {
        section
            .lines
            .iter()
            .map(|line| render_section_line(line, theme))
            .collect()
    };

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(inner, buf);
}

fn render_section_line(line: &str, theme: &Theme) -> Line<'static> {
    if let Some((icon, label)) = parse_tool_icon_line(line) {
        return Line::from(vec![
            Span::styled(format!("{icon} "), theme.accent_style()),
            Span::raw(label.to_string()),
        ]);
    }

    if let Some(rest) = line.strip_prefix("• ") {
        if let Some((label, value)) = rest.split_once(':') {
            return Line::from(vec![
                Span::styled("• ", theme.accent_style()),
                Span::styled(
                    format!("{label}:"),
                    workflow_or_meta_label_style(label, theme),
                ),
                Span::raw(value.to_string()),
            ]);
        }

        return Line::from(vec![
            Span::styled("• ", theme.accent_style()),
            Span::raw(rest.to_string()),
        ]);
    }

    Line::from(Span::styled(line.to_string(), theme.muted_style()))
}

fn workflow_or_meta_label_style(label: &str, theme: &Theme) -> Style {
    if label.starts_with('/') || label == "rules" {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        theme.muted_style()
    }
}

fn parse_tool_icon_line(line: &str) -> Option<(&str, &str)> {
    let (icon, label) = line.split_once(' ')?;
    if matches!(
        icon,
        "?" | "▣" | "$" | "◧" | "✎" | "◇" | "◆" | "⌕" | "◎" | "⚗" | "⚑"
    ) {
        Some((icon, label))
    } else {
        None
    }
}

pub fn action_block_height(width: u16, action_count: usize) -> u16 {
    if action_count == 0 {
        return 0;
    }

    if width >= 96 && action_count >= 4 {
        4
    } else {
        (action_count as u16 + 2).clamp(4, 8)
    }
}

pub fn visible_section_count(width: u16, height: u16, total: usize) -> usize {
    if total == 0 {
        return 0;
    }

    if width < 48 || height < 10 {
        total.min(1)
    } else if width < 72 || height < 16 {
        total.min(2)
    } else if width < 110 || height < 22 {
        total.min(3)
    } else {
        total
    }
}

pub fn summarize_lines(lines: Vec<String>, max_items: usize) -> Vec<String> {
    if lines.len() <= max_items {
        return lines;
    }

    let hidden = lines.len() - max_items;
    let mut visible = lines.into_iter().take(max_items).collect::<Vec<_>>();
    visible.push(format!("… +{hidden} more"));
    visible
}

pub fn summarize_inline(items: Vec<String>, max_items: usize) -> String {
    if items.is_empty() {
        return "none".to_string();
    }

    if items.len() <= max_items {
        return items.join(", ");
    }

    let hidden = items.len() - max_items;
    let visible = items.into_iter().take(max_items).collect::<Vec<_>>();
    format!("{} … +{hidden} more", visible.join(", "))
}

pub fn truncate_preview(text: &str, max_lines: usize, max_chars: usize) -> String {
    if max_lines == 0 || max_chars == 0 || text.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    let mut used_chars = 0usize;
    let mut truncated = false;

    for line in text.lines() {
        let next_len = line.chars().count() + usize::from(!lines.is_empty());
        if lines.len() >= max_lines || used_chars + next_len > max_chars {
            truncated = true;
            break;
        }
        used_chars += next_len;
        lines.push(line.to_string());
    }

    let mut preview = lines.join("\n");
    if truncated {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str("[… truncated preview]");
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::{
        render_section_line, summarize_inline, summarize_lines, truncate_preview,
        visible_section_count,
    };
    use crate::theme::Theme;

    #[test]
    fn summarize_lines_appends_hidden_count() {
        let lines = vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ];

        let summarized = summarize_lines(lines, 2);
        assert_eq!(summarized, vec!["one", "two", "… +2 more"]);
    }

    #[test]
    fn summarize_inline_compacts_items() {
        let text = summarize_inline(
            vec!["ask".into(), "bash".into(), "read".into(), "edit".into()],
            2,
        );
        assert_eq!(text, "ask, bash … +2 more");
    }

    #[test]
    fn render_section_line_styles_tool_icon_and_label_consistently() {
        let line = render_section_line("◧ Read", &Theme::default());
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content.as_ref(), "◧ ");
        assert_eq!(line.spans[1].content.as_ref(), "Read");
        assert_eq!(line.spans[1].style.fg, None);
    }

    #[test]
    fn render_section_line_styles_workflow_slash_command_white() {
        let line = render_section_line("• /plan: Create or update workflow", &Theme::default());
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[1].content.as_ref(), "/plan:");
        assert_eq!(line.spans[1].style.fg, None);
    }

    #[test]
    fn truncate_preview_marks_truncation() {
        let text = "a\nb\nc\nd";
        let preview = truncate_preview(text, 2, 32);
        assert_eq!(preview, "a\nb\n[… truncated preview]");
    }

    #[test]
    fn narrow_layout_prioritizes_fewer_sections() {
        assert_eq!(visible_section_count(44, 20, 4), 1);
        assert_eq!(visible_section_count(68, 14, 4), 2);
        assert_eq!(visible_section_count(100, 20, 4), 3);
        assert_eq!(visible_section_count(120, 24, 4), 4);
    }
}
