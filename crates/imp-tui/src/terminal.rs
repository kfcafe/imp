use std::io::{self, Write};

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type InteractiveTerminal = Terminal<CrosstermBackend<io::Stdout>>;

pub fn set_window_title(title: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "\x1b]0;{title}\x07")?;
    stdout.flush()?;
    Ok(())
}

pub struct TerminalSession {
    terminal: InteractiveTerminal,
    last_title: Option<String>,
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        #[cfg(unix)]
        crossterm::execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
        #[cfg(not(unix))]
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            last_title: None,
        })
    }

    pub fn terminal_mut(&mut self) -> &mut InteractiveTerminal {
        &mut self.terminal
    }

    pub fn set_window_title(&mut self, title: &str) -> io::Result<()> {
        if self.last_title.as_deref() == Some(title) {
            return Ok(());
        }

        let mut stdout = io::stdout();
        write!(stdout, "\x1b]0;{title}\x07")?;
        stdout.flush()?;
        self.last_title = Some(title.to_string());
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        #[cfg(unix)]
        let _ = crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableBracketedPaste,
            PopKeyboardEnhancementFlags
        );
        #[cfg(not(unix))]
        let _ = crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}
