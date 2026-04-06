use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use futures_core::Stream;
use imp_core::agent::{Agent, AgentEvent, AgentHandle};
use imp_core::config::Config;
use imp_core::builder::AgentBuilder;
use imp_core::tools::{
    bash::BashTool, diff::{DiffApplyTool, DiffShowTool, DiffTool}, edit::EditTool,
    find::FindTool, grep::GrepTool, ls::LsTool, read::ReadTool, scan::ScanTool, write::WriteTool,
};
use imp_llm::auth::{ApiKey, AuthStore};
use imp_llm::model::{Capabilities, ModelMeta, ModelPricing, ModelRegistry};
use imp_llm::provider::Provider;
use imp_llm::providers::create_provider;
use imp_llm::{AssistantMessage, ContentBlock, Context, Model, RequestOptions, StopReason, StreamEvent, Usage};
use serde::Serialize;
use serde_json::json;
use tokio::sync::Mutex;

struct MockProvider {
    responses: Mutex<Vec<Vec<StreamEvent>>>,
}

impl MockProvider {
    fn new(responses: Vec<Vec<StreamEvent>>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn stream(
        &self,
        _model: &Model,
        _context: Context,
        _options: RequestOptions,
        _api_key: &str,
    ) -> Pin<Box<dyn Stream<Item = imp_llm::Result<StreamEvent>> + Send>> {
        let mut responses = self.responses.try_lock().expect("MockProvider lock");
        let events = if responses.is_empty() {
            vec![StreamEvent::Error {
                error: "No more mock responses".to_string(),
            }]
        } else {
            responses.remove(0)
        };
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }

    async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
        Ok("mock-key".to_string())
    }

    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelMeta] {
        &[]
    }
}

fn test_model(provider: Arc<dyn Provider>) -> Model {
    Model {
        meta: ModelMeta {
            id: "test-model".to_string(),
            provider: "mock".to_string(),
            name: "Test Model".to_string(),
            context_window: 200_000,
            max_output_tokens: 16_384,
            pricing: ModelPricing {
                input_per_mtok: 3.0,
                output_per_mtok: 15.0,
                cache_read_per_mtok: 0.3,
                cache_write_per_mtok: 3.75,
            },
            capabilities: Capabilities {
                reasoning: true,
                images: false,
                tool_use: true,
            },
        },
        provider,
    }
}

fn text_response(text: &str, input_tokens: u32, output_tokens: u32) -> Vec<StreamEvent> {
    vec![
        StreamEvent::MessageStart {
            model: "test-model".to_string(),
        },
        StreamEvent::TextDelta {
            text: text.to_string(),
        },
        StreamEvent::MessageEnd {
            message: AssistantMessage {
                content: vec![ContentBlock::Text {
                    text: text.to_string(),
                }],
                usage: Some(Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                }),
                stop_reason: StopReason::EndTurn,
                timestamp: 1000,
            },
        },
    ]
}

fn tool_call_response(
    call_id: &str,
    tool_name: &str,
    args: serde_json::Value,
    input_tokens: u32,
    output_tokens: u32,
) -> Vec<StreamEvent> {
    vec![
        StreamEvent::MessageStart {
            model: "test-model".to_string(),
        },
        StreamEvent::ToolCall {
            id: call_id.to_string(),
            name: tool_name.to_string(),
            arguments: args.clone(),
        },
        StreamEvent::MessageEnd {
            message: AssistantMessage {
                content: vec![ContentBlock::ToolCall {
                    id: call_id.to_string(),
                    name: tool_name.to_string(),
                    arguments: args,
                }],
                usage: Some(Usage {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                }),
                stop_reason: StopReason::ToolUse,
                timestamp: 1000,
            },
        },
    ]
}

async fn collect_events(mut handle: AgentHandle) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    while let Some(event) = handle.event_rx.recv().await {
        events.push(event);
    }
    events
}

fn create_agent_legacy(provider: Arc<dyn Provider>, cwd: PathBuf) -> (Agent, AgentHandle) {
    let model = test_model(provider);
    let (mut agent, handle) = Agent::new(model, cwd);
    agent.tools.register(Arc::new(WriteTool));
    agent.tools.register(Arc::new(ReadTool));
    agent.tools.register(Arc::new(EditTool));
    agent.tools.register(Arc::new(GrepTool));
    agent.tools.register(Arc::new(FindTool));
    agent.tools.register(Arc::new(LsTool));
    agent.tools.register(Arc::new(DiffTool));
    agent.tools.register(Arc::new(DiffShowTool));
    agent.tools.register(Arc::new(DiffApplyTool));
    agent.tools.register(Arc::new(ScanTool));
    agent.tools.register(Arc::new(BashTool));
    (agent, handle)
}

