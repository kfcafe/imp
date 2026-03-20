use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::provider::Provider;

/// Static metadata describing a model's capabilities and pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    /// Canonical model identifier (e.g. "claude-sonnet-4-20250514").
    pub id: String,
    /// Provider that serves this model (e.g. "anthropic").
    pub provider: String,
    /// Human-readable display name.
    pub name: String,
    /// Maximum input context in tokens.
    pub context_window: u32,
    /// Maximum tokens the model can generate.
    pub max_output_tokens: u32,
    /// Per-million-token pricing.
    pub pricing: ModelPricing,
    /// Feature flags.
    pub capabilities: Capabilities,
}

/// Per-million-token pricing for a model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Dollars per million input tokens.
    pub input_per_mtok: f64,
    /// Dollars per million output tokens.
    pub output_per_mtok: f64,
    /// Dollars per million cache-read tokens.
    pub cache_read_per_mtok: f64,
    /// Dollars per million cache-write tokens.
    pub cache_write_per_mtok: f64,
}

/// Feature flags indicating what a model supports.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    /// Supports extended thinking / chain-of-thought.
    pub reasoning: bool,
    /// Supports image inputs.
    pub images: bool,
    /// Supports tool/function calling.
    pub tool_use: bool,
}

/// Resolved model ready for use (metadata + provider reference).
pub struct Model {
    /// Static metadata for this model.
    pub meta: ModelMeta,
    /// The provider that will serve requests.
    pub provider: Arc<dyn Provider>,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("meta", &self.meta)
            .field("provider", &self.provider.id())
            .finish()
    }
}

/// Central index of available models with alias resolution.
///
/// Stores [`ModelMeta`] entries and short aliases (e.g. "sonnet" → canonical id).
/// Create with [`ModelRegistry::with_builtins`] for a pre-populated registry.
#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: Vec<ModelMeta>,
    aliases: HashMap<String, String>,
}

impl ModelRegistry {
    /// Empty registry with no models or aliases.
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            aliases: HashMap::new(),
        }
    }

    /// Registry pre-populated with built-in models and aliases for
    /// Anthropic, OpenAI, and Google.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        for meta in builtin_models() {
            reg.register(meta);
        }
        for (alias, canonical) in builtin_aliases() {
            reg.aliases.insert(alias, canonical);
        }
        reg
    }

    /// Add a model to the registry.
    pub fn register(&mut self, meta: ModelMeta) {
        // Avoid duplicates by id.
        if !self.models.iter().any(|m| m.id == meta.id) {
            self.models.push(meta);
        }
    }

    /// Register a short alias that maps to a canonical model id.
    pub fn register_alias(&mut self, alias: impl Into<String>, canonical_id: impl Into<String>) {
        self.aliases.insert(alias.into(), canonical_id.into());
    }

    /// Find a model by exact canonical id.
    pub fn find(&self, id: &str) -> Option<&ModelMeta> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Resolve an alias to a model. Falls back to exact-id lookup if no alias matches.
    pub fn find_by_alias(&self, alias: &str) -> Option<&ModelMeta> {
        if let Some(canonical) = self.aliases.get(alias) {
            self.find(canonical)
        } else {
            self.find(alias)
        }
    }

    /// All registered models.
    pub fn list(&self) -> &[ModelMeta] {
        &self.models
    }

    /// Models from a specific provider.
    pub fn list_by_provider(&self, provider: &str) -> Vec<&ModelMeta> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

// ---------------------------------------------------------------------------
// Built-in model catalogue
// ---------------------------------------------------------------------------

