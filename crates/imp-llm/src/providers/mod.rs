pub mod anthropic;
pub mod google;
pub mod openai;

use crate::provider::Provider;

pub use anthropic::AnthropicProvider;
pub use google::GoogleProvider;
pub use openai::OpenAiProvider;

/// Create a provider by name.
pub fn create_provider(name: &str) -> Option<Box<dyn Provider>> {
    match name {
        "anthropic" => Some(Box::new(AnthropicProvider::new())),
        "openai" => Some(Box::new(OpenAiProvider::new())),
        "google" => Some(Box::new(GoogleProvider::new())),
        _ => None,
    }
}
