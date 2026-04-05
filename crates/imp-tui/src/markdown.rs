use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::highlight::Highlighter;
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TableAlignment {
    Left,
    Center,
    Right,
}

/// Render markdown text to styled ratatui Lines.
///
/// Handles: headers, pipe tables, bold, italic, inline code, code blocks (with
/// syntax highlighting), lists, and links.
pub fn render_markdown<'a>(text: &str, theme: &Theme, highlighter: &Highlighter) -> Vec<Line<'a>> {
    render_markdown_inner(text, theme, highlighter, None)
}

/// Render markdown while constraining tables to a target content width.
///
/// This is primarily used by the chat view so markdown tables can wrap inside
/// the available pane width instead of being broken by outer line wrapping.
pub fn render_markdown_with_width<'a>(
    text: &str,
    theme: &Theme,
    highlighter: &Highlighter,
    width: usize,
) -> Vec<Line<'a>> {
    render_markdown_inner(text, theme, highlighter, Some(width))
}

fn render_markdown_inner<'a>(
    text: &str,
    theme: &Theme,
    highlighter: &Highlighter,
    table_width: Option<usize>,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();
    let raw_lines: Vec<&str> = text.lines().collect();
    let mut idx = 0;

    while idx < raw_lines.len() {
        let raw_line = raw_lines[idx];

        // Fenced code block toggle
        if raw_line.trim_start().starts_with("```") {
            if in_code_block {
                // End of code block — highlight and emit
                let highlighted = highlighter.highlight_code(&code_buf, &code_lang);
                for hl_line in highlighted {
                    lines.push(hl_line);
                }
                code_buf.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                // Start of code block
                code_lang = raw_line
                    .trim_start()
                    .trim_start_matches('`')
                    .trim()
                    .to_string();
                in_code_block = true;
            }
            idx += 1;
            continue;
        }

        if in_code_block {
            if !code_buf.is_empty() {
                code_buf.push('\n');
            }
            code_buf.push_str(raw_line);
            idx += 1;
            continue;
        }

        if let Some((table_lines, consumed)) =
            render_table_block(&raw_lines[idx..], theme, table_width)
        {
            lines.extend(table_lines);
            idx += consumed;
            continue;
        }

        // Headers
        if let Some(stripped) = raw_line.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                stripped.to_string(),
                Style::default()
                    .fg(theme.header_fg)
                    .add_modifier(Modifier::BOLD),
            )));
            idx += 1;
            continue;
        }
        if let Some(stripped) = raw_line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                stripped.to_string(),
                Style::default()
                    .fg(theme.header_fg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            idx += 1;
            continue;
        }
        if let Some(stripped) = raw_line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                stripped.to_string(),
                Style::default()
                    .fg(theme.header_fg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            idx += 1;
            continue;
        }

        // List items (merge - and * handling)
        let (indent, rest) = if let Some(stripped) = raw_line
            .strip_prefix("- ")
            .or_else(|| raw_line.strip_prefix("* "))
        {
            ("  • ".to_string(), stripped)
        } else if is_ordered_list(raw_line) {
            let dot = raw_line.find('.').unwrap_or(0);
            let prefix = format!("  {}. ", &raw_line[..dot]);
            let rest_start = (dot + 2).min(raw_line.len());
            (prefix, raw_line[rest_start..].trim_start())
        } else {
            (String::new(), raw_line)
        };

        let mut spans = Vec::new();
        if !indent.is_empty() {
            spans.push(Span::raw(indent));
        }
        spans.extend(parse_inline(rest, theme));
        lines.push(Line::from(spans));
        idx += 1;
    }

    // Handle unclosed code block
    if in_code_block && !code_buf.is_empty() {
        let highlighted = highlighter.highlight_code(&code_buf, &code_lang);
        for hl_line in highlighted {
            lines.push(hl_line);
        }
    }

    lines
}

fn render_table_block<'a>(
    lines: &[&str],
    theme: &Theme,
    max_width: Option<usize>,
) -> Option<(Vec<Line<'a>>, usize)> {
    if lines.len() < 2 {
        return None;
    }

    let header = parse_table_row(lines[0])?;
    let alignments = parse_table_separator(lines[1])?;
    if header.len() != alignments.len() {
        return None;
    }

    let mut rows = Vec::new();
    let mut consumed = 2;

    while let Some(line) = lines.get(consumed) {
        if line.trim().is_empty() {
            break;
        }

        match parse_table_row(line) {
            Some(row) if row.len() == alignments.len() => {
                rows.push(row);
                consumed += 1;
            }
            _ => break,
        }
    }

    Some((
        build_table_lines(header, rows, alignments, theme, max_width),
        consumed,
    ))
}

