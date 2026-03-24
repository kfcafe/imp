use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

/// Selection state for a single option in the ask overlay.
#[derive(Debug, Clone)]
pub struct AskOption {
    pub label: String,
    pub description: Option<String>,
    pub checked: bool, // only used in multi-select mode
}

/// The mode of the ask overlay.
#[derive(Debug, Clone, PartialEq)]
pub enum AskMode {
    SingleSelect,
    MultiSelect,
    FreeText,
}

/// State for the ask overlay bar.
#[derive(Debug, Clone)]
pub struct AskState {
    pub question: String,
    pub context: String,
    pub options: Vec<AskOption>,
    pub mode: AskMode,
    pub cursor: usize,       // highlighted option index
    pub input: String,       // user's typed text
    pub input_cursor: usize, // cursor position in input
    pub input_active: bool,  // true when user is typing (options dimmed)
}

impl AskState {
    pub fn new(question: String, context: String, options: Vec<AskOption>, multi: bool) -> Self {
        let input_active = options.is_empty();
        let mode = if options.is_empty() {
            AskMode::FreeText
        } else if multi {
            AskMode::MultiSelect
        } else {
            AskMode::SingleSelect
        };
        Self {
            question,
            context,
            options,
            mode,
            cursor: 0,
            input: String::new(),
            input_cursor: 0,
            input_active,
        }
    }

    /// Height needed to render this overlay.
    pub fn height(&self) -> u16 {
        let mut h: u16 = 1; // question line
        if !self.context.is_empty() {
            h += 1; // context line
        }
        if !self.options.is_empty() {
            h += self.options.len() as u16; // one per option
            h += 1; // blank line between options and input
        }
        h += 1; // input line
        h += 1; // bottom border/hint line
        h
    }

    /// Move cursor up.
    pub fn cursor_up(&mut self) {
        if !self.options.is_empty() {
            self.input_active = false;
            if self.cursor > 0 {
                self.cursor -= 1;
            } else {
                self.cursor = self.options.len() - 1;
            }
        }
    }

    /// Move cursor down.
    pub fn cursor_down(&mut self) {
        if !self.options.is_empty() {
            self.input_active = false;
            if self.cursor < self.options.len() - 1 {
                self.cursor += 1;
            } else {
                self.cursor = 0;
            }
        }
    }

    /// Toggle checkbox in multi-select mode.
    pub fn toggle_current(&mut self) {
        if self.mode == AskMode::MultiSelect && !self.input_active {
            if let Some(opt) = self.options.get_mut(self.cursor) {
                opt.checked = !opt.checked;
            }
        }
    }

    /// Tab: copy highlighted option text into the input editor.
    pub fn tab_to_edit(&mut self) {
        if !self.options.is_empty() && !self.input_active {
            self.input = self.options[self.cursor].label.clone();
            self.input_cursor = self.input.len();
            self.input_active = true;
        }
    }

    /// Quick-select by number (1-9).
    pub fn quick_select(&mut self, n: usize) -> bool {
        if n > 0 && n <= self.options.len() && !self.input_active {
            self.cursor = n - 1;
            true
        } else {
            false
        }
    }

    /// Insert a character into the input.
    pub fn type_char(&mut self, ch: char) {
        self.input_active = true;
        self.input.insert(self.input_cursor, ch);
        self.input_cursor += ch.len_utf8();
    }

    /// Backspace in the input.
    pub fn backspace(&mut self) {
        if self.input_cursor > 0 && !self.input.is_empty() {
            let prev = self.input[..self.input_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.drain(prev..self.input_cursor);
            self.input_cursor = prev;
        }
        // If input is now empty and we have options, go back to option mode
        if self.input.is_empty() && !self.options.is_empty() {
            self.input_active = false;
        }
    }

    /// Get the final answer when Enter is pressed.
    pub fn confirm(&self) -> AskResult {
        if self.input_active && !self.input.is_empty() {
            // User typed custom text
            return AskResult::Text(self.input.clone());
        }

        match self.mode {
            AskMode::FreeText => AskResult::Text(self.input.clone()),
            AskMode::SingleSelect => {
                if self.options.is_empty() {
                    AskResult::Text(self.input.clone())
                } else {
                    AskResult::Selected(vec![self.cursor])
                }
            }
            AskMode::MultiSelect => {
                let selected: Vec<usize> = self
                    .options
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| o.checked)
                    .map(|(i, _)| i)
                    .collect();
                if selected.is_empty() {
                    // If nothing checked, use the highlighted one
                    AskResult::Selected(vec![self.cursor])
                } else {
                    AskResult::Selected(selected)
                }
            }
        }
    }
}

