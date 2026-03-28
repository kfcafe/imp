use std::path::PathBuf;
use std::sync::Arc;

use imp_llm::Model;

use crate::agent::{Agent, AgentHandle};
use crate::config::Config;
use crate::error::Result;
use crate::resources;
use crate::roles::Role;
use crate::system_prompt::{self, TaskContext};
use crate::tools::ToolRegistry;

/// Builder for creating a fully wired [`Agent`] from config and context.
///
/// Handles resource discovery, hook loading, system prompt assembly, and tool
/// registration so callers don't need to repeat this boilerplate.
pub struct AgentBuilder {
    config: Config,
    cwd: PathBuf,
    model: Model,
    api_key: String,
    role: Option<Role>,
    task: Option<TaskContext>,
    /// Override the assembled system prompt entirely.
    system_prompt_override: Option<String>,
    /// Additional tool registrar called after native tools are registered.
    #[allow(clippy::type_complexity)]
    extra_tools: Option<Box<dyn FnOnce(&mut ToolRegistry) + Send>>,
    /// Lua extension tool loader — called after native and extra tools.
    ///
    /// The imp-lua crate provides the actual implementation; the binary
    /// crate wires it in to avoid a cyclic dependency between imp-core
    /// and imp-lua.
    #[allow(clippy::type_complexity)]
    lua_tool_loader: Option<Box<dyn FnOnce(&mut ToolRegistry) + Send>>,
}

impl AgentBuilder {
    /// Create a new builder.
    pub fn new(config: Config, cwd: PathBuf, model: Model, api_key: String) -> Self {
        Self {
            config,
            cwd,
            model,
            api_key,
            role: None,
            task: None,
            system_prompt_override: None,
            extra_tools: None,
            lua_tool_loader: None,
        }
    }

    /// Set the role for this agent.
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Set the task context (headless/task mode — Layer 5 of the system prompt).
    pub fn task(mut self, task: TaskContext) -> Self {
        self.task = Some(task);
        self
    }

