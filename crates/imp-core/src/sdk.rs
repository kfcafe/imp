//! Curated Rust SDK surface for embedding imp in other hosts.
//!
//! This module is the first stable-shaped entry point for using imp as a
//! reusable runtime instead of only through the CLI or TUI. It intentionally
//! wraps the existing `imp_session`, `agent`, and `ui` pieces in one place so
//! host applications can depend on a small public surface.
//!
//! Current SDK v1 focus:
//! - create and manage an [`ImpSession`]
//! - prompt, steer, follow up, cancel, and wait for completion
//! - consume the runtime [`AgentEvent`] stream
//! - provide a host-side [`UserInterface`] bridge
//! - switch models and thinking levels for later prompts
//!
//! Explicitly out of scope for this first slice:
//! - TypeScript extension/runtime loading
//! - packaged customization discovery
//! - a higher-level runtime/session-replacement wrapper above [`ImpSession`]
//! - CLI/TUI-specific orchestration helpers
//! - provider registration and broader host lifecycle policy
//!
//! # Example
//! ```no_run
//! use imp_core::sdk::{AgentEvent, ImpSession, Result, SessionOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut session = ImpSession::create(SessionOptions {
//!         cwd: std::env::current_dir()?,
//!         ..Default::default()
//!     })
//!     .await?;
//!
//!     session.prompt("Summarize the project in this directory.").await?;
//!
//!     while let Some(event) = session.recv_event().await {
//!         match event {
//!             AgentEvent::MessageDelta { .. } => {}
//!             AgentEvent::AgentEnd { .. } => break,
//!             _ => {}
//!         }
//!     }
//!
//!     session.wait().await
//! }
//! ```

pub use crate::agent::{AgentCommand, AgentEvent, TimingEvent, TimingStage};
pub use crate::error::{Error, Result};
pub use crate::imp_session::{
    ImpSession, ResolvedRuntimeConnection, RuntimeConnectionIntent, SessionChoice, SessionOptions,
};
pub use crate::mana_review::{ManaReviewState, TurnManaReview};
pub use crate::ui::{
    ComponentSpec, NotifyLevel, NullInterface, SelectOption, UserInterface, WidgetContent,
};
pub use imp_llm::{Model, ThinkingLevel};