fn create_agent_reduced(provider: Arc<dyn Provider>, cwd: PathBuf) -> (Agent, AgentHandle) {
    let model = test_model(provider);
    let (mut agent, handle) = Agent::new(model, cwd);
    agent.tools.register(Arc::new(WriteTool));
    agent.tools.register(Arc::new(ReadTool));
    agent.tools.register(Arc::new(EditTool));
    agent.tools.register(Arc::new(ScanTool));
    agent.tools.register(Arc::new(BashTool));
    (agent, handle)
}

#[derive(Serialize)]
struct RunSummary {
    variant: &'static str,
    scenario: &'static str,
    mode: &'static str,
    turns: usize,
    tool_calls: usize,
    tool_names: Vec<String>,
    input_tokens: u32,
    output_tokens: u32,
    wall_ms: u128,
    tool_output_deltas: usize,
    warnings: usize,
    errors: usize,
    final_text: Option<String>,
}

fn summarize_events(
    variant: &'static str,
    scenario: &'static str,
    mode: &'static str,
    events: &[AgentEvent],
    wall_ms: u128,
) -> RunSummary {
    let turns = events.iter().filter(|e| matches!(e, AgentEvent::TurnStart { .. })).count();
    let tool_names: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::ToolExecutionStart { tool_name, .. } => Some(tool_name.clone()),
            _ => None,
        })
        .collect();
    let tool_calls = tool_names.len();
    let tool_output_deltas = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::ToolOutputDelta { .. }))
        .count();

    let (input_tokens, output_tokens) = events
        .iter()
        .find_map(|e| match e {
            AgentEvent::AgentEnd { usage, .. } => Some((usage.input_tokens, usage.output_tokens)),
            _ => None,
        })
        .unwrap_or((0, 0));

    let mut warnings = 0usize;
    let mut errors = 0usize;
    for event in events {
        if let AgentEvent::ToolExecutionEnd { result, .. } = event {
            if result.is_error {
                errors += 1;
            }
            let text = result
                .content
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            if text.contains("Warning:") {
                warnings += 1;
            }
        }
    }

    let final_text = events.iter().rev().find_map(|e| match e {
        AgentEvent::TurnEnd { message, .. } => message.content.iter().find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        }),
        _ => None,
    });

    RunSummary {
        variant,
        scenario,
        mode,
        turns,
        tool_calls,
        tool_names,
        input_tokens,
        output_tokens,
        wall_ms,
        tool_output_deltas,
        warnings,
        errors,
        final_text,
    }
}

async fn run_mock_variant(
    summaries: &mut Vec<RunSummary>,
    variant_name: &'static str,
    scenario_name: &'static str,
    prompt: &str,
    setup: fn(&std::path::Path),
    responses: fn() -> Vec<Vec<StreamEvent>>,
    create_agent: fn(Arc<dyn Provider>, PathBuf) -> (Agent, AgentHandle),
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    setup(dir.path());
    let provider = Arc::new(MockProvider::new(responses()));
    let (mut agent, handle) = create_agent(provider, dir.path().to_path_buf());
    let events_task = tokio::spawn(collect_events(handle));
    let started = Instant::now();
    agent.run(prompt.to_string()).await?;
    drop(agent);
    let events = events_task.await?;
    summaries.push(summarize_events(
        variant_name,
        scenario_name,
        "mock",
        &events,
        started.elapsed().as_millis(),
    ));
    Ok(())
}

