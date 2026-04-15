use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

use futures::FutureExt;
use imp_core::config::Config;
use imp_core::session::SessionManager;
use imp_llm::model::ModelRegistry;

use crate::app::App;
use crate::terminal::TerminalSession;

pub struct InteractiveRunner {
    app: App,
    terminal: TerminalSession,
}

#[derive(Debug)]
pub enum InteractiveRunError {
    Runtime(Box<dyn std::error::Error>),
    Panic(String),
}

impl std::fmt::Display for InteractiveRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Panic(message) => write!(f, "interactive panic: {message}"),
        }
    }
}

impl std::error::Error for InteractiveRunError {}

impl InteractiveRunner {
    pub fn new(
        config: Config,
        session: SessionManager,
        model_registry: ModelRegistry,
        cwd: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let app = App::new(config, session, model_registry, cwd);
        let terminal = TerminalSession::enter()?;
        Ok(Self { app, terminal })
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.set_window_title(&self.app.terminal_title());
        self.app.run(self.terminal.terminal_mut()).await
    }

    pub async fn run_guarded(&mut self) -> Result<(), InteractiveRunError> {
        let result = AssertUnwindSafe(self.run()).catch_unwind().await;
        let _ = self.terminal.restore();

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(InteractiveRunError::Runtime(error)),
            Err(payload) => Err(InteractiveRunError::Panic(panic_payload_message(payload))),
        }
    }
}

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    let payload = match payload.downcast::<String>() {
        Ok(message) => return *message,
        Err(payload) => payload,
    };

    match payload.downcast::<&'static str>() {
        Ok(message) => (*message).to_string(),
        Err(_) => "panic with non-string payload".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_payload_message_supports_string_and_str() {
        assert_eq!(panic_payload_message(Box::new("boom")), "boom".to_string());
        assert_eq!(
            panic_payload_message(Box::new("kaboom".to_string())),
            "kaboom".to_string()
        );
    }
}
