use imp_core::sdk::{AgentEvent, ImpSession, Result, SessionOptions};

#[tokio::main]
async fn main() -> Result<()> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Summarize the current project.".to_string());

    let mut session = ImpSession::create(SessionOptions {
        cwd: std::env::current_dir()?,
        ..Default::default()
    })
    .await?;

    session.prompt(&prompt).await?;

    while let Some(event) = session.recv_event().await {
        match event {
            AgentEvent::MessageDelta { delta } => {
                if let imp_core::imp_llm::StreamEvent::TextDelta { text } = delta {
                    print!("{text}");
                }
            }
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                eprintln!("\n[tool:{tool_name}]");
            }
            AgentEvent::AgentEnd { .. } => break,
            AgentEvent::Error { error } => eprintln!("\n[error] {error}"),
            _ => {}
        }
    }

    session.wait().await
}