async fn run_live_variant(
    summaries: &mut Vec<RunSummary>,
    variant_name: &'static str,
    scenario_name: &'static str,
    prompt: &str,
    setup: fn(&std::path::Path),
) -> Result<(), Box<dyn std::error::Error>> {
    let model_id = std::env::var("IMP_AB_MODEL")?;
    let provider_name = std::env::var("IMP_AB_PROVIDER").ok();

    let registry = ModelRegistry::with_builtins();
    let meta = registry
        .resolve_meta(&model_id, provider_name.as_deref())
        .ok_or_else(|| format!("Model not found: {model_id}"))?;
    let provider = create_provider(&meta.provider)
        .ok_or_else(|| format!("Provider not found: {}", meta.provider))?;

    let auth_path = Config::user_config_dir().join("auth.json");
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
    let api_key = auth_store.resolve(&meta.provider)?;
    let model = Model {
        meta,
        provider: Arc::from(provider),
    };

    let dir = tempfile::tempdir()?;
    setup(dir.path());
    let config = Config::default();
    let mut builder = AgentBuilder::new(config, dir.path().to_path_buf(), model, api_key);

    if variant_name == "legacy" {
        builder = builder.extra_tools(|tools| {
            tools.register(Arc::new(WriteTool));
            tools.register(Arc::new(ReadTool));
            tools.register(Arc::new(EditTool));
            tools.register(Arc::new(GrepTool));
            tools.register(Arc::new(FindTool));
            tools.register(Arc::new(LsTool));
            tools.register(Arc::new(DiffTool));
            tools.register(Arc::new(DiffShowTool));
            tools.register(Arc::new(DiffApplyTool));
            tools.register(Arc::new(ScanTool));
            tools.register(Arc::new(BashTool));
        });
    }

    let (mut agent, handle) = builder.build()?;
    let events_task = tokio::spawn(collect_events(handle));
    let started = Instant::now();
    agent.run(prompt.to_string()).await?;
    drop(agent);
    let events = events_task.await?;
    summaries.push(summarize_events(
        variant_name,
        scenario_name,
        "live",
        &events,
        started.elapsed().as_millis(),
    ));
    Ok(())
}

fn setup_search(path: &std::path::Path) {
    std::fs::write(
        path.join("search_me.txt"),
        "line one\nunique_pattern_xyz here\nline three\n",
    )
    .unwrap();
}

fn responses_search_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "grep", json!({"pattern": "unique_pattern_xyz", "path": "."}), 100, 20),
        text_response("Found it", 100, 20),
    ]
}

fn responses_search_reduced() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "bash", json!({"command": "grep --no-color -rn 'unique_pattern_xyz' ."}), 100, 20),
        text_response("Found it", 100, 20),
    ]
}

fn setup_list(path: &std::path::Path) {
    std::fs::create_dir(path.join("src")).unwrap();
    std::fs::write(path.join("README.md"), "hello").unwrap();
    std::fs::write(path.join("src").join("main.rs"), "fn main() {}\n").unwrap();
}

fn responses_list_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "ls", json!({"path": "."}), 100, 20),
        text_response("Listed files", 100, 20),
    ]
}

fn responses_list_reduced() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "bash", json!({"command": "ls"}), 100, 20),
        text_response("Listed files", 100, 20),
    ]
}

fn setup_find(path: &std::path::Path) {
    std::fs::create_dir(path.join("nested")).unwrap();
    std::fs::write(path.join("nested").join("mod.rs"), "pub fn hi() {}\n").unwrap();
    std::fs::write(path.join("nested").join("data.txt"), "x\n").unwrap();
}

fn responses_find_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "find", json!({"pattern": "*.rs", "path": "."}), 100, 20),
        text_response("Found rust files", 100, 20),
    ]
}

fn responses_find_reduced() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "bash", json!({"command": "find . -name '*.rs'"}), 100, 20),
        text_response("Found rust files", 100, 20),
    ]
}

fn setup_scan_extract(path: &std::path::Path) {
    std::fs::create_dir(path.join("src")).unwrap();
    std::fs::write(
        path.join("src").join("lib.rs"),
        "pub struct User { pub id: u64 }\n\npub fn load_user() -> User { User { id: 1 } }\n",
    )
    .unwrap();
}

fn responses_scan_extract_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "grep", json!({"extract": ["src/lib.rs#load_user"]}), 100, 20),
        text_response("Extracted symbol", 100, 20),
    ]
}

fn responses_scan_extract_reduced() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "scan", json!({"action": "extract", "files": ["src/lib.rs#load_user"]}), 100, 20),
        text_response("Extracted symbol", 100, 20),
    ]
}

fn setup_search_then_read(path: &std::path::Path) {
    std::fs::write(
        path.join("module.rs"),
        "fn helper() {}\n\npub fn important() {\n    helper();\n}\n",
    )
    .unwrap();
}

fn responses_search_then_read_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "grep", json!({"pattern": "important", "path": "."}), 100, 20),
        tool_call_response("call_2", "read", json!({"path": "module.rs"}), 100, 20),
        text_response("Inspected important function", 100, 20),
    ]
}

