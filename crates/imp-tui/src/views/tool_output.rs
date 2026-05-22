use std::path::Path;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json::Value;

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
        "work" => styled_work_output(tc, theme),
        "web" => styled_web_output(tc, theme),
        "ask_user" | "extend" | "audit_scan" | "openrouter_secret_run" => {
            styled_status_output(tc, theme)
        }
        "color_palette" => styled_palette_output(tc, theme),
        _ => styled_plain_output(tc, theme),
    }
}

pub fn styled_sidebar_tool_output_lines(
    tc: &DisplayToolCall,
    _highlighter: &Highlighter,
    theme: &Theme,
    with_line_numbers: bool,
) -> Vec<Line<'static>> {
    if tc.name == "git" && tc.details.get("action").and_then(|v| v.as_str()) == Some("diff") {
        return styled_git_diff_output_with_line_numbers(tc, theme);
    }

    match tc.name.as_str() {
        "bash" | "shell" => styled_shell_sidebar_output(tc, theme),
        "git" => styled_git_sidebar_output(tc, theme),
        "scan" => styled_scan_sidebar_output(tc, theme),
        "mana" => tool_card_output(
            "Mana",
            tc.details.get("action").and_then(Value::as_str),
            "•",
            styled_plain_output_with(tc, theme, mana_line_style),
            theme,
        ),
        "web" => styled_web_sidebar_output(tc, theme),
        "prototype" => styled_prototype_sidebar_output(tc, theme),
        "work" => styled_work_output(tc, theme),
        "read" => styled_read_sidebar_output(tc, _highlighter, theme, with_line_numbers),
        "write" => styled_write_sidebar_output(tc, _highlighter, theme),
        "edit" | "multi_edit" => styled_edit_sidebar_output(tc, theme),
        "ask_user" | "extend" | "audit_scan" | "openrouter_secret_run" => tool_card_output(
            &tc.name,
            None,
            "•",
            styled_plain_output_with(tc, theme, status_line_style),
            theme,
        ),
        _ => tool_card_output(
            &tc.name,
            None,
            "•",
            styled_plain_output_with(tc, theme, plain_line_style),
            theme,
        ),
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
    let Some(output) = tool_output_text(tc) else {
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

fn tool_card_output(
    title: &str,
    action: Option<&str>,
    icon: &str,
    body: Vec<Line<'static>>,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled(icon.to_string(), theme.accent_style()),
        Span::styled(title.to_string(), theme.header_style()),
        action
            .filter(|action| !action.is_empty())
            .map(|action| Span::styled(format!(" · {action}"), theme.muted_style()))
            .unwrap_or_else(|| Span::raw(String::new())),
    ])];
    if !body.is_empty() {
        lines.push(Line::raw(""));
        lines.extend(body);
    }
    lines
}

fn card_meta_line(label: &str, value: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {label}: "), theme.muted_style()),
        Span::styled(value.to_string(), Style::default().fg(theme.fg)),
    ])
}

fn append_card_meta(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    value: Option<&str>,
    theme: &Theme,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        lines.push(card_meta_line(label, value, theme));
    }
}

fn styled_shell_sidebar_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let mut body = Vec::new();
    append_card_meta(
        &mut body,
        "command",
        tc.details.get("command").and_then(Value::as_str),
        theme,
    );
    append_card_meta(
        &mut body,
        "workdir",
        tc.details.get("workdir").and_then(Value::as_str),
        theme,
    );
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_plain_output_with(tc, theme, terminal_line_style));
    tool_card_output("Terminal", None, "$", body, theme)
}

fn styled_git_sidebar_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let action = tc.details.get("action").and_then(Value::as_str);
    let mut body = Vec::new();
    for key in ["base", "head"] {
        append_card_meta(
            &mut body,
            key,
            tc.details.get(key).and_then(Value::as_str),
            theme,
        );
    }
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_plain_output_with(tc, theme, git_line_style));
    tool_card_output("Git", action, "◆", body, theme)
}

