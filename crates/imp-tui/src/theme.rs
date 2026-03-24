use imp_llm::ThinkingLevel;
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

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
    /// Deep blue-black with warm bronze, Aegean teal, and Mediterranean earth tones.
    fn default() -> Self {
        Self {
            bg: Color::Rgb(0x15, 0x18, 0x20),           // deep blue-black
            fg: Color::Rgb(0xe8, 0xe0, 0xd4),           // warm parchment
            accent: Color::Rgb(0x4a, 0x9a, 0x8c),       // Aegean teal
            error: Color::Rgb(0xc4, 0x7a, 0x5a),        // terracotta
            warning: Color::Rgb(0xe5, 0xa8, 0x4b),      // bright gold
            success: Color::Rgb(0x8a, 0x9a, 0x6b),      // sage green
            muted: Color::Rgb(0x6b, 0x65, 0x60),        // warm gray
            border: Color::Rgb(0x2d, 0x3a, 0x4a),       // slate blue
            user_prefix: Color::Rgb(0xd4, 0xa5, 0x74),  // warm bronze
            tool_name: Color::Rgb(0x8a, 0x6a, 0x8a),    // dusty purple
            code_bg: Color::Rgb(0x25, 0x2a, 0x35),      // darker blue-black
            header_fg: Color::Rgb(0xe8, 0xe0, 0xd4),    // parchment
            selection_bg: Color::Rgb(0x2d, 0x3a, 0x4a), // slate blue
            selection_fg: Color::Rgb(0xe8, 0xe0, 0xd4), // parchment
        }
    }
}

impl Theme {
    /// Load a named built-in theme.
    pub fn named(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            _ => Self::default(),
        }
    }

    /// Light theme for bright terminals.
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(0xfa, 0xf8, 0xf5),
            fg: Color::Rgb(0x2a, 0x2a, 0x2a),
            accent: Color::Rgb(0x2a, 0x7a, 0x6c),
            error: Color::Rgb(0xb0, 0x40, 0x30),
            warning: Color::Rgb(0xb0, 0x80, 0x20),
            success: Color::Rgb(0x50, 0x70, 0x40),
            muted: Color::Rgb(0x99, 0x95, 0x90),
            border: Color::Rgb(0xd0, 0xcc, 0xc4),
            user_prefix: Color::Rgb(0x90, 0x70, 0x40),
            tool_name: Color::Rgb(0x70, 0x50, 0x70),
            code_bg: Color::Rgb(0xf0, 0xec, 0xe6),
            header_fg: Color::Rgb(0x2a, 0x2a, 0x2a),
            selection_bg: Color::Rgb(0xd0, 0xdc, 0xe8),
            selection_fg: Color::Rgb(0x2a, 0x2a, 0x2a),
        }
    }

    /// Apply overrides from a TOML config section.
    pub fn apply_overrides(&mut self, overrides: &ThemeOverrides) {
        if let Some(ref c) = overrides.fg {
            if let Some(c) = parse_hex(c) {
                self.fg = c;
            }
        }
        if let Some(ref c) = overrides.bg {
            if let Some(c) = parse_hex(c) {
                self.bg = c;
            }
        }
        if let Some(ref c) = overrides.accent {
            if let Some(c) = parse_hex(c) {
                self.accent = c;
            }
        }
        if let Some(ref c) = overrides.error {
            if let Some(c) = parse_hex(c) {
                self.error = c;
            }
        }
        if let Some(ref c) = overrides.warning {
            if let Some(c) = parse_hex(c) {
                self.warning = c;
            }
        }
        if let Some(ref c) = overrides.success {
            if let Some(c) = parse_hex(c) {
                self.success = c;
            }
        }
        if let Some(ref c) = overrides.muted {
            if let Some(c) = parse_hex(c) {
                self.muted = c;
            }
        }
        if let Some(ref c) = overrides.border {
            if let Some(c) = parse_hex(c) {
                self.border = c;
            }
        }
        if let Some(ref c) = overrides.user_prefix {
            if let Some(c) = parse_hex(c) {
                self.user_prefix = c;
            }
        }
        if let Some(ref c) = overrides.tool_name {
            if let Some(c) = parse_hex(c) {
                self.tool_name = c;
            }
        }
        if let Some(ref c) = overrides.code_bg {
            if let Some(c) = parse_hex(c) {
                self.code_bg = c;
            }
        }
    }

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
        Style::default().fg(self.warning).bg(self.code_bg)
    }

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.header_fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_style(&self) -> Style {
        Style::default().fg(self.selection_fg).bg(self.selection_bg)
    }

    /// Border color progresses through palette warmth with thinking level.
    pub fn thinking_border_color(&self, level: ThinkingLevel) -> Color {
        match level {
            ThinkingLevel::Off => self.border,
            ThinkingLevel::Minimal => Color::Rgb(0x2d, 0x3a, 0x4a), // slate
            ThinkingLevel::Low => Color::Rgb(0x4a, 0x9a, 0x8c),     // teal
            ThinkingLevel::Medium => self.accent,                   // teal
            ThinkingLevel::High => Color::Rgb(0xd4, 0xa5, 0x74),    // bronze
            ThinkingLevel::XHigh => Color::Rgb(0xe5, 0xa8, 0x4b),   // gold
        }
    }
}

/// Config-driven theme overrides. All fields optional — only set ones override the base theme.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeOverrides {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub accent: Option<String>,
    pub error: Option<String>,
    pub warning: Option<String>,
    pub success: Option<String>,
    pub muted: Option<String>,
    pub border: Option<String>,
    pub user_prefix: Option<String>,
    pub tool_name: Option<String>,
    pub code_bg: Option<String>,
}

/// Parse a "#rrggbb" hex string into a ratatui Color.
fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_valid() {
        assert_eq!(parse_hex("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex("00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_hex("#151820"), Some(Color::Rgb(0x15, 0x18, 0x20)));
    }

    #[test]
    fn parse_hex_invalid() {
        assert_eq!(parse_hex("nope"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex(""), None);
    }

    #[test]
    fn default_theme_is_warm() {
        let t = Theme::default();
        // Bronze user prefix
        assert_eq!(t.user_prefix, Color::Rgb(0xd4, 0xa5, 0x74));
        // Teal accent
        assert_eq!(t.accent, Color::Rgb(0x4a, 0x9a, 0x8c));
    }

    #[test]
    fn overrides_apply() {
        let mut t = Theme::default();
        let overrides = ThemeOverrides {
            accent: Some("#ff0000".into()),
            ..Default::default()
        };
        t.apply_overrides(&overrides);
        assert_eq!(t.accent, Color::Rgb(255, 0, 0));
        // Other fields unchanged
        assert_eq!(t.user_prefix, Color::Rgb(0xd4, 0xa5, 0x74));
    }

    #[test]
    fn named_themes() {
        let default = Theme::named("default");
        assert_eq!(default.accent, Color::Rgb(0x4a, 0x9a, 0x8c));

        let light = Theme::named("light");
        assert_eq!(light.bg, Color::Rgb(0xfa, 0xf8, 0xf5));

        // Unknown falls back to default
        let unknown = Theme::named("nonexistent");
        assert_eq!(unknown.accent, Color::Rgb(0x4a, 0x9a, 0x8c));
    }
}