fn responses_search_then_read_reduced() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "bash", json!({"command": "grep --no-color -rn 'important' ."}), 100, 20),
        tool_call_response("call_2", "read", json!({"path": "module.rs"}), 100, 20),
        text_response("Inspected important function", 100, 20),
    ]
}

fn setup_search_then_edit(path: &std::path::Path) {
    std::fs::write(
        path.join("rename.rs"),
        "pub fn old_name() -> i32 {\n    1\n}\n",
    )
    .unwrap();
}

fn responses_search_then_edit_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "grep", json!({"pattern": "old_name", "path": "."}), 100, 20),
        tool_call_response(
            "call_2",
            "edit",
            json!({"path": "rename.rs", "oldText": "old_name", "newText": "new_name"}),
            100,
            20,
        ),
        text_response("Renamed function", 100, 20),
    ]
}

fn responses_search_then_edit_reduced() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "bash", json!({"command": "grep --no-color -rn 'old_name' ."}), 100, 20),
        tool_call_response(
            "call_2",
            "edit",
            json!({"path": "rename.rs", "oldText": "old_name", "newText": "new_name"}),
            100,
            20,
        ),
        text_response("Renamed function", 100, 20),
    ]
}

fn setup_repeat(path: &std::path::Path) {
    std::fs::write(path.join("repeat.txt"), "same content\n").unwrap();
}

fn responses_repeat_legacy() -> Vec<Vec<StreamEvent>> {
    vec![
        tool_call_response("call_1", "read", json!({"path": "repeat.txt"}), 100, 20),
        tool_call_response("call_2", "read", json!({"path": "repeat.txt"}), 100, 20),
        tool_call_response("call_3", "read", json!({"path": "repeat.txt"}), 100, 20),
        tool_call_response("call_4", "read", json!({"path": "repeat.txt"}), 100, 20),
        text_response("Repeated reads finished", 100, 20),
    ]
}

fn responses_repeat_reduced() -> Vec<Vec<StreamEvent>> {
    responses_repeat_legacy()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let variant = std::env::args().nth(1).unwrap_or_else(|| "both".to_string());
    let mode = std::env::args().nth(2).unwrap_or_else(|| "mock".to_string());

    let scenarios = vec![
        (
            "search",
            "Find the unique pattern in the workspace",
            setup_search as fn(&std::path::Path),
            responses_search_legacy as fn() -> Vec<Vec<StreamEvent>>,
            responses_search_reduced as fn() -> Vec<Vec<StreamEvent>>,
        ),
        (
            "list",
            "List the project root",
            setup_list,
            responses_list_legacy,
            responses_list_reduced,
        ),
        (
            "find",
            "Find all Rust files",
            setup_find,
            responses_find_legacy,
            responses_find_reduced,
        ),
        (
            "scan_extract",
            "Extract the load_user symbol",
            setup_scan_extract,
            responses_scan_extract_legacy,
            responses_scan_extract_reduced,
        ),
        (
            "search_then_read",
            "Search for important and then inspect the file",
            setup_search_then_read,
            responses_search_then_read_legacy,
            responses_search_then_read_reduced,
        ),
        (
            "search_then_edit",
            "Search for old_name and rename it",
            setup_search_then_edit,
            responses_search_then_edit_legacy,
            responses_search_then_edit_reduced,
        ),
        (
            "repeat_read_loop",
            "Read the same file repeatedly",
            setup_repeat,
            responses_repeat_legacy,
            responses_repeat_reduced,
        ),
    ];

    let mut summaries = Vec::new();

    for (name, prompt, setup, legacy_responses, reduced_responses) in scenarios {
        if variant == "both" || variant == "legacy" {
            match mode.as_str() {
                "live" => run_live_variant(&mut summaries, "legacy", name, prompt, setup).await?,
                _ => run_mock_variant(
                    &mut summaries,
                    "legacy",
                    name,
                    prompt,
                    setup,
                    legacy_responses,
                    create_agent_legacy,
                )
                .await?,
            }
        }

        if variant == "both" || variant == "reduced" {
            match mode.as_str() {
                "live" => run_live_variant(&mut summaries, "reduced", name, prompt, setup).await?,
                _ => run_mock_variant(
                    &mut summaries,
                    "reduced",
                    name,
                    prompt,
                    setup,
                    reduced_responses,
                    create_agent_reduced,
                )
                .await?,
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&summaries)?);
    Ok(())
}
