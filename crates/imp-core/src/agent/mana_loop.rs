#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaActionClass {
    ReadHelp,
    Inspect,
    ProgressCheckpoint,
    GraphMutation,
    DecisionFact,
    Lifecycle,
    Orchestration,
    Destructive,
    Unknown,
}

pub fn classify_mana_action(action: &str) -> ManaActionClass {
    match action {
        "guide" | "template" => ManaActionClass::ReadHelp,
        "status" | "list" | "show" | "logs" | "agents" | "next" | "tree" | "run_state" => {
            ManaActionClass::Inspect
        }
        "update" | "notes_append" => ManaActionClass::ProgressCheckpoint,
        "create" | "dep_add" | "dep_remove" | "reparent" => ManaActionClass::GraphMutation,
        "decision_add" | "decision_resolve" | "fact_create" | "fact_verify" => {
            ManaActionClass::DecisionFact
        }
        "claim" | "release" | "verify" | "close" | "reopen" | "fail" => ManaActionClass::Lifecycle,
        "run" | "evaluate" => ManaActionClass::Orchestration,
        "delete" => ManaActionClass::Destructive,
        _ => ManaActionClass::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManaPolicyDecision {
    pub action: Option<String>,
    pub class: ManaActionClass,
    pub allowed: bool,
    pub reason: Option<String>,
}

impl ManaPolicyDecision {
    pub fn details(&self) -> serde_json::Value {
        serde_json::json!({
            "mana_loop_policy": {
                "action": self.action,
                "class": self.class.as_str(),
                "allowed": self.allowed,
                "reason": self.reason,
            }
        })
    }
}

impl ManaActionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadHelp => "read_help",
            Self::Inspect => "inspect",
            Self::ProgressCheckpoint => "progress_checkpoint",
            Self::GraphMutation => "graph_mutation",
            Self::DecisionFact => "decision_fact",
            Self::Lifecycle => "lifecycle",
            Self::Orchestration => "orchestration",
            Self::Destructive => "destructive",
            Self::Unknown => "unknown",
        }
    }
}

pub fn evaluate_mana_policy(
    mode: crate::config::AgentMode,
    args: &serde_json::Value,
) -> ManaPolicyDecision {
    let action = args
        .get("action")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let class = action
        .as_deref()
        .map(classify_mana_action)
        .unwrap_or(ManaActionClass::Unknown);

    let denied_reason = action.as_deref().and_then(|action_name| {
        if matches!(action_name, "guide" | "template") || mode.allows_mana_action(action_name) {
            None
        } else {
            Some(format!(
                "Mana action '{action_name}' is not available in {} mode",
                format!("{mode:?}").to_lowercase()
            ))
        }
    });

    ManaPolicyDecision {
        action,
        class,
        allowed: denied_reason.is_none(),
        reason: denied_reason,
    }
}

pub fn enrich_mana_result_details(
    mut details: serde_json::Value,
    policy: &ManaPolicyDecision,
) -> serde_json::Value {
    let policy_details = policy.details();
    let policy_value = policy_details
        .get("mana_loop_policy")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    match details {
        serde_json::Value::Object(ref mut object) => {
            object.insert(
                "mana_action_class".to_string(),
                serde_json::json!(policy.class.as_str()),
            );
            object.insert("mana_loop_policy".to_string(), policy_value);
            details
        }
        serde_json::Value::Null => serde_json::json!({
            "mana_action_class": policy.class.as_str(),
            "mana_loop_policy": policy_value,
        }),
        other => serde_json::json!({
            "value": other,
            "mana_action_class": policy.class.as_str(),
            "mana_loop_policy": policy_value,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{mana::ManaTool, Tool};

    #[test]
    fn mana_action_classification_covers_tool_schema_actions() {
        let schema = ManaTool::default().parameters();
        let actions = schema
            .get("properties")
            .and_then(|properties| properties.get("action"))
            .and_then(|action| action.get("enum"))
            .and_then(|actions| actions.as_array())
            .expect("mana action enum should be present in schema");

        let mut unknown = Vec::new();
        for action in actions {
            let action = action.as_str().expect("mana action should be a string");
            if classify_mana_action(action) == ManaActionClass::Unknown {
                unknown.push(action.to_string());
            }
        }

        assert!(
            unknown.is_empty(),
            "unclassified mana schema actions: {}",
            unknown.join(", ")
        );
    }

    #[test]
    fn mana_action_classification_groups_actions_by_loop_purpose() {
        let cases = [
            ("guide", ManaActionClass::ReadHelp),
            ("template", ManaActionClass::ReadHelp),
            ("show", ManaActionClass::Inspect),
            ("run_state", ManaActionClass::Inspect),
            ("update", ManaActionClass::ProgressCheckpoint),
            ("notes_append", ManaActionClass::ProgressCheckpoint),
            ("create", ManaActionClass::GraphMutation),
            ("reparent", ManaActionClass::GraphMutation),
            ("decision_add", ManaActionClass::DecisionFact),
            ("fact_verify", ManaActionClass::DecisionFact),
            ("verify", ManaActionClass::Lifecycle),
            ("close", ManaActionClass::Lifecycle),
            ("run", ManaActionClass::Orchestration),
            ("evaluate", ManaActionClass::Orchestration),
            ("delete", ManaActionClass::Destructive),
            ("not_real", ManaActionClass::Unknown),
        ];

        for (action, expected) in cases {
            assert_eq!(classify_mana_action(action), expected, "{action}");
        }
    }
}
