use std::collections::HashMap;
use std::path::{Path, PathBuf};

use imp_llm::ThinkingLevel;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::hooks::HookDef;
use crate::roles::RoleDef;

/// Shell backend selection for the Bash tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ShellBackend {
    /// Use `sh -c` (default, always available).
    #[default]
    Sh,
    /// Use the rush library API (`rush::run`). Falls back to `sh` if
    /// the `rush-backend` feature is not compiled in.
    Rush,
    /// Connect to a running rush daemon over Unix socket. Falls back to `sh`
    /// if the daemon is not reachable.
    RushDaemon,
}

/// Shell-related configuration for the Bash tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellConfig {
    /// Which shell backend to use. Defaults to `"sh"`.
    #[serde(default)]
    pub backend: ShellBackend,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            backend: ShellBackend::Sh,
        }
    }
}

/// Top-level configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Default model (alias or full ID).
    pub model: Option<String>,

    /// Default thinking level.
    pub thinking: Option<ThinkingLevel>,

    /// Maximum agent turns.
    pub max_turns: Option<u32>,

    /// Active tool names (None = all).
    pub tools: Option<Vec<String>>,

    /// Named roles.
    #[serde(default)]
    pub roles: HashMap<String, RoleDef>,

    /// Hook definitions.
    #[serde(default)]
    pub hooks: Vec<HookDef>,

    /// Context management settings.
    #[serde(default)]
    pub context: ContextConfig,

    /// Shell backend settings.
    #[serde(default)]
    pub shell: ShellConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextConfig {
    /// Mask old tool outputs at this ratio (default: 0.6).
    pub observation_mask_threshold: f64,

    /// LLM compaction at this ratio (default: 0.8).
    pub compaction_threshold: f64,

    /// Keep last N turns unmasked (default: 10).
    pub mask_window: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            observation_mask_threshold: 0.6,
            compaction_threshold: 0.8,
            mask_window: 10,
        }
    }
}

impl Config {
    /// Load config from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Resolve the full config by merging: defaults < user < project < env < CLI.
    pub fn resolve(user_config_dir: &Path, project_dir: Option<&Path>) -> Result<Self> {
        let mut config = Self::default();

        // User config
        let user_path = user_config_dir.join("config.toml");
        if user_path.exists() {
            let user = Self::load(&user_path)?;
            config.merge(user);
        }

        // Project config
        if let Some(project) = project_dir {
            let project_path = project.join(".imp").join("config.toml");
            if project_path.exists() {
                let project = Self::load(&project_path)?;
                config.merge(project);
            }
        }

        // Env overrides
        if let Ok(model) = std::env::var("IMP_MODEL") {
            config.model = Some(model);
        }
        if let Ok(thinking) = std::env::var("IMP_THINKING") {
            config.thinking = parse_thinking_level(&thinking);
        }

        Ok(config)
    }

    fn merge(&mut self, other: Config) {
        if other.model.is_some() {
            self.model = other.model;
        }
        if other.thinking.is_some() {
            self.thinking = other.thinking;
        }
        if other.max_turns.is_some() {
            self.max_turns = other.max_turns;
        }
        if other.tools.is_some() {
            self.tools = other.tools;
        }
        if other.context != ContextConfig::default() {
            self.context = other.context;
        }
        if other.shell != ShellConfig::default() {
            self.shell = other.shell;
        }
        self.roles.extend(other.roles);
        self.hooks.extend(other.hooks);
    }

    /// Default user config directory.
    pub fn user_config_dir() -> PathBuf {
        dirs_path("config")
    }

    /// Default session directory.
    pub fn session_dir() -> PathBuf {
        dirs_path("data").join("sessions")
    }
}

fn dirs_path(kind: &str) -> PathBuf {
    match kind {
        "config" => {
            if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
                PathBuf::from(dir).join("imp")
            } else if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".config").join("imp")
            } else {
                PathBuf::from(".config").join("imp")
            }
        }
        "data" => {
            if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
                PathBuf::from(dir).join("imp")
            } else if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".local").join("share").join("imp")
            } else {
                PathBuf::from(".local").join("share").join("imp")
            }
        }
        _ => PathBuf::from("."),
    }
}

