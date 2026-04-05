use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// High-level action triggered by a key binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Editor actions
    Submit,
    NewLine,
    InsertChar(char),
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    CursorHome,
    CursorEnd,
    WordLeft,
    WordRight,
    DeleteWordBack,
    DeleteToStart,
    DeleteToEnd,
    SelectAll,
    // Navigation
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    // Agent
    Cancel,
    FollowUp,
    // Mode switching
    SelectModel,
    CycleModelForward,
    CycleModelBackward,
    CycleThinking,
    Peek,
    SidebarToggle,
    SessionTree,
    Reload,
    Quit,
    // Overlays
    OpenFileFinder,
    OpenCommandPalette,
    // Overlay navigation
    OverlayUp,
    OverlayDown,
    OverlaySelect,
    OverlayDismiss,
    OverlayFilter(char),
    OverlayBackspace,
    // Tool call navigation
    ToolFocusNext,
    ToolFocusPrev,
    /// Toggle the focused tool call's expansion (or all if no focus).
    ToolToggle,
}

/// Resolve a key event to an action in normal mode.
pub fn resolve_normal(key: KeyEvent) -> Option<Action> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        // Submit / newline
        KeyCode::Enter if alt => Some(Action::FollowUp),
        KeyCode::Enter if shift => Some(Action::NewLine),
        KeyCode::Enter => Some(Action::Submit),
        KeyCode::Char('j') if ctrl => Some(Action::NewLine),

        // Cancel / quit
        KeyCode::Char('c') if ctrl => Some(Action::Cancel),
        KeyCode::Esc => Some(Action::Cancel),

        // Model / thinking
        KeyCode::Char('l') if ctrl => Some(Action::SelectModel),
        KeyCode::Char('p') if ctrl && shift => Some(Action::CycleModelBackward),
        KeyCode::Char('p') if ctrl => Some(Action::CycleModelForward),
        KeyCode::Char('o') if ctrl => Some(Action::ToolToggle),
        KeyCode::BackTab => Some(Action::CycleThinking),

        // Sidebar / tool navigation
        KeyCode::Tab => Some(Action::SidebarToggle),

        // Cursor movement
        KeyCode::Left if ctrl => Some(Action::WordLeft),
        KeyCode::Right if ctrl => Some(Action::WordRight),
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
        KeyCode::Up if ctrl => Some(Action::ToolFocusPrev),
        KeyCode::Down if ctrl => Some(Action::ToolFocusNext),
        KeyCode::Up => Some(Action::CursorUp),
        KeyCode::Down => Some(Action::CursorDown),
        KeyCode::Home => Some(Action::CursorHome),
        KeyCode::End => Some(Action::CursorEnd),

        // Editing shortcuts
        KeyCode::Char('a') if ctrl => Some(Action::CursorHome),
        KeyCode::Char('e') if ctrl => Some(Action::CursorEnd),
        KeyCode::Char('w') if ctrl => Some(Action::DeleteWordBack),
        KeyCode::Char('u') if ctrl => Some(Action::DeleteToStart),
        KeyCode::Char('k') if ctrl => Some(Action::DeleteToEnd),

        // Delete
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Delete => Some(Action::Delete),

        // Scroll
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Char('b') if ctrl => Some(Action::PageUp),
        KeyCode::Char('f') if ctrl => Some(Action::PageDown),

        // Character input
        KeyCode::Char(c) => Some(Action::InsertChar(c)),

        _ => None,
    }
}

/// Resolve a key event to an action in overlay mode (model selector, command palette, file finder).
pub fn resolve_overlay(key: KeyEvent) -> Option<Action> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Up => Some(Action::OverlayUp),
        KeyCode::Down => Some(Action::OverlayDown),
        KeyCode::Tab => Some(Action::OverlayDown),
        KeyCode::BackTab => Some(Action::OverlayUp),
        KeyCode::Char('n') if ctrl => Some(Action::OverlayDown),
        KeyCode::Char('p') if ctrl => Some(Action::OverlayUp),
        KeyCode::Enter => Some(Action::OverlaySelect),
        KeyCode::Esc => Some(Action::OverlayDismiss),
        KeyCode::Backspace => Some(Action::OverlayBackspace),
        KeyCode::Char(c) => Some(Action::OverlayFilter(c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_p_cycles_model_forward() {
        assert_eq!(
            resolve_normal(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)),
            Some(Action::CycleModelForward)
        );
    }

    #[test]
    fn ctrl_shift_p_cycles_model_backward() {
        assert_eq!(
            resolve_normal(KeyEvent::new(
                KeyCode::Char('p'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            Some(Action::CycleModelBackward)
        );
    }

    #[test]
    fn tab_toggles_sidebar() {
        assert_eq!(
            resolve_normal(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty())),
            Some(Action::SidebarToggle)
        );
    }

    #[test]
    fn ctrl_p_no_longer_toggles_sidebar() {
        assert_ne!(
            resolve_normal(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)),
            Some(Action::SidebarToggle)
        );
    }
}
