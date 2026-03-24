use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// Information displayed in the status bar.
#[derive(Debug, Clone, Default)]
pub struct StatusInfo {
    pub cwd: String,
    pub session_name: String,
    pub model: String,
    pub thinking: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
    pub context_percent: f64,
    pub peek: bool,
    pub extension_items: HashMap<String, String>,
}

/// Footer status bar: cwd | session | tokens (↑input ↓output) | cost ($X.XX) | context% | model.
pub struct StatusBar<'a> {
    info: &'a StatusInfo,
    theme: &'a Theme,
}

impl<'a> StatusBar<'a> {
    pub fn new(info: &'a StatusInfo, theme: &'a Theme) -> Self {
        Self { info, theme }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        // Build left side: cwd | session
        let cwd_short = shorten_path(&self.info.cwd, 30);
        let mut left_parts = vec![Span::styled(cwd_short, self.theme.accent_style())];

        if !self.info.session_name.is_empty() {
            left_parts.push(Span::styled(" │ ", self.theme.muted_style()));
            left_parts.push(Span::styled(
                self.info.session_name.clone(),
                self.theme.muted_style(),
            ));
        }

        // Extension status items
        for (key, val) in &self.info.extension_items {
            left_parts.push(Span::styled(" │ ", self.theme.muted_style()));
            left_parts.push(Span::styled(
                format!("{key}: {val}"),
                self.theme.muted_style(),
            ));
        }

        // Build right side: tokens | cost | context% | model
        let tokens_str = format!(
            "↑{} ↓{}",
            format_tokens(self.info.input_tokens),
            format_tokens(self.info.output_tokens)
        );
        let cost_str = format!("${:.2}", self.info.cost);
        let context_str = format!("{:.0}%", self.info.context_percent * 100.0);
        // Color the context% to give an at-a-glance warning before compaction fires.
        let context_style = if self.info.context_percent > 0.75 {
            self.theme.error_style()
        } else if self.info.context_percent > 0.50 {
            self.theme.warning_style()
        } else {
            self.theme.muted_style()
        };

        let mut right_parts = Vec::new();
        if self.info.peek {
            right_parts.push(Span::styled("👁 PEEK", self.theme.accent_style()));
            right_parts.push(Span::styled(" │ ", self.theme.muted_style()));
        }
        right_parts.extend([
            Span::styled(tokens_str, self.theme.muted_style()),
            Span::styled(" │ ", self.theme.muted_style()),
            Span::styled(cost_str, self.theme.muted_style()),
            Span::styled(" │ ", self.theme.muted_style()),
            Span::styled(context_str, context_style),
            Span::styled(" │ ", self.theme.muted_style()),
            Span::styled(self.info.model.clone(), self.theme.accent_style()),
        ]);

        // Compute widths
        let right_width: usize = right_parts.iter().map(|s| s.content.len()).sum();
        let available = area.width as usize;

        let line = if available > right_width + 4 {
            // Space between left and right
            let left_width: usize = left_parts.iter().map(|s| s.content.len()).sum();
            let gap = available.saturating_sub(left_width + right_width);
            let mut spans = left_parts;
            spans.push(Span::raw(" ".repeat(gap)));
            spans.extend(right_parts);
            Line::from(spans)
        } else {
            // Just show right side if terminal is narrow
            Line::from(right_parts)
        };

        buf.set_line(area.x, area.y, &line, area.width);
    }
}

fn format_tokens(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    // Try to show just the last N components
    let parts: Vec<&str> = path.split('/').collect();
    let mut result = String::new();
    for part in parts.iter().rev() {
        let candidate = if result.is_empty() {
            part.to_string()
        } else {
            format!("{part}/{result}")
        };
        if candidate.len() > max_len {
            break;
        }
        result = candidate;
    }
    if result.len() < path.len() {
        format!("…/{result}")
    } else {
        result
    }
}
