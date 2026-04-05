use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{suggest_similar_files, truncate_head, Tool, ToolContext, ToolOutput};
use crate::error::Result;

const MAX_BYTES: usize = 50_000;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }
    fn label(&self) -> &str {
        "Read File"
    }
    fn description(&self) -> &str {
        "Read a specific file with stable, line-oriented output. Supports offsets and limits for focused inspection, and supports images."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to read" },
                "offset": { "type": "number", "description": "Optional 1-indexed line number to start reading from" },
                "limit": { "type": "number", "description": "Optional maximum number of lines to read" }
            },
            "required": ["path"]
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
        let raw_path = params["path"]
            .as_str()
            .unwrap_or("")
            .trim_start_matches('@');

        if raw_path.is_empty() {
            return Ok(ToolOutput::error("Missing required parameter: path"));
        }

        let path = super::resolve_path(&ctx.cwd, raw_path);

        if !path.exists() {
            let suggestions = suggest_similar_files(&ctx.cwd, raw_path);
            let mut msg = format!("File not found: {}", path.display());
            if !suggestions.is_empty() {
                msg.push_str("\n\nDid you mean:");
                for s in &suggestions {
                    msg.push_str(&format!("\n  {s}"));
                }
            }
            return Ok(ToolOutput::error(msg));
        }

        // Check for image files
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                return read_image(&path).await;
            }
        }

        // Read raw bytes and check for binary
        let bytes = tokio::fs::read(&path).await?;
        let check_len = bytes.len().min(8192);
        if bytes[..check_len].contains(&0) {
            return Ok(ToolOutput::error(format!(
                "Binary file detected: {}. Cannot display binary content.",
                path.display()
            )));
        }

        let content = String::from_utf8_lossy(&bytes).into_owned();

        // Apply offset/limit
        let offset = params["offset"].as_u64().map(|v| v as usize);
        let limit = params["limit"].as_u64().map(|v| v as usize);

        let sliced = apply_offset_limit(&content, offset, limit);

        // Apply truncation
        let max_lines = ctx.read_max_lines;
        let result = if max_lines == 0 {
            super::TruncationResult {
                content: sliced.clone(),
                truncated: false,
                output_lines: sliced.lines().count(),
                total_lines: sliced.lines().count(),
                output_bytes: sliced.len(),
                total_bytes: sliced.len(),
                temp_file: None,
            }
        } else {
            truncate_head(&sliced, max_lines, MAX_BYTES)
        };

        let mut output = result.content.clone();
        if result.truncated {
            let note = format!(
                "\n[…truncated: showing {}/{} lines, {}/{} bytes",
                result.output_lines, result.total_lines, result.output_bytes, result.total_bytes,
            );
            if let Some(ref tf) = result.temp_file {
                output.push_str(&format!("{note}, full output: {}]", tf.display()));
            } else {
                output.push_str(&format!("{note}]"));
            }
        }

        // Record that this file was read (for staleness and unread-edit detection).
        if let Ok(mut tracker) = ctx.file_tracker.lock() {
            tracker.record_read(&path);
        }

        Ok(ToolOutput {
            content: vec![imp_llm::ContentBlock::Text { text: output }],
            details: json!({
                "path": path.display().to_string(),
                "truncated": result.truncated,
                "lines": result.output_lines,
                "total_lines": result.total_lines,
            }),
            is_error: false,
        })
    }
}

fn apply_offset_limit(content: &str, offset: Option<usize>, limit: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = offset.map(|o| o.saturating_sub(1)).unwrap_or(0); // 1-indexed to 0-indexed
    let end = match limit {
        Some(l) => (start + l).min(lines.len()),
        None => lines.len(),
    };

    if start >= lines.len() {
        return String::new();
    }

    lines[start..end].join("\n")
}

async fn read_image(path: &Path) -> Result<ToolOutput> {
    let bytes = tokio::fs::read(path).await?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let media_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };

    use std::io::Write;
    let mut encoded = Vec::new();
    {
        let mut encoder = base64_encoder(&mut encoded);
        encoder.write_all(&bytes)?;
        encoder.finish()?;
    }
    let data = String::from_utf8(encoded).unwrap_or_default();

    Ok(ToolOutput {
        content: vec![imp_llm::ContentBlock::Image {
            media_type: media_type.to_string(),
            data,
        }],
        details: json!({
            "path": path.display().to_string(),
            "media_type": media_type,
            "bytes": bytes.len(),
        }),
        is_error: false,
    })
}

/// Simple base64 encoder without adding a dependency. We only need this for images.
fn base64_encoder(output: &mut Vec<u8>) -> Base64Writer<'_> {
    Base64Writer {
        output,
        buffer: [0; 3],
        buffer_len: 0,
    }
}

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

struct Base64Writer<'a> {
    output: &'a mut Vec<u8>,
    buffer: [u8; 3],
    buffer_len: usize,
}

