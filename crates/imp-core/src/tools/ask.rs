use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;
use crate::ui::SelectOption;

pub struct AskTool;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OptionItem {
    Label(String),
    Rich {
        label: String,
        description: Option<String>,
    },
}

impl OptionItem {
    #[allow(dead_code)]
    fn into_select_option(self) -> SelectOption {
        match self {
            OptionItem::Label(label) => SelectOption {
                label,
                description: None,
            },
            OptionItem::Rich { label, description } => SelectOption { label, description },
        }
    }

    fn label(&self) -> &str {
        match self {
            OptionItem::Label(l) => l,
            OptionItem::Rich { label, .. } => label,
        }
    }
}

#[async_trait]
impl Tool for AskTool {
    fn name(&self) -> &str {
        "ask"
    }
    fn label(&self) -> &str {
        "Ask User"
    }
    fn description(&self) -> &str {
        "Ask the user a question. Provide options for multiple choice, omit for free text."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "question": { "type": "string" },
                "context": { "type": "string" },
                "options": { "type": "array", "items": {}, "description": "Strings or {label, description}" },
                "multiSelect": { "type": "boolean" },
                "allowOther": { "type": "boolean" },
                "default": {},
                "placeholder": { "type": "string" }
            },
            "required": ["question"]
        })
    }
    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput> {
        if !ctx.ui.has_ui() {
            return Ok(ToolOutput::error("Cannot ask user in this mode"));
        }

        let question = match params["question"].as_str() {
            Some(q) => q,
            None => return Ok(ToolOutput::error("Missing required parameter: question")),
        };

        let context = params["context"].as_str().unwrap_or("");
        let allow_other = params["allowOther"].as_bool().unwrap_or(false);
        let _multi_select = params["multiSelect"].as_bool().unwrap_or(false);
        let placeholder = params["placeholder"].as_str().unwrap_or("");

        // Build the title: context + question
        let title = if context.is_empty() {
            question.to_string()
        } else {
            format!("{context}\n\n{question}")
        };

        // If options are provided, use select; otherwise use text input
        let raw_options: Option<Vec<OptionItem>> = params
            .get("options")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        match raw_options {
            Some(items) if !items.is_empty() => {
                let mut options: Vec<SelectOption> = items
                    .iter()
                    .map(|item| SelectOption {
                        label: item.label().to_string(),
                        description: match item {
                            OptionItem::Rich { description, .. } => description.clone(),
                            OptionItem::Label(_) => None,
                        },
                    })
                    .collect();

                if allow_other {
                    options.push(SelectOption {
                        label: "Other...".to_string(),
                        description: None,
                    });
                }

                match ctx.ui.select(&title, &options).await {
                    Some(idx) => {
                        // If "Other..." was selected and allow_other is on
                        if allow_other && idx == options.len() - 1 {
                            match ctx.ui.input("Enter your answer:", placeholder).await {
                                Some(text) => Ok(ToolOutput::text(text)),
                                None => Ok(ToolOutput::error("User cancelled input")),
                            }
                        } else {
                            Ok(ToolOutput::text(&options[idx].label))
                        }
                    }
                    None => Ok(ToolOutput::error("User cancelled selection")),
                }
            }
            _ => {
                // Free text input
                match ctx.ui.input(&title, placeholder).await {
                    Some(text) => Ok(ToolOutput::text(text)),
                    None => Ok(ToolOutput::error("User cancelled input")),
                }
            }
        }
    }
}

/// Format options into a display string (for logging/debugging).
pub fn format_options(options: &[SelectOption]) -> String {
    options
        .iter()
        .enumerate()
        .map(|(i, opt)| match &opt.description {
            Some(desc) => format!("  {}. {} — {}", i + 1, opt.label, desc),
            None => format!("  {}. {}", i + 1, opt.label),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use crate::ui::NullInterface;
    use std::sync::Arc;

    fn test_ctx() -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: std::path::PathBuf::from("/tmp"),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
        }
    }

    #[tokio::test]
    async fn ask_null_interface_returns_error() {
        let tool = AskTool;
        let result = tool
            .execute("c1", json!({"question": "What color?"}), test_ctx())
            .await
            .unwrap();

        assert!(result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("Cannot ask user in this mode"));
    }

    #[tokio::test]
    async fn ask_null_interface_with_options_returns_error() {
        let tool = AskTool;
        let result = tool
            .execute(
                "c2",
                json!({
                    "question": "Pick a color",
                    "options": ["red", "blue", "green"]
                }),
                test_ctx(),
            )
            .await
            .unwrap();

        assert!(result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("Cannot ask user in this mode"));
    }

    #[tokio::test]
    async fn ask_missing_question_returns_error() {
        // Use a mock UI that has_ui=true to bypass the first check
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let ctx = ToolContext {
            cwd: std::path::PathBuf::from("/tmp"),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(MockUi),
            file_cache: Arc::new(crate::tools::FileCache::new()),
        };

        let tool = AskTool;
        let result = tool.execute("c3", json!({}), ctx).await.unwrap();

        assert!(result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("Missing required parameter: question"));
    }

    #[test]
    fn format_options_plain() {
        let options = vec![
            SelectOption {
                label: "Red".into(),
                description: None,
            },
            SelectOption {
                label: "Blue".into(),
                description: None,
            },
        ];
        let formatted = format_options(&options);
        assert!(formatted.contains("1. Red"));
        assert!(formatted.contains("2. Blue"));
    }

    #[test]
    fn format_options_with_descriptions() {
        let options = vec![
            SelectOption {
                label: "Rust".into(),
                description: Some("Systems language".into()),
            },
            SelectOption {
                label: "Python".into(),
                description: Some("Scripting language".into()),
            },
        ];
        let formatted = format_options(&options);
        assert!(formatted.contains("Rust — Systems language"));
        assert!(formatted.contains("Python — Scripting language"));
    }

    #[test]
    fn option_item_parsing() {
        // String options
        let items: Vec<OptionItem> = serde_json::from_value(json!(["a", "b"])).unwrap();
        assert_eq!(items[0].label(), "a");
        assert_eq!(items[1].label(), "b");

        // Rich options
        let items: Vec<OptionItem> = serde_json::from_value(json!([
            {"label": "Rust", "description": "Fast"},
            {"label": "Go"}
        ]))
        .unwrap();
        assert_eq!(items[0].label(), "Rust");
        assert_eq!(items[1].label(), "Go");
    }

    // Simple mock UI that has_ui returns true but all interactions return None
    struct MockUi;

    #[async_trait]
    impl crate::ui::UserInterface for MockUi {
        fn has_ui(&self) -> bool {
            true
        }
        async fn notify(&self, _: &str, _: crate::ui::NotifyLevel) {}
        async fn confirm(&self, _: &str, _: &str) -> Option<bool> {
            None
        }
        async fn select(&self, _: &str, _: &[SelectOption]) -> Option<usize> {
            None
        }
        async fn input(&self, _: &str, _: &str) -> Option<String> {
            None
        }
        async fn set_status(&self, _: &str, _: Option<&str>) {}
        async fn set_widget(&self, _: &str, _: Option<crate::ui::WidgetContent>) {}
        async fn custom(&self, _: crate::ui::ComponentSpec) -> Option<serde_json::Value> {
            None
        }
    }

    fn extract_text(output: &ToolOutput) -> String {
        output
            .content
            .iter()
            .filter_map(|b| match b {
                imp_llm::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
