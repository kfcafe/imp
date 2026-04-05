use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::animation::format_elapsed;
use crate::theme::Theme;
use crate::views::status::StatusInfo;

/// Single-line header bar focused on conversation identity.
///
/// Format: `~/tower/imp · fix top bar chat title`
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
        if area.height == 0 || area.width == 0 {
            return;
        }

        let sep = Span::styled(" · ", self.theme.muted_style());
        let cwd = shorten_path(&self.info.cwd, 36);
        let title = self.info.session_name.trim();

        let left = if title.is_empty() {
            vec![Span::styled(cwd, self.theme.muted_style())]
        } else {
            vec![
                Span::styled(cwd, self.theme.muted_style()),
                sep,
                Span::styled(title.to_string(), self.theme.accent_style()),
            ]
        };

        let right = self
            .info
            .turn_elapsed
            .map(|elapsed| vec![Span::styled(format_elapsed(elapsed), self.theme.muted_style())])
            .unwrap_or_default();

        let left_width: usize = left.iter().map(|span| span.content.chars().count()).sum();
        let right_width: usize = right.iter().map(|span| span.content.chars().count()).sum();
        let available = area.width as usize;

        let line = if !right.is_empty() && available > left_width + right_width + 2 {
            let gap = available.saturating_sub(left_width + right_width);
            let mut spans = left;
            spans.push(Span::raw(" ".repeat(gap)));
            spans.extend(right);
            Line::from(spans)
        } else if !right.is_empty() && left_width == 0 {
            Line::from(right)
        } else {
            Line::from(left)
        };

        buf.set_line(area.x, area.y, &line, area.width);
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
    use imp_core::config::AnimationLevel;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn default_info() -> StatusInfo {
        StatusInfo {
            model: "sonnet".into(),
            input_tokens: 8_200,
            output_tokens: 1_500,
            current_context_tokens: 8_200,
            cost: 0.12,
            context_percent: 0.04,
            context_window: 200_000,
            show_cost: true,
            show_context_usage: true,
            cwd: "~/tower/imp".into(),
            session_name: "debug-oauth".into(),
            is_streaming: false,
            active_tools: 0,
            turn_elapsed: None,
            tick: 0,
            animation_level: AnimationLevel::Minimal,
            ..StatusInfo::default()
        }
    }

    fn render_to_string(info: &StatusInfo, width: u16) -> String {
        let theme = Theme::default();
        let bar = TopBar::new(info, &theme);
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        bar.render(area, &mut buf);

        (0..area.width)
            .map(|x| {
                buf.cell((x, 0))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect()
    }

    #[test]
    fn top_bar_renders_cwd_and_title() {
        let info = default_info();
        let content = render_to_string(&info, 100);

        assert!(content.contains("~/tower/imp"), "should contain cwd");
        assert!(content.contains("debug-oauth"), "should contain chat title");
        assert!(
            !content.contains("sonnet"),
            "should no longer contain model name"
        );
    }

    #[test]
    fn top_bar_renders_elapsed_when_streaming() {
        let mut info = default_info();
        info.turn_elapsed = Some(std::time::Duration::from_secs(7));
        let content = render_to_string(&info, 100);

        assert!(content.contains("7s"));
    }

    #[test]
    fn top_bar_zero_height_noop() {
        let info = default_info();
        let theme = Theme::default();
        let bar = TopBar::new(&info, &theme);

        let area = Rect::new(0, 0, 80, 0);
        let mut buf = Buffer::empty(area);
        bar.render(area, &mut buf);
    }

    #[test]
    fn top_bar_narrow_terminal_prefers_identity_over_elapsed() {
        let mut info = default_info();
        info.turn_elapsed = Some(std::time::Duration::from_secs(75));
        let content = render_to_string(&info, 20);

        assert!(
            content.contains("imp") || content.contains("debug"),
            "should still show cwd tail or chat title"
        );
    }

    #[test]
    fn shorten_path_under_limit() {
        assert_eq!(shorten_path("~/tower/imp", 30), "~/tower/imp");
    }

    #[test]
    fn shorten_path_over_limit() {
        let long = "/Users/asher/very/long/deeply/nested/project/path";
        let short = shorten_path(long, 25);
        assert!(short.len() <= 27);
        assert!(short.starts_with('…'));
    }
}
