//! Standalone multi-provider LLM streaming client.
//!
//! Core data types for messages, models, usage tracking, and provider traits.
//! No I/O happens in this module — it defines the shared vocabulary used by
//! provider implementations and the agent runtime.

pub mod auth;
pub mod error;
pub mod message;
pub mod model;
pub mod oauth;
pub mod provider;
pub mod providers;
pub mod stream;
pub mod text;
pub mod usage;

pub use error::{Error, Result};
pub use message::{
    AssistantMessage, ContentBlock, Message, StopReason, ToolResultMessage, UserMessage,
};
pub use model::{
    ApiStyle, Capabilities, Model, ModelMeta, ModelPricing, ModelRegistry, ProviderMeta,
    ProviderRegistry,
};
pub use provider::{
    CacheOptions, Context, Provider, RequestOptions, ThinkingLevel, ToolDefinition,
};
pub use stream::{ProviderError, StreamEvent};
pub use text::{prefix_chars, truncate_chars, truncate_chars_with_suffix};
pub use usage::{Cost, Usage};

/// Current unix timestamp in seconds.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
