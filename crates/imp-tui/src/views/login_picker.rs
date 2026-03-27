use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct LoginProviderOption {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub logged_in: bool,
}

pub fn oauth_login_providers() -> Vec<LoginProviderOption> {
    vec![
        LoginProviderOption {
            id: "anthropic",
            label: "Anthropic",
            description: "Claude Max/Pro subscription",
            logged_in: false,
        },
        LoginProviderOption {
            id: "openai",
            label: "OpenAI",
            description: "OpenAI / ChatGPT account (OAuth)",
            logged_in: false,
        },
    ]
}

#[derive(Debug, Clone)]
pub struct LoginPickerState {
    pub providers: Vec<LoginProviderOption>,
    pub selected: usize,
}

impl LoginPickerState {
    pub fn new(providers: Vec<LoginProviderOption>) -> Self {
        Self {
            providers,
            selected: 0,
        }
    }

    pub fn oauth() -> Self {
        Self::new(oauth_login_providers())
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.providers.len() {
            self.selected += 1;
        }
    }

    pub fn selected_provider(&self) -> Option<&LoginProviderOption> {
        self.providers.get(self.selected)
    }
}

pub struct LoginPickerView<'a> {
    state: &'a LoginPickerState,
    theme: &'a Theme,
}

impl<'a> LoginPickerView<'a> {
    pub fn new(state: &'a LoginPickerState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for LoginPickerView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 6 || area.width < 20 {
            return;
        }

        Clear.render(area, buf);
        let block = Block::default()
            .title(" OAuth Login ")
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.providers.is_empty() {
            let line = Line::from(Span::styled(
                "  No OAuth providers available",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        let footer = "Use `imp login <provider>` in your shell for API-key providers.";
        let footer_y = inner.y + inner.height.saturating_sub(1);

        for (i, provider) in self.state.providers.iter().enumerate() {
            if inner.y + i as u16 >= footer_y {
                break;
            }

            let is_selected = i == self.state.selected;
            let row_style = if is_selected {
                self.theme.selected_style()
            } else {
                Style::default()
            };

            let status = if provider.logged_in {
                vec![
                    Span::raw("  "),
                    Span::styled("✓ logged in", self.theme.success_style()),
                ]
            } else {
                Vec::new()
            };

            let mut spans = vec![
                Span::styled(
                    if is_selected { " ▸ " } else { "   " },
                    self.theme.accent_style(),
                ),
                Span::styled(provider.label, row_style),
                Span::raw("  "),
                Span::styled(provider.description, self.theme.muted_style()),
            ];
            spans.extend(status);
            let line = Line::from(spans);
            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }

        let footer_line = Line::from(Span::styled(footer, self.theme.muted_style()));
        buf.set_line(inner.x, footer_y, &footer_line, inner.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_picker_includes_anthropic_and_openai() {
        let state = LoginPickerState::oauth();
        let ids: Vec<&str> = state.providers.iter().map(|provider| provider.id).collect();
        assert_eq!(ids, vec!["anthropic", "openai"]);
    }

    #[test]
    fn picker_selection_moves_with_bounds() {
        let mut state = LoginPickerState::oauth();
        assert_eq!(
            state.selected_provider().map(|provider| provider.id),
            Some("anthropic")
        );

        state.move_down();
        assert_eq!(
            state.selected_provider().map(|provider| provider.id),
            Some("openai")
        );

        state.move_down();
        assert_eq!(
            state.selected_provider().map(|provider| provider.id),
            Some("openai")
        );

        state.move_up();
        assert_eq!(
            state.selected_provider().map(|provider| provider.id),
            Some("anthropic")
        );
    }
}