fn builtin_models() -> Vec<ModelMeta> {
    vec![
        // -- Anthropic --
        ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "Claude Sonnet 4".into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cache_read_per_mtok: 0.3,
                cache_write_per_mtok: 3.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "claude-haiku-3-5-20241022".into(),
            provider: "anthropic".into(),
            name: "Claude 3.5 Haiku".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 0.80,
                output_per_mtok: 4.0,
                cache_read_per_mtok: 0.08,
                cache_write_per_mtok: 1.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "claude-opus-4-20250514".into(),
            provider: "anthropic".into(),
            name: "Claude Opus 4".into(),
            context_window: 200_000,
            max_output_tokens: 32_000,
            pricing: ModelPricing {
                input_per_mtok: 15.0,
                output_per_mtok: 75.0,
                cache_read_per_mtok: 1.5,
                cache_write_per_mtok: 18.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        // -- OpenAI --
        ModelMeta {
            id: "gpt-4o".into(),
            provider: "openai".into(),
            name: "GPT-4o".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing {
                input_per_mtok: 2.5,
                output_per_mtok: 10.0,
                cache_read_per_mtok: 1.25,
                cache_write_per_mtok: 2.5,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "o3".into(),
            provider: "openai".into(),
            name: "o3".into(),
            context_window: 200_000,
            max_output_tokens: 100_000,
            pricing: ModelPricing {
                input_per_mtok: 2.0,
                output_per_mtok: 8.0,
                cache_read_per_mtok: 0.5,
                cache_write_per_mtok: 2.0,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "o4-mini".into(),
            provider: "openai".into(),
            name: "o4-mini".into(),
            context_window: 200_000,
            max_output_tokens: 100_000,
            pricing: ModelPricing {
                input_per_mtok: 1.1,
                output_per_mtok: 4.4,
                cache_read_per_mtok: 0.275,
                cache_write_per_mtok: 1.1,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        // -- Google --
        ModelMeta {
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
        },
        ModelMeta {
            id: "gemini-2.5-flash".into(),
            provider: "google".into(),
            name: "Gemini 2.5 Flash".into(),
            context_window: 1_048_576,
            max_output_tokens: 65_536,
            pricing: ModelPricing {
                input_per_mtok: 0.15,
                output_per_mtok: 3.5,
                cache_read_per_mtok: 0.0375,
                cache_write_per_mtok: 0.15,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
    ]
}

fn builtin_aliases() -> Vec<(String, String)> {
    vec![
        // Anthropic
        ("sonnet".into(), "claude-sonnet-4-20250514".into()),
        ("claude-sonnet".into(), "claude-sonnet-4-20250514".into()),
        ("haiku".into(), "claude-haiku-3-5-20241022".into()),
        ("claude-haiku".into(), "claude-haiku-3-5-20241022".into()),
        ("opus".into(), "claude-opus-4-20250514".into()),
        ("claude-opus".into(), "claude-opus-4-20250514".into()),
        // OpenAI
        ("gpt4o".into(), "gpt-4o".into()),
        ("4o".into(), "gpt-4o".into()),
        // Google
        ("gemini-pro".into(), "gemini-2.5-pro".into()),
        ("gemini-flash".into(), "gemini-2.5-flash".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_by_alias_resolves_sonnet() {
        let reg = ModelRegistry::with_builtins();
        let model = reg.find_by_alias("sonnet").expect("sonnet alias should resolve");
        assert_eq!(model.id, "claude-sonnet-4-20250514");
        assert_eq!(model.provider, "anthropic");
    }

    #[test]
    fn find_by_alias_resolves_haiku() {
        let reg = ModelRegistry::with_builtins();
        let model = reg.find_by_alias("haiku").expect("haiku alias should resolve");
        assert_eq!(model.id, "claude-haiku-3-5-20241022");
    }

    #[test]
    fn find_by_alias_resolves_opus() {
        let reg = ModelRegistry::with_builtins();
        let model = reg.find_by_alias("opus").expect("opus alias should resolve");
        assert_eq!(model.id, "claude-opus-4-20250514");
    }

    #[test]
    fn find_by_alias_resolves_gpt4o() {
        let reg = ModelRegistry::with_builtins();
        let model = reg.find_by_alias("gpt4o").expect("gpt4o alias should resolve");
        assert_eq!(model.id, "gpt-4o");
    }

    #[test]
    fn find_by_alias_resolves_gemini_pro() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("gemini-pro")
            .expect("gemini-pro alias should resolve");
        assert_eq!(model.id, "gemini-2.5-pro");
    }

    #[test]
    fn find_by_alias_falls_back_to_exact_id() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("o3")
            .expect("exact id lookup should work as fallback");
        assert_eq!(model.id, "o3");
    }

    #[test]
    fn find_by_alias_returns_none_for_unknown() {
        let reg = ModelRegistry::with_builtins();
        assert!(reg.find_by_alias("nonexistent-model").is_none());
    }

    #[test]
    fn list_by_provider_filters_correctly() {
        let reg = ModelRegistry::with_builtins();
        let anthropic = reg.list_by_provider("anthropic");
        assert_eq!(anthropic.len(), 3);
        assert!(anthropic.iter().all(|m| m.provider == "anthropic"));

        let openai = reg.list_by_provider("openai");
        assert_eq!(openai.len(), 3);

        let google = reg.list_by_provider("google");
        assert_eq!(google.len(), 2);
    }

    #[test]
    fn register_skips_duplicates() {
        let mut reg = ModelRegistry::new();
        let meta = ModelMeta {
            id: "test-model".into(),
            provider: "test".into(),
            name: "Test".into(),
            context_window: 1000,
            max_output_tokens: 100,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        };
        reg.register(meta.clone());
        reg.register(meta);
        assert_eq!(reg.list().len(), 1);
    }
}
