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

/// Classic braille spinner, but slowed slightly so it feels less jittery.
pub fn spinner_frame(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[(tick / 3) as usize % FRAMES.len()]
}

/// A softer, more directional runner used for "waiting for response" states.
pub fn runner_frame(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"];
    FRAMES[(tick / 3) as usize % FRAMES.len()]
}

pub fn waiting_badge(tick: u64, level: AnimationLevel) -> String {
    match level {
        AnimationLevel::None => String::new(),
        AnimationLevel::Spinner => spinner_frame(tick).to_string(),
        AnimationLevel::Minimal => runner_frame(tick).to_string(),
    }
}

pub fn activity_label(
    state: AnimationState,
    tick: u64,
    level: AnimationLevel,
    surface: ActivitySurface,
) -> String {
    match state {
        AnimationState::Idle => String::new(),
        AnimationState::WaitingForResponse => match level {
            AnimationLevel::None => "waiting".into(),
            AnimationLevel::Spinner => format!("{} waiting", spinner_frame(tick)),
            AnimationLevel::Minimal => match surface {
                ActivitySurface::TopBar => {
                    format!("{} waiting for response", waiting_badge(tick, level))
                }
                ActivitySurface::Chat => {
                    format!("{} waiting", waiting_badge(tick, level))
                }
                ActivitySurface::Editor => String::new(),
            },
        },
        AnimationState::Thinking => match level {
            AnimationLevel::None => "thinking".into(),
            AnimationLevel::Spinner => format!("{} thinking", spinner_frame(tick)),
            AnimationLevel::Minimal => {
                format!("{} thinking", waiting_badge(tick, level))
            }
        },
        AnimationState::ExecutingTools { active_tools } => match level {
            AnimationLevel::None => {
                format!(
                    "working · {active_tools} tool{}",
                    if active_tools == 1 { "" } else { "s" }
                )
            }
            AnimationLevel::Spinner | AnimationLevel::Minimal => format!(
                "{} working · {active_tools} tool{}",
                spinner_frame(tick),
                if active_tools == 1 { "" } else { "s" }
            ),
        },
        AnimationState::Streaming => match surface {
            ActivitySurface::TopBar => match level {
                AnimationLevel::None => "responding".into(),
                AnimationLevel::Spinner | AnimationLevel::Minimal => {
                    format!("{} responding", spinner_frame(tick))
                }
            },
            ActivitySurface::Chat => match level {
                AnimationLevel::None => "responding".into(),
                AnimationLevel::Spinner | AnimationLevel::Minimal => {
                    format!("{} responding", spinner_frame(tick))
                }
            },
            ActivitySurface::Editor => String::new(),
        },
        AnimationState::Queued => match level {
            AnimationLevel::None => "queued".into(),
            AnimationLevel::Spinner => format!("{} queued", spinner_frame(tick)),
            AnimationLevel::Minimal => {
                format!("{} queued", waiting_badge(tick, level))
            }
        },
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
}
