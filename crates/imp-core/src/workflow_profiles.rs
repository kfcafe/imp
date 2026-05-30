use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowSuggest {
    #[default]
    Ask,
    Auto,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkflowProfileDef {
    pub description: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub suggest: Option<WorkflowSuggest>,
    pub readonly: Option<bool>,
    pub tools: Option<Vec<String>>,
    pub triggers: Option<Vec<String>>,
    pub confirm_title: Option<String>,
    pub confirm_body: Option<String>,
    pub instructions: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowProfile {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub suggest: WorkflowSuggest,
    pub readonly: bool,
    pub tools: Vec<String>,
    pub triggers: Vec<String>,
    pub confirm_title: String,
    pub confirm_body: String,
    pub instructions: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSuggestion {
    pub profile: WorkflowProfile,
    pub trigger: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowProfileError {
    InvalidName(String),
    InvalidAlias {
        profile: String,
        alias: String,
    },
    UnknownWorkflow(String),
    AliasConflict {
        alias: String,
        first: String,
        second: String,
    },
    MissingInstructions(String),
}

impl std::fmt::Display for WorkflowProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "invalid workflow name `{name}`"),
            Self::InvalidAlias { profile, alias } => {
                write!(f, "invalid alias `{alias}` for workflow `{profile}`")
            }
            Self::UnknownWorkflow(name) => write!(f, "unknown workflow `{name}`"),
            Self::AliasConflict {
                alias,
                first,
                second,
            } => write!(
                f,
                "workflow alias `{alias}` is used by both `{first}` and `{second}`"
            ),
            Self::MissingInstructions(name) => {
                write!(f, "workflow `{name}` must define instructions")
            }
        }
    }
}

impl std::error::Error for WorkflowProfileError {}

#[derive(Debug, Clone)]
pub struct WorkflowRegistry {
    profiles: BTreeMap<String, WorkflowProfile>,
    aliases: BTreeMap<String, String>,
}

impl WorkflowRegistry {
    pub fn from_overrides(
        overrides: impl IntoIterator<Item = (String, WorkflowProfileDef)>,
    ) -> Result<Self, WorkflowProfileError> {
        let mut defs: BTreeMap<String, WorkflowProfileDef> = builtin_workflow_defs()
            .into_iter()
            .map(|(name, def)| (name.to_string(), def))
            .collect();

        for (name, override_def) in overrides {
            validate_name(&name)?;
            let base = defs.remove(&name).unwrap_or_default();
            defs.insert(name, merge_def(base, override_def));
        }

        let mut profiles = BTreeMap::new();
        for (name, def) in defs {
            let profile = WorkflowProfile::from_def(&name, def)?;
            profiles.insert(name, profile);
        }

        let mut aliases = BTreeMap::new();
        for (name, profile) in &profiles {
            for alias in &profile.aliases {
                validate_alias(name, alias)?;
                if profiles.contains_key(alias) {
                    return Err(WorkflowProfileError::AliasConflict {
                        alias: alias.clone(),
                        first: alias.clone(),
                        second: name.clone(),
                    });
                }
                if let Some(first) = aliases.insert(alias.clone(), name.clone()) {
                    return Err(WorkflowProfileError::AliasConflict {
                        alias: alias.clone(),
                        first,
                        second: name.clone(),
                    });
                }
            }
        }

        Ok(Self { profiles, aliases })
    }

    pub fn iter(&self) -> impl Iterator<Item = &WorkflowProfile> {
        self.profiles.values()
    }

    pub fn get(&self, name_or_alias: &str) -> Option<&WorkflowProfile> {
        self.profiles.get(name_or_alias).or_else(|| {
            self.aliases
                .get(name_or_alias)
                .and_then(|name| self.profiles.get(name))
        })
    }

    pub fn resolve(&self, name_or_alias: &str) -> Result<&WorkflowProfile, WorkflowProfileError> {
        self.get(name_or_alias)
            .ok_or_else(|| WorkflowProfileError::UnknownWorkflow(name_or_alias.to_string()))
    }

