use std::path::Path;
use std::sync::Arc;

use glob::Pattern;
use imp_llm::{AssistantMessage, ContentBlock, Message, ToolResultMessage};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Hook definition from TOML config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    pub event: String,
    #[serde(rename = "match")]
    pub match_pattern: Option<String>,
    pub action: String,
    pub command: Option<String>,
    #[serde(default)]
    pub blocking: bool,
    pub threshold: Option<f64>,
}

/// What a hook does when triggered.
#[derive(Clone)]
pub enum HookAction {
    /// Run a shell command with interpolation ({file}, {tool_name}).
    Shell { command: String },
    /// A programmatic callback (for Lua or other extensions).
    Callback(Arc<dyn Fn(&HookEvent<'_>) -> HookResult + Send + Sync>),
}

impl std::fmt::Debug for HookAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookAction::Shell { command } => {
                f.debug_struct("Shell").field("command", command).finish()
            }
            HookAction::Callback(_) => f.write_str("Callback(...)"),
        }
    }
}

/// A fully resolved hook definition ready for execution.
#[derive(Debug, Clone)]
pub struct HookDefinition {
    pub event: String,
    pub match_pattern: Option<String>,
    pub action: HookAction,
    pub blocking: bool,
    pub threshold: Option<f64>,
}

/// Runtime hook events.
pub enum HookEvent<'a> {
    AfterFileWrite {
        file: &'a Path,
    },
    BeforeToolCall {
        tool_name: &'a str,
        args: &'a serde_json::Value,
    },
    AfterToolCall {
        tool_name: &'a str,
        result: &'a ToolResultMessage,
    },
    BeforeLlmCall,
    OnContextThreshold {
        ratio: f64,
    },
    OnSessionStart,
    OnSessionShutdown,
    OnAgentStart {
        prompt: &'a str,
    },
    OnAgentEnd {
        messages: &'a [Message],
    },
    OnTurnEnd {
        index: u32,
        message: &'a AssistantMessage,
    },
}

impl<'a> HookEvent<'a> {
    /// Return the canonical event name for matching against hook definitions.
    fn event_name(&self) -> &'static str {
        match self {
            HookEvent::AfterFileWrite { .. } => "after_file_write",
            HookEvent::BeforeToolCall { .. } => "before_tool_call",
            HookEvent::AfterToolCall { .. } => "after_tool_call",
            HookEvent::BeforeLlmCall => "before_llm_call",
            HookEvent::OnContextThreshold { .. } => "on_context_threshold",
            HookEvent::OnSessionStart => "on_session_start",
            HookEvent::OnSessionShutdown => "on_session_shutdown",
            HookEvent::OnAgentStart { .. } => "on_agent_start",
            HookEvent::OnAgentEnd { .. } => "on_agent_end",
            HookEvent::OnTurnEnd { .. } => "on_turn_end",
        }
    }
}

/// Result from a hook execution.
#[derive(Default, Debug)]
pub struct HookResult {
    pub block: bool,
    pub reason: Option<String>,
    pub modified_content: Option<Vec<ContentBlock>>,
}

/// Manages and executes hooks.
pub struct HookRunner {
    /// TOML-defined hooks (fire first, in config order).
    toml_hooks: Vec<HookDefinition>,
    /// Programmatically registered hooks (fire after TOML hooks, in registration order).
    programmatic_hooks: Vec<HookDefinition>,
}

impl HookRunner {
    pub fn new() -> Self {
        Self {
            toml_hooks: Vec::new(),
            programmatic_hooks: Vec::new(),
        }
    }

    /// Add a single TOML hook def (raw from config).
    pub fn add(&mut self, def: HookDef) {
        if let Some(resolved) = resolve_hook_def(def) {
            self.toml_hooks.push(resolved);
        }
    }

    /// Load multiple TOML hook defs from config.
    pub fn load_from_config(&mut self, defs: Vec<HookDef>) {
        for def in defs {
            self.add(def);
        }
    }