    /// Override the assembled system prompt with a custom string.
    /// When set, resource discovery and assembly are skipped.
    pub fn system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt_override = Some(prompt);
        self
    }

    /// Register additional tools after the native tools are registered.
    pub fn extra_tools<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut ToolRegistry) + Send + 'static,
    {
        self.extra_tools = Some(Box::new(f));
        self
    }

    /// Register a Lua extension tool loader.
    ///
    /// The provided closure should discover `.lua` extensions, create a
    /// `LuaRuntime`, and register the resulting tools onto the registry.
    /// This is called after native + extra tools are registered but before
    /// mode filtering.
    ///
    /// The binary crate typically calls this with a closure that invokes
    /// `imp_lua::load_lua_extensions()`.
    pub fn lua_tool_loader<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut ToolRegistry) + Send + 'static,
    {
        self.lua_tool_loader = Some(Box::new(f));
        self
    }

    /// Build the agent, wiring config → thresholds, hooks, resources, and tools.
    ///
    /// Returns `(Agent, AgentHandle)` ready for use.
    pub fn build(self) -> Result<(Agent, AgentHandle)> {
        let (mut agent, handle) = Agent::new(self.model, self.cwd.clone());

        // Wire API key
        agent.api_key = self.api_key;

        // Wire thinking level from config
        if let Some(thinking) = self.config.thinking {
            agent.thinking_level = thinking;
        }

        // Wire max turns from config
        if let Some(max_turns) = self.config.max_turns {
            agent.max_turns = max_turns;
        }

        // Wire context thresholds from config
        agent.context_config = self.config.context.clone();

        // Wire role overrides (role can further override thinking/max_turns)
        if let Some(ref role) = self.role {
            if let Some(thinking) = role.thinking_level {
                agent.thinking_level = thinking;
            }
            if let Some(max_turns) = role.max_turns {
                agent.max_turns = max_turns;
            }
            agent.role = Some(role.clone());
        }

        // Load hooks from config
        agent.hooks.load_from_config(self.config.hooks.clone());

        // Wire agent mode from config
        agent.mode = self.config.mode;

        // Wire guardrails config
        agent.guardrail_config = self.config.guardrails.clone();
        agent.guardrail_profile = if self.config.guardrails.is_enabled() {
            Some(self.config.guardrails.resolve_effective_profile(&self.cwd))
        } else {
            None
        };

        // Register native tools
        register_native_tools(&mut agent.tools);

        // Register any extra tools provided by the caller
        if let Some(extra) = self.extra_tools {
            extra(&mut agent.tools);
        }

        // Load Lua extension tools (provided by the binary crate via lua_tool_loader)
        if let Some(lua_loader) = self.lua_tool_loader {
            lua_loader(&mut agent.tools);
        }

        // Filter registered tools to those allowed by the mode.
        // Full mode allows everything — no filtering needed.
        if agent.mode != crate::config::AgentMode::Full {
            let mode = agent.mode;
            agent.tools.retain(|name| mode.allows_tool(name));
        }

        // Assemble system prompt
        agent.system_prompt = if let Some(prompt) = self.system_prompt_override {
            prompt
        } else {
            let user_config_dir = Config::user_config_dir();
            let agents_md = resources::discover_agents_md(&self.cwd, &user_config_dir);
            let skills = resources::discover_skills(&self.cwd, &user_config_dir);

            // Layer 6: Load agent memory if learning is enabled
            let (memory_block, user_block) = if self.config.learning.enabled {
                let mem = crate::memory::MemoryStore::load(
                    &user_config_dir.join("memory.md"),
                    self.config.learning.memory_char_limit,
                )
                .ok()
                .map(|s| s.render("MEMORY (your personal notes)"));

                let user = crate::memory::MemoryStore::load(
                    &user_config_dir.join("user.md"),
                    self.config.learning.user_char_limit,
                )
                .ok()
                .map(|s| s.render("USER PROFILE"));

                (mem, user)
            } else {
                (None, None)
            };

            system_prompt::assemble(&system_prompt::AssembleParams {
                tools: &agent.tools,
                agents_md: &agents_md,
                skills: &skills,
                facts: &[],
                task: self.task.as_ref(),
                role: self.role.as_ref(),
                mode: &agent.mode,
                memory: memory_block.as_deref(),
                user_profile: user_block.as_deref(),
                cwd: Some(&self.cwd),
                learning_enabled: self.config.learning.enabled,
                guardrail_profile: agent.guardrail_profile,
            })
            .text
        };

        Ok((agent, handle))
    }
}