fn styled_scan_sidebar_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let action = tc.details.get("action").and_then(Value::as_str);
    let mut body = Vec::new();
    for key in ["query", "target", "directory"] {
        append_card_meta(
            &mut body,
            key,
            tc.details.get(key).and_then(Value::as_str),
            theme,
        );
    }
    if let Some(files) = tc.details.get("files").and_then(Value::as_array) {
        let value = files
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        append_card_meta(&mut body, "files", Some(&value), theme);
    }
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_plain_output_with(tc, theme, scan_line_style));
    tool_card_output("Scan", action, "⌕", body, theme)
}

fn styled_web_sidebar_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let action = tc.details.get("action").and_then(Value::as_str);
    let mut body = Vec::new();
    append_card_meta(
        &mut body,
        "query",
        tc.details.get("query").and_then(Value::as_str),
        theme,
    );
    append_card_meta(
        &mut body,
        "url",
        tc.details.get("url").and_then(Value::as_str),
        theme,
    );
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_plain_output_with(tc, theme, web_line_style));
    tool_card_output("Web", action, "◎", body, theme)
}

fn styled_prototype_sidebar_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let mut body = Vec::new();
    append_card_meta(
        &mut body,
        "question",
        tc.details.get("question").and_then(Value::as_str),
        theme,
    );
    append_card_meta(
        &mut body,
        "language",
        tc.details.get("language").and_then(Value::as_str),
        theme,
    );
    if let Some(exit_code) = tc.details.get("exit_code").and_then(Value::as_i64) {
        body.push(card_meta_line("exit", &exit_code.to_string(), theme));
    }
    append_card_meta(
        &mut body,
        "outcome",
        tc.details.get("outcome").and_then(Value::as_str),
        theme,
    );
    append_card_meta(
        &mut body,
        "hypothesis",
        tc.details.get("hypothesis_result").and_then(Value::as_str),
        theme,
    );
    if let Some(sandbox) = tc.details.get("sandbox").and_then(Value::as_str) {
        append_card_meta(&mut body, "sandbox", Some(sandbox), theme);
    }
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_plain_output_with(tc, theme, terminal_line_style));
    tool_card_output(
        "Prototype",
        tc.details.get("action").and_then(Value::as_str),
        "⚗",
        body,
        theme,
    )
}

fn styled_read_sidebar_output(
    tc: &DisplayToolCall,
    highlighter: &Highlighter,
    theme: &Theme,
    with_line_numbers: bool,
) -> Vec<Line<'static>> {
    let mut body = Vec::new();
    append_card_meta(
        &mut body,
        "path",
        tc.details.get("path").and_then(Value::as_str),
        theme,
    );
    if let Some(lines) = tc.details.get("lines").and_then(Value::as_u64) {
        body.push(card_meta_line("lines", &lines.to_string(), theme));
    }
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_read_output(
        tc,
        highlighter,
        theme,
        with_line_numbers,
    ));
    tool_card_output("Read", None, "◧", body, theme)
}

fn styled_write_sidebar_output(
    tc: &DisplayToolCall,
    highlighter: &Highlighter,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut body = Vec::new();
    append_card_meta(
        &mut body,
        "path",
        tc.details.get("path").and_then(Value::as_str),
        theme,
    );
    append_card_meta(
        &mut body,
        "mode",
        tc.details.get("mode").and_then(Value::as_str),
        theme,
    );
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_write_output(tc, highlighter, theme));
    tool_card_output("Write", None, "✎", body, theme)
}

