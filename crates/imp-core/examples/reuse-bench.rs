use std::time::Instant;

use futures::StreamExt;
use imp_core::config::Config;
use imp_llm::auth::AuthStore;
use imp_llm::model::ModelRegistry;
use imp_llm::provider::{Context, RequestOptions, ThinkingLevel};
use imp_llm::providers::create_provider;
use imp_llm::{Message, Model, StreamEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "gpt-5.4-mini".to_string());
    let runs: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let auth_path = Config::user_config_dir().join("auth.json");
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
    let registry = ModelRegistry::with_builtins();
    let meta = registry
        .resolve_meta(&model_id, None)
        .ok_or_else(|| format!("Unknown model: {model_id}"))?;
    let provider_name = meta.provider.clone();

    println!("same-process repeated prompt benchmark");
    println!("model={model_id} provider={provider_name} runs={runs}");
    println!(
        "{:>4} {:>10} {:>10} {:>10}",
        "run", "1st_event", "1st_text", "msg_end"
    );
    println!("{}", "-".repeat(42));

    let mut first_event_times = Vec::new();
    let mut first_text_times = Vec::new();
    let mut end_times = Vec::new();

    for run in 0..runs {
        // Recreate provider every iteration to mimic top-level prompt handling,
        // while still staying inside one process so the shared reqwest client can persist.
        let provider = create_provider(&provider_name)
            .ok_or_else(|| format!("Unknown provider: {provider_name}"))?;
        let api_key = auth_store.resolve(&provider_name)?;
        let model = Model {
            meta: meta.clone(),
            provider: provider.into(),
        };

        let context = Context {
            messages: vec![Message::user("Reply with exactly the single word: ready")],
        };
        let options = RequestOptions {
            thinking_level: ThinkingLevel::Low,
            max_tokens: None,
            temperature: None,
            system_prompt: "You are imp.".to_string(),
            tools: Vec::new(),
            cache_options: Default::default(),
        };

        let started = Instant::now();
        let mut stream = model.provider.stream(&model, context, options, &api_key);
        let mut first_event_ms = None;
        let mut first_text_ms = None;
        let mut message_end_ms = None;

        while let Some(item) = stream.next().await {
            let now_ms = started.elapsed().as_millis() as u64;
            match item? {
                StreamEvent::MessageStart { .. } => {
                    first_event_ms.get_or_insert(now_ms);
                }
                StreamEvent::TextDelta { .. } => {
                    first_event_ms.get_or_insert(now_ms);
                    first_text_ms.get_or_insert(now_ms);
                }
                StreamEvent::ThinkingDelta { .. } => {
                    first_event_ms.get_or_insert(now_ms);
                }
                StreamEvent::ToolCall { .. } => {
                    first_event_ms.get_or_insert(now_ms);
                }
                StreamEvent::MessageEnd { .. } => {
                    message_end_ms = Some(now_ms);
                    break;
                }
                StreamEvent::Error { error } => {
                    return Err(format!("stream error: {error}").into());
                }
            }
        }

        let first_event_ms = first_event_ms.unwrap_or(0);
        let first_text_ms = first_text_ms.unwrap_or(first_event_ms);
        let message_end_ms = message_end_ms.unwrap_or(first_text_ms);
        first_event_times.push(first_event_ms);
        first_text_times.push(first_text_ms);
        end_times.push(message_end_ms);

        println!(
            "{:>4} {:>10} {:>10} {:>10}",
            run + 1,
            first_event_ms,
            first_text_ms,
            message_end_ms
        );
    }

    fn avg(xs: &[u64]) -> f64 {
        xs.iter().copied().map(|x| x as f64).sum::<f64>() / xs.len() as f64
    }
    fn median(xs: &mut [u64]) -> u64 {
        xs.sort_unstable();
        xs[xs.len() / 2]
    }

    let mut first_event_copy = first_event_times.clone();
    let mut first_text_copy = first_text_times.clone();
    let mut end_copy = end_times.clone();

    println!();
    println!(
        "avg first_event={}ms median={}ms",
        avg(&first_event_times).round(),
        median(&mut first_event_copy)
    );
    println!(
        "avg first_text={}ms median={}ms",
        avg(&first_text_times).round(),
        median(&mut first_text_copy)
    );
    println!(
        "avg msg_end={}ms median={}ms",
        avg(&end_times).round(),
        median(&mut end_copy)
    );

    Ok(())
}
