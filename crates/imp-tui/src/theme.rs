use imp_llm::ThinkingLevel;
use ratatui::style::{Color, Modifier, Style};

/// Color theme for the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub accent: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub muted: Color,
    pub border: Color,
    pub user_prefix: Color,
    pub tool_name: Color,
    pub code_bg: Color,
    pub header_fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            accent: Color::Cyan,
            error: Color::Red,
            warning: Color::Yellow,
            success: Color::Green,
            muted: Color::DarkGray,
            border: Color::DarkGray,
            user_prefix: Color::Cyan,
            tool_name: Color::Magenta,
            code_bg: Color::Rgb(40, 40, 40),
            header_fg: Color::White,
            selection_bg: Color::Rgb(60, 60, 100),
            selection_fg: Color::White,
        }
    }
}

impl Theme {
    pub fn style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn warning_style(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn success_style(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn bold_style(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    pub fn italic_style(&self) -> Style {
        Style::default().add_modifier(Modifier::ITALIC)
    }

    pub fn code_inline_style(&self) -> Style {
        Style::default().fg(Color::Yellow).bg(self.code_bg)
    }

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.header_fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_style(&self) -> Style {
        Style::default().fg(self.selection_fg).bg(self.selection_bg)
    }

    /// Border color based on thinking level.
    pub fn thinking_border_color(&self, level: ThinkingLevel) -> Color {
        match level {
            ThinkingLevel::Off => self.border,
            ThinkingLevel::Minimal => Color::Rgb(80, 80, 120),
            ThinkingLevel::Low => Color::Rgb(100, 100, 180),
            ThinkingLevel::Medium => self.accent,
            ThinkingLevel::High => Color::Rgb(100, 200, 255),
            ThinkingLevel::XHigh => Color::Rgb(150, 230, 255),
        }
    }
}
