use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::provider::Provider;

/// How a provider's API should be called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiStyle {
    /// Native Anthropic Messages API.
    Anthropic,
    /// Native OpenAI Responses API.
    OpenAi,
    /// Native Google Gemini API.
    Google,
    /// OpenAI-compatible Chat Completions API (DeepSeek, Groq, etc.).
    OpenAiCompat,
}

/// Metadata about an LLM provider.
#[derive(Debug, Clone)]
pub struct ProviderMeta {
    /// Provider identifier (e.g. "anthropic", "deepseek").
    pub id: &'static str,
    /// Human-readable name (e.g. "Anthropic", "DeepSeek").
    pub name: &'static str,
    /// Environment variable names for API key resolution, in priority order.
    pub env_vars: &'static [&'static str],
    /// Base URL for API requests. None for native providers that hardcode their URL.
    pub api_base_url: Option<&'static str>,
    /// URL where users can get an API key (shown in welcome flow).
    pub docs_url: &'static str,
    /// Which API protocol this provider uses.
    pub api_style: ApiStyle,
}

/// Registry of known LLM providers.
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    providers: Vec<ProviderMeta>,
}

impl ProviderRegistry {
    /// Empty registry with no providers.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registry pre-populated with all built-in providers.
    pub fn with_builtins() -> Self {
        Self {
            providers: builtin_providers(),
        }
    }

