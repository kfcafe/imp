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

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.filtered().len();
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

    pub fn selected_model(&self) -> Option<&ModelMeta> {
        let filtered = self.filtered();
        filtered.get(self.selected).copied()
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

        // Group by provider
        let mut current_provider = String::new();
        let mut row: usize = 0;

        for (i, model) in filtered.iter().enumerate() {
            if row >= inner.height as usize {
                break;
            }

            // Provider header
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
            let price_str = format!(
                "${:.2}/{:.2}",
                model.pricing.input_per_mtok, model.pricing.output_per_mtok
            );

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

        if filtered.is_empty() {
            let line = Line::from(Span::styled(
                "  No matching models",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
        }
    }
}
