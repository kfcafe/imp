use std::time::Duration;

use imp_core::config::AnimationLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationState {
    #[default]
    Idle,
    WaitingForResponse,
    Thinking,
    ExecutingTools {
        active_tools: u32,
    },
    Streaming,
    Queued,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivitySurface {
    TopBar,
    Editor,
    Chat,
}

impl AnimationState {
    pub fn from_streaming(
        is_streaming: bool,
        has_content: bool,
        has_tools: bool,
        active_tools: u32,
        has_queued: bool,
    ) -> Self {
        if !is_streaming {
            return Self::Idle;
        }
        if has_queued {
            return Self::Queued;
        }
        if active_tools > 0 {
            return Self::ExecutingTools { active_tools };
        }
        if !has_content && has_tools {
            return Self::Thinking;
        }
        if !has_content {
            return Self::WaitingForResponse;
        }
        Self::Streaming
    }
}

/// Main working/thinking spinner.
pub fn spinner_frame(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⣠", "⡴", "⠞", "⠋", "⠙", "⠳", "⢦", "⣄"];
    FRAMES[(tick / 3) as usize % FRAMES.len()]
}

/// Braille dot spinner for global agent work in the terminal title.
pub fn title_spinner_frame(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[(tick / 4) as usize % FRAMES.len()]
}

/// Static title glyph for active work when animated motion is disabled.
pub fn title_working_glyph() -> &'static str {
    "•"
}

/// Main working/thinking spinner.
pub fn thinking_frame(tick: u64) -> &'static str {
    spinner_frame(tick)
}

/// Main working/thinking spinner used while streaming responses.
pub fn responding_frame(tick: u64) -> &'static str {
    spinner_frame(tick)
}

/// Main working/thinking spinner used for concrete tool execution.
pub fn tool_frame(tick: u64) -> &'static str {
    spinner_frame(tick)
}

/// Static glyph for running states when animated motion is disabled.
pub fn static_working_glyph() -> &'static str {
    "•"
}

/// Static glyph for queued work.
pub fn queued_glyph() -> &'static str {
    "◌"
}

/// Backward-compatible alias for the response runner.
pub fn runner_frame(tick: u64) -> &'static str {
    responding_frame(tick)
}

pub fn waiting_badge(tick: u64, level: AnimationLevel) -> String {
    match level {
        AnimationLevel::None => static_working_glyph().to_string(),
        AnimationLevel::Spinner | AnimationLevel::Minimal => thinking_frame(tick).to_string(),
    }
}

pub fn activity_label(
    state: AnimationState,
    tick: u64,
    level: AnimationLevel,
    surface: ActivitySurface,
) -> String {
    let animated = level != AnimationLevel::None;
    match state {
        AnimationState::Idle => String::new(),
        AnimationState::WaitingForResponse => {
            let glyph = if animated {
                thinking_frame(tick)
            } else {
                static_working_glyph()
            };
            match surface {
                ActivitySurface::TopBar => format!("{glyph} waiting for response"),
                ActivitySurface::Chat => format!("{glyph} waiting"),
                ActivitySurface::Editor => String::new(),
            }
        }
        AnimationState::Thinking => {
            let glyph = if animated {
                thinking_frame(tick)
            } else {
                static_working_glyph()
            };
            format!("{glyph} thinking")
        }
        AnimationState::ExecutingTools { active_tools } => {
            let glyph = if animated {
                tool_frame(tick)
            } else {
                static_working_glyph()
            };
            format!(
                "{glyph} working · {active_tools} tool{}",
                if active_tools == 1 { "" } else { "s" }
            )
        }
        AnimationState::Streaming => match surface {
            ActivitySurface::TopBar | ActivitySurface::Chat => {
                let glyph = if animated {
                    responding_frame(tick)
                } else {
                    static_working_glyph()
                };
                format!("{glyph} responding")
            }
            ActivitySurface::Editor => String::new(),
        },
        AnimationState::Queued => format!("{} queued", queued_glyph()),
    }
}

pub fn format_elapsed(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 60 {
        format!("{}m{:02}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_formats_seconds_and_minutes() {
        assert_eq!(format_elapsed(Duration::from_secs(7)), "7s");
        assert_eq!(format_elapsed(Duration::from_secs(75)), "1m15s");
    }

    #[test]
    fn title_spinner_uses_braille_dot_cycle() {
        assert_eq!(title_spinner_frame(0), "⠋");
        assert_eq!(title_spinner_frame(3), "⠋");
        assert_eq!(title_spinner_frame(4), "⠙");
        assert_eq!(title_spinner_frame(8), "⠹");
        assert_eq!(title_spinner_frame(16), "⠼");
        assert_eq!(title_spinner_frame(36), "⠏");
        assert_eq!(title_spinner_frame(40), "⠋");
    }

    #[test]
    fn activity_labels_use_state_specific_glyphs() {
        assert_eq!(
            activity_label(
                AnimationState::Thinking,
                0,
                AnimationLevel::Minimal,
                ActivitySurface::Chat,
            ),
            "⣠ thinking"
        );
        assert_eq!(
            activity_label(
                AnimationState::Streaming,
                0,
                AnimationLevel::Minimal,
                ActivitySurface::Chat,
            ),
            "⣠ responding"
        );
        assert_eq!(
            activity_label(
                AnimationState::ExecutingTools { active_tools: 2 },
                0,
                AnimationLevel::Minimal,
                ActivitySurface::Chat,
            ),
            "⣠ working · 2 tools"
        );
        assert_eq!(
            activity_label(
                AnimationState::Queued,
                0,
                AnimationLevel::None,
                ActivitySurface::Chat,
            ),
            "◌ queued"
        );
    }

    #[test]
    fn activity_labels_keep_static_glyphs_when_motion_disabled() {
        assert_eq!(
            activity_label(
                AnimationState::Thinking,
                99,
                AnimationLevel::None,
                ActivitySurface::Chat,
            ),
            "• thinking"
        );
        assert_eq!(
            activity_label(
                AnimationState::Streaming,
                99,
                AnimationLevel::None,
                ActivitySurface::Chat,
            ),
            "• responding"
        );
    }
}
