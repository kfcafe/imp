use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::theme::Theme;

#[derive(Debug, Clone, Default)]
pub struct StartupSection {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StartupPanelData {
    pub headline: String,
    pub subtitle: String,
    pub hint: String,
    pub sections: Vec<StartupSection>,
    pub prompt_preview: String,
    pub prompt_tokens: u32,
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
            .title(Line::from(vec![
                Span::styled(" startup ", self.theme.accent_style()),
                Span::styled("available here", self.theme.muted_style()),
            ]))
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        let inner = outer.inner(area);
        outer.render(area, buf);

        let prompt_height = if !self.data.prompt_preview.is_empty() && inner.height >= 18 {
            (inner.height / 3).clamp(7, 14)
        } else {
            0
        };

        let chunks = if prompt_height > 0 {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(6),
                    Constraint::Length(prompt_height),
                ])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(4), Constraint::Min(6)])
                .split(inner)
        };

        render_header(chunks[0], buf, self.theme, self.data);
        render_sections(chunks[1], buf, self.theme, &self.data.sections);

        if prompt_height > 0 {
            render_prompt_preview(chunks[2], buf, self.theme, self.data);
        }
    }
}

fn render_header(area: Rect, buf: &mut Buffer, theme: &Theme, data: &StartupPanelData) {
    let text = vec![
        Line::from(Span::styled(
            data.headline.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(data.subtitle.as_str(), theme.muted_style())),
        Line::from(vec![
            Span::styled("Tip: ", theme.accent_style()),
            Span::styled(data.hint.as_str(), theme.muted_style()),
        ]),
    ];

    Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn render_sections(area: Rect, buf: &mut Buffer, theme: &Theme, sections: &[StartupSection]) {
    if sections.is_empty() || area.height == 0 || area.width == 0 {
        return;
    }

    if area.width >= 140 && sections.len() >= 4 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);
        for (idx, rect) in cols.iter().copied().enumerate().take(4) {
            if let Some(section) = sections.get(idx) {
                render_section(rect, buf, theme, section);
            }
        }
        return;
    }

    if area.width >= 90 && sections.len() >= 2 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        for (idx, rect) in top
            .iter()
            .copied()
            .chain(bottom.iter().copied())
            .enumerate()
        {
            if let Some(section) = sections.get(idx) {
                render_section(rect, buf, theme, section);
            }
        }
        return;
    }

    let constraints =
        vec![Constraint::Length((area.height / sections.len() as u16).max(3)); sections.len()];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    for (section, rect) in sections.iter().zip(rows.iter().copied()) {
        render_section(rect, buf, theme, section);
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
            .map(|line| Line::from(Span::raw(line.clone())))
            .collect()
    };

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(inner, buf);
}

fn render_prompt_preview(area: Rect, buf: &mut Buffer, theme: &Theme, data: &StartupPanelData) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" generated prompt preview ", theme.header_style()),
            Span::styled(
                format!("~{} tok · excludes file-backed context", data.prompt_tokens),
                theme.muted_style(),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(theme.border_style());
    let inner = block.inner(area);
    block.render(area, buf);

    Paragraph::new(data.prompt_preview.as_str())
        .wrap(Wrap { trim: false })
        .render(inner, buf);
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
            preview.push_str("\n");
        }
        preview.push_str("[… truncated preview]");
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::{summarize_lines, truncate_preview};

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
    fn truncate_preview_marks_truncation() {
        let text = "a\nb\nc\nd";
        let preview = truncate_preview(text, 2, 32);
        assert_eq!(preview, "a\nb\n[… truncated preview]");
    }
}
