use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event};
use tokio::sync::mpsc;

const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TERMINAL_EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Owns the blocking crossterm input reader thread and forwards terminal events
/// to the async TUI loop.
pub struct TerminalEventSource {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TerminalEventSource {
    pub fn spawn() -> (Self, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel(TERMINAL_EVENT_CHANNEL_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);

        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match event::poll(INPUT_POLL_INTERVAL) {
                    Ok(true) => match event::read() {
                        Ok(event) => {
                            if tx.blocking_send(event).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        });

        (
            Self {
                stop,
                thread: Some(thread),
            },
            rx,
        )
    }
}

impl Drop for TerminalEventSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_event_source_can_be_dropped_without_events() {
        let (source, _rx) = TerminalEventSource::spawn();
        drop(source);
    }
}
