use std::collections::BTreeMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Widget};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlashCommandKind {
    Builtin,
    Extension,
    Workflow,
    Skill,
}

impl SlashCommandKind {
    fn label(self) -> &'static str {
        match self {
            Self::Builtin => "commands",
            Self::Extension => "extensions",
            Self::Workflow => "workflows",
            Self::Skill => "skills",
        }
    }
}

/// A slash command definition.
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub kind: SlashCommandKind,
}

/// Merge built-in and extension-provided slash commands for discovery menus.
pub fn merge_extension_commands(
    mut commands: Vec<SlashCommand>,
    extension_commands: impl IntoIterator<Item = (String, String)>,
) -> Vec<SlashCommand> {
    let mut by_name: BTreeMap<String, SlashCommand> = commands
        .drain(..)
        .map(|command| (command.name.clone(), command))
        .collect();

    for (name, description) in extension_commands {
        by_name.entry(name.clone()).or_insert_with(|| SlashCommand {
            name,
            kind: SlashCommandKind::Extension,
            description: if description.trim().is_empty() {
                "Extension command".into()
            } else {
                description
            },
        });
    }

    by_name.into_values().collect()
}

/// Merge skill commands into the slash menu without overriding real commands.
pub fn merge_skill_commands(
    mut commands: Vec<SlashCommand>,
    skills: impl IntoIterator<Item = (String, String)>,
) -> Vec<SlashCommand> {
    let mut by_name: BTreeMap<String, SlashCommand> = commands
        .drain(..)
        .map(|command| (command.name.clone(), command))
        .collect();

    for (name, description) in skills {
        by_name.entry(name.clone()).or_insert_with(|| SlashCommand {
            name,
            kind: SlashCommandKind::Skill,
            description: if description.trim().is_empty() {
                "Skill".into()
            } else {
                format!("Skill: {description}")
            },
        });
    }

    by_name.into_values().collect()
}

/// Merge workflow commands into the slash menu without overriding real commands.
pub fn merge_workflow_commands(
    mut commands: Vec<SlashCommand>,
    workflows: impl IntoIterator<Item = (String, String)>,
) -> Vec<SlashCommand> {
    let mut by_name: BTreeMap<String, SlashCommand> = commands
        .drain(..)
        .map(|command| (command.name.clone(), command))
        .collect();

    for (name, description) in workflows {
        by_name.entry(name.clone()).or_insert_with(|| SlashCommand {
            name,
            kind: SlashCommandKind::Workflow,
            description: if description.trim().is_empty() {
                "Workflow".into()
            } else {
                format!("Workflow: {description}")
            },
        });
    }

    by_name.into_values().collect()
}

pub fn builtin_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "improve".into(),
            description: "Switch workflow mode to Improve in a sandbox branch/worktree".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "improve-safe".into(),
            description: "Switch workflow mode to research-only Improve".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "improve-merge".into(),
            description: "Merge active Improve branch after reviewing changelog".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "improve-help".into(),
            description: "Explain Improve autoresearch guardrails".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "eval".into(),
            description: "Save latest run as an eval candidate (/eval <expected> [--note ...] [--verifier ...])".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "status".into(),
            description: "Show active imp work status".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "autonomy".into(),
            description: "Set autonomy mode (/autonomy safe|local-auto|allow-all-local)".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "clean".into(),
            description: "Clean active sandbox/artifacts safely".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "loop".into(),
            description: "Loop current mana work or a prompt (/loop [message|continue])".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "queue".into(),
            description: "Show or clear queued follow-up prompts (/queue clear)".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "run".into(),
            description: "Set active mana run (/run <id>, /run clear)".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "stop".into(),
            description: "Stop active imp work and clear pending/queued loop work".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "scope".into(),
            description: "Set active mana scope (/scope <id>, /scope clear)".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "model".into(),
            description: "Select model".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "settings".into(),
            description: "Open settings".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "mana".into(),
            description: "Open mana work graph navigator".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "tree".into(),
            description: "Session tree view".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "fork".into(),
            description: "Fork session at current point".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "compact".into(),
            description: "Compact context".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "new".into(),
            description: "New session".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "resume".into(),
            description: "Resume/search sessions".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "name".into(),
            description: "Name current session".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "copy".into(),
            description: "Copy last response".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "export".into(),
            description: "Export session".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "personality".into(),
            description: "Customize imp personality".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "memory".into(),
            description: "View/edit agent memory".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "checkpoints".into(),
            description: "List recorded file checkpoints".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "restore-checkpoint".into(),
            description: "Restore files from a checkpoint by id or label".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "reload".into(),
            description: "Reload extensions".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "hotkeys".into(),
            description: "Show keyboard shortcuts".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "login".into(),
            description: "OAuth login for Anthropic or OpenAI/ChatGPT".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "secrets".into(),
            description: "Configure API keys / multi-field service secrets".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
            name: "setup".into(),
            description: "Run setup wizard".into(),
        },
        SlashCommand {
            kind: SlashCommandKind::Builtin,
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
        let show_kind_labels = filtered.windows(2).any(|pair| pair[0].kind != pair[1].kind);

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

            let kind_text = if show_kind_labels {
                format!(" [{:<10}]", cmd.kind.label())
            } else {
                String::new()
            };

            let line = Line::from(vec![
                Span::styled(indicator, row_style),
                Span::styled(name_text, name_style),
                Span::styled(kind_text, desc_style),
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