/// Result of the ask interaction.
#[derive(Debug)]
pub enum AskResult {
    Selected(Vec<usize>),
    Text(String),
}

/// Widget that renders the ask overlay bar.
pub struct AskBar<'a> {
    state: &'a AskState,
}

impl<'a> AskBar<'a> {
    pub fn new(state: &'a AskState) -> Self {
        Self { state }
    }
}

impl Widget for AskBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let s = self.state;
        let dim = Style::default().fg(Color::DarkGray);
        let highlight = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let normal = Style::default().fg(Color::White);
        let question_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let mut y = area.y;
        let w = area.width as usize;

        // Question
        let q = truncate(&s.question, w);
        buf.set_line(
            area.x,
            y,
            &Line::from(Span::styled(q, question_style)),
            area.width,
        );
        y += 1;

        // Context (if any)
        if !s.context.is_empty() {
            let c = truncate(&s.context, w);
            buf.set_line(area.x, y, &Line::from(Span::styled(c, dim)), area.width);
            y += 1;
        }

        // Options
        if !s.options.is_empty() {
            for (i, opt) in s.options.iter().enumerate() {
                if y >= area.y + area.height {
                    break;
                }

                let is_highlighted = i == s.cursor && !s.input_active;

                let prefix = match s.mode {
                    AskMode::MultiSelect => {
                        if opt.checked {
                            "[x] "
                        } else {
                            "[ ] "
                        }
                    }
                    AskMode::SingleSelect => {
                        if is_highlighted {
                            " ❯ "
                        } else {
                            "   "
                        }
                    }
                    AskMode::FreeText => "",
                };

                let num = format!("[{}] ", i + 1);
                let label = &opt.label;
                let desc = opt.description.as_deref().unwrap_or("");

                let style = if s.input_active {
                    dim // dim all options when user is typing
                } else if is_highlighted {
                    highlight
                } else {
                    normal
                };

                let mut spans = vec![
                    Span::styled(prefix, style),
                    Span::styled(label.to_string(), style),
                ];
                if !desc.is_empty() {
                    spans.push(Span::styled(format!(" — {desc}"), dim));
                }
                // Right-align the number hint
                let content_len: usize = spans.iter().map(|s| s.content.len()).sum();
                let num_hint_style = if s.input_active {
                    dim
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                if content_len + num.len() + 1 < w {
                    let padding = w - content_len - num.len();
                    spans.push(Span::raw(" ".repeat(padding)));
                    spans.push(Span::styled(num, num_hint_style));
                }

                buf.set_line(area.x, y, &Line::from(spans), area.width);
                y += 1;
            }

            // Blank line before input
            y += 1;
        }

        // Input line
        if y < area.y + area.height {
            let cursor_char = if s.input_active { "│" } else { " " };
            let input_display = if s.input.is_empty() && !s.input_active {
                "type to answer freely…".to_string()
            } else {
                s.input.clone()
            };

            let input_style = if s.input_active || s.mode == AskMode::FreeText {
                normal
            } else {
                dim
            };

            let line = Line::from(vec![
                Span::styled("❯ ", Style::default().fg(Color::Cyan)),
                Span::styled(input_display, input_style),
                Span::styled(cursor_char, Style::default().fg(Color::Cyan)),
            ]);
            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }

        // Hint line
        if y < area.y + area.height {
            let hints = match s.mode {
                AskMode::FreeText => "Enter: send  Esc: skip",
                AskMode::SingleSelect => "↑↓: navigate  Tab: edit  Enter: pick  Esc: skip",
                AskMode::MultiSelect => {
                    "↑↓: navigate  Space: toggle  Tab: edit  Enter: confirm  Esc: skip"
                }
            };
            buf.set_line(area.x, y, &Line::from(Span::styled(hints, dim)), area.width);
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_select_confirm() {
        let opts = vec![
            AskOption {
                label: "React".into(),
                description: None,
                checked: false,
            },
            AskOption {
                label: "Svelte".into(),
                description: None,
                checked: false,
            },
        ];
        let mut state = AskState::new("Pick one".into(), String::new(), opts, false);
        assert_eq!(state.mode, AskMode::SingleSelect);

        state.cursor_down();
        let result = state.confirm();
        assert!(matches!(result, AskResult::Selected(v) if v == vec![1]));
    }

    #[test]
    fn multi_select_toggle() {
        let opts = vec![
            AskOption {
                label: "A".into(),
                description: None,
                checked: false,
            },
            AskOption {
                label: "B".into(),
                description: None,
                checked: false,
            },
            AskOption {
                label: "C".into(),
                description: None,
                checked: false,
            },
        ];
        let mut state = AskState::new("Pick".into(), String::new(), opts, true);
        assert_eq!(state.mode, AskMode::MultiSelect);

        state.toggle_current(); // toggle A
        state.cursor_down();
        state.cursor_down();
        state.toggle_current(); // toggle C

        let result = state.confirm();
        assert!(matches!(result, AskResult::Selected(v) if v == vec![0, 2]));
    }

    #[test]
    fn free_text_input() {
        let mut state = AskState::new("What color?".into(), String::new(), vec![], false);
        assert_eq!(state.mode, AskMode::FreeText);
        assert!(state.input_active);

        state.type_char('r');
        state.type_char('e');
        state.type_char('d');

        let result = state.confirm();
        assert!(matches!(result, AskResult::Text(t) if t == "red"));
    }

    #[test]
    fn tab_copies_option_to_input() {
        let opts = vec![AskOption {
            label: "React".into(),
            description: None,
            checked: false,
        }];
        let mut state = AskState::new("Pick".into(), String::new(), opts, false);

        state.tab_to_edit();
        assert!(state.input_active);
        assert_eq!(state.input, "React");

        // Modify it
        state.type_char('!');
        let result = state.confirm();
        assert!(matches!(result, AskResult::Text(t) if t == "React!"));
    }

    #[test]
    fn typing_activates_input_mode() {
        let opts = vec![AskOption {
            label: "A".into(),
            description: None,
            checked: false,
        }];
        let mut state = AskState::new("Pick".into(), String::new(), opts, false);
        assert!(!state.input_active);

        state.type_char('c');
        assert!(state.input_active);
        assert_eq!(state.input, "c");
    }

    #[test]
    fn backspace_returns_to_option_mode() {
        let opts = vec![AskOption {
            label: "A".into(),
            description: None,
            checked: false,
        }];
        let mut state = AskState::new("Pick".into(), String::new(), opts, false);

        state.type_char('x');
        assert!(state.input_active);

        state.backspace();
        assert!(!state.input_active); // back to option mode
    }

    #[test]
    fn quick_select() {
        let opts = vec![
            AskOption {
                label: "A".into(),
                description: None,
                checked: false,
            },
            AskOption {
                label: "B".into(),
                description: None,
                checked: false,
            },
        ];
        let mut state = AskState::new("Pick".into(), String::new(), opts, false);

        assert!(state.quick_select(2));
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn height_calculation() {
        let opts = vec![
            AskOption {
                label: "A".into(),
                description: None,
                checked: false,
            },
            AskOption {
                label: "B".into(),
                description: None,
                checked: false,
            },
        ];
        let state = AskState::new("Q".into(), "ctx".into(), opts, false);
        // question(1) + context(1) + 2 options(2) + blank(1) + input(1) + hints(1) = 7
        assert_eq!(state.height(), 7);
    }
}
