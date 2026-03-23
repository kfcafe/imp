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
    ToggleToolExpand,
    ToggleThinking,
    Peek,
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
        KeyCode::BackTab => Some(Action::CycleThinking),

        // Toggle tool/thinking
        KeyCode::Char('o') if ctrl => Some(Action::ToggleToolExpand),
        KeyCode::Char('t') if ctrl => Some(Action::ToggleThinking),
        KeyCode::Tab => Some(Action::Peek),

        // Cursor movement
        KeyCode::Left if ctrl => Some(Action::WordLeft),
        KeyCode::Right if ctrl => Some(Action::WordRight),
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
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

        // Character input
        KeyCode::Char(c) => Some(Action::InsertChar(c)),

        _ => None,
    }
}

/// Resolve a key event to an action in overlay mode (model selector, command palette, file finder).
pub fn resolve_overlay(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::OverlayUp),
        KeyCode::Down => Some(Action::OverlayDown),
        KeyCode::Enter => Some(Action::OverlaySelect),
        KeyCode::Esc => Some(Action::OverlayDismiss),
        KeyCode::Backspace => Some(Action::OverlayBackspace),
        KeyCode::Char(c) => Some(Action::OverlayFilter(c)),
        _ => None,
    }
}
