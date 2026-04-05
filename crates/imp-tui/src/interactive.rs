use std::path::PathBuf;

use imp_core::config::Config;
use imp_core::session::SessionManager;
use imp_llm::model::ModelRegistry;

use crate::app::App;
use crate::terminal::TerminalSession;

pub struct InteractiveRunner {
    app: App,
    terminal: TerminalSession,
}

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
}