    /// Register a programmatic hook (for Lua or other extensions).
    pub fn register(&mut self, hook: HookDefinition) {
        self.programmatic_hooks.push(hook);
    }

    /// Returns the total number of registered hooks (TOML + programmatic).
    pub fn len(&self) -> usize {
        self.toml_hooks.len() + self.programmatic_hooks.len()
    }

    /// Returns true if no hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.toml_hooks.is_empty() && self.programmatic_hooks.is_empty()
    }

    /// Register a callback hook for a specific event.
    pub fn register_callback(
        &mut self,
        event: &str,
        callback: Arc<dyn Fn(&HookEvent<'_>) -> HookResult + Send + Sync>,
    ) {
        self.programmatic_hooks.push(HookDefinition {
            event: event.to_string(),
            match_pattern: None,
            action: HookAction::Callback(callback),
            blocking: true,
            threshold: None,
        });
    }

    /// Fire a hook event and collect results.
    ///
    /// Execution order: TOML hooks first (config order), then programmatic hooks (registration order).
    /// Blocking hooks execute sequentially and await completion.
    /// Non-blocking hooks are spawned as background tokio tasks.
    pub async fn fire(&self, event: &HookEvent<'_>) -> Vec<HookResult> {
        let mut results = Vec::new();

        // TOML hooks first, then programmatic hooks
        let all_hooks = self.toml_hooks.iter().chain(self.programmatic_hooks.iter());

        for hook in all_hooks {
            if !matches_event(hook, event) {
                continue;
            }

            if hook.blocking {
                let result = execute_hook(hook, event).await;
                results.push(result);
            } else {
                // Fire-and-forget: spawn the command but don't wait for it
                if let HookAction::Shell { command } = &hook.action {
                    let cmd = interpolate_command(command, event);
                    tokio::spawn(async move {
                        let _ = Command::new("sh")
                            .arg("-c")
                            .arg(&cmd)
                            .stdin(std::process::Stdio::null())
                            .output()
                            .await;
                    });
                }
                // Non-blocking hooks don't contribute results
            }
        }

        results
    }
}

impl Default for HookRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a raw TOML HookDef into a resolved HookDefinition.
fn resolve_hook_def(def: HookDef) -> Option<HookDefinition> {
    let action = match def.action.as_str() {
        "shell" => {
            let command = def.command?;
            HookAction::Shell { command }
        }
        _ => return None,
    };

    Some(HookDefinition {
        event: def.event,
        match_pattern: def.match_pattern,
        action,
        blocking: def.blocking,
        threshold: def.threshold,
    })
}

