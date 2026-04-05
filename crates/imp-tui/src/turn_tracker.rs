//! Per-turn activity tracker that accumulates statistics while an agent turn
//! is in progress. Feeds into progress indicators and post-turn summaries.

use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

/// Tracks tool calls, file accesses, and command runs during a single agent turn.
///
/// Designed to be reset at `AgentStart` and queried at any point during or
/// after the turn to drive progress indicators and summaries.
pub struct TurnTracker {
    pub started_at: Instant,
    pub tool_calls_started: u32,
    pub tool_calls_completed: u32,
    pub tool_errors: u32,
    /// Unique paths that were read (via the `read` tool).
    pub files_read: BTreeSet<String>,
    /// Unique paths that were written or edited (edit / multi_edit / write).
    pub files_written: BTreeSet<String>,
    /// Unique paths that were created by the `write` tool.
    pub files_created: BTreeSet<String>,
    /// Bash commands that were executed (first 80 chars each).
    pub commands_run: Vec<String>,
    /// Number of search-like tool calls (grep / find / probe_search / probe_extract).
    pub searches: u32,

    /// Maps tool_call_id → tool_name so `record_tool_end` can act on the name
    /// even though `ToolExecutionEnd` only carries the id.
    pending: HashMap<String, String>,
}

impl TurnTracker {
    /// Create a fresh tracker with the clock started now.
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            tool_calls_started: 0,
            tool_calls_completed: 0,
            tool_errors: 0,
            files_read: BTreeSet::new(),
            files_written: BTreeSet::new(),
            files_created: BTreeSet::new(),
            commands_run: Vec::new(),
            searches: 0,
            pending: HashMap::new(),
        }
    }

    /// Reset all counters and restart the clock. Called at `AgentStart`.
    pub fn reset(&mut self) {
        self.started_at = Instant::now();
        self.tool_calls_started = 0;
        self.tool_calls_completed = 0;
        self.tool_errors = 0;
        self.files_read.clear();
        self.files_written.clear();
        self.files_created.clear();
        self.commands_run.clear();
        self.searches = 0;
        self.pending.clear();
    }

    /// Wall-clock time since the turn started (or last reset).
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Called at `ToolExecutionStart`. Records the tool call and classifies it.
    ///
    /// The `tool_call_id` is stored so that `record_tool_end` can look up the
    /// name when the result arrives.
    pub fn record_tool_start(&mut self, tool_call_id: &str, name: &str, args: &serde_json::Value) {
        self.tool_calls_started += 1;
        self.pending
            .insert(tool_call_id.to_string(), name.to_string());
        self.classify(name, args);
    }

    /// Called at `ToolExecutionEnd`. Increments completed / error counters.
    pub fn record_tool_end(&mut self, tool_call_id: &str, is_error: bool) {
        self.pending.remove(tool_call_id);
        self.tool_calls_completed += 1;
        if is_error {
            self.tool_errors += 1;
        }
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn classify(&mut self, name: &str, args: &serde_json::Value) {
        match name {
            "read" => {
                if let Some(path) = args["path"].as_str() {
                    self.files_read.insert(path.to_string());
                }
            }
            "edit" | "multi_edit" => {
                if let Some(path) = args["path"].as_str() {
                    self.files_written.insert(path.to_string());
                }
            }
            "write" => {
                if let Some(path) = args["path"].as_str() {
                    self.files_written.insert(path.to_string());
                    self.files_created.insert(path.to_string());
                }
            }
            "bash" => {
                if let Some(cmd) = args["command"].as_str() {
                    let truncated = cmd.chars().take(80).collect::<String>();
                    self.commands_run.push(truncated);
                    if cmd.trim_start().starts_with("grep ")
                        || cmd.trim_start().starts_with("find ")
                        || cmd.trim_start() == "find"
                        || cmd.trim_start().starts_with("ls ")
                        || cmd.trim_start() == "ls"
                    {
                        self.searches += 1;
                    }
                }
            }
            "grep" | "find" | "probe_search" | "probe_extract" => {
                self.searches += 1;
            }
            _ => {
                // Counted via tool_calls_started — no further classification needed.
            }
        }
    }
}

impl Default for TurnTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifies_read_and_write_tools() {
        let mut tracker = TurnTracker::new();

        tracker.record_tool_start("id-1", "read", &json!({"path": "/tmp/foo.txt"}));
        tracker.record_tool_start("id-2", "write", &json!({"path": "/tmp/bar.txt"}));
        tracker.record_tool_start("id-3", "edit", &json!({"path": "/tmp/baz.txt"}));

        assert_eq!(tracker.tool_calls_started, 3);
        assert!(tracker.files_read.contains("/tmp/foo.txt"));
        assert!(tracker.files_written.contains("/tmp/bar.txt"));
        assert!(tracker.files_created.contains("/tmp/bar.txt"));
        assert!(tracker.files_written.contains("/tmp/baz.txt"));
        // edit should NOT go into files_created
        assert!(!tracker.files_created.contains("/tmp/baz.txt"));

        tracker.record_tool_end("id-1", false);
        tracker.record_tool_end("id-2", false);
        tracker.record_tool_end("id-3", true);

        assert_eq!(tracker.tool_calls_completed, 3);
        assert_eq!(tracker.tool_errors, 1);
    }

    #[test]
    fn classifies_bash_and_search_tools() {
        let mut tracker = TurnTracker::new();

        let long_cmd = "a".repeat(120);
        tracker.record_tool_start("id-bash", "bash", &json!({"command": long_cmd}));
        tracker.record_tool_start("id-bash-grep", "bash", &json!({"command": "grep foo ."}));
        tracker.record_tool_start("id-grep", "grep", &json!({"pattern": "foo"}));
        tracker.record_tool_start("id-find", "find", &json!({"pattern": "*.rs"}));
        tracker.record_tool_start(
            "id-probe",
            "probe_search",
            &json!({"query": "error handling"}),
        );
        tracker.record_tool_start("id-probe2", "probe_extract", &json!({"targets": []}));

        assert_eq!(tracker.commands_run.len(), 2);
        // Command should be truncated to 80 chars
        assert_eq!(tracker.commands_run[0].len(), 80);
        // bash grep + grep, find, probe_search, probe_extract = 5 search calls
        assert_eq!(tracker.searches, 5);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut tracker = TurnTracker::new();

        tracker.record_tool_start("id-1", "read", &json!({"path": "/tmp/a.txt"}));
        tracker.record_tool_start("id-2", "bash", &json!({"command": "ls"}));
        tracker.record_tool_end("id-1", false);
        tracker.record_tool_end("id-2", true);

        tracker.reset();

        assert_eq!(tracker.tool_calls_started, 0);
        assert_eq!(tracker.tool_calls_completed, 0);
        assert_eq!(tracker.tool_errors, 0);
        assert!(tracker.files_read.is_empty());
        assert!(tracker.commands_run.is_empty());
        assert_eq!(tracker.searches, 0);
    }

    #[test]
    fn deduplicates_file_paths() {
        let mut tracker = TurnTracker::new();

        for i in 0..5 {
            tracker.record_tool_start(
                &format!("id-{i}"),
                "read",
                &json!({"path": "/tmp/same.txt"}),
            );
        }

        // BTreeSet deduplicates
        assert_eq!(tracker.files_read.len(), 1);
    }

    #[test]
    fn elapsed_increases_over_time() {
        let tracker = TurnTracker::new();
        let d1 = tracker.elapsed();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let d2 = tracker.elapsed();
        assert!(d2 > d1);
    }
}
