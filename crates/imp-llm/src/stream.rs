use serde::{Deserialize, Serialize};

use crate::message::AssistantMessage;

/// Normalized stream events produced by all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// New message starting.
    MessageStart { model: String },

    /// Incremental text from the assistant.
    TextDelta { text: String },

    /// Incremental thinking/reasoning output.
    ThinkingDelta { text: String },

    /// A complete tool call (accumulated from deltas).
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },

    /// Message complete with final message.
    MessageEnd { message: AssistantMessage },

    /// Unrecoverable stream error.
    Error { error: String },
}

/// Structured error returned by a provider's API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderError {
    /// Machine-readable error code (e.g. "rate_limit_exceeded").
    pub code: String,
    /// Human-readable error description.
    pub message: String,
    /// Whether the caller should retry the request.
    pub retryable: bool,
}