fn styled_edit_sidebar_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let mut body = Vec::new();
    append_card_meta(
        &mut body,
        "path",
        tc.details.get("path").and_then(Value::as_str),
        theme,
    );
    if let Some(edits) = tc.details.get("edits").and_then(Value::as_array) {
        body.push(card_meta_line(
            "edits",
            &format!(
                "{} change{}",
                edits.len(),
                if edits.len() == 1 { "" } else { "s" }
            ),
            theme,
        ));
    }
    if !body.is_empty() {
        body.push(Line::raw(""));
    }
    body.extend(styled_diff_output(tc, theme));
    tool_card_output("Edit", None, "◇", body, theme)
}

fn styled_diff_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let Some(output) = tc.output.as_deref().or({
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

fn styled_git_diff_output_with_line_numbers(
    tc: &DisplayToolCall,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let Some(output) = tool_output_text(tc) else {
        return vec![Line::from(Span::styled("Running…", theme.muted_style()))];
    };

    let lines = git_diff_lines_with_line_numbers(output, theme, tc.is_error);
    if lines.is_empty() {
        vec![Line::from(Span::styled("(no output)", theme.muted_style()))]
    } else {
        lines
    }
}

fn styled_scan_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, scan_line_style)
}

fn styled_mana_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    styled_plain_output_with(tc, theme, mana_line_style)
}

fn styled_work_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
    let action = tc
        .details
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("work");
    let mut lines = vec![Line::from(vec![
        Span::styled("▣", theme.accent_style()),
        Span::styled("Work", theme.header_style()),
        Span::styled(format!(" · {action}"), theme.muted_style()),
    ])];

    if let Some(kind) = tc.details.get("kind").and_then(Value::as_str) {
        lines.push(work_kv_line("kind", kind, theme));
    }
    if let Some(status) = tc.details.get("status").and_then(Value::as_str) {
        lines.push(work_kv_line("status", status, theme));
    }
    if let Some(path) = tc.details.get("path").and_then(Value::as_str) {
        lines.push(work_kv_line("path", path, theme));
    }

    if let Some(item) = tc.details.get("item") {
        push_work_item(&mut lines, item, theme);
    }
    if let Some(items) = tc.details.get("items").and_then(Value::as_array) {
        push_blank_line(&mut lines);
        lines.push(Line::from(Span::styled(
            format!(
                "{} item{}",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            ),
            theme.header_style(),
        )));
        for item in items {
            push_work_item(&mut lines, item, theme);
        }
    }
    if let Some(policy) = tc.details.get("policy") {
        push_work_policy_summary(&mut lines, policy, theme);
    }
    if let Some(provenance) = tc.details.get("provenance") {
        push_work_provenance_summary(&mut lines, provenance, theme);
    }

    if lines.len() == 1 {
        if let Some(output) = tool_output_text(tc) {
            lines.extend(
                output
                    .lines()
                    .map(|line| Line::from(Span::styled(line.to_string(), theme.muted_style()))),
            );
        } else {
            lines.push(Line::from(Span::styled("Running…", theme.muted_style())));
        }
    }

    lines
}

