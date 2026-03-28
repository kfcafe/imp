use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::highlight::Highlighter;
use crate::theme::Theme;

/// Render markdown text to styled ratatui Lines.
///
/// Handles: headers, bold, italic, inline code, code blocks (with syntax
/// highlighting), lists, and links.
pub fn render_markdown<'a>(text: &str, theme: &Theme, highlighter: &Highlighter) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    for raw_line in text.lines() {
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
            continue;
        }

        if in_code_block {
            if !code_buf.is_empty() {
                code_buf.push('\n');
            }
            code_buf.push_str(raw_line);
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
            continue;
        }
        if let Some(stripped) = raw_line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                stripped.to_string(),
                Style::default()
                    .fg(theme.header_fg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }
        if let Some(stripped) = raw_line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                stripped.to_string(),
                Style::default()
                    .fg(theme.header_fg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
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
