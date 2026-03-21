use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use super::{truncate_head, Tool, ToolContext, ToolOutput};
use crate::error::Result;

const MAX_LINES: usize = 2000;
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
        "Read the contents of a file. Supports text files and images."
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to read" },
                "offset": { "type": "number", "description": "Line number to start reading from (1-indexed)" },
                "limit": { "type": "number", "description": "Maximum number of lines to read" }
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

        let path = resolve_path(&ctx.cwd, raw_path);

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
        let result = truncate_head(&sliced, MAX_LINES, MAX_BYTES);

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

fn resolve_path(cwd: &Path, raw: &str) -> std::path::PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
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

/// Find files with similar names to suggest when a file isn't found.
fn suggest_similar_files(cwd: &Path, target: &str) -> Vec<String> {
    let target_name = Path::new(target)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(target);

    let mut candidates: Vec<(usize, String)> = Vec::new();

    let walker = walkdir::WalkDir::new(cwd)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                let dist = levenshtein(target_name, name);
                if dist <= 3 {
                    let rel = entry
                        .path()
                        .strip_prefix(cwd)
                        .unwrap_or(entry.path())
                        .display()
                        .to_string();
                    candidates.push((dist, rel));
                }
            }
        }
    }

    candidates.sort_by_key(|(d, _)| *d);
    candidates.truncate(3);
    candidates.into_iter().map(|(_, p)| p).collect()
}

/// Simple Levenshtein distance.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
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
    async fn read_directory_error() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let tool = ReadTool;
        let result = tool
            .execute("c8", json!({"path": "subdir"}), test_ctx(dir.path()))
            .await;

        // Reading a directory should either error or produce an error output
        match result {
            Ok(output) => assert!(output.is_error),
            Err(_) => {} // Also acceptable
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
