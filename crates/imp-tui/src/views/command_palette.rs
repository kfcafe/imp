use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
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
            name: "reload".into(),
            description: "Reload extensions".into(),
        },
        SlashCommand {
            name: "hotkeys".into(),
            description: "Show keyboard shortcuts".into(),
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
            self.commands
                .iter()
                .filter(|c| c.name.to_lowercase().contains(&lower))
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
        if area.height < 3 || area.width < 15 {
            return;
        }

        Clear.render(area, buf);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());
        let inner = block.inner(area);
        block.render(area, buf);

        let filtered = self.state.filtered();

        for (i, cmd) in filtered.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let is_selected = i == self.state.selected;
            let style = if is_selected {
                self.theme.selected_style()
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!("  /{}", cmd.name), style),
                Span::raw("  "),
                Span::styled(cmd.description.clone(), self.theme.muted_style()),
            ]);

            buf.set_line(inner.x, inner.y + i as u16, &line, inner.width);
        }
    }
}