/// Check if a hook definition matches the given event.
fn matches_event(hook: &HookDefinition, event: &HookEvent<'_>) -> bool {
    // Event name must match
    if hook.event != event.event_name() {
        return false;
    }

    // Check match_pattern if present
    if let Some(pattern) = &hook.match_pattern {
        match event {
            HookEvent::AfterFileWrite { file } => {
                let file_str = file.to_string_lossy();
                // Try glob matching against the full path and filename
                if let Ok(glob) = Pattern::new(pattern) {
                    let file_name = file
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if !glob.matches(&file_str) && !glob.matches(&file_name) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            HookEvent::BeforeToolCall { tool_name, .. }
            | HookEvent::AfterToolCall { tool_name, .. } => {
                if pattern != *tool_name {
                    // Also try glob matching on tool name
                    if let Ok(glob) = Pattern::new(pattern) {
                        if !glob.matches(tool_name) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            _ => {
                // Other events ignore match_pattern
            }
        }
    }

    // Check threshold for OnContextThreshold
    if let HookEvent::OnContextThreshold { ratio } = event {
        if let Some(threshold) = hook.threshold {
            if *ratio < threshold {
                return false;
            }
        }
    }

    true
}

/// Interpolate variables into a shell command string.
fn interpolate_command(command: &str, event: &HookEvent<'_>) -> String {
    let mut result = command.to_string();

    match event {
        HookEvent::AfterFileWrite { file } => {
            result = result.replace("{file}", &file.to_string_lossy());
        }
        HookEvent::BeforeToolCall { tool_name, .. } => {
            result = result.replace("{tool_name}", tool_name);
        }
        HookEvent::AfterToolCall {
            tool_name,
            result: tool_result,
        } => {
            result = result.replace("{tool_name}", tool_name);
            result = result.replace(
                "{is_error}",
                if tool_result.is_error {
                    "true"
                } else {
                    "false"
                },
            );
            // Extract exit_code from details if present (bash tool sets this)
            let exit_code = tool_result
                .details
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .map(|c| c.to_string())
                .unwrap_or_default();
            result = result.replace("{exit_code}", &exit_code);
            // First line of output for summary
            let output_first = tool_result
                .content
                .iter()
                .filter_map(|b| match b {
                    imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .next()
                .and_then(|t| t.lines().next())
                .unwrap_or("");
            result = result.replace("{output_first_line}", output_first);
            // Extract command from details (bash tool stores it)
            let command = tool_result
                .details
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            result = result.replace("{command}", command);
        }
        HookEvent::OnContextThreshold { ratio } => {
            result = result.replace("{ratio}", &ratio.to_string());
        }
        HookEvent::OnTurnEnd { index, .. } => {
            result = result.replace("{index}", &index.to_string());
        }
        _ => {}
    }

    result
}

/// Execute a single hook and return its result.
async fn execute_hook(hook: &HookDefinition, event: &HookEvent<'_>) -> HookResult {
    match &hook.action {
        HookAction::Shell { command } => {
            let cmd = interpolate_command(command, event);
            match Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .stdin(std::process::Stdio::null())
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    // A non-zero exit code on a BeforeToolCall hook means "block"
                    let block = matches!(event, HookEvent::BeforeToolCall { .. })
                        && !output.status.success();

                    let reason = if block {
                        Some(if stderr.is_empty() {
                            stdout.clone()
                        } else {
                            stderr
                        })
                    } else {
                        None
                    };

                    // For AfterToolCall, stdout is treated as modified content
                    let modified_content = if matches!(event, HookEvent::AfterToolCall { .. })
                        && !stdout.trim().is_empty()
                        && output.status.success()
                    {
                        Some(vec![ContentBlock::Text {
                            text: stdout.trim().to_string(),
                        }])
                    } else {
                        None
                    };

                    HookResult {
                        block,
                        reason,
                        modified_content,
                    }
                }
                Err(e) => HookResult {
                    block: false,
                    reason: Some(format!("Hook command failed: {e}")),
                    modified_content: None,
                },
            }
        }
        HookAction::Callback(cb) => cb(event),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn hook_def_toml_parsing() {
        let toml_str = r#"
[[hooks]]
event = "after_file_write"
match = "*.rs"
action = "shell"
command = "rustfmt {file}"
blocking = true

[[hooks]]
event = "on_context_threshold"
action = "shell"
command = "echo threshold"
threshold = 0.8
"#;

        #[derive(Deserialize)]
        struct Wrapper {
            hooks: Vec<HookDef>,
        }

        let parsed: Wrapper = toml::from_str(toml_str).expect("TOML parsing failed");
        assert_eq!(parsed.hooks.len(), 2);

        let h0 = &parsed.hooks[0];
        assert_eq!(h0.event, "after_file_write");
        assert_eq!(h0.match_pattern.as_deref(), Some("*.rs"));
        assert_eq!(h0.action, "shell");
        assert_eq!(h0.command.as_deref(), Some("rustfmt {file}"));
        assert!(h0.blocking);
        assert!(h0.threshold.is_none());

        let h1 = &parsed.hooks[1];
        assert_eq!(h1.event, "on_context_threshold");
        assert!(h1.match_pattern.is_none());
        assert_eq!(h1.threshold, Some(0.8));
    }

    #[test]
    fn hook_interpolation_file() {
        let event = HookEvent::AfterFileWrite {
            file: Path::new("/tmp/test.rs"),
        };
        let result = interpolate_command("rustfmt {file}", &event);
        assert_eq!(result, "rustfmt /tmp/test.rs");
    }

    #[test]
    fn hook_interpolation_tool_name() {
        let args = serde_json::json!({"path": "/tmp"});
        let event = HookEvent::BeforeToolCall {
            tool_name: "bash",
            args: &args,
        };
        let result = interpolate_command("echo {tool_name}", &event);
        assert_eq!(result, "echo bash");
    }

    #[test]
    fn hook_interpolation_ratio() {
        let event = HookEvent::OnContextThreshold { ratio: 0.75 };
        let result = interpolate_command("echo ratio={ratio}", &event);
        assert_eq!(result, "echo ratio=0.75");
    }

    #[test]
    fn hook_event_name_mapping() {
        let path = PathBuf::from("/tmp/test.rs");
        assert_eq!(
            HookEvent::AfterFileWrite { file: &path }.event_name(),
            "after_file_write"
        );
        assert_eq!(HookEvent::BeforeLlmCall.event_name(), "before_llm_call");
        assert_eq!(HookEvent::OnSessionStart.event_name(), "on_session_start");
        assert_eq!(
            HookEvent::OnSessionShutdown.event_name(),
            "on_session_shutdown"
        );
        assert_eq!(
            HookEvent::OnContextThreshold { ratio: 0.5 }.event_name(),
            "on_context_threshold"
        );
    }

    #[test]
    fn hook_matches_event_name() {
        let hook = HookDefinition {
            event: "after_file_write".into(),
            match_pattern: None,
            action: HookAction::Shell {
                command: "echo hi".into(),
            },
            blocking: false,
            threshold: None,
        };
        let path = PathBuf::from("/tmp/test.rs");
        let event = HookEvent::AfterFileWrite { file: &path };
        assert!(matches_event(&hook, &event));

        let wrong_event = HookEvent::BeforeLlmCall;
        assert!(!matches_event(&hook, &wrong_event));
    }

    #[test]
    fn hook_matches_file_glob() {
        let hook = HookDefinition {
            event: "after_file_write".into(),
            match_pattern: Some("*.rs".into()),
            action: HookAction::Shell {
                command: "echo hi".into(),
            },
            blocking: false,
            threshold: None,
        };

        let rs_path = PathBuf::from("/tmp/test.rs");
        let rs_event = HookEvent::AfterFileWrite { file: &rs_path };
        assert!(matches_event(&hook, &rs_event));

        let py_path = PathBuf::from("/tmp/test.py");
        let py_event = HookEvent::AfterFileWrite { file: &py_path };
        assert!(!matches_event(&hook, &py_event));
    }

    #[test]
    fn hook_matches_tool_name() {
        let hook = HookDefinition {
            event: "before_tool_call".into(),
            match_pattern: Some("bash".into()),
            action: HookAction::Shell {
                command: "echo hi".into(),
            },
            blocking: true,
            threshold: None,
        };

        let args = serde_json::json!({});
        let match_event = HookEvent::BeforeToolCall {
            tool_name: "bash",
            args: &args,
        };
        assert!(matches_event(&hook, &match_event));

        let no_match_event = HookEvent::BeforeToolCall {
            tool_name: "read",
            args: &args,
        };
        assert!(!matches_event(&hook, &no_match_event));
    }

    #[test]
    fn hook_threshold_filtering() {
        let hook = HookDefinition {
            event: "on_context_threshold".into(),
            match_pattern: None,
            action: HookAction::Shell {
                command: "echo hi".into(),
            },
            blocking: true,
            threshold: Some(0.8),
        };

        // Below threshold — should not match
        let below = HookEvent::OnContextThreshold { ratio: 0.5 };
        assert!(!matches_event(&hook, &below));

        // At threshold — should match
        let at = HookEvent::OnContextThreshold { ratio: 0.8 };
        assert!(matches_event(&hook, &at));

        // Above threshold — should match
        let above = HookEvent::OnContextThreshold { ratio: 0.95 };
        assert!(matches_event(&hook, &above));
    }

    #[test]
    fn hook_resolve_shell() {
        let def = HookDef {
            event: "after_file_write".into(),
            match_pattern: Some("*.rs".into()),
            action: "shell".into(),
            command: Some("rustfmt {file}".into()),
            blocking: true,
            threshold: None,
        };
        let resolved = resolve_hook_def(def).expect("should resolve");
        assert_eq!(resolved.event, "after_file_write");
        assert!(resolved.blocking);
        assert!(matches!(resolved.action, HookAction::Shell { .. }));
    }

    #[test]
    fn hook_resolve_missing_command_returns_none() {
        let def = HookDef {
            event: "after_file_write".into(),
            match_pattern: None,
            action: "shell".into(),
            command: None,
            blocking: false,
            threshold: None,
        };
        assert!(resolve_hook_def(def).is_none());
    }

    #[test]
    fn hook_resolve_unknown_action_returns_none() {
        let def = HookDef {
            event: "after_file_write".into(),
            match_pattern: None,
            action: "unknown".into(),
            command: Some("echo".into()),
            blocking: false,
            threshold: None,
        };
        assert!(resolve_hook_def(def).is_none());
    }

    #[tokio::test]
    async fn hook_blocking_shell_executes() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "after_file_write".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some("echo hello".into()),
            blocking: true,
            threshold: None,
        }]);

        let path = PathBuf::from("/tmp/test.txt");
        let event = HookEvent::AfterFileWrite { file: &path };
        let results = runner.fire(&event).await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].block);
    }

    #[tokio::test]
    async fn hook_non_blocking_fires_and_forgets() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "on_session_start".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some("echo non-blocking".into()),
            blocking: false,
            threshold: None,
        }]);

        let event = HookEvent::OnSessionStart;
        let results = runner.fire(&event).await;
        // Non-blocking hooks don't return results
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn hook_before_tool_call_blocks() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "before_tool_call".into(),
            match_pattern: Some("bash".into()),
            action: "shell".into(),
            command: Some("exit 1".into()),
            blocking: true,
            threshold: None,
        }]);

        let args = serde_json::json!({"command": "rm -rf /"});
        let event = HookEvent::BeforeToolCall {
            tool_name: "bash",
            args: &args,
        };
        let results = runner.fire(&event).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].block);
    }

    #[tokio::test]
    async fn hook_before_tool_call_allows() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "before_tool_call".into(),
            match_pattern: Some("read".into()),
            action: "shell".into(),
            command: Some("exit 0".into()),
            blocking: true,
            threshold: None,
        }]);

        let args = serde_json::json!({});
        let event = HookEvent::BeforeToolCall {
            tool_name: "read",
            args: &args,
        };
        let results = runner.fire(&event).await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].block);
    }

    #[tokio::test]
    async fn hook_after_tool_call_modifies_result() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "after_tool_call".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some("echo modified output".into()),
            blocking: true,
            threshold: None,
        }]);

        let result_msg = ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "test".into(),
            content: vec![ContentBlock::Text {
                text: "original".into(),
            }],
            is_error: false,
            details: serde_json::Value::Null,
            timestamp: 0,
        };
        let event = HookEvent::AfterToolCall {
            tool_name: "test",
            result: &result_msg,
        };
        let results = runner.fire(&event).await;
        assert_eq!(results.len(), 1);
        let modified = results[0]
            .modified_content
            .as_ref()
            .expect("should have modified content");
        assert_eq!(modified.len(), 1);
        if let ContentBlock::Text { text } = &modified[0] {
            assert_eq!(text, "modified output");
        } else {
            panic!("expected Text content block");
        }
    }

    #[tokio::test]
    async fn hook_context_threshold_fires_at_correct_ratio() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "on_context_threshold".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some("echo threshold hit at {ratio}".into()),
            blocking: true,
            threshold: Some(0.8),
        }]);

        // Below threshold — no results
        let below = HookEvent::OnContextThreshold { ratio: 0.5 };
        let results = runner.fire(&below).await;
        assert!(results.is_empty());

        // At threshold — should fire
        let at = HookEvent::OnContextThreshold { ratio: 0.8 };
        let results = runner.fire(&at).await;
        assert_eq!(results.len(), 1);

        // Above threshold — should fire
        let above = HookEvent::OnContextThreshold { ratio: 0.95 };
        let results = runner.fire(&above).await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn hook_execution_order_toml_first_then_programmatic() {
        use std::sync::Mutex;

        let order = Arc::new(Mutex::new(Vec::new()));

        let mut runner = HookRunner::new();

        // TOML hook
        runner.load_from_config(vec![HookDef {
            event: "on_session_start".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some("echo toml".into()),
            blocking: true,
            threshold: None,
        }]);

        // Programmatic hook
        let order_clone = Arc::clone(&order);
        runner.register_callback(
            "on_session_start",
            Arc::new(move |_event| {
                order_clone.lock().unwrap().push("programmatic");
                HookResult::default()
            }),
        );

        let event = HookEvent::OnSessionStart;
        let results = runner.fire(&event).await;

        // Both should fire
        assert_eq!(results.len(), 2);

        // Programmatic should have recorded its execution
        let recorded = order.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], "programmatic");
    }

    #[tokio::test]
    async fn hook_callback_blocks_tool_call() {
        let mut runner = HookRunner::new();
        runner.register_callback(
            "before_tool_call",
            Arc::new(|_event| HookResult {
                block: true,
                reason: Some("blocked by callback".into()),
                modified_content: None,
            }),
        );

        let args = serde_json::json!({});
        let event = HookEvent::BeforeToolCall {
            tool_name: "bash",
            args: &args,
        };
        let results = runner.fire(&event).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].block);
        assert_eq!(results[0].reason.as_deref(), Some("blocked by callback"));
    }

    #[tokio::test]
    async fn hook_shell_interpolation_in_execution() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();
        let marker_file = tempfile::NamedTempFile::new().unwrap();
        let marker_path = marker_file.path().to_string_lossy().to_string();

        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "after_file_write".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some(format!("echo {{file}} > {marker_path}")),
            blocking: true,
            threshold: None,
        }]);

        let event = HookEvent::AfterFileWrite { file: &tmp_path };
        runner.fire(&event).await;

        // Verify the marker file contains the interpolated path
        let content = std::fs::read_to_string(&marker_path).unwrap();
        assert!(
            content.contains(&tmp_path.to_string_lossy().to_string()),
            "Expected marker to contain file path, got: {content}"
        );
    }

    #[test]
    fn hook_runner_load_from_config_resolves_all() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![
            HookDef {
                event: "after_file_write".into(),
                match_pattern: Some("*.rs".into()),
                action: "shell".into(),
                command: Some("rustfmt {file}".into()),
                blocking: true,
                threshold: None,
            },
            HookDef {
                event: "before_tool_call".into(),
                match_pattern: Some("bash".into()),
                action: "shell".into(),
                command: Some("echo checking".into()),
                blocking: true,
                threshold: None,
            },
        ]);
        assert_eq!(runner.toml_hooks.len(), 2);
    }

    #[tokio::test]
    async fn hook_unmatched_event_returns_empty() {
        let mut runner = HookRunner::new();
        runner.load_from_config(vec![HookDef {
            event: "on_session_start".into(),
            match_pattern: None,
            action: "shell".into(),
            command: Some("echo hi".into()),
            blocking: true,
            threshold: None,
        }]);

        // Fire a different event
        let event = HookEvent::BeforeLlmCall;
        let results = runner.fire(&event).await;
        assert!(results.is_empty());
    }
}