fn parse_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return None;
    }

    let mut parts: Vec<&str> = trimmed.split('|').collect();
    if trimmed.starts_with('|') && !parts.is_empty() {
        parts.remove(0);
    }
    if trimmed.ends_with('|') && !parts.is_empty() {
        parts.pop();
    }

    if parts.is_empty() {
        return None;
    }

    Some(
        parts
            .into_iter()
            .map(|part| part.trim().to_string())
            .collect(),
    )
}

fn parse_table_separator(line: &str) -> Option<Vec<TableAlignment>> {
    let cells = parse_table_row(line)?;
    if cells.is_empty() {
        return None;
    }

    cells
        .into_iter()
        .map(|cell| parse_table_alignment(&cell))
        .collect()
}

fn parse_table_alignment(cell: &str) -> Option<TableAlignment> {
    let trimmed = cell.trim();
    let dashes = trimmed.chars().filter(|&ch| ch == '-').count();
    if dashes < 3 || !trimmed.chars().all(|ch| ch == '-' || ch == ':') {
        return None;
    }

    Some(match (trimmed.starts_with(':'), trimmed.ends_with(':')) {
        (true, true) => TableAlignment::Center,
        (false, true) => TableAlignment::Right,
        _ => TableAlignment::Left,
    })
}

fn build_table_lines<'a>(
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    alignments: Vec<TableAlignment>,
    theme: &Theme,
    max_width: Option<usize>,
) -> Vec<Line<'a>> {
    let header_cells: Vec<Vec<Span<'a>>> = header
        .iter()
        .map(|cell| bold_spans(parse_inline(cell, theme)))
        .collect();
    let body_cells: Vec<Vec<Vec<Span<'a>>>> = rows
        .iter()
        .map(|row| row.iter().map(|cell| parse_inline(cell, theme)).collect())
        .collect();

    let natural_widths = natural_table_widths(&header_cells, &body_cells);
    let widths = fit_table_widths(&natural_widths, max_width);

    let mut rendered = Vec::new();
    rendered.push(table_border('┌', '─', '┬', '┐', &widths, theme));
    rendered.extend(table_row_lines(&header_cells, &widths, &alignments, theme));
    rendered.push(table_border('├', '─', '┼', '┤', &widths, theme));

    for row in &body_cells {
        rendered.extend(table_row_lines(row, &widths, &alignments, theme));
    }

    rendered.push(table_border('└', '─', '┴', '┘', &widths, theme));
    rendered
}

fn natural_table_widths<'a>(header: &[Vec<Span<'a>>], rows: &[Vec<Vec<Span<'a>>>]) -> Vec<usize> {
    let mut widths: Vec<usize> = header
        .iter()
        .map(|cell| spans_display_width(cell).max(1))
        .collect();

    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(spans_display_width(cell).max(1));
        }
    }

    widths
}

fn fit_table_widths(widths: &[usize], max_width: Option<usize>) -> Vec<usize> {
    let mut fitted = widths.to_vec();

    let Some(max_width) = max_width else {
        return fitted;
    };

    if fitted.is_empty() {
        return fitted;
    }

    let border_overhead = fitted.len() * 3 + 1;
    if max_width <= border_overhead {
        return vec![1; fitted.len()];
    }

    let available = max_width - border_overhead;
    let mut total: usize = fitted.iter().sum();
    if total <= available {
        return fitted;
    }

    while total > available {
        let mut reduced = false;
        let mut widest_idx = None;
        let mut widest = 0;

        for (idx, width) in fitted.iter().copied().enumerate() {
            if width > 1 && width >= widest {
                widest = width;
                widest_idx = Some(idx);
            }
        }

        if let Some(idx) = widest_idx {
            fitted[idx] -= 1;
            total -= 1;
            reduced = true;
        }

        if !reduced {
            break;
        }
    }

    fitted
}

fn table_border<'a>(
    left: char,
    fill: char,
    junction: char,
    right: char,
    widths: &[usize],
    theme: &Theme,
) -> Line<'a> {
    let border_style = theme.muted_style();
    let mut spans = Vec::new();
    spans.push(Span::styled(left.to_string(), border_style));

    for (idx, width) in widths.iter().enumerate() {
        spans.push(Span::styled(
            fill.to_string().repeat(*width + 2),
            border_style,
        ));
        spans.push(Span::styled(
            if idx + 1 == widths.len() {
                right.to_string()
            } else {
                junction.to_string()
            },
            border_style,
        ));
    }

    Line::from(spans)
}

