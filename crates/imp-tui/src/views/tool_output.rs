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
        "edit" | "multi_edit" => styled_diff_output(tc, theme),
        "bash" | "shell" => styled_terminal_output(tc, theme),
        "git" => styled_git_output(tc, theme),
        "scan" => styled_scan_output(tc, theme),
        "mana" => styled_mana_output(tc, theme),
        "web" => styled_web_output(tc, theme),
        "ask_user" | "recall" | "extend" | "audit_scan" | "openrouter_secret_run" => {
            styled_status_output(tc, theme)
        }
        "color_palette" => styled_palette_output(tc, theme),
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
    let Some(output) = tc.output.as_deref().or_else(|| {
        if tc.streaming_output.is_empty() {
            None
        } else {
            Some(tc.streaming_output.as_str())
        }
    }) else {
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
    let Some(output) = tc.output.as_deref().or_else(|| {
        if tc.streaming_output.is_empty() {
            None
        } else {
            Some(tc.streaming_output.as_str())
        }
    }) else {
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

fn styled_terminal_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, terminal_line_style)
}

fn styled_git_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, git_line_style)
}

fn styled_scan_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, scan_line_style)
}

fn styled_mana_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, mana_line_style)
}

fn styled_web_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, web_line_style)
}

fn styled_palette_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, palette_line_style)
}

fn styled_status_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, status_line_style)
}

fn styled_plain_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, plain_line_style)
}

fn styled_plain_output_with(
    tc: &DisplayToolCall,
    theme: &Theme,
    style_for_line: fn(&str, &Theme, bool) -> Style,
) -> Vec<Line<'static>> {
    let Some(output) = tc.output.as_deref().or_else(|| {
        if tc.streaming_output.is_empty() {
            None
        } else {
            Some(tc.streaming_output.as_str())
        }
    }) else {
        return vec![Line::from(Span::styled("Running…", theme.muted_style()))];
    };

    let rendered: Vec<Line<'static>> = output
        .lines()
        .map(|line| {
            Line::from(Span::styled(
                line.to_string(),
                style_for_line(line, theme, tc.is_error),
            ))
        })
        .collect();

    if rendered.is_empty() {
        vec![Line::from(Span::styled("(no output)", theme.muted_style()))]
    } else {
        rendered
    }
}

fn plain_line_style(_line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error {
        theme.error_style()
    } else {
        Style::default().fg(theme.fg)
    }
}

fn terminal_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error {
        return theme.error_style();
    }

    let trimmed = line.trim_start();
    if trimmed.starts_with("error") || trimmed.starts_with("Error") || trimmed.starts_with("FAIL") {
        theme.error_style()
    } else if trimmed.starts_with("warning")
        || trimmed.starts_with("Warning")
        || trimmed.starts_with("WARN")
    {
        theme.warning_style()
    } else if trimmed.starts_with("ok")
        || trimmed.starts_with("PASS")
        || trimmed.contains(" finished ")
        || trimmed.contains(" passed")
    {
        theme.success_style()
    } else if trimmed.starts_with('$') || trimmed.starts_with('>') {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    }
}

fn git_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error {
        return theme.error_style();
    }

    if line.starts_with('+') && !line.starts_with("+++") {
        theme.success_style()
    } else if line.starts_with('-') && !line.starts_with("---") {
        theme.error_style()
    } else if line.starts_with("@@")
        || line.starts_with("diff --git")
        || line.starts_with("commit ")
        || line.starts_with("On branch ")
    {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if line.starts_with("modified:")
        || line.starts_with("new file:")
        || line.starts_with("deleted:")
        || line.contains("Changes")
    {
        theme.warning_style()
    } else {
        Style::default().fg(theme.fg)
    }
}

fn scan_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error {
        return theme.error_style();
    }

    if line.ends_with(":") || line.starts_with("Action:") || line.starts_with("Task:") {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if line.trim_start().starts_with('-')
        || line.contains("Functions")
        || line.contains("Types")
    {
        Style::default().fg(theme.tool_name)
    } else {
        Style::default().fg(theme.fg)
    }
}

fn mana_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error || line.contains("failed") || line.contains("blocked") {
        return theme.error_style();
    }

    let trimmed = line.trim_start();
    if line.starts_with("mana delta") || trimmed.starts_with('✓') || trimmed.starts_with("done") {
        theme.success_style()
    } else if trimmed.starts_with('▶') || trimmed.starts_with("running") {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if trimmed.starts_with('!') || line.contains("awaiting") || line.contains("skipped") {
        theme.warning_style()
    } else if line.ends_with(':') || matches!(line, "summary" | "units") {
        Style::default()
            .fg(theme.tool_name)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    }
}

fn web_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error {
        return theme.error_style();
    }

    let trimmed = line.trim_start();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || line.contains("://") {
        Style::default().fg(theme.accent)
    } else if line.starts_with('#') || line.ends_with(':') {
        Style::default()
            .fg(theme.tool_name)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    }
}

fn palette_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error {
        return theme.error_style();
    }

    if line.contains('#') || line.contains("oklch") || line.contains("rgb") {
        Style::default().fg(theme.accent)
    } else if line.ends_with(':') {
        Style::default()
            .fg(theme.tool_name)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    }
}

fn status_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
    if is_error || line.contains("error") || line.contains("failed") {
        theme.error_style()
    } else if line.contains("success") || line.contains("completed") || line.contains("created") {
        theme.success_style()
    } else if line.contains("warning") || line.contains("skipped") {
        theme.warning_style()
    } else if line.ends_with(':') {
        Style::default()
            .fg(theme.tool_name)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
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
                compact_line_number_prefix(idx + 1),
                theme.muted_style(),
            )];
            spans.extend(line.spans);
            Line::from(spans)
        })
        .collect()
}

fn compact_line_number_prefix(line_number: usize) -> String {
    if line_number <= 999 {
        format!("{line_number:>3}│")
    } else {
        format!("{line_number}│")
    }
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
            streaming_output: String::new(),
        }
    }

    #[test]
    fn read_output_prefers_live_streaming_transcript_while_running() {
        let mut tc = make_tc("bash", None);
        tc.streaming_output = "first\nsecond".into();

        let lines = styled_tool_output_lines(&tc, &Highlighter::new(), &Theme::default(), false);
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();

        assert_eq!(plain, vec!["first".to_string(), "second".to_string()]);
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
}
