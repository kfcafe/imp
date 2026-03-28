use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_core::Stream;
use imp_core::builder::register_native_tools;
use imp_core::config::AgentMode;
use imp_core::context::{context_usage, estimate_tokens, mask_observations};
use imp_core::imp_llm::model::{Capabilities, ModelMeta, ModelPricing};
use imp_core::imp_llm::provider::Provider;
use imp_core::imp_llm::{
    AssistantMessage, ContentBlock, Message, Model, RequestOptions, StopReason, StreamEvent,
    ToolResultMessage,
};
use imp_core::session::{sanitize_messages, SessionEntry, SessionManager};
use imp_core::tools::{truncate_tail, FileCache, ToolRegistry};

fn imp_crate_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct BenchResult {
    name: String,
    iterations: usize,
    min: Duration,
    max: Duration,
    avg: Duration,
    unit: String,
}

impl std::fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<42} avg {:>8.2}ms  min {:>8.2}ms  max {:>8.2}ms  ({} iters, {})",
            self.name,
            self.avg.as_secs_f64() * 1000.0,
            self.min.as_secs_f64() * 1000.0,
            self.max.as_secs_f64() * 1000.0,
            self.iterations,
            self.unit,
        )
    }
}

fn bench<F>(name: &str, iterations: usize, mut f: F) -> BenchResult
where
    F: FnMut() -> String,
{
    let _ = f();

    let mut times = Vec::with_capacity(iterations);
    let mut unit = String::new();

    for _ in 0..iterations {
        let start = Instant::now();
        unit = f();
        times.push(start.elapsed());
    }

    let total: Duration = times.iter().copied().sum();
    let min = *times.iter().min().unwrap();
    let max = *times.iter().max().unwrap();
    let avg = total / iterations as u32;

    BenchResult {
        name: name.to_string(),
        iterations,
        min,
        max,
        avg,
        unit,
    }
}

struct NullProvider;