fn push_work_item(lines: &mut Vec<Line<'static>>, item: &Value, theme: &Theme) {
    let Some(obj) = item.as_object() else {
        lines.push(Line::from(Span::styled(
            format_work_value(item),
            theme.muted_style(),
        )));
        return;
    };

    push_blank_line(lines);
    let id = obj.get("id").and_then(Value::as_str).unwrap_or("work item");
    let title = obj
        .get("title")
        .or_else(|| obj.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let status = obj.get("status").and_then(Value::as_str);
    lines.push(Line::from(vec![
        Span::styled("● ", style_for_work_status(status, theme)),
        Span::styled(
            id.to_string(),
            theme.accent_style().add_modifier(Modifier::BOLD),
        ),
        if title.is_empty() {
            Span::raw(String::new())
        } else {
            Span::styled(format!("  {title}"), Style::default().fg(theme.fg))
        },
    ]));
    if let Some(status) = status {
        lines.push(work_kv_line("status", status, theme));
    }
    for key in ["parent", "parent_work", "context_pack", "stream_id"] {
        if let Some(value) = obj.get(key) {
            push_work_value_line(lines, key, value, theme);
        }
    }
    for key in [
        "acceptance",
        "checks",
        "depends_on",
        "links",
        "source_refs",
        "evidence_required",
        "topics",
    ] {
        if let Some(value) = obj.get(key) {
            push_work_value_line(lines, key, value, theme);
        }
    }
}

fn push_work_policy_summary(lines: &mut Vec<Line<'static>>, policy: &Value, theme: &Theme) {
    let Some(map) = policy.as_object() else {
        return;
    };

    push_blank_line(lines);
    let decision = map
        .get("decision")
        .and_then(|value| value.get("decision"))
        .and_then(Value::as_str)
        .unwrap_or("checked");
    let tool = map
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("work");
    let mode = map
        .get("autonomy_mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    lines.push(Line::from(vec![
        Span::styled("policy ", theme.header_style()),
        Span::styled(
            decision.to_string(),
            style_for_policy_decision(decision, theme),
        ),
        Span::styled(format!(" · {tool} · {mode}"), theme.muted_style()),
    ]));

    if let Some(scope) = map.get("resource_scope") {
        let formatted = format_work_value(scope);
        if !formatted.is_empty() {
            lines.push(work_kv_line("scope", &formatted, theme));
        }
    }
    if let Some(labels) = map.get("trust_labels") {
        let formatted = format_work_value(labels);
        if !formatted.is_empty() {
            lines.push(work_kv_line("trust", &formatted, theme));
        }
    }
}

fn push_work_provenance_summary(lines: &mut Vec<Line<'static>>, provenance: &Value, theme: &Theme) {
    let Some(map) = provenance.as_object() else {
        return;
    };
    let trust = map.get("trust").and_then(Value::as_str).unwrap_or("");
    let risk = map.get("risk").map(format_work_value).unwrap_or_default();
    if trust.is_empty() && risk.is_empty() {
        return;
    }

    push_blank_line(lines);
    lines.push(Line::from(vec![
        Span::styled("provenance ", theme.header_style()),
        Span::styled(trust.to_string(), theme.muted_style()),
        if risk.is_empty() {
            Span::raw(String::new())
        } else {
            Span::styled(format!(" · {risk}"), theme.muted_style())
        },
    ]));
}

fn style_for_policy_decision(decision: &str, theme: &Theme) -> Style {
    match decision {
        "allow" | "allowed" => theme.success_style(),
        "deny" | "denied" | "blocked" => theme.error_style(),
        _ => theme.warning_style(),
    }
}

fn push_work_value_line(lines: &mut Vec<Line<'static>>, key: &str, value: &Value, theme: &Theme) {
    match value {
        Value::Null => {}
        Value::Array(items) => {
            lines.push(work_kv_line(
                key,
                &format!(
                    "{} item{}",
                    items.len(),
                    if items.len() == 1 { "" } else { "s" }
                ),
                theme,
            ));
            for item in items {
                lines.push(Line::from(vec![
                    Span::styled("    • ", theme.muted_style()),
                    Span::styled(format_work_value(item), Style::default().fg(theme.fg)),
                ]));
            }
        }
        Value::Object(map) => {
            lines.push(work_kv_line(
                key,
                &format!(
                    "{} field{}",
                    map.len(),
                    if map.len() == 1 { "" } else { "s" }
                ),
                theme,
            ));
            let mut fields = map.iter().collect::<Vec<_>>();
            fields.sort_by(|left, right| left.0.cmp(right.0));
            for (field, field_value) in fields {
                lines.push(Line::from(vec![
                    Span::styled(format!("    {field}: "), theme.muted_style()),
                    Span::styled(
                        format_work_value(field_value),
                        Style::default().fg(theme.fg),
                    ),
                ]));
            }
        }
        _ => lines.push(work_kv_line(key, &format_work_value(value), theme)),
    }
}

