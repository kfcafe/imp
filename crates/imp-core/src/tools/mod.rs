pub mod ask;
pub mod bash;
pub mod diff;
pub mod edit;
pub mod find;
pub mod grep;
pub mod ls;
pub mod lua;
pub mod multi_edit;
pub mod read;
pub mod shell;
pub mod tree_sitter;
pub mod write;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use imp_llm::provider::ToolDefinition;
use imp_llm::{ContentBlock, ToolResultMessage};

use crate::error::Result;
use crate::ui::UserInterface;

/// A tool that can be invoked by the agent.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used in LLM tool calls).
    fn name(&self) -> &str;

    /// Human-readable label.
    fn label(&self) -> &str;

    /// Description shown to the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for parameters.
    fn parameters(&self) -> serde_json::Value;

    /// Whether this tool only reads (no side effects).
    fn is_readonly(&self) -> bool;

    /// Execute the tool.
    async fn execute(
        &self,
        call_id: &str,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolOutput>;
}

/// Context provided to tools during execution.
pub struct ToolContext {
    pub cwd: PathBuf,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub update_tx: tokio::sync::mpsc::Sender<ToolUpdate>,
    pub ui: Arc<dyn UserInterface>,
}

impl ToolContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Result of a tool execution.
pub struct ToolOutput {
    pub content: Vec<ContentBlock>,
    pub details: serde_json::Value,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text { text: text.into() }],
            details: serde_json::Value::Null,
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text { text: text.into() }],
            details: serde_json::Value::Null,
            is_error: true,
        }
    }

    pub fn into_tool_result(self, call_id: &str, tool_name: &str) -> ToolResultMessage {
        ToolResultMessage {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            content: self.content,
            is_error: self.is_error,
            details: self.details,
            timestamp: imp_llm::now(),
        }
    }
}

/// Partial update from a running tool (for streaming output).
pub struct ToolUpdate {
    pub content: Vec<ContentBlock>,
    pub details: serde_json::Value,
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a native Rust tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Get all tool definitions (for LLM context).
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Get only readonly tool definitions (for readonly roles).
    pub fn readonly_definitions(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .filter(|t| t.is_readonly())
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// List all tool names.
    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Truncation helpers ──────────────────────────────────────────────

pub struct TruncationResult {
    pub content: String,
    pub truncated: bool,
    pub output_lines: usize,
    pub total_lines: usize,
    pub output_bytes: usize,
    pub total_bytes: usize,
    pub temp_file: Option<PathBuf>,
}

/// Truncate a single line to max_bytes, appending "…" if truncated.
pub fn truncate_line(line: &str, max_bytes: usize) -> String {
    if line.len() <= max_bytes {
        return line.to_string();
    }
    let mut end = max_bytes.min(line.len());
    while end > 0 && !line.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &line[..end])
}

/// Write full content to a temp file, returning the path.
fn write_temp_file(content: &str) -> Option<PathBuf> {
    let dir = std::env::temp_dir().join("imp-tools");
    std::fs::create_dir_all(&dir).ok()?;
    let name = format!("truncated-{}.txt", uuid::Uuid::new_v4());
    let path = dir.join(name);
    std::fs::write(&path, content).ok()?;
    Some(path)
}

/// Truncate keeping the head (first N lines/bytes).
/// When truncated, writes full output to a temp file.
pub fn truncate_head(input: &str, max_lines: usize, max_bytes: usize) -> TruncationResult {
    let lines: Vec<&str> = input.lines().collect();
    let total_lines = lines.len();
    let total_bytes = input.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: input.to_string(),
            truncated: false,
            output_lines: total_lines,
            total_lines,
            output_bytes: total_bytes,
            total_bytes,
            temp_file: None,
        };
    }

    let mut result = String::new();
    let mut byte_count = 0;
    let mut line_count = 0;

    for line in &lines {
        let line_with_newline = format!("{line}\n");
        if line_count >= max_lines || byte_count + line_with_newline.len() > max_bytes {
            break;
        }
        result.push_str(&line_with_newline);
        byte_count += line_with_newline.len();
        line_count += 1;
    }

    let temp_file = write_temp_file(input);

    TruncationResult {
        content: result,
        truncated: true,
        output_lines: line_count,
        total_lines,
        output_bytes: byte_count,
        total_bytes,
        temp_file,
    }
}

