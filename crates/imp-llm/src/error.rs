/// Errors that can occur within the imp-llm crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A provider-specific error (e.g. invalid request).
    #[error("Provider error: {0}")]
    Provider(String),

    /// Authentication failure (missing key, expired token, etc.).
    #[error("Auth error: {0}")]
    Auth(String),

    /// Error during response streaming.
    #[error("Stream error: {0}")]
    Stream(String),

    /// JSON serialization / deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// File system I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Provider is rate-limiting requests.
    #[error("Rate limited: retry after {retry_after_secs:?}s")]
    RateLimited {
        /// Suggested wait time before retrying, if the provider reported one.
        retry_after_secs: Option<u64>,
    },

    /// The conversation exceeds the model's context window.
    #[error("Context too long: {used} tokens exceeds {limit}")]
    ContextTooLong {
        /// Tokens used by the current conversation.
        used: u32,
        /// Maximum tokens the model supports.
        limit: u32,
    },
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
