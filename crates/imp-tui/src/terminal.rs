use std::io::{self, Write};

use crossterm::cursor::Show;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
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

pub fn ring_terminal_bell() -> io::Result<()> {
    #[cfg(test)]
    {
        return Ok(());
    }

    #[cfg(not(test))]
    {
        let mut stdout = io::stdout();
        write!(stdout, "\x07")?;
        stdout.flush()?;
        Ok(())
    }
}

fn restore_terminal<W: Write>(writer: &mut W) -> io::Result<()> {
    let _ = disable_raw_mode();
    write!(writer, "\x1b[0m")?;
    #[cfg(unix)]
    crossterm::execute!(
        writer,
        Show,
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    #[cfg(not(unix))]
    crossterm::execute!(writer, Show, LeaveAlternateScreen, DisableMouseCapture)?;
    writer.flush()?;
    Ok(())
}

fn restore_terminal_if_needed<W: Write>(writer: &mut W, restored: &mut bool) -> io::Result<()> {
    if *restored {
        return Ok(());
    }
    *restored = true;
    restore_terminal(writer)
}

pub struct TerminalSession {
    terminal: InteractiveTerminal,
    last_title: Option<String>,
    restored: bool,
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
            EnableBracketedPaste
        )?;
        #[cfg(not(unix))]
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            last_title: None,
            restored: false,
        })
    }

    pub fn terminal_mut(&mut self) -> &mut InteractiveTerminal {
        &mut self.terminal
    }

    pub fn restore(&mut self) -> io::Result<()> {
        restore_terminal_if_needed(self.terminal.backend_mut(), &mut self.restored)
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
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_terminal_writes_reset_and_exit_sequences() {
        let mut output = Vec::new();
        restore_terminal(&mut output).unwrap();
        let text = String::from_utf8(output).unwrap();

        assert!(text.contains("\u{1b}[0m"));
        assert!(text.contains("\u{1b}[?25h"));
        assert!(text.contains("\u{1b}[?1049l"));
    }

    #[test]
    fn restore_terminal_if_needed_is_idempotent() {
        let mut output = Vec::new();
        let mut restored = false;

        restore_terminal_if_needed(&mut output, &mut restored).unwrap();
        let first = output.clone();
        restore_terminal_if_needed(&mut output, &mut restored).unwrap();

        assert!(restored);
        assert_eq!(output, first);
    }
}
