use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

/// A slash command definition.
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
}

/// Built-in slash commands.
pub fn builtin_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand {
            name: "model".into(),
            description: "Select model".into(),
        },
        SlashCommand {
            name: "settings".into(),
            description: "Open settings".into(),
        },
        SlashCommand {
            name: "tree".into(),
            description: "Session tree view".into(),
        },
        SlashCommand {
            name: "fork".into(),
            description: "Fork session at current point".into(),
        },
        SlashCommand {
            name: "compact".into(),
            description: "Compact context".into(),
        },
        SlashCommand {
            name: "new".into(),
            description: "New session".into(),
        },
        SlashCommand {
            name: "resume".into(),
            description: "Resume last session".into(),
        },
        SlashCommand {
            name: "session".into(),
            description: "List sessions".into(),
        },
        SlashCommand {
            name: "name".into(),
            description: "Name current session".into(),
        },
        SlashCommand {
            name: "copy".into(),
            description: "Copy last response".into(),
        },
        SlashCommand {
            name: "export".into(),
            description: "Export session".into(),
        },
        SlashCommand {
            name: "memory".into(),
            description: "View/edit agent memory".into(),
        },
        SlashCommand {
            name: "reload".into(),
            description: "Reload extensions".into(),
        },
        SlashCommand {
            name: "hotkeys".into(),
            description: "Show keyboard shortcuts".into(),
        },
        SlashCommand {
            name: "login".into(),
            description: "Open OAuth provider picker (Anthropic, ChatGPT/OpenAI)".into(),
        },
        SlashCommand {
            name: "setup".into(),
            description: "Run setup wizard".into(),
        },
        SlashCommand {
            name: "quit".into(),
            description: "Quit".into(),
        },
    ]
}

/// State for the command palette.
#[derive(Debug, Clone)]
pub struct CommandPaletteState {
    pub commands: Vec<SlashCommand>,
    pub filter: String,
    pub selected: usize,
}

impl CommandPaletteState {
    pub fn new(commands: Vec<SlashCommand>) -> Self {
        Self {
            commands,
            filter: String::new(),
            selected: 0,
        }
    }

    pub fn filtered(&self) -> Vec<&SlashCommand> {
        if self.filter.is_empty() {
            self.commands.iter().collect()
        } else {
            let lower = self.filter.to_lowercase();
            let mut results: Vec<(usize, &SlashCommand)> = self
                .commands
                .iter()
                .filter_map(|c| {
                    let name = c.name.to_lowercase();
                    let desc = c.description.to_lowercase();
                    // Exact prefix gets priority 0, contains gets 1, description match gets 2
                    if name.starts_with(&lower) {
                        Some((0, c))
                    } else if name.contains(&lower) {
                        Some((1, c))
                    } else if desc.contains(&lower) {
                        Some((2, c))
                    } else {
                        None
                    }
                })
                .collect();
            results.sort_by_key(|(priority, _)| *priority);
            results.into_iter().map(|(_, c)| c).collect()
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

    pub fn selected_command(&self) -> Option<&SlashCommand> {
        let filtered = self.filtered();
        filtered.get(self.selected).copied()
    }
}

/// Command palette overlay widget (shown above the editor).
pub struct CommandPaletteView<'a> {
    state: &'a CommandPaletteState,
    theme: &'a Theme,
}

impl<'a> CommandPaletteView<'a> {
    pub fn new(state: &'a CommandPaletteState, theme: &'a Theme) -> Self {
        Self { state, theme }
    }
}

impl Widget for CommandPaletteView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 20 {
            return;
        }

        Clear.render(area, buf);

        let title = if self.state.filter.is_empty() {
            " Commands ".to_string()
        } else {
            format!(" /{} ", self.state.filter)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.accent_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let filtered = self.state.filtered();
        let total = filtered.len();

        if total == 0 {
            let line = Line::from(Span::styled(
                "  No matching commands",
                self.theme.muted_style(),
            ));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        // Find the longest command name for alignment
        let max_name_len = filtered.iter().map(|c| c.name.len()).max().unwrap_or(0);

        // Scroll to keep selected visible
        let visible = inner.height as usize;
        let scroll_offset = if self.state.selected >= visible {
            self.state.selected - visible + 1
        } else {
            0
        };

        for (i, cmd) in filtered.iter().skip(scroll_offset).enumerate() {
            if i >= visible {
                break;
            }

            let abs_idx = scroll_offset + i;
            let is_selected = abs_idx == self.state.selected;

            // Selection indicator
            let indicator = if is_selected { " ▸ " } else { "   " };

            // Build the command name with / prefix, padded for alignment
            let name_text = format!("/{:<width$}", cmd.name, width = max_name_len);

            // Build the line with full-row highlight when selected
            let row_style = if is_selected {
                self.theme.selected_style()
            } else {
                Style::default()
            };

            let name_style = if is_selected {
                self.theme.selected_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };

            let desc_style = if is_selected {
                self.theme.selected_style()
            } else {
                self.theme.muted_style()
            };

            let line = Line::from(vec![
                Span::styled(indicator, row_style),
                Span::styled(name_text, name_style),
                Span::styled("  ", row_style),
                Span::styled(&cmd.description, desc_style),
            ]);

            // Fill the entire row with the background color first
            if is_selected {
                let fill = " ".repeat(inner.width as usize);
                buf.set_line(
                    inner.x,
                    inner.y + i as u16,
                    &Line::from(Span::styled(fill, row_style)),
                    inner.width,
                );
            }

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }

        // Scroll indicators
        if scroll_offset > 0 {
            let hint = Line::from(Span::styled("  ↑ more", self.theme.muted_style()));
            buf.set_line(inner.x + inner.width.saturating_sub(10), inner.y, &hint, 10);
        }
        if scroll_offset + visible < total {
            let y = inner.y + inner.height.saturating_sub(1);
            let hint = Line::from(Span::styled("  ↓ more", self.theme.muted_style()));
            buf.set_line(inner.x + inner.width.saturating_sub(10), y, &hint, 10);
        }

        // Footer hint
        if inner.height > 1 && total > 0 {
            let hint_y = area.y + area.height - 1;
            let hint_text = " ↑↓/Tab  Enter  Esc ";
            let hint_x = area.x + area.width.saturating_sub(hint_text.len() as u16 + 1);
            let hint_line = Line::from(Span::styled(hint_text, self.theme.muted_style()));
            buf.set_line(hint_x, hint_y, &hint_line, hint_text.len() as u16);
        }
    }
}
