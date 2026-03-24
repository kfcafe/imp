use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;
use crate::views::status::StatusInfo;

/// Single-line header bar replacing the old bottom status bar.
///
/// Format: `model · 8.2k/200k (4%) · $0.12 · ~/tower/imp · session: debug-oauth`
pub struct TopBar<'a> {
    info: &'a StatusInfo,
    theme: &'a Theme,
}

impl<'a> TopBar<'a> {
    pub fn new(info: &'a StatusInfo, theme: &'a Theme) -> Self {
        Self { info, theme }
    }
}

impl Widget for TopBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let sep = Span::styled(" · ", self.theme.muted_style());

        // Model name (accent)
        let model_span = Span::styled(self.info.model.clone(), self.theme.accent_style());

        // Context gauge: "8.2k/200k (4%)" colored green→yellow→red
        let context_style = context_color(self.info.context_percent, self.theme);
        let current_tokens = self.info.input_tokens;
        let context_window = if self.info.context_window > 0 {
            self.info.context_window
        } else {
            200_000
        };
        let context_span = Span::styled(
            format!(
                "{}/{} ({:.0}%)",
                format_tokens(current_tokens),
                format_tokens(context_window),
                self.info.context_percent * 100.0,
            ),
            context_style,
        );

        // Cost
        let cost_span = Span::styled(format!("${:.2}", self.info.cost), self.theme.muted_style());

        // cwd (muted, shortened)
        let cwd_span = Span::styled(shorten_path(&self.info.cwd, 30), self.theme.muted_style());

        // Session name
        let session_span = if !self.info.session_name.is_empty() {
            Some(Span::styled(
                format!("session: {}", self.info.session_name),
                self.theme.muted_style(),
            ))
        } else {
            None
        };

        // PEEK indicator
        let peek_span = if self.info.peek {
            Some(Span::styled("👁 PEEK", self.theme.accent_style()))
        } else {
            None
        };

        // Assemble left side: model · context · cost
        let mut left: Vec<Span> = vec![
            model_span,
            sep.clone(),
            context_span,
            sep.clone(),
            cost_span,
        ];
        if let Some(peek) = peek_span {
            left.push(sep.clone());
            left.push(peek);
        }

        // Assemble right side: cwd · session
        let mut right: Vec<Span> = vec![cwd_span];
        if let Some(s) = session_span {
            right.push(sep.clone());
            right.push(s);
        }

        // Extension items on the right
        for (key, val) in &self.info.extension_items {
            right.push(sep.clone());
            right.push(Span::styled(
                format!("{key}: {val}"),
                self.theme.muted_style(),
            ));
        }

        let left_width: usize = left.iter().map(|s| s.content.len()).sum();
        let right_width: usize = right.iter().map(|s| s.content.len()).sum();
        let available = area.width as usize;

        let line = if available > left_width + right_width + 2 {
            let gap = available.saturating_sub(left_width + right_width);
            let mut spans = left;
            spans.push(Span::raw(" ".repeat(gap)));
            spans.extend(right);
            Line::from(spans)
        } else if available > left_width + 2 {
            // Narrow: just show left side
            Line::from(left)
        } else {
            // Very narrow: just model + context
            Line::from(vec![
                Span::styled(self.info.model.clone(), self.theme.accent_style()),
                sep,
                Span::styled(
                    format!("{:.0}%", self.info.context_percent * 100.0),
                    context_style,
                ),
            ])
        };

        buf.set_line(area.x, area.y, &line, area.width);
    }
}

fn context_color(percent: f64, theme: &Theme) -> ratatui::style::Style {
    if percent > 0.75 {
        theme.error_style()
    } else if percent > 0.50 {
        theme.warning_style()
    } else {
        theme.success_style()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::status::StatusInfo;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn default_info() -> StatusInfo {
        StatusInfo {
            model: "sonnet".into(),
            input_tokens: 8_200,
            output_tokens: 1_500,
            cost: 0.12,
            context_percent: 0.04,
            context_window: 200_000,
            cwd: "~/tower/imp".into(),
            session_name: "debug-oauth".into(),
            ..StatusInfo::default()
        }
    }

    #[test]
    fn top_bar_renders_model_and_context() {
        let info = default_info();
        let theme = Theme::default();
        let bar = TopBar::new(&info, &theme);

        let area = Rect::new(0, 0, 100, 1);
        let mut buf = Buffer::empty(area);
        bar.render(area, &mut buf);

        let content: String = (0..area.width)
            .map(|x| {
                buf.cell((x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();

        assert!(content.contains("sonnet"), "should contain model name");
        assert!(
            content.contains("8.2k"),
            "should contain current context tokens"
        );
        assert!(content.contains("200.0k"), "should contain context window");
        assert!(content.contains("$0.12"), "should contain cost");
        assert!(content.contains("~/tower/imp"), "should contain cwd");
        assert!(
            content.contains("debug-oauth"),
            "should contain session name"
        );
    }

    #[test]
    fn top_bar_zero_height_noop() {
        let info = default_info();
        let theme = Theme::default();
        let bar = TopBar::new(&info, &theme);

        let area = Rect::new(0, 0, 80, 0);
        let mut buf = Buffer::empty(area);
        bar.render(area, &mut buf);
        // No panic, no content
    }

    #[test]
    fn top_bar_narrow_terminal() {
        let info = default_info();
        let theme = Theme::default();
        let bar = TopBar::new(&info, &theme);

        // Very narrow — should degrade gracefully
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        bar.render(area, &mut buf);

        let content: String = (0..area.width)
            .map(|x| {
                buf.cell((x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();

        assert!(content.contains("sonnet"), "should still show model name");
    }

    #[test]
    fn context_color_thresholds() {
        let theme = Theme::default();

        let low = context_color(0.30, &theme);
        assert_eq!(low, theme.success_style());

        let mid = context_color(0.60, &theme);
        assert_eq!(mid, theme.warning_style());

        let high = context_color(0.80, &theme);
        assert_eq!(high, theme.error_style());
    }

    #[test]
    fn format_tokens_units() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(8_200), "8.2k");
        assert_eq!(format_tokens(200_000), "200.0k");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn shorten_path_under_limit() {
        assert_eq!(shorten_path("~/tower/imp", 30), "~/tower/imp");
    }

    #[test]
    fn shorten_path_over_limit() {
        let long = "/Users/asher/very/long/deeply/nested/project/path";
        let short = shorten_path(long, 25);
        assert!(short.len() <= 27); // …/ prefix + up to 25
        assert!(short.starts_with('…'));
    }
}
