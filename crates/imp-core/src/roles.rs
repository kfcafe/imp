use imp_llm::ThinkingLevel;
use serde::{Deserialize, Serialize};

/// Role definition from config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDef {
    pub model: Option<String>,
    pub thinking: Option<ThinkingLevel>,
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub readonly: bool,
    pub instructions: Option<String>,
    pub max_turns: Option<u32>,
}

/// Resolved role ready for use.
#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub model: Option<String>,
    pub thinking_level: Option<ThinkingLevel>,
    pub tool_set: ToolSet,
    pub readonly: bool,
    pub instructions: Option<String>,
    pub max_turns: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum ToolSet {
    All,
    Only(Vec<String>),
}

impl Role {
    /// Create a role from a definition.
    pub fn from_def(name: &str, def: &RoleDef) -> Self {
        let tool_set = match &def.tools {
            Some(tools) => ToolSet::Only(tools.clone()),
            None if def.readonly => ToolSet::Only(vec![
                "read".into(),
                "grep".into(),
                "find".into(),
                "ls".into(),
                "probe_search".into(),
                "probe_extract".into(),
            ]),
            None => ToolSet::All,
        };

        Self {
            name: name.to_string(),
            model: def.model.clone(),
            thinking_level: def.thinking,
            tool_set,
            readonly: def.readonly,
            instructions: def.instructions.clone(),
            max_turns: def.max_turns,
        }
    }
}

/// Built-in role definitions.
pub fn builtin_roles() -> Vec<(&'static str, RoleDef)> {
    vec![
        ("worker", RoleDef {
            model: None,
            thinking: Some(ThinkingLevel::Medium),
            tools: None,
            readonly: false,
            instructions: None,
            max_turns: None,
        }),
        ("explorer", RoleDef {
            model: Some("haiku".into()),
            thinking: Some(ThinkingLevel::Off),
            tools: Some(vec![
                "read".into(), "grep".into(), "find".into(),
                "ls".into(), "probe_search".into(), "probe_extract".into(),
            ]),
            readonly: true,
            instructions: Some("Explore and summarize. Do not modify files.".into()),
            max_turns: Some(20),
        }),
        ("reviewer", RoleDef {
            model: Some("sonnet".into()),
            thinking: Some(ThinkingLevel::High),
            tools: None,
            readonly: true,
            instructions: None,
            max_turns: Some(10),
        }),
    ]
}
