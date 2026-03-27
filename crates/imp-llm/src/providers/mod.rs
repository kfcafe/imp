pub mod anthropic;
pub mod google;
pub mod openai;
pub mod openai_codex;
pub mod openai_compat;

use crate::model::{ApiStyle, ProviderRegistry};
use crate::provider::Provider;

pub use anthropic::AnthropicProvider;
pub use google::GoogleProvider;
pub use openai::OpenAiProvider;
pub use openai_codex::OpenAiCodexProvider;
pub use openai_compat::OpenAiCompatProvider;

/// Create a provider by name, using the provider registry for metadata.
pub fn create_provider(name: &str) -> Option<Box<dyn Provider>> {
    let registry = ProviderRegistry::with_builtins();
    let meta = registry.find(name)?;

    match meta.api_style {
        ApiStyle::Anthropic => Some(Box::new(AnthropicProvider::new())),
        ApiStyle::OpenAi => Some(Box::new(OpenAiProvider::new())),
        ApiStyle::OpenAiCodex => Some(Box::new(OpenAiCodexProvider::new())),
        ApiStyle::Google => Some(Box::new(GoogleProvider::new())),
        ApiStyle::OpenAiCompat => {
            let base_url = meta.api_base_url.unwrap_or("https://api.openai.com");
            let model_registry = crate::model::ModelRegistry::with_builtins();
            let models: Vec<crate::model::ModelMeta> = model_registry
                .list_by_provider(name)
                .into_iter()
                .cloned()
                .collect();
            Some(Box::new(OpenAiCompatProvider::new(name, base_url, models)))
        }
    }
}
