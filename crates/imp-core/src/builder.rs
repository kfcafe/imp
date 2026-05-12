use std::path::PathBuf;
use std::sync::Arc;

use imp_llm::Model;

use crate::agent::{Agent, AgentHandle};
use crate::config::{Config, LuaCapabilityPolicy};
use crate::error::Result;
use crate::mana_prompt_context;
use crate::policy::RunPolicy;
use crate::resources;
use crate::roles::Role;
use crate::system_prompt::{self, Fact, TaskContext};
use crate::tools::{LuaToolLoader, ToolRegistry};
use crate::workflow::{AutonomyMode, ImplicitWorkflowContractInput, WorkflowContract};

fn load_scoped_memory_block(
    cwd: &std::path::Path,
    path: &std::path::Path,
    label: &str,
    char_limit: usize,
) -> Option<String> {
    let store = crate::memory::MemoryStore::load(path, char_limit).ok()?;
    let filtered: Vec<String> = store
        .entries()
        .iter()
        .filter(|entry| !entry.contains("/tower") || cwd.to_string_lossy().contains("/tower"))
        .cloned()
        .collect();

    if filtered.is_empty() {
        return None;
    }

    let used: usize = filtered.iter().map(|e| e.len()).sum::<usize>()
        + if filtered.len() > 1 {
            (filtered.len() - 1) * 3
        } else {
            0
        };
    let pct = if char_limit > 0 {
        (used as f64 / char_limit as f64 * 100.0) as u32
    } else {
        0
    };
    let bar = "══════════════════════════════════════════════";
    Some(format!(
        "{bar}\n{label} [{pct}% — {used}/{char_limit} chars]\n{bar}\n{}",
        filtered.join("\n§\n")
    ))
}

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
    facts: Vec<Fact>,
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
    lua_tool_loader: Option<LuaToolLoader>,
    /// Per-run tool/write policy layered on top of AgentMode.
    run_policy: RunPolicy,
    /// Optional workflow contract override. If absent, build creates an implicit contract.
    workflow_contract: Option<WorkflowContract>,
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
            facts: Vec::new(),
            system_prompt_override: None,
            extra_tools: None,
            lua_tool_loader: None,
            run_policy: RunPolicy::default(),
            workflow_contract: None,
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

    /// Set task-specific facts to inject into the system prompt.
    pub fn facts(mut self, facts: Vec<Fact>) -> Self {
        self.facts = facts;
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
        F: Fn(&LuaCapabilityPolicy, &mut ToolRegistry) + Send + Sync + 'static,
    {
        self.lua_tool_loader = Some(Arc::new(f));
        self
    }

    /// Apply a per-run policy on top of the configured agent mode.
    pub fn run_policy(mut self, policy: RunPolicy) -> Self {
        self.run_policy = policy;
        self
    }

    /// Override the implicit workflow contract for this agent run.
    pub fn workflow_contract(mut self, contract: WorkflowContract) -> Self {
        self.workflow_contract = Some(contract);
        self
    }

    /// Set the autonomy mode on the implicit workflow contract.
    pub fn autonomy_mode(mut self, mode: AutonomyMode) -> Self {
        let mut contract = self.workflow_contract.unwrap_or_else(|| {
            WorkflowContract::implicit_from(
                ImplicitWorkflowContractInput::prompt("").cwd(&self.cwd),
            )
        });
        contract.autonomy_mode = mode;
        self.workflow_contract = Some(contract);
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

        // Wire max output tokens from config
        if let Some(max_tokens) = self.config.max_tokens {
            agent.max_tokens = Some(max_tokens);
        }

        // Wire context thresholds from config
        agent.context_config = self.config.context.clone();

        // Wire role overrides.
        if let Some(ref role) = self.role {
            if let Some(thinking) = role.thinking_level {
                agent.thinking_level = thinking;
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

        // Wire read tool truncation from config
        agent.read_max_lines = self.config.ui.read_max_lines;
        agent.continue_policy = self.config.ui.continue_policy;
        agent.config = Arc::new(self.config.clone());
        agent.run_policy = self.run_policy;
        agent.lua_tool_loader = self.lua_tool_loader.clone();

        // Register native tools
        register_native_tools(&mut agent.tools);

        // Register any extra tools provided by the caller
        if let Some(extra) = self.extra_tools {
            extra(&mut agent.tools);
        }

        // Load Lua extension tools (provided by the binary crate via lua_tool_loader)
        if let Some(lua_loader) = self.lua_tool_loader {
            let lua_policy = self.config.lua.resolve_policy(agent.mode);
            lua_loader(&lua_policy, &mut agent.tools);
        }

        // Load project-local TypeScript extension tools from .imp/extensions.
        if let Err(err) =
            crate::typescript_extensions::load_typescript_extensions(&self.cwd, &mut agent.tools)
        {
            eprintln!("Failed to load TypeScript extensions: {err}");
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
            let soul = resources::discover_soul(&self.cwd, &user_config_dir);
            let skills = resources::discover_skills(&self.cwd, &user_config_dir);
            agent.has_mana_skill = skills.iter().any(|skill| skill.name == "mana");
            agent.has_mana_basics_skill = skills.iter().any(|skill| skill.name == "mana-basics");
            agent.has_mana_delegation_skill =
                skills.iter().any(|skill| skill.name == "mana-delegation");

            // Layer 6: Load agent memory if learning is enabled
            let (memory_block, user_block) = if self.config.learning.enabled {
                let mem = load_scoped_memory_block(
                    &self.cwd,
                    &user_config_dir.join("memory.md"),
                    "MEMORY (your personal notes)",
                    self.config.learning.memory_char_limit,
                );

                let user = load_scoped_memory_block(
                    &self.cwd,
                    &user_config_dir.join("user.md"),
                    "USER PROFILE",
                    self.config.learning.user_char_limit,
                );

                (mem, user)
            } else {
                (None, None)
            };

            let prompt_context = if self.facts.is_empty() {
                mana_prompt_context::load_session_prompt_context(&self.cwd)
            } else {
                mana_prompt_context::SessionPromptContext {
                    facts: self.facts.clone(),
                    project_memory_status: None,
                }
            };

            system_prompt::assemble(&system_prompt::AssembleParams {
                tools: &agent.tools,
                agents_md: &agents_md,
                skills: &skills,
                facts: &prompt_context.facts,
                project_memory_status: prompt_context.project_memory_status.as_deref(),
                personality: Some(&self.config.personality.profile),
                soul: soul.as_ref(),
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
        ask::AskTool, bash::BashTool, edit::EditTool, extend::ExtendTool, git::GitTool,
        imp::ImpTool, mana::ManaTool, read::ReadTool, scan::ScanTool,
        session_search::SessionSearchTool, web::WebTool, worktree::WorktreeTool, write::WriteTool,
    };

    tools.register(Arc::new(AskTool));
    tools.register(Arc::new(BashTool::canonical()));
    tools.register(Arc::new(EditTool));
    tools.register(Arc::new(ExtendTool));
    tools.register(Arc::new(GitTool));
    tools.register(Arc::new(ImpTool));
    tools.register(Arc::new(ManaTool::default()));
    tools.register(Arc::new(ReadTool));
    tools.register(Arc::new(WriteTool));
    tools.register(Arc::new(ScanTool));
    tools.register(Arc::new(SessionSearchTool));
    tools.register(Arc::new(WebTool));
    tools.register(Arc::new(WorktreeTool));
    tools.register_alias("imp", "ask_agent");
    tools.register_alias("session_search", "recall");
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
    fn builder_applies_config_max_tokens() {
        let config = Config {
            max_tokens: Some(2048),
            ..Default::default()
        };

        let (agent, _handle) =
            AgentBuilder::new(config, PathBuf::from("/tmp"), test_model(), "key".into())
                .build()
                .unwrap();

        assert_eq!(agent.max_tokens, Some(2048));
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
    fn builder_registers_canonical_tools_and_compat_aliases() {
        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            PathBuf::from("/tmp"),
            test_model(),
            "key".into(),
        )
        .build()
        .unwrap();

        assert!(agent.tools.get("bash").is_some());
        assert!(agent.tools.get("shell").is_none());
        assert!(agent.tools.get("sh").is_none());
        assert!(agent.tools.get("ask_agent").is_some());
        assert!(agent.tools.get("imp").is_some());
        assert!(agent.tools.get("spawn").is_none());
        assert!(agent.tools.get("edit").is_some());
        assert!(agent.tools.get("multi_edit").is_none());
        assert!(agent.tools.get("memory").is_none());
        assert!(agent.tools.get("recall").is_some());
        assert!(agent.tools.get("session_search").is_some());
        assert!(agent.tools.get("git").is_some());

        let mut definition_names: Vec<_> = agent
            .tools
            .definitions()
            .into_iter()
            .map(|definition| definition.name)
            .collect();
        definition_names.sort();

        assert!(definition_names.contains(&"bash".to_string()));
        assert!(definition_names.contains(&"ask_agent".to_string()));
        assert!(!definition_names.contains(&"spawn".to_string()));
        assert!(definition_names.contains(&"edit".to_string()));
        assert!(!definition_names.contains(&"imp".to_string()));
        assert!(!definition_names.contains(&"multi_edit".to_string()));
        assert!(definition_names.contains(&"recall".to_string()));
        assert!(!definition_names.contains(&"session_search".to_string()));
        assert!(!definition_names.contains(&"memory".to_string()));
    }

    #[test]
    fn builder_filters_tower_memory_outside_tower_projects() {
        let temp = tempfile::TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", temp.path());

        let imp_dir = temp.path().join("imp");
        std::fs::create_dir_all(&imp_dir).unwrap();
        std::fs::write(
            imp_dir.join("memory.md"),
            "Project lives at /Users/asher/tower and uses root mana.",
        )
        .unwrap();
        std::fs::write(
            imp_dir.join("user.md"),
            "User prefers root mana in /tower for Tower work.",
        )
        .unwrap();

        let mut config = Config::default();
        config.learning.enabled = true;

        let (agent, _handle) = AgentBuilder::new(
            config,
            PathBuf::from("/tmp/not-tower/project"),
            test_model(),
            "key".into(),
        )
        .build()
        .unwrap();

        assert!(!agent.system_prompt.contains("/Users/asher/tower"));
        assert!(!agent.system_prompt.contains("/tower for Tower work"));

        if let Some(prev) = prev {
            std::env::set_var("XDG_CONFIG_HOME", prev);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    fn builder_keeps_tower_memory_inside_tower_projects() {
        let temp = tempfile::TempDir::new().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", temp.path());

        let imp_dir = temp.path().join("imp");
        std::fs::create_dir_all(&imp_dir).unwrap();
        std::fs::write(
            imp_dir.join("memory.md"),
            "Project lives at /Users/asher/tower and uses root mana.",
        )
        .unwrap();

        let mut config = Config::default();
        config.learning.enabled = true;

        let (agent, _handle) = AgentBuilder::new(
            config,
            PathBuf::from("/Users/asher/tower/imp"),
            test_model(),
            "key".into(),
        )
        .build()
        .unwrap();

        assert!(agent.system_prompt.contains("/Users/asher/tower"));

        if let Some(prev) = prev {
            std::env::set_var("XDG_CONFIG_HOME", prev);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    fn builder_injects_mana_facts_into_system_prompt_when_available() {
        let temp = tempfile::TempDir::new().unwrap();
        let mana_dir = temp.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();

        let mut mana_config = mana_core::config::Config::default();
        mana_config.project = "test".to_string();
        mana_config.save(&mana_dir).unwrap();

        let mut working = mana_core::unit::Unit::new("1", "Implement auth flow");
        working.status = mana_core::unit::Status::InProgress;
        working.paths = vec!["src/auth.rs".to_string()];
        working.requires = vec!["AuthProvider".to_string()];
        let working_slug = mana_core::util::title_to_slug(&working.title);
        working
            .to_file(mana_dir.join(format!("1-{}.md", working_slug)))
            .unwrap();

        let mut fact = mana_core::unit::Unit::new("2", "Auth uses RS256 signing");
        fact.unit_type = "fact".to_string();
        fact.paths = vec!["src/auth.rs".to_string()];
        fact.produces = vec!["AuthProvider".to_string()];
        fact.last_verified = Some(chrono::Utc::now() - chrono::Duration::hours(2));
        let fact_slug = mana_core::util::title_to_slug(&fact.title);
        fact.to_file(mana_dir.join(format!("2-{}.md", fact_slug)))
            .unwrap();

        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            temp.path().join("src"),
            test_model(),
            "key".into(),
        )
        .build()
        .unwrap();

        assert!(agent.system_prompt.contains("Project facts:"));
        assert!(agent.system_prompt.contains("Auth uses RS256 signing"));
        assert!(agent.system_prompt.contains("verified 2h ago"));
        assert!(agent.system_prompt.contains("Project memory status:"));
        assert!(agent.system_prompt.contains("Working on:"));
        assert!(agent.system_prompt.contains("[1] Implement auth flow"));
    }

    #[test]
    fn builder_injects_project_memory_status_into_system_prompt_when_available() {
        let temp = tempfile::TempDir::new().unwrap();
        let mana_dir = temp.path().join(".mana");
        std::fs::create_dir(&mana_dir).unwrap();

        let mut mana_config = mana_core::config::Config::default();
        mana_config.project = "test".to_string();
        mana_config.save(&mana_dir).unwrap();

        let mut working = mana_core::unit::Unit::new("1", "Implement auth flow");
        working.status = mana_core::unit::Status::InProgress;
        working.claimed_by = Some("imp".to_string());
        let working_slug = mana_core::util::title_to_slug(&working.title);
        working
            .to_file(mana_dir.join(format!("1-{}.md", working_slug)))
            .unwrap();

        let mut recent = mana_core::unit::Unit::new("3", "Recently closed cleanup");
        recent.status = mana_core::unit::Status::Closed;
        recent.closed_at = Some(chrono::Utc::now() - chrono::Duration::hours(2));
        let recent_slug = mana_core::util::title_to_slug(&recent.title);
        let archive_dir = mana_dir.join("archive").join("closed");
        std::fs::create_dir_all(&archive_dir).unwrap();
        recent
            .to_file(archive_dir.join(format!("3-{}.md", recent_slug)))
            .unwrap();

        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            temp.path().join("src"),
            test_model(),
            "key".into(),
        )
        .build()
        .unwrap();

        assert!(agent.system_prompt.contains("Project memory status:"));
        assert!(agent.system_prompt.contains("Working on:"));
        assert!(agent.system_prompt.contains("[1] Implement auth flow"));
        assert!(agent.system_prompt.contains("Recent work:"));
        assert!(agent.system_prompt.contains("[3] Recently closed cleanup"));
    }

    #[test]
    fn builder_task_fact_override_does_not_add_project_memory_status() {
        let facts = vec![Fact {
            text: "Auth uses RS256 signing".into(),
            verified_ago: "2h ago".into(),
        }];

        let (agent, _handle) = AgentBuilder::new(
            Config::default(),
            PathBuf::from("/tmp"),
            test_model(),
            "key".into(),
        )
        .facts(facts)
        .build()
        .unwrap();

        assert!(agent.system_prompt.contains("Project facts:"));
        assert!(!agent.system_prompt.contains("Project memory status:"));
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