fn work_kv_line(key: &str, value: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key}: "), theme.muted_style()),
        Span::styled(value.to_string(), Style::default().fg(theme.fg)),
    ])
}

fn push_blank_line(lines: &mut Vec<Line<'static>>) {
    if lines.last().is_some_and(|line| !line.spans.is_empty()) {
        lines.push(Line::raw(""));
    }
}

fn style_for_work_status(status: Option<&str>, theme: &Theme) -> Style {
    match status.unwrap_or_default() {
        "done" | "closed" | "resolved" => theme.success_style(),
        "active" | "ready" | "review" => theme.accent_style(),
        "blocked" | "failed" | "needs_context" => theme.error_style(),
        "todo" | "open" => theme.warning_style(),
        _ => theme.muted_style(),
    }
}

fn format_work_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(items) => items
            .iter()
            .map(format_work_value)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(map) => {
            if let (Some(id), Some(title)) = (
                map.get("id").and_then(Value::as_str),
                map.get("title").and_then(Value::as_str),
            ) {
                return format!("{id} · {title}");
            }
            let mut fields = map
                .iter()
                .filter_map(|(key, value)| {
                    let formatted = format_work_value(value);
                    (!formatted.is_empty()).then(|| format!("{key}: {formatted}"))
                })
                .collect::<Vec<_>>();
            fields.sort();
            fields.join(" · ")
        }
    }
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
    let Some(output) = tool_output_text(tc) else {
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

fn tool_output_text(tc: &DisplayToolCall) -> Option<&str> {
    tc.output.as_deref().or({
        if tc.streaming_output.is_empty() {
            None
        } else {
            Some(tc.streaming_output.as_str())
        }
    })
}

fn git_diff_lines_with_line_numbers(
    output: &str,
    theme: &Theme,
    is_error: bool,
) -> Vec<Line<'static>> {
    let mut old_line: Option<usize> = None;
    let mut new_line: Option<usize> = None;
    let mut rendered = Vec::new();

    for raw_line in output.lines() {
        if let Some((old_start, new_start)) = parse_unified_hunk_header(raw_line) {
            old_line = Some(old_start);
            new_line = Some(new_start);
            rendered.push(Line::from(Span::styled(
                raw_line.to_string(),
                git_line_style(raw_line, theme, is_error),
            )));
            continue;
        }

        if is_diff_metadata_line(raw_line) || old_line.is_none() || new_line.is_none() {
            rendered.push(Line::from(Span::styled(
                raw_line.to_string(),
                git_line_style(raw_line, theme, is_error),
            )));
            continue;
        }

        let (old_label, new_label, advance_old, advance_new) = if raw_line.starts_with('+') {
            (String::new(), format_line_number(new_line), false, true)
        } else if raw_line.starts_with('-') {
            (format_line_number(old_line), String::new(), true, false)
        } else if raw_line.starts_with('\\') {
            (String::new(), String::new(), false, false)
        } else {
            (
                format_line_number(old_line),
                format_line_number(new_line),
                true,
                true,
            )
        };

        let content_style = git_line_style(raw_line, theme, is_error);
        rendered.push(Line::from(vec![
            Span::styled(format!("{old_label:>4}"), theme.muted_style()),
            Span::styled("│", theme.muted_style()),
            Span::styled(format!("{new_label:>4}"), theme.muted_style()),
            Span::styled("│ ", theme.muted_style()),
            Span::styled(raw_line.to_string(), content_style),
        ]));

        if advance_old {
            old_line = old_line.map(|line| line + 1);
        }
        if advance_new {
            new_line = new_line.map(|line| line + 1);
        }
    }

    rendered
}

fn format_line_number(line: Option<usize>) -> String {
    line.map(|line| line.to_string()).unwrap_or_default()
}

