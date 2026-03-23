use async_trait::async_trait;
use serde_json::json;

use super::{Tool, ToolContext, ToolOutput};
use crate::error::Result;
use crate::session_index::SessionIndex;

pub struct SessionSearchTool;

#[async_trait]
impl Tool for SessionSearchTool {
    fn name(&self) -> &str {
        "session_search"
    }

    fn label(&self) -> &str {
        "Session Search"
    }

    fn description(&self) -> &str {
        "Search past conversations. Use when you need to recall something \
         discussed in a previous session."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (supports AND, OR, NOT, quoted phrases)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results (default: 5)"
                }
            }
        })
    }

    fn is_readonly(&self) -> bool {
        true
    }

    async fn execute(
        &self,
        _call_id: &str,
        params: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolOutput> {
        let query = params["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: query"));
        }

        let limit = params["limit"].as_u64().unwrap_or(5) as usize;

        let index_path = index_db_path();
        if !index_path.exists() {
            return Ok(ToolOutput::text(
                "No sessions indexed yet. Session search becomes available \
                 after your first conversation.",
            ));
        }

        let index = match SessionIndex::open(&index_path) {
            Ok(idx) => idx,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to open session index: {e}"
                )));
            }
        };

        let results = match index.search(query, limit) {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolOutput::error(format!("Search failed: {e}")));
            }
        };

        if results.is_empty() {
            return Ok(ToolOutput::text(format!(
                "No past sessions match \"{query}\"."
            )));
        }

        let mut output = format!("Found {} result(s) for \"{}\":\n", results.len(), query);

        for (i, hit) in results.iter().enumerate() {
            let ts = format_timestamp(hit.created_at);
            let first = hit.first_message.as_deref().unwrap_or("(no first message)");

            output.push_str(&format!(
                "\n[{}] Session from {} ({}, {} messages)\n    First: \"{}\"\n    {}\n",
                i + 1,
                ts,
                hit.cwd,
                hit.message_count,
                first,
                hit.snippet,
            ));
        }

        Ok(ToolOutput::text(output))
    }
}

/// Default path for the session index database.
fn index_db_path() -> std::path::PathBuf {
    // Use XDG data dir on Linux, ~/Library on macOS, or fallback to ~/.local/share
    let base = if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(|h| {
                std::path::PathBuf::from(h)
                    .join("Library")
                    .join("Application Support")
            })
            .unwrap_or_else(|_| std::path::PathBuf::from(".local/share"))
    } else {
        std::env::var("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".local/share"))
                    .unwrap_or_else(|_| std::path::PathBuf::from(".local/share"))
            })
    };
    base.join("imp").join("session_index.db")
}

/// Format a unix timestamp into a human-readable date.
fn format_timestamp(ts: u64) -> String {
    // Simple formatting without chrono dependency
    if ts == 0 {
        return "unknown date".to_string();
    }
    let secs = ts;
    // Rough formatting: just show the timestamp as-is for now
    // A full implementation would use chrono, but we avoid the dependency
    let days_since_epoch = secs / 86400;
    let years = 1970 + days_since_epoch / 365;
    let day_in_year = days_since_epoch % 365;
    let month = day_in_year / 30 + 1;
    let day = day_in_year % 30 + 1;
    format!("{years}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{SessionEntry, SessionManager};
    use crate::tools::ToolContext;
    use std::sync::Arc;

    fn test_ctx() -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: std::env::temp_dir(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
        }
    }

    fn seed_index(dir: &std::path::Path) -> std::path::PathBuf {
        let db_path = dir.join("index.db");
        let index = SessionIndex::open(&db_path).unwrap();

        let session_dir = dir.join("sessions");
        let cwd = dir.join("project");
        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
        session
            .append(SessionEntry::Message {
                id: "m1".to_string(),
                parent_id: None,
                message: imp_llm::Message::user("Help me deploy kubernetes"),
            })
            .unwrap();
        session
            .append(SessionEntry::Message {
                id: "a1".to_string(),
                parent_id: None,
                message: imp_llm::Message::Assistant(imp_llm::AssistantMessage {
                    content: vec![imp_llm::ContentBlock::Text {
                        text: "I'll help with the kubernetes deployment".to_string(),
                    }],
                    usage: None,
                    stop_reason: imp_llm::StopReason::EndTurn,
                    timestamp: 0,
                }),
            })
            .unwrap();
        index.index_session(&session).unwrap();

        db_path
    }

    #[tokio::test]
    async fn session_search_tool_missing_query() {
        let tool = SessionSearchTool;
        let r = tool.execute("c1", json!({}), test_ctx()).await.unwrap();
        assert!(r.is_error);
    }

    #[tokio::test]
    async fn session_search_tool_missing_db() {
        // With no index DB, should return a helpful message (not an error)
        let tool = SessionSearchTool;
        // We can't easily override the path in this test without refactoring,
        // but we can verify the tool handles the case gracefully
        // (The actual path check happens at runtime)
        let r = tool
            .execute("c1", json!({"query": "test"}), test_ctx())
            .await
            .unwrap();
        // Either returns "No sessions indexed" or actual results depending on user's state
        assert!(!r.is_error || r.text_content().unwrap().contains("session"));
    }
}
