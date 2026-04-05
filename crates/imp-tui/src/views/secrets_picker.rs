use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use imp_llm::model::ProviderRegistry;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct SecretProviderOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub configured: bool,
}

pub fn secret_providers(registry: &ProviderRegistry) -> Vec<SecretProviderOption> {
    let mut providers: Vec<SecretProviderOption> = registry
        .list()
        .iter()
        .filter(|provider| !matches!(provider.id, "anthropic" | "openai" | "openai-codex"))
        .map(|provider| SecretProviderOption {
            id: provider.id.to_string(),
            label: provider.name.to_string(),
            description: if provider.docs_url.is_empty() {
                "Secure API/service secrets".into()
            } else {
                format!("Secure API/service secrets · {}", provider.docs_url)
            },
            configured: false,
        })
        .collect();

    providers.sort_by(|a, b| a.label.cmp(&b.label));
    providers
}

#[derive(Debug, Clone)]
pub struct SecretsPickerState {
    pub providers: Vec<SecretProviderOption>,
    pub selected: usize,
}

impl SecretsPickerState {
    pub fn new(providers: Vec<SecretProviderOption>) -> Self {
        Self {
            providers,
            selected: 0,
        }
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

    pub fn selected_provider(&self) -> Option<&SecretProviderOption> {
        self.providers.get(self.selected)
    }
}

pub struct SecretsPickerView<'a> {
    state: &'a SecretsPickerState,
    theme: &'a Theme,
}

impl<'a> SecretsPickerView<'a> {
    pub fn new(state: &'a SecretsPickerState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for SecretsPickerView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 6 || area.width < 20 {
            return;
        }

        Clear.render(area, buf);
        let block = Block::default()
            .title(" Secure Secrets ")
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.providers.is_empty() {
            let line = Line::from(Span::styled(
                "  No providers available",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        let footer = "Enter: configure provider · Esc: cancel";
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

            let status = if provider.configured {
                vec![
                    Span::raw("  "),
                    Span::styled("✓ configured", self.theme.success_style()),
                ]
            } else {
                Vec::new()
            };

            let mut spans = vec![
                Span::styled(
                    if is_selected { " ▸ " } else { "   " },
                    self.theme.accent_style(),
                ),
                Span::styled(&provider.label, row_style),
                Span::raw("  "),
                Span::styled(&provider.description, self.theme.muted_style()),
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
    fn secrets_picker_excludes_oauth_providers() {
        let registry = ProviderRegistry::with_builtins();
        let providers = secret_providers(&registry);
        let ids: Vec<&str> = providers.iter().map(|provider| provider.id.as_str()).collect();
        assert!(ids.contains(&"exa"));
        assert!(!ids.contains(&"anthropic"));
        assert!(!ids.contains(&"openai"));
    }
}
