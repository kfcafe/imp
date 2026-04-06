use std::time::Instant;

use futures::StreamExt;
use imp_core::builder::AgentBuilder;
use imp_core::config::Config;
use imp_core::tools::{bash::BashTool, edit::EditTool, read::ReadTool, scan::ScanTool, write::WriteTool};
use imp_llm::auth::AuthStore;
use imp_llm::model::ModelRegistry;
use imp_llm::providers::create_provider;
use imp_llm::Model;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "gpt-5.4-mini".to_string());
    let runs: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let auth_path = Config::user_config_dir().join("auth.json");
    let auth_store = AuthStore::load(&auth_path).unwrap_or_else(|_| AuthStore::new(auth_path));
    let registry = ModelRegistry::with_builtins();
    let meta = registry
        .resolve_meta(&model_id, None)
        .ok_or_else(|| format!("Unknown model: {model_id}"))?;
    let provider_name = meta.provider.clone();
    let api_key = auth_store.resolve(&provider_name)?;

    println!("tool-surface live benchmark");
    println!("model={model_id} provider={provider_name} runs={runs}");
    println!("{:>8} {:>8} {:>12}", "variant", "run", "ms");
    println!("{}", "-".repeat(34));

    for run in 0..runs {
        for variant in ["legacy", "reduced"] {
            let provider = create_provider(&provider_name)
                .ok_or_else(|| format!("Unknown provider: {provider_name}"))?;
            let model = Model {
                meta: meta.clone(),
                provider: provider.into(),
            };

            let cwd = tempfile::tempdir()?;
            std::fs::write(
                cwd.path().join("sample.rs"),
                "pub fn answer() -> i32 {\n    42\n}\n",
            )?;

            let config = Config::default();
            let mut builder = AgentBuilder::new(config, cwd.path().to_path_buf(), model, api_key.clone());
            if variant == "legacy" {
                builder = builder.extra_tools(|tools| {
                    tools.register(std::sync::Arc::new(WriteTool));
                    tools.register(std::sync::Arc::new(ReadTool));
                    tools.register(std::sync::Arc::new(EditTool));
                    tools.register(std::sync::Arc::new(ScanTool));
                    tools.register(std::sync::Arc::new(BashTool));
                });
            }
            let (mut agent, _handle) = builder.build()?;

            let started = Instant::now();
            agent
                .run("Find the function answer and tell me what it returns.".to_string())
                .await?;
            let elapsed = started.elapsed().as_millis();
            println!("{:>8} {:>8} {:>12}", variant, run + 1, elapsed);
        }
    }

    Ok(())
}