    pub fn infer(&self, prompt: &str) -> Option<WorkflowSuggestion> {
        let normalized = normalize(prompt);
        self.profiles
            .values()
            .filter(|profile| profile.suggest != WorkflowSuggest::Never)
            .filter_map(|profile| {
                profile
                    .triggers
                    .iter()
                    .find(|trigger| normalized.contains(&normalize(trigger)))
                    .map(|trigger| WorkflowSuggestion {
                        profile: profile.clone(),
                        trigger: trigger.clone(),
                    })
            })
            .max_by_key(|suggestion| normalize(&suggestion.trigger).len())
    }
}

impl WorkflowProfile {
    fn from_def(name: &str, def: WorkflowProfileDef) -> Result<Self, WorkflowProfileError> {
        validate_name(name)?;
        let instructions = def
            .instructions
            .ok_or_else(|| WorkflowProfileError::MissingInstructions(name.to_string()))?;
        if instructions.trim().is_empty() {
            return Err(WorkflowProfileError::MissingInstructions(name.to_string()));
        }
        Ok(Self {
            name: name.to_string(),
            description: def.description.unwrap_or_default(),
            aliases: def.aliases.unwrap_or_default(),
            suggest: def.suggest.unwrap_or_default(),
            readonly: def.readonly.unwrap_or(false),
            tools: def.tools.unwrap_or_default(),
            triggers: def.triggers.unwrap_or_default(),
            confirm_title: def.confirm_title.unwrap_or_else(|| format!("Use /{name}?")),
            confirm_body: def.confirm_body.unwrap_or_default(),
            instructions,
            role: def.role,
        })
    }

    pub fn wrap_prompt(&self, prompt: &str) -> String {
        if self.instructions.contains("{{prompt}}") {
            self.instructions.replace("{{prompt}}", prompt)
        } else {
            format!(
                "{}\n\nUser request:\n{}",
                self.instructions.trim_end(),
                prompt
            )
        }
    }
}

fn merge_def(mut base: WorkflowProfileDef, override_def: WorkflowProfileDef) -> WorkflowProfileDef {
    if override_def.description.is_some() {
        base.description = override_def.description;
    }
    if override_def.aliases.is_some() {
        base.aliases = override_def.aliases;
    }
    if override_def.suggest.is_some() {
        base.suggest = override_def.suggest;
    }
    if override_def.readonly.is_some() {
        base.readonly = override_def.readonly;
    }
    if override_def.tools.is_some() {
        base.tools = override_def.tools;
    }
    if override_def.triggers.is_some() {
        base.triggers = override_def.triggers;
    }
    if override_def.confirm_title.is_some() {
        base.confirm_title = override_def.confirm_title;
    }
    if override_def.confirm_body.is_some() {
        base.confirm_body = override_def.confirm_body;
    }
    if override_def.instructions.is_some() {
        base.instructions = override_def.instructions;
    }
    if override_def.role.is_some() {
        base.role = override_def.role;
    }
    base
}

fn normalize(value: &str) -> String {
    value.to_lowercase()
}

fn validate_name(name: &str) -> Result<(), WorkflowProfileError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(WorkflowProfileError::InvalidName(name.to_string()));
    }
    Ok(())
}

fn validate_alias(profile: &str, alias: &str) -> Result<(), WorkflowProfileError> {
    if validate_name(alias).is_err() {
        return Err(WorkflowProfileError::InvalidAlias {
            profile: profile.to_string(),
            alias: alias.to_string(),
        });
    }
    Ok(())
}