/// Truncate keeping the tail (last N lines/bytes).
/// When truncated, writes full output to a temp file.
pub fn truncate_tail(input: &str, max_lines: usize, max_bytes: usize) -> TruncationResult {
    let lines: Vec<&str> = input.lines().collect();
    let total_lines = lines.len();
    let total_bytes = input.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: input.to_string(),
            truncated: false,
            output_lines: total_lines,
            total_lines,
            output_bytes: total_bytes,
            total_bytes,
            temp_file: None,
        };
    }

    // Walk backwards from the end, collecting lines that fit.
    let start = total_lines.saturating_sub(max_lines);
    let mut actual_start = start;
    let mut remaining_bytes = max_bytes;

    for (i, line) in lines[start..].iter().enumerate() {
        let line_with_newline = format!("{line}\n");
        if line_with_newline.len() > remaining_bytes {
            actual_start = start + i + 1;
            remaining_bytes = max_bytes;
            // Recalculate from new start
            for line2 in &lines[actual_start..] {
                let l = format!("{line2}\n");
                if l.len() > remaining_bytes {
                    break;
                }
                remaining_bytes -= l.len();
            }
            break;
        }
        remaining_bytes -= line_with_newline.len();
    }

    let mut result = String::new();
    for line in &lines[actual_start..] {
        result.push_str(&format!("{line}\n"));
    }

    let output_lines = total_lines - actual_start;
    let output_bytes = result.len();
    let temp_file = write_temp_file(input);

    TruncationResult {
        content: result,
        truncated: true,
        output_lines,
        total_lines,
        output_bytes,
        total_bytes,
        temp_file,
    }
}

// ── Fuzzy matching for edit tools ───────────────────────────────────

pub(crate) mod fuzzy {
    /// Normalize text for fuzzy matching: strip trailing whitespace per line,
    /// convert smart quotes and unicode dashes to ASCII equivalents.
    pub fn normalize_for_matching(text: &str) -> String {
        text.lines()
            .map(|line| normalize_unicode(line.trim_end()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn normalize_unicode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '\u{2018}' | '\u{2019}' => '\'',
                '\u{201C}' | '\u{201D}' => '"',
                '\u{2013}' | '\u{2014}' => '-',
                '\u{00A0}' | '\u{2003}' | '\u{2002}' | '\u{2009}' => ' ',
                other => other,
            })
            .collect()
    }

    /// Result of a fuzzy find: byte range in original content.
    pub struct FuzzyMatch {
        pub start: usize,
        pub end: usize,
    }

    /// Try to find old_text in content using fuzzy matching.
    /// Works line-by-line: normalizes both sides and does sliding-window
    /// matching over lines, then returns the byte range in the original.
    pub fn fuzzy_find(content: &str, old_text: &str) -> Option<FuzzyMatch> {
        let content_lines: Vec<&str> = content.lines().collect();
        let search_norm = normalize_for_matching(old_text);
        let search_lines: Vec<&str> = search_norm.lines().collect();

        if search_lines.is_empty() {
            return None;
        }

        let content_norm_lines: Vec<String> = content_lines
            .iter()
            .map(|l| normalize_unicode(l.trim_end()))
            .collect();

        if search_lines.len() > content_norm_lines.len() {
            return None;
        }

        // Sliding window over content lines
        let window_size = search_lines.len();
        for start_line in 0..=(content_norm_lines.len() - window_size) {
            let matches = content_norm_lines[start_line..start_line + window_size]
                .iter()
                .zip(search_lines.iter())
                .all(|(content_line, search_line)| content_line == search_line);

            if matches {
                // Calculate byte offsets in original content
                let byte_start: usize = content_lines[..start_line]
                    .iter()
                    .map(|l| l.len() + 1) // +1 for \n
                    .sum();

                let end_line = start_line + window_size - 1;
                let byte_end: usize = content_lines[..end_line]
                    .iter()
                    .map(|l| l.len() + 1)
                    .sum::<usize>()
                    + content_lines[end_line].len();

                return Some(FuzzyMatch {
                    start: byte_start,
                    end: byte_end,
                });
            }
        }

        None
    }
}

// ── Diff generation ─────────────────────────────────────────────────

/// Generate a unified diff between old and new content.
pub fn generate_diff(file_path: &str, old: &str, new: &str) -> String {
    use similar::TextDiff;

    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();
    output.push_str(&format!("--- {file_path}\n"));
    output.push_str(&format!("+++ {file_path}\n"));

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{hunk}"));
    }

    output
}