#[async_trait]
impl Provider for NullProvider {
    fn stream(
        &self,
        _model: &Model,
        _context: imp_core::imp_llm::Context,
        _options: RequestOptions,
        _api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = imp_core::imp_llm::Result<StreamEvent>> + Send>> {
        Box::pin(futures::stream::empty())
    }

    async fn resolve_auth(
        &self,
        _auth: &imp_core::imp_llm::auth::AuthStore,
    ) -> imp_core::imp_llm::Result<imp_core::imp_llm::auth::ApiKey> {
        Ok("bench-key".to_string())
    }

    fn id(&self) -> &str {
        "null"
    }

    fn models(&self) -> &[ModelMeta] {
        &[]
    }
}

fn test_model() -> Model {
    Model {
        meta: ModelMeta {
            id: "bench-model".into(),
            provider: "null".into(),
            name: "Bench Model".into(),
            context_window: 200_000,
            max_output_tokens: 4096,
            pricing: ModelPricing::default(),
            capabilities: Capabilities::default(),
        },
        provider: Arc::new(NullProvider),
    }
}

fn make_user(text: &str) -> Message {
    Message::user(text)
}

fn make_assistant_tool_call(call_id: &str, tool_name: &str, args: serde_json::Value) -> Message {
    Message::Assistant(AssistantMessage {
        content: vec![ContentBlock::ToolCall {
            id: call_id.into(),
            name: tool_name.into(),
            arguments: args,
        }],
        usage: None,
        stop_reason: StopReason::ToolUse,
        timestamp: 1000,
    })
}

fn make_tool_result(call_id: &str, tool_name: &str, output: &str) -> Message {
    Message::ToolResult(ToolResultMessage {
        tool_call_id: call_id.into(),
        tool_name: tool_name.into(),
        content: vec![ContentBlock::Text {
            text: output.into(),
        }],
        is_error: false,
        details: serde_json::Value::Null,
        timestamp: 1000,
    })
}

fn synthetic_conversation(turns: usize, output_bytes: usize) -> Vec<Message> {
    let mut messages = Vec::with_capacity(1 + turns * 2);
    messages.push(make_user("benchmark prompt"));

    for i in 0..turns {
        let call_id = format!("call_{i}");
        messages.push(make_assistant_tool_call(
            &call_id,
            "bash",
            serde_json::json!({
                "command": format!("printf turn-{i}"),
                "timeout": 5,
            }),
        ));
        messages.push(make_tool_result(
            &call_id,
            "bash",
            &"x".repeat(output_bytes),
        ));
    }

    messages
}

fn synthetic_out_of_order_messages(pairs: usize) -> Vec<Message> {
    let mut messages = vec![make_user("sanitize this")];

    for i in 0..pairs {
        let call_id = format!("call_{i}");
        let assistant = make_assistant_tool_call(
            &call_id,
            "read",
            serde_json::json!({"path": format!("src/file_{i}.rs")}),
        );
        let result = make_tool_result(&call_id, "read", "fn main() {}\n");

        if i % 2 == 0 {
            messages.push(result);
            messages.push(assistant);
        } else {
            messages.push(assistant);
            messages.push(result);
        }
    }

    messages
}

fn make_session_message(id: &str, text: &str) -> SessionEntry {
    SessionEntry::Message {
        id: id.to_string(),
        parent_id: None,
        message: Message::user(text),
    }
}

fn build_session_file(root: &Path, count: usize) -> (std::path::PathBuf, usize) {
    let cwd = root.join("project");
    let session_dir = root.join("sessions");
    let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
    for i in 0..count {
        mgr.append(make_session_message(
            &format!("m{i}"),
            &format!("message {i}"),
        ))
        .unwrap();
    }
    (mgr.path().unwrap().to_path_buf(), count)
}

fn create_session_corpus(root: &Path, sessions: usize, messages_per_session: usize) -> usize {
    let session_dir = root.join("sessions");
    for s in 0..sessions {
        let cwd = root.join(format!("project-{s}"));
        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        for i in 0..messages_per_session {
            mgr.append(make_session_message(
                &format!("{s}-{i}"),
                &format!("session {s} message {i}"),
            ))
            .unwrap();
        }
    }
    sessions
}

fn create_large_log(lines: usize, line_len: usize) -> String {
    let payload = "y".repeat(line_len);
    (0..lines)
        .map(|i| format!("{i:05}: {payload}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn file_cache_fixture(root: &Path) -> std::path::PathBuf {
    let path = root.join("cached.txt");
    std::fs::write(&path, "cache fixture\n".repeat(256)).unwrap();
    path
}

fn main() {
    let iters = 5;
    let model = test_model();

    println!("=== imp-core Hot Path Benchmark ===");
    println!("crate dir: {}", imp_crate_dir().display());
    println!("iterations: {iters}");
    println!(
        "estimate_tokens('benchmark'): {}",
        estimate_tokens("benchmark")
    );
    println!();

    let conversation = synthetic_conversation(80, 4096);
    let masked_conversation = {
        let mut m = conversation.clone();
        mask_observations(&mut m, 10);
        m
    };
    let sanitize_fixture = synthetic_out_of_order_messages(200);
    let large_log = create_large_log(5_000, 96);

    println!("── Context ───────────────────────────────────────────");
    let usage_unmasked = bench("context_usage (unmasked)", iters, || {
        let usage = context_usage(&conversation, &model);
        format!("~{} tokens", usage.used)
    });
    println!("{usage_unmasked}");

    let usage_masked = bench("context_usage (masked)", iters, || {
        let usage = context_usage(&masked_conversation, &model);
        format!("~{} tokens", usage.used)
    });
    println!("{usage_masked}");

    let mask_turns = bench("mask_observations", iters, || {
        let mut messages = conversation.clone();
        mask_observations(&mut messages, 10);
        let placeholders = messages
            .iter()
            .filter_map(|m| match m {
                Message::ToolResult(tr) => Some(tr),
                _ => None,
            })
            .filter(|tr| match &tr.content[0] {
                ContentBlock::Text { text } => text.starts_with("[Output omitted"),
                _ => false,
            })
            .count();
        format!("{placeholders} placeholders")
    });
    println!("{mask_turns}");

    println!("\n── Session I/O ───────────────────────────────────────");
    let session_append = bench("session append (disk)", iters, || {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path().join("project");
        let session_dir = tmp.path().join("sessions");
        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
        for i in 0..500 {
            mgr.append(make_session_message(
                &format!("m{i}"),
                &format!("message {i}"),
            ))
            .unwrap();
        }
        format!("{} messages", mgr.get_messages().len())
    });
    println!("{session_append}");

    let session_open = bench("session open + branch walk", iters, || {
        let tmp = tempfile::tempdir().unwrap();
        let (path, expected) = build_session_file(tmp.path(), 800);
        let mgr = SessionManager::open(&path).unwrap();
        format!("{} messages", mgr.get_messages().len().max(expected))
    });
    println!("{session_open}");

    let session_list = bench("session list", iters, || {
        let tmp = tempfile::tempdir().unwrap();
        let session_count = create_session_corpus(tmp.path(), 32, 40);
        let listed = SessionManager::list(&tmp.path().join("sessions"))
            .unwrap()
            .len();
        format!("{listed}/{session_count} sessions")
    });
    println!("{session_list}");

    let sanitize = bench("sanitize_messages", iters, || {
        let mut messages = sanitize_fixture.clone();
        sanitize_messages(&mut messages);
        format!("{} messages", messages.len())
    });
    println!("{sanitize}");

    println!("\n── Tools / helpers ───────────────────────────────────");
    let tool_defs = bench("tool registry definitions", iters, || {
        let mut registry = ToolRegistry::new();
        register_native_tools(&mut registry);
        let defs = registry.definitions();
        let worker_defs = registry.definitions_for_mode(&AgentMode::Worker);
        format!("{} defs, {} worker", defs.len(), worker_defs.len())
    });
    println!("{tool_defs}");

    let truncate = bench("truncate_tail", iters, || {
        let result = truncate_tail(&large_log, 2000, 50 * 1024);
        format!("{} of {} lines", result.output_lines, result.total_lines)
    });
    println!("{truncate}");

    let file_cache_read = bench("file cache hot read", iters, || {
        let tmp = tempfile::tempdir().unwrap();
        let path = file_cache_fixture(tmp.path());
        let cache = FileCache::new();
        let _ = cache.read(&path).unwrap();
        let content = cache.read(&path).unwrap();
        format!("{} bytes", content.len())
    });
    println!("{file_cache_read}");

    println!("\n=== Done ===");
}
