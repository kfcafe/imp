use std::path::Path;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::highlight::Highlighter;
use crate::theme::Theme;
use crate::views::tools::DisplayToolCall;

pub fn styled_tool_output_lines(
    tc: &DisplayToolCall,
    highlighter: &Highlighter,
    theme: &Theme,
    with_line_numbers: bool,
) -> Vec<Line<'static>> {
    match tc.name.as_str() {
        "read" => styled_read_output(tc, highlighter, theme, with_line_numbers),
        "write" => styled_write_output(tc, highlighter, theme),
        "edit" | "multi_edit" | "diff" => styled_diff_output(tc, theme),
        _ => styled_plain_output(tc, theme),
    }
}

pub fn wrap_styled_lines(lines: &[Line<'static>], width: usize) -> Vec<Line<'static>> {
    let mut wrapped = Vec::new();
    for line in lines {
        wrapped.extend(wrap_line(line, width));
    }
    wrapped
}

fn styled_read_output(
    tc: &DisplayToolCall,
    highlighter: &Highlighter,
    theme: &Theme,
    with_line_numbers: bool,
) -> Vec<Line<'static>> {
    let Some(output) = tc.output.as_deref() else {
        return vec![Line::from(Span::styled("Running…", theme.muted_style()))];
    };

    let total_code_lines = tc
        .details
        .get("lines")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or_else(|| output.lines().count());

    let all_lines: Vec<&str> = output.lines().collect();
    let code_lines = all_lines
        .iter()
        .take(total_code_lines)
        .copied()
        .collect::<Vec<_>>();
    let extra_lines = all_lines
        .iter()
        .skip(total_code_lines)
        .copied()
        .collect::<Vec<_>>();

    let code = code_lines.join("\n");
    let path = tc
        .details
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(&tc.args_summary);
    let language = language_token_from_path(path);

    let mut rendered =
        highlight_code_lines(highlighter, &code, &language, with_line_numbers, theme);
    for line in extra_lines {
        rendered.push(Line::from(Span::styled(
            line.to_string(),
            theme.muted_style(),
        )));
    }

    if rendered.is_empty() {
        vec![Line::from(Span::styled(
            "(empty file)",
            theme.muted_style(),
        ))]
    } else {
        rendered
    }
}

fn styled_write_output(
    tc: &DisplayToolCall,
    highlighter: &Highlighter,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let summary = tc
        .details
        .get("summary")
        .and_then(|v| v.as_str())
        .or_else(|| tc.output.as_deref().and_then(|out| out.lines().next()))
        .unwrap_or("Write completed");

    let warnings = tc
        .details
        .get("warnings")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let display_content = tc
        .details
        .get("display_content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let display_note = tc
        .details
        .get("display_note")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let path = tc
        .details
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(&tc.args_summary);
    let language = language_token_from_path(path);

    let mut rendered = vec![Line::from(Span::styled(
        summary.to_string(),
        Style::default().fg(theme.fg),
    ))];

    for warning in warnings {
        rendered.push(Line::from(Span::styled(warning, theme.warning_style())));
    }

    if display_content.is_empty() {
        rendered.push(Line::from(Span::styled(
            "(empty file)",
            theme.muted_style(),
        )));
    } else {
        rendered.extend(highlight_code_lines(
            highlighter,
            display_content,
            &language,
            false,
            theme,
        ));
    }

    if !display_note.is_empty() {
        rendered.push(Line::raw(""));
        rendered.push(Line::from(Span::styled(
            display_note.to_string(),
            theme.muted_style(),
        )));
    }

    rendered
}

fn styled_diff_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let Some(output) = tc.output.as_deref() else {
        return vec![Line::from(Span::styled("Running…", theme.muted_style()))];
    };

    let mut rendered = Vec::new();
    for line in output.lines() {
        rendered.push(styled_diff_line(line, theme, tc.is_error));
    }

    if rendered.is_empty() {
        vec![Line::from(Span::styled("(no output)", theme.muted_style()))]
    } else {
        rendered
    }
}

fn styled_plain_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let Some(output) = tc.output.as_deref() else {
        return vec![Line::from(Span::styled("Running…", theme.muted_style()))];
    };

    let style = if tc.is_error {
        theme.error_style()
    } else {
        theme.muted_style()
    };

    let rendered: Vec<Line<'static>> = output
        .lines()
        .map(|line| Line::from(Span::styled(line.to_string(), style)))
        .collect();

    if rendered.is_empty() {
        vec![Line::from(Span::styled("(no output)", theme.muted_style()))]
    } else {
        rendered
    }
}