    /// Find a provider by its id (e.g. "anthropic", "deepseek").
    pub fn find(&self, id: &str) -> Option<&ProviderMeta> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// All registered providers.
    pub fn list(&self) -> &[ProviderMeta] {
        &self.providers
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Built-in provider catalogue covering all supported LLM providers.
pub fn builtin_providers() -> Vec<ProviderMeta> {
    vec![
        ProviderMeta {
            id: "anthropic",
            name: "Anthropic",
            env_vars: &["ANTHROPIC_API_KEY"],
            api_base_url: None,
            docs_url: "console.anthropic.com/settings/keys",
            api_style: ApiStyle::Anthropic,
        },
        ProviderMeta {
            id: "openai",
            name: "OpenAI",
            env_vars: &["OPENAI_API_KEY"],
            api_base_url: None,
            docs_url: "platform.openai.com/api-keys",
            api_style: ApiStyle::OpenAi,
        },
        ProviderMeta {
            id: "google",
            name: "Google",
            env_vars: &["GOOGLE_API_KEY"],
            api_base_url: None,
            docs_url: "aistudio.google.dev/apikey",
            api_style: ApiStyle::Google,
        },
        ProviderMeta {
            id: "deepseek",
            name: "DeepSeek",
            env_vars: &["DEEPSEEK_API_KEY"],
            api_base_url: Some("https://api.deepseek.com"),
            docs_url: "platform.deepseek.com/api_keys",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "groq",
            name: "Groq",
            env_vars: &["GROQ_API_KEY"],
            api_base_url: Some("https://api.groq.com/openai"),
            docs_url: "console.groq.com/keys",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "together",
            name: "Together",
            env_vars: &["TOGETHER_API_KEY"],
            api_base_url: Some("https://api.together.xyz"),
            docs_url: "api.together.ai/settings/api-keys",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "mistral",
            name: "Mistral",
            env_vars: &["MISTRAL_API_KEY"],
            api_base_url: Some("https://api.mistral.ai"),
            docs_url: "console.mistral.ai/api-keys",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "xai",
            name: "xAI",
            env_vars: &["XAI_API_KEY"],
            api_base_url: Some("https://api.x.ai"),
            docs_url: "console.x.ai",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "openrouter",
            name: "OpenRouter",
            env_vars: &["OPENROUTER_API_KEY"],
            api_base_url: Some("https://openrouter.ai/api"),
            docs_url: "openrouter.ai/settings/keys",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "fireworks",
            name: "Fireworks",
            env_vars: &["FIREWORKS_API_KEY"],
            api_base_url: Some("https://api.fireworks.ai/inference"),
            docs_url: "fireworks.ai/account/api-keys",
            api_style: ApiStyle::OpenAiCompat,
        },
        ProviderMeta {
            id: "cerebras",
            name: "Cerebras",
            env_vars: &["CEREBRAS_API_KEY"],
            api_base_url: Some("https://api.cerebras.ai"),
            docs_url: "cloud.cerebras.ai",
            api_style: ApiStyle::OpenAiCompat,
        },
    ]
}

/// Static metadata describing a model's capabilities and pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    /// Canonical model identifier (e.g. "claude-sonnet-4-6").
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

    /// Resolve a built-in model, or synthesize metadata for a custom model id.
    pub fn resolve_meta(&self, model_name: &str, provider_hint: Option<&str>) -> Option<ModelMeta> {
        let canonical_name = self
            .aliases
            .get(model_name)
            .map(String::as_str)
            .unwrap_or(model_name);

        if let Some(meta) = self.find(canonical_name) {
            return Some(meta.clone());
        }

        let provider_name = guess_provider_for_custom_model(canonical_name).or_else(|| {
            provider_hint
                .filter(|provider| ProviderRegistry::with_builtins().find(provider).is_some())
        })?;

        Some(synthesize_custom_model_meta(canonical_name, provider_name))
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
    let mut models = vec![
        // -- Anthropic --
        // Latest: Sonnet 4.6 (released 2026-02)
        ModelMeta {
            id: "claude-sonnet-4-6".into(),
            provider: "anthropic".into(),
            name: "Claude Sonnet 4.6".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
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
        // Sonnet 4.5 (released 2025-09)
        ModelMeta {
            id: "claude-sonnet-4-5-20250929".into(),
            provider: "anthropic".into(),
            name: "Claude Sonnet 4.5".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
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
        // Sonnet 4.0 (released 2025-05)
        ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            name: "Claude Sonnet 4".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
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
        // Latest: Haiku 4.5 (released 2025-10)
        ModelMeta {
            id: "claude-haiku-4-5-20251001".into(),
            provider: "anthropic".into(),
            name: "Claude Haiku 4.5".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            pricing: ModelPricing {
                input_per_mtok: 1.0,
                output_per_mtok: 5.0,
                cache_read_per_mtok: 0.1,
                cache_write_per_mtok: 1.25,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        // Legacy: Haiku 3.5 (kept for compatibility)
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
        // Latest: Opus 4.6 (released 2026-02)
        ModelMeta {
            id: "claude-opus-4-6".into(),
            provider: "anthropic".into(),
            name: "Claude Opus 4.6".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing {
                input_per_mtok: 5.0,
                output_per_mtok: 25.0,
                cache_read_per_mtok: 0.5,
                cache_write_per_mtok: 6.25,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        // Opus 4.5 (released 2025-11)
        ModelMeta {
            id: "claude-opus-4-5-20251101".into(),
            provider: "anthropic".into(),
            name: "Claude Opus 4.5".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            pricing: ModelPricing {
                input_per_mtok: 5.0,
                output_per_mtok: 25.0,
                cache_read_per_mtok: 0.5,
                cache_write_per_mtok: 6.25,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        // Opus 4.0 (released 2025-05, deprecated 2026-05)
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
                cache_read_per_mtok: 0.125,
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
                input_per_mtok: 0.30,
                output_per_mtok: 2.50,
                cache_read_per_mtok: 0.03,
                cache_write_per_mtok: 0.30,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        // -- DeepSeek --
        ModelMeta {
            id: "deepseek-chat".into(),
            provider: "deepseek".into(),
            name: "DeepSeek V3".into(),
            context_window: 64_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 0.27,
                output_per_mtok: 1.10,
                cache_read_per_mtok: 0.07,
                cache_write_per_mtok: 0.27,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "deepseek-reasoner".into(),
            provider: "deepseek".into(),
            name: "DeepSeek R1".into(),
            context_window: 64_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 0.55,
                output_per_mtok: 2.19,
                cache_read_per_mtok: 0.14,
                cache_write_per_mtok: 0.55,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: false,
            },
        },
        // -- Groq --
        ModelMeta {
            id: "llama-3.3-70b-versatile".into(),
            provider: "groq".into(),
            name: "Llama 3.3 70B".into(),
            context_window: 128_000,
            max_output_tokens: 32_768,
            pricing: ModelPricing {
                input_per_mtok: 0.59,
                output_per_mtok: 0.79,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
        // -- Together --
        ModelMeta {
            id: "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
            provider: "together".into(),
            name: "Llama 3.3 70B Turbo".into(),
            context_window: 128_000,
            max_output_tokens: 4_096,
            pricing: ModelPricing {
                input_per_mtok: 0.88,
                output_per_mtok: 0.88,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "Qwen/Qwen2.5-72B-Instruct-Turbo".into(),
            provider: "together".into(),
            name: "Qwen 2.5 72B Turbo".into(),
            context_window: 128_000,
            max_output_tokens: 4_096,
            pricing: ModelPricing {
                input_per_mtok: 1.20,
                output_per_mtok: 1.20,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
        // -- Mistral --
        ModelMeta {
            id: "mistral-large-latest".into(),
            provider: "mistral".into(),
            name: "Mistral Large".into(),
            context_window: 128_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 2.0,
                output_per_mtok: 6.0,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "codestral-latest".into(),
            provider: "mistral".into(),
            name: "Codestral".into(),
            context_window: 256_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 0.30,
                output_per_mtok: 0.90,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
        // -- xAI --
        ModelMeta {
            id: "grok-3".into(),
            provider: "xai".into(),
            name: "Grok 3".into(),
            context_window: 131_072,
            max_output_tokens: 131_072,
            pricing: ModelPricing {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "grok-3-mini".into(),
            provider: "xai".into(),
            name: "Grok 3 Mini".into(),
            context_window: 131_072,
            max_output_tokens: 131_072,
            pricing: ModelPricing {
                input_per_mtok: 0.30,
                output_per_mtok: 0.50,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
        // -- OpenRouter --
        ModelMeta {
            id: "anthropic/claude-sonnet-4".into(),
            provider: "openrouter".into(),
            name: "Claude Sonnet 4 (via OpenRouter)".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
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
        // -- Fireworks --
        ModelMeta {
            id: "accounts/fireworks/models/llama-v3p3-70b-instruct".into(),
            provider: "fireworks".into(),
            name: "Llama 3.3 70B (Fireworks)".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing {
                input_per_mtok: 0.90,
                output_per_mtok: 0.90,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
        // -- Cerebras --
        ModelMeta {
            id: "llama-3.3-70b".into(),
            provider: "cerebras".into(),
            name: "Llama 3.3 70B (Cerebras)".into(),
            context_window: 128_000,
            max_output_tokens: 8_192,
            pricing: ModelPricing {
                input_per_mtok: 0.60,
                output_per_mtok: 0.60,
                cache_read_per_mtok: 0.0,
                cache_write_per_mtok: 0.0,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
    ];

    let openai_insert_at = models
        .iter()
        .take_while(|model| model.provider == "anthropic")
        .count();
    models.splice(openai_insert_at..openai_insert_at, builtin_openai_models());
    models
}

pub fn builtin_openai_models() -> Vec<ModelMeta> {
    vec![
        ModelMeta {
            id: "gpt-5.4".into(),
            provider: "openai".into(),
            name: "GPT-5.4".into(),
            context_window: 1_050_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing {
                input_per_mtok: 2.5,
                output_per_mtok: 15.0,
                cache_read_per_mtok: 0.25,
                cache_write_per_mtok: 2.5,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "gpt-5.4-mini".into(),
            provider: "openai".into(),
            name: "GPT-5.4 mini".into(),
            context_window: 400_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing {
                input_per_mtok: 0.75,
                output_per_mtok: 4.5,
                cache_read_per_mtok: 0.075,
                cache_write_per_mtok: 0.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "gpt-5.4-nano".into(),
            provider: "openai".into(),
            name: "GPT-5.4 nano".into(),
            context_window: 400_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing {
                input_per_mtok: 0.20,
                output_per_mtok: 1.25,
                cache_read_per_mtok: 0.02,
                cache_write_per_mtok: 0.20,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "gpt-5.3-chat-latest".into(),
            provider: "openai".into(),
            name: "GPT-5.3 ChatGPT".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing {
                input_per_mtok: 1.75,
                output_per_mtok: 14.0,
                cache_read_per_mtok: 0.175,
                cache_write_per_mtok: 1.75,
            },
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "gpt-5.3-codex".into(),
            provider: "openai".into(),
            name: "GPT-5.3 Codex".into(),
            context_window: 400_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing {
                input_per_mtok: 1.75,
                output_per_mtok: 14.0,
                cache_read_per_mtok: 0.175,
                cache_write_per_mtok: 1.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
        ModelMeta {
            id: "gpt-5.3-codex-spark".into(),
            provider: "openai".into(),
            name: "GPT-5.3 Codex Spark".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
    ]
}

fn guess_provider_for_custom_model(model_name: &str) -> Option<&'static str> {
    let lower = model_name.to_lowercase();

    if lower.starts_with("gpt-")
        || lower.starts_with("chatgpt")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
        || lower.contains("codex")
    {
        return Some("openai");
    }

    if lower.starts_with("claude") {
        return Some("anthropic");
    }

    if lower.starts_with("gemini") {
        return Some("google");
    }

    None
}

fn synthesize_custom_model_meta(model_id: &str, provider: &str) -> ModelMeta {
    match provider {
        "openai" => synthesize_openai_model_meta(model_id),
        "anthropic" => ModelMeta {
            id: model_id.into(),
            provider: provider.into(),
            name: model_id.into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        "google" => ModelMeta {
            id: model_id.into(),
            provider: provider.into(),
            name: model_id.into(),
            context_window: 1_048_576,
            max_output_tokens: 65_536,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        _ => ModelMeta {
            id: model_id.into(),
            provider: provider.into(),
            name: model_id.into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: false,
                images: false,
                tool_use: true,
            },
        },
    }
}

fn synthesize_openai_model_meta(model_id: &str) -> ModelMeta {
    match model_id {
        "gpt-4o" => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: "GPT-4o (legacy custom)".into(),
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
        "o3" => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: "o3 (legacy custom)".into(),
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
        "o4-mini" => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: "o4-mini (legacy custom)".into(),
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
        "gpt-5.3-codex-spark" => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: "GPT-5.3 Codex Spark (preview)".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
        _ if model_id.starts_with("gpt-5.3-codex") || model_id.contains("codex") => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: model_id.into(),
            context_window: 400_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
        _ if model_id.contains("chat-latest") => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: model_id.into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
        _ if model_id.starts_with("gpt-5") => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: model_id.into(),
            context_window: 400_000,
            max_output_tokens: 128_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        _ if model_id.starts_with('o') => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: model_id.into(),
            context_window: 200_000,
            max_output_tokens: 100_000,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: true,
                images: true,
                tool_use: true,
            },
        },
        _ => ModelMeta {
            id: model_id.into(),
            provider: "openai".into(),
            name: model_id.into(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing::default(),
            capabilities: Capabilities {
                reasoning: false,
                images: true,
                tool_use: true,
            },
        },
    }
}

fn builtin_aliases() -> Vec<(String, String)> {
    vec![
        // Anthropic — sonnet
        ("sonnet".into(), "claude-sonnet-4-6".into()),
        ("claude-sonnet".into(), "claude-sonnet-4-6".into()),
        ("sonnet-4.6".into(), "claude-sonnet-4-6".into()),
        ("sonnet-4.5".into(), "claude-sonnet-4-5-20250929".into()),
        ("sonnet-4".into(), "claude-sonnet-4-20250514".into()),
        // Anthropic — haiku
        ("haiku".into(), "claude-haiku-4-5-20251001".into()),
        ("claude-haiku".into(), "claude-haiku-4-5-20251001".into()),
        ("haiku-4.5".into(), "claude-haiku-4-5-20251001".into()),
        ("haiku-3.5".into(), "claude-haiku-3-5-20241022".into()),
        // Anthropic — opus
        ("opus".into(), "claude-opus-4-6".into()),
        ("claude-opus".into(), "claude-opus-4-6".into()),
        ("opus-4.6".into(), "claude-opus-4-6".into()),
        ("opus-4.5".into(), "claude-opus-4-5-20251101".into()),
        ("opus-4".into(), "claude-opus-4-20250514".into()),
        // OpenAI
        ("gpt5".into(), "gpt-5.4".into()),
        ("gpt5.4".into(), "gpt-5.4".into()),
        ("gpt-5".into(), "gpt-5.4".into()),
        ("gpt-5.4".into(), "gpt-5.4".into()),
        ("gpt5mini".into(), "gpt-5.4-mini".into()),
        ("gpt-5-mini".into(), "gpt-5.4-mini".into()),
        ("gpt5nano".into(), "gpt-5.4-nano".into()),
        ("gpt-5-nano".into(), "gpt-5.4-nano".into()),
        ("chatgpt".into(), "gpt-5.3-chat-latest".into()),
        ("chatgpt-latest".into(), "gpt-5.3-chat-latest".into()),
        ("gpt5chat".into(), "gpt-5.3-chat-latest".into()),
        ("codex".into(), "gpt-5.3-codex".into()),
        ("gpt5codex".into(), "gpt-5.3-codex".into()),
        ("spark".into(), "gpt-5.3-codex-spark".into()),
        ("codex-spark".into(), "gpt-5.3-codex-spark".into()),
        // Google
        ("gemini-pro".into(), "gemini-2.5-pro".into()),
        ("gemini-flash".into(), "gemini-2.5-flash".into()),
        // DeepSeek
        ("deepseek".into(), "deepseek-chat".into()),
        ("deepseek-v3".into(), "deepseek-chat".into()),
        ("deepseek-r1".into(), "deepseek-reasoner".into()),
        // Groq
        ("llama-groq".into(), "llama-3.3-70b-versatile".into()),
        // Mistral
        ("mistral".into(), "mistral-large-latest".into()),
        ("codestral".into(), "codestral-latest".into()),
        // xAI
        ("grok".into(), "grok-3".into()),
        ("grok-mini".into(), "grok-3-mini".into()),
        // Cerebras
        ("cerebras".into(), "llama-3.3-70b".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_by_alias_resolves_sonnet() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("sonnet")
            .expect("sonnet alias should resolve");
        assert_eq!(model.id, "claude-sonnet-4-6");
        assert_eq!(model.provider, "anthropic");
    }

    #[test]
    fn find_by_alias_resolves_haiku() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("haiku")
            .expect("haiku alias should resolve");
        assert_eq!(model.id, "claude-haiku-4-5-20251001");
    }

    #[test]
    fn find_by_alias_resolves_haiku_legacy() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("haiku-3.5")
            .expect("haiku-3.5 alias should resolve");
        assert_eq!(model.id, "claude-haiku-3-5-20241022");
    }

    #[test]
    fn find_by_alias_resolves_opus() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("opus")
            .expect("opus alias should resolve");
        assert_eq!(model.id, "claude-opus-4-6");
    }

    #[test]
    fn find_by_alias_resolves_gpt5() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("gpt5")
            .expect("gpt5 alias should resolve");
        assert_eq!(model.id, "gpt-5.4");
    }

    #[test]
    fn find_by_alias_resolves_chatgpt() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("chatgpt")
            .expect("chatgpt alias should resolve");
        assert_eq!(model.id, "gpt-5.3-chat-latest");
    }

    #[test]
    fn find_by_alias_resolves_codex() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .find_by_alias("codex")
            .expect("codex alias should resolve");
        assert_eq!(model.id, "gpt-5.3-codex");
    }

    #[test]
    fn resolve_meta_synthesizes_spark_preview() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .resolve_meta("spark", None)
            .expect("spark alias should synthesize");
        assert_eq!(model.id, "gpt-5.3-codex-spark");
        assert_eq!(model.provider, "openai");
    }

    #[test]
    fn resolve_meta_synthesizes_legacy_openai_model() {
        let reg = ModelRegistry::with_builtins();
        let model = reg
            .resolve_meta("gpt-4o", None)
            .expect("legacy openai model should synthesize");
        assert_eq!(model.id, "gpt-4o");
        assert_eq!(model.provider, "openai");
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
            .find_by_alias("gpt-5.3-codex")
            .expect("exact id lookup should work as fallback");
        assert_eq!(model.id, "gpt-5.3-codex");
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
        assert_eq!(anthropic.len(), 8);
        assert!(anthropic.iter().all(|m| m.provider == "anthropic"));

        let openai = reg.list_by_provider("openai");
        assert_eq!(openai.len(), 6);

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