fn table_row_lines<'a>(
    cells: &[Vec<Span<'a>>],
    widths: &[usize],
    alignments: &[TableAlignment],
    theme: &Theme,
) -> Vec<Line<'a>> {
    let wrapped_cells: Vec<Vec<Vec<Span<'a>>>> = cells
        .iter()
        .enumerate()
        .map(|(idx, cell)| wrap_spans(cell, widths[idx]))
        .collect();
    let row_height = wrapped_cells.iter().map(Vec::len).max().unwrap_or(1);
    let border_style = theme.muted_style();
    let mut lines = Vec::with_capacity(row_height);

    for line_idx in 0..row_height {
        let mut spans = Vec::new();
        spans.push(Span::styled("│", border_style));

        for col_idx in 0..cells.len() {
            let content = wrapped_cells[col_idx]
                .get(line_idx)
                .cloned()
                .unwrap_or_default();
            let content_width = spans_display_width(&content);
            let remaining = widths[col_idx].saturating_sub(content_width);
            let (left_pad, right_pad) = alignment_padding(remaining, alignments[col_idx]);

            spans.push(Span::raw(" "));
            if left_pad > 0 {
                spans.push(Span::raw(" ".repeat(left_pad)));
            }
            spans.extend(content);
            if right_pad > 0 {
                spans.push(Span::raw(" ".repeat(right_pad)));
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled("│", border_style));
        }

        lines.push(Line::from(spans));
    }

    lines
}

fn alignment_padding(remaining: usize, alignment: TableAlignment) -> (usize, usize) {
    match alignment {
        TableAlignment::Left => (0, remaining),
        TableAlignment::Center => {
            let left = remaining / 2;
            (left, remaining - left)
        }
        TableAlignment::Right => (remaining, 0),
    }
}

fn wrap_spans<'a>(spans: &[Span<'a>], width: usize) -> Vec<Vec<Span<'a>>> {
    let chars = flatten_spans_chars(spans);
    if chars.is_empty() {
        return vec![Vec::new()];
    }

    wrap_styled_chars_by_width(&chars, width.max(1))
        .into_iter()
        .map(chars_to_spans)
        .collect()
}

fn flatten_spans_chars(spans: &[Span<'_>]) -> Vec<(char, Style)> {
    let mut chars = Vec::new();
    for span in spans {
        for ch in span.content.chars() {
            chars.push((ch, span.style));
        }
    }
    chars
}

fn wrap_styled_chars_by_width(chars: &[(char, Style)], width: usize) -> Vec<Vec<(char, Style)>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let width = width.max(1);

    while start < chars.len() {
        let mut end = start;
        let mut used = 0;
        let mut last_space = None;

        while end < chars.len() {
            let ch = chars[end].0;
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            let next = used + ch_width;

            if end > start && next > width {
                break;
            }

            if ch.is_whitespace() {
                last_space = Some(end);
            }

            used = next;
            end += 1;

            if used >= width && end < chars.len() {
                break;
            }
        }

        if end == start {
            end = (start + 1).min(chars.len());
        }

        let break_at = if end < chars.len() {
            last_space.filter(|&idx| idx > start)
        } else {
            None
        };

        if let Some(space_idx) = break_at {
            chunks.push(chars[start..space_idx].to_vec());
            start = space_idx + 1;
            while start < chars.len() && chars[start].0.is_whitespace() {
                start += 1;
            }
        } else {
            chunks.push(chars[start..end].to_vec());
            start = end;
        }
    }

    if chunks.is_empty() {
        chunks.push(Vec::new());
    }

    chunks
}

fn chars_to_spans<'a>(chars: Vec<(char, Style)>) -> Vec<Span<'a>> {
    if chars.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut current_style = chars[0].1;
    let mut current_text = String::new();

    for (ch, style) in chars {
        if style == current_style {
            current_text.push(ch);
        } else {
            spans.push(Span::styled(current_text, current_style));
            current_text = ch.to_string();
            current_style = style;
        }
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, current_style));
    }

    spans
}