fn is_diff_metadata_line(line: &str) -> bool {
    line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("new file mode ")
        || line.starts_with("deleted file mode ")
        || line.starts_with("old mode ")
        || line.starts_with("new mode ")
        || line.starts_with("similarity index ")
        || line.starts_with("rename from ")
        || line.starts_with("rename to ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
}

fn parse_unified_hunk_header(line: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old_range, rest) = rest.split_once(' ')?;
    let rest = rest.strip_prefix('+')?;
    let (new_range, _) = rest.split_once(' ')?;

    Some((parse_hunk_start(old_range)?, parse_hunk_start(new_range)?))
}

fn parse_hunk_start(range: &str) -> Option<usize> {
    range
        .split_once(',')
        .map(|(start, _)| start)
        .unwrap_or(range)
        .parse()
        .ok()
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

    fn plain_lines(lines: Vec<Line<'static>>) -> Vec<String> {
        lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect()
    }

    #[test]
    fn prototype_sidebar_card_shows_experiment_metadata() {
        let mut tc = make_tc(
            "prototype",
            Some("Prototype answered question\nOutput:\nok"),
        );
        tc.details = json!({
            "action": "run",
            "question": "Can the parser handle nested cards?",
            "language": "python",
            "exit_code": 0,
            "outcome": "observed",
            "hypothesis_result": "supported",
            "sandbox": "/tmp/prototype"
        });

        let plain = plain_lines(styled_sidebar_tool_output_lines(
            &tc,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));

        assert_eq!(plain[0], "⚗Prototype · run");
        assert!(plain.iter().any(|line| line.contains("language: python")));
        assert!(plain
            .iter()
            .any(|line| line.contains("hypothesis: supported")));
        assert!(plain.iter().any(|line| line == "ok"));
    }

    #[test]
    fn read_write_edit_sidebar_cards_show_file_metadata() {
        let mut read = make_tc("read", Some("fn main() {}"));
        read.details = json!({"path": "src/main.rs", "lines": 1});
        let read_plain = plain_lines(styled_sidebar_tool_output_lines(
            &read,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));
        assert_eq!(read_plain[0], "◧Read");
        assert!(read_plain
            .iter()
            .any(|line| line.contains("path: src/main.rs")));
        assert!(read_plain.iter().any(|line| line.contains("fn main")));

        let mut write = make_tc("write", Some("src/main.rs: 12 bytes written"));
        write.details = json!({
            "path": "src/main.rs",
            "mode": "overwrite",
            "summary": "src/main.rs: 12 bytes written",
            "display_content": "fn main() {}"
        });
        let write_plain = plain_lines(styled_sidebar_tool_output_lines(
            &write,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));
        assert_eq!(write_plain[0], "✎Write");
        assert!(write_plain
            .iter()
            .any(|line| line.contains("mode: overwrite")));

        let mut edit = make_tc("edit", Some("@@ -1 +1 @@\n-old\n+new"));
        edit.details = json!({
            "path": "src/main.rs",
            "edits": [{"old_text": "old", "new_text": "new"}]
        });
        let edit_plain = plain_lines(styled_sidebar_tool_output_lines(
            &edit,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));
        assert_eq!(edit_plain[0], "◇Edit");
        assert!(edit_plain
            .iter()
            .any(|line| line.contains("edits: 1 change")));
        assert!(edit_plain.iter().any(|line| line.contains("+new")));
    }

    #[test]
    fn shell_sidebar_card_shows_command_and_output() {
        let mut tc = make_tc("bash", Some("PASS tests\nwarning: slow"));
        tc.details = json!({"command": "cargo test -p imp-tui", "workdir": "/repo"});

        let plain = plain_lines(styled_sidebar_tool_output_lines(
            &tc,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));

        assert_eq!(plain[0], "$Terminal");
        assert!(plain
            .iter()
            .any(|line| line.contains("command: cargo test")));
        assert!(plain.iter().any(|line| line.contains("workdir: /repo")));
        assert!(plain.iter().any(|line| line == "PASS tests"));
    }

    #[test]
    fn git_scan_and_web_sidebar_cards_show_key_metadata() {
        let mut git = make_tc("git", Some("On branch nightly"));
        git.details = json!({"action": "status", "base": "HEAD~1", "head": "HEAD"});
        let git_plain = plain_lines(styled_sidebar_tool_output_lines(
            &git,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));
        assert_eq!(git_plain[0], "◆Git · status");
        assert!(git_plain.iter().any(|line| line.contains("base: HEAD~1")));

        let mut scan = make_tc("scan", Some("Functions: 2"));
        scan.details = json!({"action": "search", "query": "WorkTool", "directory": "crates"});
        let scan_plain = plain_lines(styled_sidebar_tool_output_lines(
            &scan,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));
        assert_eq!(scan_plain[0], "⌕Scan · search");
        assert!(scan_plain
            .iter()
            .any(|line| line.contains("query: WorkTool")));

        let mut web = make_tc("web", Some("https://example.com"));
        web.details = json!({"action": "read", "url": "https://example.com"});
        let web_plain = plain_lines(styled_sidebar_tool_output_lines(
            &web,
            &Highlighter::new(),
            &Theme::default(),
            false,
        ));
        assert_eq!(web_plain[0], "◎Web · read");
        assert!(web_plain
            .iter()
            .any(|line| line.contains("url: https://example.com")));
    }

    #[test]
    fn work_output_renders_tasks_as_styled_cards() {
        let mut tc = make_tc("work", Some("1 global project task(s)"));
        tc.details = json!({
            "action": "list",
            "kind": "task",
            "items": [{
                "id": "T-improve-work-sidebar",
                "title": "Improve work sidebar",
                "status": "ready",
                "acceptance": ["shows full task detail"],
                "checks": [{"command": "cargo test -p imp-tui work_output"}]
            }],
            "policy": {"decision": "allowed", "tool_name": "work"}
        });

        let lines = styled_tool_output_lines(&tc, &Highlighter::new(), &Theme::default(), false);
        let plain = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert!(plain.iter().any(|line| line.contains("▣Work · list")));
        assert!(plain
            .iter()
            .any(|line| line.contains("T-improve-work-sidebar")));
        assert!(plain
            .iter()
            .any(|line| line.contains("shows full task detail")));
        assert!(plain.iter().any(|line| line.contains("policy")));
        assert!(!plain.iter().any(|line| line.contains("{\"")));
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

    #[test]
    fn git_diff_sidebar_output_adds_compact_line_number_gutter() {
        let mut tc = make_tc(
            "git",
            Some(
                "diff --git a/src/main.rs b/src/main.rs\n@@ -10,3 +10,4 @@ fn main() {\n context\n-old\n+new\n+extra\n",
            ),
        );
        tc.details = json!({ "action": "diff" });

        let lines =
            styled_sidebar_tool_output_lines(&tc, &Highlighter::new(), &Theme::default(), false);
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();

        assert_eq!(plain[0], "diff --git a/src/main.rs b/src/main.rs");
        assert_eq!(plain[1], "@@ -10,3 +10,4 @@ fn main() {");
        assert_eq!(plain[2], "  10│  10│  context");
        assert_eq!(plain[3], "  11│    │ -old");
        assert_eq!(plain[4], "    │  11│ +new");
        assert_eq!(plain[5], "    │  12│ +extra");
    }

    #[test]
    fn git_diff_line_numbers_are_sidebar_only() {
        let mut tc = make_tc("git", Some("@@ -1 +1 @@\n-old\n+new\n"));
        tc.details = json!({ "action": "diff" });

        let lines = styled_tool_output_lines(&tc, &Highlighter::new(), &Theme::default(), false);
        let plain: Vec<String> = lines
            .into_iter()
            .map(|line| line.spans.into_iter().map(|span| span.content).collect())
            .collect();

        assert_eq!(plain, vec!["@@ -1 +1 @@", "-old", "+new"]);
    }
}