pub fn builtin_workflow_defs() -> Vec<(&'static str, WorkflowProfileDef)> {
    vec![
        (
            "plan",
            WorkflowProfileDef {
                description: Some("Plan work without editing files.".into()),
                aliases: Some(vec!["spec".into(), "design".into()]),
                suggest: Some(WorkflowSuggest::Ask),
                readonly: Some(true),
                tools: Some(vec!["read".into(), "rg".into(), "mana".into()]),
                triggers: Some(vec![
                    "plan".into(),
                    "outline".into(),
                    "break down".into(),
                    "decompose".into(),
                    "design".into(),
                    "spec".into(),
                ]),
                confirm_title: Some("Use /plan?".into()),
                confirm_body: Some("Save plan".into()),
                role: Some("planner".into()),
                instructions: Some(
                    "Task workflow: plan\n\nUser request:\n{{prompt}}\n\nInstructions:\n- Do not edit files.\n- Inspect relevant context before proposing implementation.\n- Ask clarifying questions only if needed.\n- Produce a clear plan with risks, tests, and next steps.\n- Save durable work only when it materially helps.\n- Stop after the plan unless the user asks to implement."
                        .into(),
                ),
            },
        ),
        (
            "review",
            WorkflowProfileDef {
                description: Some("Review work without editing files.".into()),
                aliases: Some(vec!["audit".into()]),
                suggest: Some(WorkflowSuggest::Ask),
                readonly: Some(true),
                tools: Some(vec!["read".into(), "rg".into(), "bash".into()]),
                triggers: Some(vec![
                    "review".into(),
                    "audit".into(),
                    "look over".into(),
                    "critique".into(),
                    "find issues".into(),
                ]),
                confirm_title: Some("Use /review?".into()),
                confirm_body: Some("Review only".into()),
                role: Some("reviewer".into()),
                instructions: Some(
                    "Task workflow: review\n\nUser request:\n{{prompt}}\n\nInstructions:\n- Review only; do not edit files unless explicitly asked.\n- Inspect relevant files, diffs, and tests.\n- Prioritize correctness, regressions, security, UX, and maintainability.\n- Return actionable findings with evidence.\n- If no issues are found, say what was checked."
                        .into(),
                ),
            },
        ),
        (
            "verify",
            WorkflowProfileDef {
                description: Some("Verify behavior and report evidence.".into()),
                aliases: Some(vec!["test".into(), "check".into()]),
                suggest: Some(WorkflowSuggest::Ask),
                readonly: Some(true),
                tools: Some(vec!["read".into(), "rg".into(), "bash".into()]),
                triggers: Some(vec![
                    "verify".into(),
                    "validate".into(),
                    "make sure".into(),
                    "confirm".into(),
                    "run tests".into(),
                ]),
                confirm_title: Some("Use /verify?".into()),
                confirm_body: Some("Run verification".into()),
                role: Some("verifier".into()),
                instructions: Some(
                    "Task workflow: verify\n\nUser request:\n{{prompt}}\n\nInstructions:\n- Verify the requested behavior with the narrowest useful checks.\n- Do not edit files unless explicitly asked.\n- Report commands, results, evidence, and remaining uncertainty.\n- Stop when verification is complete or blocked."
                        .into(),
                ),
            },
        ),
        (
            "implement",
            WorkflowProfileDef {
                description: Some("Make a focused change and verify it.".into()),
                aliases: Some(vec!["fix".into(), "build".into()]),
                suggest: Some(WorkflowSuggest::Never),
                readonly: Some(false),
                tools: Some(vec!["read".into(), "rg".into(), "edit".into(), "write".into(), "bash".into()]),
                triggers: Some(vec!["implement".into(), "build".into(), "fix".into(), "change".into()]),
                confirm_title: Some("Use /implement?".into()),
                confirm_body: Some("Implement".into()),
                role: Some("coder".into()),
                instructions: Some(
                    "Task workflow: implement\n\nUser request:\n{{prompt}}\n\nInstructions:\n- Make the smallest coherent change that solves the request.\n- Inspect relevant files before editing.\n- Verify with the narrowest useful check.\n- Summarize what changed, what was verified, and what remains.\n- Stop after closeout."
                        .into(),
                ),
            },
        ),
        (
            "research",
            WorkflowProfileDef {
                description: Some("Research and answer with evidence.".into()),
                aliases: Some(vec!["investigate".into()]),
                suggest: Some(WorkflowSuggest::Ask),
                readonly: Some(true),
                tools: Some(vec!["read".into(), "rg".into(), "web".into()]),
                triggers: Some(vec!["research".into(), "investigate".into(), "find out".into()]),
                confirm_title: Some("Use /research?".into()),
                confirm_body: Some("Research".into()),
                role: Some("researcher".into()),
                instructions: Some(
                    "Task workflow: research\n\nUser request:\n{{prompt}}\n\nInstructions:\n- Gather evidence before answering.\n- Do not edit files unless explicitly asked.\n- Cite sources or inspected files.\n- Separate evidence from inference.\n- Stop with a concise answer and useful next steps."
                        .into(),
                ),
            },
        ),
        (
            "debug",
            WorkflowProfileDef {
                description: Some("Diagnose a problem before fixing it.".into()),
                aliases: Some(vec!["diagnose".into()]),
                suggest: Some(WorkflowSuggest::Ask),
                readonly: Some(false),
                tools: Some(vec!["read".into(), "rg".into(), "bash".into(), "edit".into()]),
                triggers: Some(vec![
                    "debug".into(),
                    "diagnose".into(),
                    "failing".into(),
                    "broken".into(),
                    "error".into(),
                    "regression".into(),
                ]),
                confirm_title: Some("Use /debug?".into()),
                confirm_body: Some("Debug".into()),
                role: Some("integrator".into()),
                instructions: Some(
                    "Task workflow: debug\n\nUser request:\n{{prompt}}\n\nInstructions:\n- Reproduce or inspect the failure before fixing.\n- Identify the likely root cause from evidence.\n- Make a focused fix only when the cause is clear.\n- Verify the fix.\n- Stop at a real blocker instead of guessing."
                        .into(),
                ),
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_includes_builtin_workflows() {
        let registry =
            WorkflowRegistry::from_overrides(Vec::<(String, WorkflowProfileDef)>::new()).unwrap();
        for name in ["plan", "review", "verify", "implement", "research", "debug"] {
            assert!(registry.get(name).is_some(), "missing {name}");
        }
        assert_eq!(registry.get("spec").unwrap().name, "plan");
    }

    #[test]
    fn config_overrides_builtin_workflow() {
        let registry = WorkflowRegistry::from_overrides(vec![(
            "plan".into(),
            WorkflowProfileDef {
                confirm_body: Some("Save plan".into()),
                instructions: Some("Custom plan: {{prompt}}".into()),
                ..WorkflowProfileDef::default()
            },
        )])
        .unwrap();
        let plan = registry.get("plan").unwrap();
        assert_eq!(plan.confirm_body, "Save plan");
        assert_eq!(plan.wrap_prompt("ship it"), "Custom plan: ship it");
        assert!(plan.readonly, "partial override should keep builtin fields");
    }

    #[test]
    fn custom_workflow_alias_and_trigger_resolve() {
        let registry = WorkflowRegistry::from_overrides(vec![(
            "security-review".into(),
            WorkflowProfileDef {
                description: Some("Security review".into()),
                aliases: Some(vec!["sec".into()]),
                triggers: Some(vec!["security review".into(), "audit auth".into()]),
                readonly: Some(true),
                instructions: Some("Security: {{prompt}}".into()),
                ..WorkflowProfileDef::default()
            },
        )])
        .unwrap();
        assert_eq!(registry.get("sec").unwrap().name, "security-review");
        let suggestion = registry
            .infer("can you audit auth in this change?")
            .unwrap();
        assert_eq!(suggestion.profile.name, "security-review");
    }

    #[test]
    fn infer_ignores_never_suggest_profiles() {
        let registry =
            WorkflowRegistry::from_overrides(Vec::<(String, WorkflowProfileDef)>::new()).unwrap();
        let suggestion = registry.infer("please implement this change");
        assert_ne!(suggestion.map(|s| s.profile.name), Some("implement".into()));
    }

    #[test]
    fn invalid_alias_conflict_fails() {
        let err = WorkflowRegistry::from_overrides(vec![
            (
                "one".into(),
                WorkflowProfileDef {
                    aliases: Some(vec!["same".into()]),
                    instructions: Some("One {{prompt}}".into()),
                    ..WorkflowProfileDef::default()
                },
            ),
            (
                "two".into(),
                WorkflowProfileDef {
                    aliases: Some(vec!["same".into()]),
                    instructions: Some("Two {{prompt}}".into()),
                    ..WorkflowProfileDef::default()
                },
            ),
        ])
        .unwrap_err();
        assert!(matches!(err, WorkflowProfileError::AliasConflict { .. }));
    }
}