fn spans_display_width(spans: &[Span<'_>]) -> usize {
    spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum()
}

fn bold_spans<'a>(spans: Vec<Span<'a>>) -> Vec<Span<'a>> {
    spans
        .into_iter()
        .map(|span| {
            Span::styled(
                span.content.to_string(),
                span.style.add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

/// Parse inline markdown formatting: **bold**, *italic*, `code`, [links](url).
fn parse_inline<'a>(text: &str, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut buf = String::new();

    while let Some((i, ch)) = chars.next() {
        match ch {
            '`' => {
                // Inline code
                if !buf.is_empty() {
                    spans.push(Span::raw(buf.clone()));
                    buf.clear();
                }
                let mut code = String::new();
                for (_, c) in chars.by_ref() {
                    if c == '`' {
                        break;
                    }
                    code.push(c);
                }
                spans.push(Span::styled(code, theme.code_inline_style()));
            }
            '*' => {
                // Bold or italic
                let next_star = chars.peek().map(|(_, c)| *c) == Some('*');
                if next_star {
                    // Bold: **text**
                    chars.next(); // consume second *
                    if !buf.is_empty() {
                        spans.push(Span::raw(buf.clone()));
                        buf.clear();
                    }
                    let mut bold_text = String::new();
                    while let Some((_, c)) = chars.next() {
                        if c == '*' && chars.peek().map(|(_, c)| *c) == Some('*') {
                            chars.next();
                            break;
                        }
                        bold_text.push(c);
                    }
                    spans.push(Span::styled(
                        bold_text,
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    // Italic: *text*
                    if !buf.is_empty() {
                        spans.push(Span::raw(buf.clone()));
                        buf.clear();
                    }
                    let mut italic_text = String::new();
                    for (_, c) in chars.by_ref() {
                        if c == '*' {
                            break;
                        }
                        italic_text.push(c);
                    }
                    spans.push(Span::styled(
                        italic_text,
                        Style::default().add_modifier(Modifier::ITALIC),
                    ));
                }
            }
            '[' => {
                // Link: [text](url)
                if !buf.is_empty() {
                    spans.push(Span::raw(buf.clone()));
                    buf.clear();
                }
                let mut link_text = String::new();
                let mut found_close = false;
                for (_, c) in chars.by_ref() {
                    if c == ']' {
                        found_close = true;
                        break;
                    }
                    link_text.push(c);
                }
                if found_close && chars.peek().map(|(_, c)| *c) == Some('(') {
                    chars.next(); // consume (
                    let mut _url = String::new();
                    for (_, c) in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                        _url.push(c);
                    }
                    spans.push(Span::styled(
                        link_text,
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                } else {
                    // Not a valid link, emit as-is
                    buf.push('[');
                    buf.push_str(&link_text);
                    if found_close {
                        buf.push(']');
                    }
                }
            }
            _ => {
                let _ = i;
                buf.push(ch);
            }
        }
    }

    if !buf.is_empty() {
        spans.push(Span::raw(buf));
    }

    spans
}

fn is_ordered_list(line: &str) -> bool {
    let trimmed = line.trim_start();
    if let Some(dot_pos) = trimmed.find('.') {
        if dot_pos > 0 && dot_pos <= 3 {
            let prefix = &trimmed[..dot_pos];
            // Require "N. " or "N." at end — must have space after dot if content follows
            let after_dot = &trimmed[dot_pos + 1..];
            if !after_dot.is_empty() && !after_dot.starts_with(' ') {
                return false;
            }
            return prefix.chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{render_markdown, render_markdown_with_width};
    use crate::highlight::Highlighter;
    use crate::theme::Theme;
    use unicode_width::UnicodeWidthStr;

    fn plain_lines(lines: Vec<ratatui::text::Line<'_>>) -> Vec<String> {
        lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect()
    }

    #[test]
    fn renders_pipe_table_as_box_table() {
        let text = "| Current prompt content | Better home | Why |\n|---|---|---|\n| Tone, brevity, independence | Preferences | Per-user, not per-agent identity |\n| AGENTS.md project map | Context assembly | Loaded per-session based on cwd |";

        let rendered = plain_lines(render_markdown(
            text,
            &Theme::default(),
            &Highlighter::new(),
        ));

        assert_eq!(rendered.len(), 6);
        assert!(rendered[0].starts_with('┌'));
        assert!(rendered[1].contains("Current prompt content"));
        assert!(rendered[1].contains("Better home"));
        assert!(rendered[2].starts_with('├'));
        assert!(rendered[3].contains("Tone, brevity, independence"));
        assert!(rendered[4].contains("AGENTS.md project map"));
        assert!(rendered[5].starts_with('└'));
    }

    #[test]
    fn wraps_tables_to_requested_width() {
        let text = "| Column A | Column B |\n|---|---|\n| a very long bit of text that should wrap | another long bit that should also wrap |";

        let rendered = plain_lines(render_markdown_with_width(
            text,
            &Theme::default(),
            &Highlighter::new(),
            30,
        ));

        assert!(rendered
            .iter()
            .all(|line| UnicodeWidthStr::width(line.as_str()) <= 30));
        assert!(rendered.iter().any(|line| line.contains("that should")));
        assert!(rendered.first().is_some_and(|line| line.starts_with('┌')));
        assert!(rendered.last().is_some_and(|line| line.starts_with('└')));
    }

    #[test]
    fn honors_table_alignment_markers() {
        let text = "| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |";

        let rendered = plain_lines(render_markdown(
            text,
            &Theme::default(),
            &Highlighter::new(),
        ));
        let row = &rendered[3];

        assert!(row.starts_with("│ a"));
        assert!(row.contains("│   b    │"));
        assert!(row.ends_with("    c │"));
    }

    #[test]
    fn leaves_non_table_pipe_text_alone() {
        let text = "this | is not a table";

        let rendered = plain_lines(render_markdown(
            text,
            &Theme::default(),
            &Highlighter::new(),
        ));

        assert_eq!(rendered, vec!["this | is not a table"]);
    }
}