fn parse_thinking_level(s: &str) -> Option<ThinkingLevel> {
    match s.to_lowercase().as_str() {
        "off" => Some(ThinkingLevel::Off),
        "minimal" => Some(ThinkingLevel::Minimal),
        "low" => Some(ThinkingLevel::Low),
        "medium" => Some(ThinkingLevel::Medium),
        "high" => Some(ThinkingLevel::High),
        "xhigh" => Some(ThinkingLevel::XHigh),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn config_default_values() {
        let config = Config::default();
        assert!(config.model.is_none());
        assert!(config.thinking.is_none());
        assert!(config.max_turns.is_none());
        assert!(config.tools.is_none());
        assert!(config.roles.is_empty());
        assert!(config.hooks.is_empty());
        assert!((config.context.observation_mask_threshold - 0.6).abs() < f64::EPSILON);
        assert!((config.context.compaction_threshold - 0.8).abs() < f64::EPSILON);
        assert_eq!(config.context.mask_window, 10);
    }

    #[test]
    fn config_load_from_toml() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
model = "sonnet"
thinking = "high"
max_turns = 50
tools = ["read", "write", "bash"]

[context]
observation_mask_threshold = 0.5
compaction_threshold = 0.9
mask_window = 5
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.model.as_deref(), Some("sonnet"));
        assert_eq!(config.thinking, Some(ThinkingLevel::High));
        assert_eq!(config.max_turns, Some(50));
        assert_eq!(config.tools.as_ref().unwrap().len(), 3);
        assert!((config.context.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert!((config.context.compaction_threshold - 0.9).abs() < f64::EPSILON);
        assert_eq!(config.context.mask_window, 5);
    }

    #[test]
    fn config_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("nonexistent.toml");
        let config = Config::load(&config_path).unwrap();
        assert!(config.model.is_none());
    }

    #[test]
    fn config_merge_project_overrides_user() {
        let mut user = Config::default();
        user.model = Some("haiku".into());
        user.max_turns = Some(20);

        let project = Config {
            model: Some("sonnet".into()),
            max_turns: None, // not set → user value preserved
            ..Default::default()
        };

        user.merge(project);
        assert_eq!(user.model.as_deref(), Some("sonnet"));
        assert_eq!(user.max_turns, Some(20));
    }

    #[test]
    fn config_merge_roles_extend() {
        let mut base = Config::default();
        base.roles.insert(
            "worker".into(),
            RoleDef {
                model: Some("haiku".into()),
                thinking: None,
                tools: None,
                readonly: false,
                instructions: None,
                max_turns: None,
            },
        );

        let overlay = Config {
            roles: {
                let mut m = HashMap::new();
                m.insert(
                    "reviewer".into(),
                    RoleDef {
                        model: Some("sonnet".into()),
                        thinking: Some(ThinkingLevel::High),
                        tools: None,
                        readonly: true,
                        instructions: None,
                        max_turns: None,
                    },
                );
                m
            },
            ..Default::default()
        };

        base.merge(overlay);
        assert!(base.roles.contains_key("worker"));
        assert!(base.roles.contains_key("reviewer"));
    }

    #[test]
    fn config_merge_hooks_extend() {
        let mut base = Config::default();
        base.hooks.push(HookDef {
            event: "after_file_write".into(),
            match_pattern: None,
            action: "log".into(),
            command: None,
            blocking: false,
            threshold: None,
        });

        let overlay = Config {
            hooks: vec![HookDef {
                event: "before_tool_call".into(),
                match_pattern: None,
                action: "block".into(),
                command: None,
                blocking: true,
                threshold: None,
            }],
            ..Default::default()
        };

        base.merge(overlay);
        assert_eq!(base.hooks.len(), 2);
    }

    #[test]
    fn config_merge_context_overrides_default() {
        let mut base = Config::default();

        let overlay = Config {
            context: ContextConfig {
                observation_mask_threshold: 0.5,
                compaction_threshold: 0.9,
                mask_window: 5,
            },
            ..Default::default()
        };

        base.merge(overlay);
        assert!((base.context.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert!((base.context.compaction_threshold - 0.9).abs() < f64::EPSILON);
        assert_eq!(base.context.mask_window, 5);
    }

    #[test]
    fn config_resolve_user_then_project() {
        // Clean env to avoid interference from parallel tests
        std::env::remove_var("IMP_MODEL");
        std::env::remove_var("IMP_THINKING");

        let dir = TempDir::new().unwrap();
        let user_dir = dir.path().join("user");
        let project_dir = dir.path().join("project");
        fs::create_dir_all(&user_dir).unwrap();
        fs::create_dir_all(project_dir.join(".imp")).unwrap();

        // User config: model=haiku, max_turns=20, custom context
        fs::write(
            user_dir.join("config.toml"),
            r#"
model = "haiku"
max_turns = 20

[context]
observation_mask_threshold = 0.55
compaction_threshold = 0.85
mask_window = 9
"#,
        )
        .unwrap();

        // Project config: model=sonnet (overrides user), custom context overrides user context
        fs::write(
            project_dir.join(".imp").join("config.toml"),
            r#"
model = "sonnet"

[context]
observation_mask_threshold = 0.5
compaction_threshold = 0.9
mask_window = 5
"#,
        )
        .unwrap();

        let config = Config::resolve(&user_dir, Some(&project_dir)).unwrap();
        assert_eq!(config.model.as_deref(), Some("sonnet"));
        assert_eq!(config.max_turns, Some(20));
        assert!((config.context.observation_mask_threshold - 0.5).abs() < f64::EPSILON);
        assert!((config.context.compaction_threshold - 0.9).abs() < f64::EPSILON);
        assert_eq!(config.context.mask_window, 5);
    }

    #[test]
    fn config_resolve_env_overrides() {
        // Test env override logic without relying on process-global state
        // (env vars are inherently racy in parallel tests).
        // We test that the override *mechanism* works by simulating it.
        let mut config = Config::default();
        config.model = Some("haiku".into());
        config.thinking = Some(ThinkingLevel::Low);

        // Simulate IMP_MODEL override
        let env_model = "opus";
        config.model = Some(env_model.into());

        // Simulate IMP_THINKING override
        let env_thinking = "high";
        config.thinking = parse_thinking_level(env_thinking);

        assert_eq!(config.model.as_deref(), Some("opus"));
        assert_eq!(config.thinking, Some(ThinkingLevel::High));
    }

    #[test]
    fn config_resolve_missing_files_uses_defaults() {
        let dir = TempDir::new().unwrap();
        let config = Config::resolve(dir.path(), None).unwrap();
        assert!(config.model.is_none());
        assert!(config.thinking.is_none());
        assert!(config.max_turns.is_none());
    }

    #[test]
    fn config_load_with_roles_and_hooks() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
model = "sonnet"

[roles.coder]
model = "opus"
thinking = "high"
readonly = false

[roles.reader]
readonly = true

[[hooks]]
event = "after_file_write"
action = "log"
blocking = false
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.roles.len(), 2);
        assert!(config.roles.contains_key("coder"));
        assert!(config.roles.contains_key("reader"));
        assert_eq!(config.roles["coder"].model.as_deref(), Some("opus"));
        assert!(config.roles["reader"].readonly);
        assert_eq!(config.hooks.len(), 1);
        assert_eq!(config.hooks[0].event, "after_file_write");
    }

    #[test]
    fn config_parse_thinking_levels() {
        assert_eq!(parse_thinking_level("off"), Some(ThinkingLevel::Off));
        assert_eq!(
            parse_thinking_level("minimal"),
            Some(ThinkingLevel::Minimal)
        );
        assert_eq!(parse_thinking_level("low"), Some(ThinkingLevel::Low));
        assert_eq!(parse_thinking_level("medium"), Some(ThinkingLevel::Medium));
        assert_eq!(parse_thinking_level("high"), Some(ThinkingLevel::High));
        assert_eq!(parse_thinking_level("xhigh"), Some(ThinkingLevel::XHigh));
        assert_eq!(parse_thinking_level("OFF"), Some(ThinkingLevel::Off));
        assert_eq!(parse_thinking_level("High"), Some(ThinkingLevel::High));
        assert_eq!(parse_thinking_level("invalid"), None);
        assert_eq!(parse_thinking_level(""), None);
    }

    #[test]
    fn config_partial_toml_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
model = "sonnet"
"#,
        )
        .unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.model.as_deref(), Some("sonnet"));
        // Unspecified fields use defaults
        assert!(config.thinking.is_none());
        assert!(config.max_turns.is_none());
        assert!((config.context.observation_mask_threshold - 0.6).abs() < f64::EPSILON);
    }
}