/// Register the standard set of native tools onto a tool registry.
///
/// This is the canonical list — update here when adding or removing tools.
pub fn register_native_tools(tools: &mut ToolRegistry) {
    use crate::tools::{
        ask::AskTool, bash::BashTool, diff::DiffTool, edit::EditTool, extend::ExtendTool,
        find::FindTool, grep::GrepTool, ls::LsTool, mana::ManaTool, read::ReadTool, scan::ScanTool,
        web::WebTool, write::WriteTool,
    };

    tools.register(Arc::new(AskTool));
    tools.register(Arc::new(BashTool));
    tools.register(Arc::new(DiffTool));
    tools.register(Arc::new(EditTool));
    tools.register(Arc::new(ExtendTool));
    tools.register(Arc::new(FindTool));
    tools.register(Arc::new(GrepTool));
    tools.register(Arc::new(LsTool));
    tools.register(Arc::new(ManaTool));
    tools.register(Arc::new(ReadTool));
    tools.register(Arc::new(WriteTool));
    tools.register(Arc::new(ScanTool));
    tools.register(Arc::new(WebTool));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_core::Stream;
    use imp_llm::{
        auth::{ApiKey, AuthStore},
        model::{Capabilities, ModelMeta, ModelPricing},
        provider::Provider,
        Context, Model, RequestOptions, StreamEvent,
    };

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn stream(
            &self,
            _model: &Model,
            _context: Context,
            _options: RequestOptions,
            _api_key: &str,
        ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
            Box::pin(futures::stream::empty())
        }

        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
            Ok("test-key".to_string())
        }

        fn id(&self) -> &str {
            "mock"
        }

        fn models(&self) -> &[ModelMeta] {
            &[]
        }
    }

    fn test_model() -> Model {
        Model {
            meta: ModelMeta {
                id: "test-model".to_string(),
                provider: "mock".to_string(),
                name: "Test Model".to_string(),
                context_window: 200_000,
                max_output_tokens: 4096,
                pricing: ModelPricing {
                    input_per_mtok: 3.0,
                    output_per_mtok: 15.0,
                    cache_read_per_mtok: 0.3,
                    cache_write_per_mtok: 3.75,
                },
                capabilities: Capabilities {
                    reasoning: false,
                    images: false,
                    tool_use: true,
                },
            },
            provider: Arc::new(MockProvider),
        }
    }

    #[test]
    fn builder_applies_config_max_turns() {
        let mut config = Config::default();
        config.max_turns = Some(42);

        let (agent, _handle) =
            AgentBuilder::new(config, PathBuf::from("/tmp"), test_model(), "key".into())
                .build()
                .unwrap();

        assert_eq!(agent.max_turns, 42);
    }

    #[test]
    fn builder_applies_context_config_thresholds() {
        let mut config = Config::default();
        config.context.observation_mask_threshold = 0.5;
        config.context.mask_window = 7;

        let (agent, _handle) =
            AgentBuilder::new(config, PathBuf::from("/tmp"), test_model(), "key".into())
                .build()
                .unwrap();

        assert!((agent.context_config.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(agent.context_config.mask_window, 7);
    }

    #[test]
    fn builder_default_config_uses_standard_thresholds() {
        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            PathBuf::from("/tmp"),
            test_model(),
            "key".into(),
        )
        .build()
        .unwrap();

        assert!((agent.context_config.observation_mask_threshold - 0.6).abs() < f64::EPSILON);
        assert_eq!(agent.context_config.mask_window, 10);
    }

    #[test]
    fn builder_system_prompt_override_skips_discovery() {
        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            PathBuf::from("/tmp"),
            test_model(),
            "key".into(),
        )
        .system_prompt("custom system prompt".into())
        .build()
        .unwrap();

        assert_eq!(agent.system_prompt, "custom system prompt");
    }

    #[test]
    fn builder_api_key_wired() {
        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            PathBuf::from("/tmp"),
            test_model(),
            "my-api-key".into(),
        )
        .build()
        .unwrap();

        assert_eq!(agent.api_key, "my-api-key");
    }

    #[test]
    fn builder_extra_tools_registered() {
        use crate::tools::{Tool, ToolContext, ToolOutput};

        struct DummyTool;

        #[async_trait]
        impl Tool for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn label(&self) -> &str {
                "Dummy"
            }
            fn description(&self) -> &str {
                "A dummy tool for testing"
            }
            fn parameters(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            fn is_readonly(&self) -> bool {
                true
            }
            async fn execute(
                &self,
                _call_id: &str,
                _params: serde_json::Value,
                _ctx: ToolContext,
            ) -> crate::error::Result<ToolOutput> {
                Ok(ToolOutput::text("ok"))
            }
        }

        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            PathBuf::from("/tmp"),
            test_model(),
            "key".into(),
        )
        .extra_tools(|tools| tools.register(Arc::new(DummyTool)))
        .build()
        .unwrap();

        assert!(agent.tools.get("dummy").is_some());
    }

    #[test]
    fn builder_hooks_loaded_from_config() {
        use crate::hooks::HookDef;

        let mut config = Config::default();
        config.hooks.push(HookDef {
            event: "after_file_write".into(),
            match_pattern: Some("*.rs".into()),
            action: "shell".into(),
            command: Some("echo hook fired".into()),
            blocking: false,
            threshold: None,
        });

        let (agent, _handle) =
            AgentBuilder::new(config, PathBuf::from("/tmp"), test_model(), "key".into())
                .build()
                .unwrap();

        // Hooks were loaded — the runner should have one registered hook
        assert_eq!(agent.hooks.len(), 1);
    }
}
