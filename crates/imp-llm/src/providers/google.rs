use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;

use crate::auth::{ApiKey, AuthStore};
use crate::error::Result;
use crate::model::{Capabilities, ModelMeta, ModelPricing};
use crate::provider::{Context, Provider, RequestOptions};
use crate::stream::StreamEvent;

/// Google Gemini API provider.
pub struct GoogleProvider {
    #[allow(dead_code)]
    client: reqwest::Client,
    models: Vec<ModelMeta>,
}

impl Default for GoogleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GoogleProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            models: builtin_models(),
        }
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn stream(
        &self,
        _model: &crate::model::Model,
        _context: Context,
        _options: RequestOptions,
        _api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
        // TODO: Implement SSE streaming against Gemini API
        Box::pin(futures::stream::empty())
    }

    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey> {
        auth.resolve("google")
    }

    fn id(&self) -> &str {
        "google"
    }

    fn models(&self) -> &[ModelMeta] {
        &self.models
    }
}

fn builtin_models() -> Vec<ModelMeta> {
    vec![ModelMeta {
        id: "gemini-2.5-pro".into(),
        provider: "google".into(),
        name: "Gemini 2.5 Pro".into(),
        context_window: 1_048_576,
        max_output_tokens: 65_536,
        pricing: ModelPricing {
            input_per_mtok: 1.25,
            output_per_mtok: 10.0,
            cache_read_per_mtok: 0.315,
            cache_write_per_mtok: 1.25,
        },
        capabilities: Capabilities {
            reasoning: true,
            images: true,
            tool_use: true,
        },
    }]
}
