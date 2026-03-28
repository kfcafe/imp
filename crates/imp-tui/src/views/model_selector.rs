use imp_llm::model::ModelMeta;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// State for the model selector overlay.
#[derive(Debug, Clone)]
pub struct ModelSelectorState {
    pub models: Vec<ModelMeta>,
    pub filter: String,
    pub selected: usize,
    pub current_model: String,
}

pub enum ModelSelection<'a> {
    Builtin(&'a ModelMeta),
    Custom(String),
}

impl ModelSelectorState {
    pub fn new(models: Vec<ModelMeta>, current_model: String) -> Self {
        Self {
            models,
            filter: String::new(),
            selected: 0,
            current_model,
        }
    }

    pub fn filtered(&self) -> Vec<&ModelMeta> {
        if self.filter.is_empty() {
            self.models.iter().collect()
        } else {
            let lower = self.filter.to_lowercase();
            self.models
                .iter()
                .filter(|m| {
                    m.name.to_lowercase().contains(&lower)
                        || m.id.to_lowercase().contains(&lower)
                        || m.provider.to_lowercase().contains(&lower)
                })
                .collect()
        }
    }

    pub fn custom_model(&self) -> Option<String> {
        let trimmed = self.filter.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn option_count(&self) -> usize {
        self.filtered().len() + usize::from(self.custom_model().is_some())
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.option_count();
        if self.selected + 1 < count {
            self.selected += 1;
        }
    }

    pub fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
    }

    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.selected = 0;
    }

    pub fn selected_choice(&self) -> Option<ModelSelection<'_>> {
        let filtered = self.filtered();

        if let Some(model) = filtered.get(self.selected).copied() {
            return Some(ModelSelection::Builtin(model));
        }

        let custom_index = filtered.len();
        self.custom_model().and_then(|custom| {
            (self.selected == custom_index).then_some(ModelSelection::Custom(custom))
        })
    }
}

/// Model selector overlay widget.
pub struct ModelSelectorView<'a> {
    state: &'a ModelSelectorState,
    theme: &'a Theme,
}

impl<'a> ModelSelectorView<'a> {
    pub fn new(state: &'a ModelSelectorState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for ModelSelectorView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 5 || area.width < 20 {
            return;
        }

        Clear.render(area, buf);

        let title = if self.state.filter.is_empty() {
            " Select Model ".to_string()
        } else {
            format!(" Select Model [{}] ", self.state.filter)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let filtered = self.state.filtered();
        let mut row: usize = 0;
        let custom_model = self.state.custom_model();

        let mut current_provider = String::new();

        for (i, model) in filtered.iter().enumerate() {
            if row >= inner.height as usize {
                break;
            }

            if model.provider != current_provider {
                current_provider = model.provider.clone();
                let header_line = Line::from(Span::styled(
                    format!("  {}", current_provider.to_uppercase()),
                    Style::default()
                        .fg(self.theme.muted)
                        .add_modifier(Modifier::BOLD),
                ));
                buf.set_line(inner.x, inner.y + row as u16, &header_line, inner.width);
                row += 1;
                if row >= inner.height as usize {
                    break;
                }
            }

            let is_selected = i == self.state.selected;
            let is_current = model.id == self.state.current_model;

            let marker = if is_current { "✓ " } else { "  " };
            let style = if is_selected {
                self.theme.selected_style()
            } else {
                Style::default()
            };

            let context_str = format!("{}k", model.context_window / 1000);
            let price_str =
                if model.pricing.input_per_mtok == 0.0 && model.pricing.output_per_mtok == 0.0 {
                    "n/a".to_string()
                } else {
                    format!(
                        "${:.2}/{:.2}",
                        model.pricing.input_per_mtok, model.pricing.output_per_mtok
                    )
                };

            let line = Line::from(vec![
                Span::styled(format!("    {marker}"), self.theme.accent_style()),
                Span::styled(model.name.clone(), style),
                Span::raw("  "),
                Span::styled(context_str, self.theme.muted_style()),
                Span::raw("  "),
                Span::styled(price_str, self.theme.muted_style()),
            ]);

            buf.set_line(inner.x, inner.y + row as u16, &line, inner.width);
            row += 1;
        }

        if let Some(ref custom_model) = custom_model {
            if row < inner.height as usize && !filtered.is_empty() {
                let spacer = Line::from(Span::styled(
                    "  Custom",
                    Style::default()
                        .fg(self.theme.muted)
                        .add_modifier(Modifier::BOLD),
                ));
                buf.set_line(inner.x, inner.y + row as u16, &spacer, inner.width);
                row += 1;
            }

            if row < inner.height as usize {
                let custom_index = filtered.len();
                let is_selected = self.state.selected == custom_index;
                let is_current = custom_model == &self.state.current_model;
                let marker = if is_current { "✓ " } else { "  " };
                let style = if is_selected {
                    self.theme.selected_style()
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(format!("  {marker}"), self.theme.accent_style()),
                    Span::styled("Use custom model: ", self.theme.muted_style()),
                    Span::styled(custom_model, style),
                ]);
                buf.set_line(inner.x, inner.y + row as u16, &line, inner.width);
            }
        }

        if filtered.is_empty() && custom_model.is_none() {
            let line = Line::from(Span::styled(
                "  No matching models",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_llm::model::{Capabilities, ModelPricing};

    fn test_model(id: &str) -> ModelMeta {
        ModelMeta {
            id: id.into(),
            provider: "openai".into(),
            name: id.into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        }
    }

    #[test]
    fn custom_model_is_available_after_builtin_matches() {
        let mut state = ModelSelectorState::new(vec![test_model("gpt-4o")], "gpt-4o".into());
        state.push_filter('g');
        state.push_filter('p');
        state.push_filter('t');
        state.push_filter('-');
        state.push_filter('4');
        state.push_filter('o');

        match state.selected_choice() {
            Some(ModelSelection::Builtin(model)) => assert_eq!(model.id, "gpt-4o"),
            _ => panic!("expected builtin model selection"),
        }

        state.move_down();
        match state.selected_choice() {
            Some(ModelSelection::Custom(model)) => assert_eq!(model, "gpt-4o"),
            _ => panic!("expected custom model selection after builtin matches"),
        }
    }

    #[test]
    fn custom_model_is_selected_when_no_builtin_matches() {
        let mut state = ModelSelectorState::new(vec![test_model("gpt-5.4")], "gpt-5.4".into());
        state.push_filter('g');
        state.push_filter('p');
        state.push_filter('t');
        state.push_filter('-');
        state.push_filter('4');
        state.push_filter('o');

        state.move_down();
        match state.selected_choice() {
            Some(ModelSelection::Custom(model)) => assert_eq!(model, "gpt-4o"),
            _ => panic!("expected custom model selection"),
        }
    }
}