fn highlight_code_lines(
    highlighter: &Highlighter,
    code: &str,
    language: &str,
    with_line_numbers: bool,
    theme: &Theme,
) -> Vec<Line<'static>> {
    if code.is_empty() {
        return Vec::new();
    }

    let highlighted = highlighter.highlight_code(code, language);
    if !with_line_numbers {
        return highlighted;
    }

    highlighted
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            let mut spans = vec![Span::styled(
                format!("{:>4} │ ", idx + 1),
                theme.muted_style(),
            )];
            spans.extend(line.spans);
            Line::from(spans)
        })
        .collect()
}

fn styled_diff_line(line: &str, theme: &Theme, is_error: bool) -> Line<'static> {
    let style = if line.starts_with("@@") {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("+++") || line.starts_with("---") {
        Style::default()
            .fg(theme.muted)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with('+') {
        theme.success_style()
    } else if line.starts_with('-') {
        theme.error_style()
    } else if line.starts_with("Hunk ") {
        Style::default().fg(theme.accent)
    } else if line.starts_with("Warning:") {
        theme.warning_style()
    } else if is_error {
        theme.error_style()
    } else {
        Style::default().fg(theme.fg)
    };

    Line::from(Span::styled(line.to_string(), style))
}

fn language_token_from_path(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "txt".to_string())
}

fn wrap_line(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![Line::raw(String::new())];
    }

    let chars = flatten_line_chars(line);
    if chars.is_empty() {
        return vec![Line::raw(String::new())];
    }

    let chunks = wrap_styled_chars(&chars, width.max(1));
    chunks
        .into_iter()
        .map(|chunk| Line::from(chars_to_spans(&chunk)))
        .collect()
}

fn flatten_line_chars(line: &Line<'static>) -> Vec<(char, Style)> {
    let mut chars = Vec::new();
    for span in &line.spans {
        for ch in span.content.chars() {
            chars.push((ch, span.style));
        }
    }
    chars
}

fn wrap_styled_chars(chars: &[(char, Style)], width: usize) -> Vec<Vec<(char, Style)>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let width = width.max(1);

    while start < chars.len() {
        let remaining = chars.len() - start;
        if remaining <= width {
            chunks.push(chars[start..].to_vec());
            break;
        }

        let end = start + width;
        let break_at = (start + 1..end)
            .rev()
            .find(|&idx| chars[idx].0.is_whitespace());

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

fn chars_to_spans(chars: &[(char, Style)]) -> Vec<Span<'static>> {
    if chars.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut current_style = chars[0].1;
    let mut current_text = String::new();

    for (ch, style) in chars {
        if *style == current_style {
            current_text.push(*ch);
        } else {
            spans.push(Span::styled(current_text, current_style));
            current_text = ch.to_string();
            current_style = *style;
        }
    }

    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, current_style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_tc(name: &str, output: Option<&str>) -> DisplayToolCall {
        DisplayToolCall {
            id: format!("tc-{name}"),
            name: name.into(),
            args_summary: "src/main.rs".into(),
            output: output.map(str::to_string),
            details: serde_json::Value::Null,
            is_error: false,
            expanded: true,
            streaming_lines: Vec::new(),
        }
    }

    #[test]
    fn write_output_uses_structured_display_content() {
        let mut tc = make_tc("write", Some("summary only"));
        tc.details = json!({
            "summary": "src/main.rs: 42 bytes created",
            "display_content": "fn main() {}",
            "path": "src/main.rs"
        });

        let lines = styled_tool_output_lines(&tc, &Highlighter::new(), &Theme::default(), false);
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();

        assert_eq!(plain[0], "src/main.rs: 42 bytes created");
        assert!(plain.iter().any(|line| line.contains("fn main()")));
    }

    #[test]
    fn diff_lines_get_classified() {
        let tc = make_tc("diff", Some("--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new"));
        let lines = styled_tool_output_lines(&tc, &Highlighter::new(), &Theme::default(), false);
        assert_eq!(lines.len(), 5);
    }
}