impl<'a> std::io::Write for Base64Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &byte in buf {
            self.buffer[self.buffer_len] = byte;
            self.buffer_len += 1;
            if self.buffer_len == 3 {
                self.encode_block();
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> Base64Writer<'a> {
    fn encode_block(&mut self) {
        let b = &self.buffer;
        self.output.push(BASE64_CHARS[(b[0] >> 2) as usize]);
        self.output
            .push(BASE64_CHARS[((b[0] & 0x03) << 4 | b[1] >> 4) as usize]);
        self.output
            .push(BASE64_CHARS[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize]);
        self.output.push(BASE64_CHARS[(b[2] & 0x3f) as usize]);
        self.buffer_len = 0;
    }

    fn finish(self) -> std::io::Result<()> {
        match self.buffer_len {
            1 => {
                let b = self.buffer[0];
                self.output.push(BASE64_CHARS[(b >> 2) as usize]);
                self.output.push(BASE64_CHARS[((b & 0x03) << 4) as usize]);
                self.output.push(b'=');
                self.output.push(b'=');
            }
            2 => {
                let b0 = self.buffer[0];
                let b1 = self.buffer[1];
                self.output.push(BASE64_CHARS[(b0 >> 2) as usize]);
                self.output
                    .push(BASE64_CHARS[((b0 & 0x03) << 4 | b1 >> 4) as usize]);
                self.output.push(BASE64_CHARS[((b1 & 0x0f) << 2) as usize]);
                self.output.push(b'=');
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolContext;
    use std::sync::Arc;

    fn test_ctx(dir: &Path) -> ToolContext {
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        ToolContext {
            cwd: dir.to_path_buf(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            update_tx: tx,
            ui: Arc::new(crate::ui::NullInterface),
            file_cache: Arc::new(crate::tools::FileCache::new()),
            file_tracker: Arc::new(std::sync::Mutex::new(crate::tools::FileTracker::new())),
            mode: crate::config::AgentMode::Full,
            read_max_lines: 500,
        }
    }

    #[tokio::test]
    async fn read_known_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "line1\nline2\nline3\n").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c1", json!({"path": "hello.txt"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("line1"));
        assert!(text.contains("line3"));
    }

    #[tokio::test]
    async fn read_offset_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("data.txt");
        std::fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute(
                "c2",
                json!({"path": "data.txt", "offset": 2, "limit": 2}),
                test_ctx(dir.path()),
            )
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("b"));
        assert!(text.contains("c"));
        assert!(!text.contains("a"));
        assert!(!text.contains("d"));
    }

    #[tokio::test]
    async fn read_file_not_found_suggestions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hi").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c3", json!({"path": "helo.txt"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("File not found"));
        assert!(text.contains("hello.txt"));
    }

    #[tokio::test]
    async fn read_binary_file_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("data.bin");
        std::fs::write(&file, b"\x00\x01\x02\x03").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c4", json!({"path": "data.bin"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(result.is_error);
        assert!(extract_text(&result).contains("Binary file"));
    }

    #[tokio::test]
    async fn read_strips_at_prefix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.txt"), "content").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c5", json!({"path": "@test.txt"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert!(extract_text(&result).contains("content"));
    }

    #[tokio::test]
    async fn read_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        std::fs::write(&file, "").unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c6", json!({"path": "empty.txt"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn read_large_file_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("big.txt");
        let mut content = String::new();
        for i in 0..3000 {
            content.push_str(&format!("line {i}\n"));
        }
        std::fs::write(&file, &content).unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c7", json!({"path": "big.txt"}), test_ctx(dir.path()))
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("truncated"));
        // Should have the first lines
        assert!(text.contains("line 0"));
        // Details should indicate truncation
        assert_eq!(result.details["truncated"], true);
    }

    #[tokio::test]
    async fn read_respects_configured_line_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("limited.txt");
        let mut content = String::new();
        for i in 0..800 {
            content.push_str(&format!("line {i}\n"));
        }
        std::fs::write(&file, &content).unwrap();

        let tool = ReadTool;
        let mut ctx = test_ctx(dir.path());
        ctx.read_max_lines = 500;
        let result = tool
            .execute("c7b", json!({"path": "limited.txt"}), ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(text.contains("truncated"));
        assert!(text.contains("showing 500/800 lines"));
        assert_eq!(result.details["lines"], 500);
        assert_eq!(result.details["total_lines"], 800);
    }

    #[tokio::test]
    async fn read_zero_line_limit_disables_line_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("unlimited.txt");
        let mut content = String::new();
        for i in 0..800 {
            content.push_str(&format!("line {i}\n"));
        }
        std::fs::write(&file, &content).unwrap();

        let tool = ReadTool;
        let mut ctx = test_ctx(dir.path());
        ctx.read_max_lines = 0;
        let result = tool
            .execute("c7c", json!({"path": "unlimited.txt"}), ctx)
            .await
            .unwrap();

        assert!(!result.is_error);
        let text = extract_text(&result);
        assert!(!text.contains("truncated"));
        assert!(text.contains("line 799"));
        assert_eq!(result.details["truncated"], false);
        assert_eq!(result.details["lines"], 800);
        assert_eq!(result.details["total_lines"], 800);
        assert!(result.details["path"]
            .as_str()
            .unwrap()
            .contains("unlimited.txt"));
    }

    #[tokio::test]
    async fn read_directory_error() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c8", json!({"path": "subdir"}), test_ctx(dir.path()))
            .await;

        // Reading a directory should either error or produce an error output
        if let Ok(output) = result {
            assert!(output.is_error)
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
