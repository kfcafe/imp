use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures_core::Stream;
use serde::{Deserialize, Serialize};

use crate::auth::{ApiKey, AuthStore};
use crate::error::Result;
use crate::message::Message;
use crate::model::{Model, ModelMeta};
use crate::stream::StreamEvent;

/// A provider handles communication with a specific LLM API.
///
/// Each provider (Anthropic, OpenAI, Google, etc.) implements this trait
/// to normalize streaming responses into [`StreamEvent`]s.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stream a completion response.
    fn stream(
        &self,
        model: &Model,
        context: Context,
        options: RequestOptions,
        api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>;

    /// Resolve an API key for this provider.
    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey>;

    /// Provider identifier (e.g., "anthropic", "openai", "google").
    fn id(&self) -> &str;

    /// List available models for this provider.
    fn models(&self) -> &[ModelMeta];
}

/// Conversation context sent to the provider.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub messages: Vec<Message>,
}

/// Tuning knobs for a single LLM request.
#[derive(Debug, Clone)]
pub struct RequestOptions {
    pub thinking_level: ThinkingLevel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: String,
    pub tools: Vec<ToolDefinition>,
    pub cache_options: CacheOptions,
}

impl Default for RequestOptions {
    fn default() -> Self {
        Self {
            thinking_level: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            system_prompt: String::new(),
            tools: Vec::new(),
            cache_options: CacheOptions::default(),
        }
    }
}

/// How much reasoning/thinking to request from the model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    /// No extended thinking.
    #[default]
    Off,
    /// Minimal reasoning.
    Minimal,
    /// Low-effort reasoning.
    Low,
    /// Moderate reasoning.
    Medium,
    /// High-effort reasoning.
    High,
    /// Maximum reasoning budget.
    XHigh,
}

/// Controls which parts of the request are eligible for prompt caching.
#[derive(Debug, Clone, Default)]
pub struct CacheOptions {
    /// Cache the system prompt across requests.
    pub cache_system_prompt: bool,
    /// Cache tool definitions.
    pub cache_tools: bool,
    /// Number of recent conversation turns to cache.
    pub cache_recent_turns: usize,
}

/// A tool the model may call, defined by a JSON Schema for its parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Retry policy for transient failures (rate limits, server errors, timeouts).
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub retry_on: Vec<RetryCondition>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            retry_on: vec![
                RetryCondition::RateLimit,
                RetryCondition::ServerError,
                RetryCondition::Timeout,
                RetryCondition::ConnectionError,
            ],
        }
    }
}

/// Conditions under which a request should be retried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryCondition {
    RateLimit,
    ServerError,
    Timeout,
    ConnectionError,
}
