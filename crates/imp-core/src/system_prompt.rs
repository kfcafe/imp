use std::fmt;

use crate::context::estimate_tokens;
use crate::resources::{AgentsMd, Skill};
use crate::roles::Role;
use crate::tools::ToolRegistry;

/// A project fact from mana-core.
#[derive(Debug, Clone)]
pub struct Fact {
    pub text: String,
    pub verified_ago: String,
}

/// Previous attempt info for task context.
#[derive(Debug, Clone)]
pub struct Attempt {
    pub number: u32,
    pub outcome: String,
    pub summary: String,
}

/// Dependency info for task context.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub status: String,
    pub detail: String,
}

/// Task context for headless/task mode (Layer 5).
#[derive(Debug, Clone)]
pub struct TaskContext {
    pub title: String,
    pub description: String,
    pub verify: Option<String>,
    pub attempts: Vec<Attempt>,
    pub dependencies: Vec<Dependency>,
}

/// Result of system prompt assembly, including size tracking.
#[derive(Debug)]
pub struct AssembledPrompt {
    pub text: String,
    pub estimated_tokens: u32,
}

impl fmt::Display for AssembledPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Assemble the system prompt from five layers.
///
/// - Layer 1: Identity + tool descriptions (+ role instructions if any)
/// - Layer 2: Project context from AGENTS.md files
/// - Layer 3: Skills index
/// - Layer 4: Mana facts (skipped if empty)
/// - Layer 5: Task context (only in headless/task mode)
pub fn assemble(
    tools: &ToolRegistry,
    agents_md: &[AgentsMd],
    skills: &[Skill],
    facts: &[Fact],
    task: Option<&TaskContext>,
    role: Option<&Role>,
) -> AssembledPrompt {
    let mut parts = Vec::new();

    // Layer 1: Identity + tool descriptions
    parts.push(identity_layer(tools, role));

    // Layer 2: Project context from AGENTS.md
    if !agents_md.is_empty() {
        parts.push(agents_md_layer(agents_md));
    }

    // Layer 3: Skills index
    if !skills.is_empty() {
        parts.push(skills_layer(skills));
    }

    // Layer 4: Mana facts
    if !facts.is_empty() {
        parts.push(facts_layer(facts));
    }

    // Layer 5: Task context (headless mode only)
    if let Some(task) = task {
        parts.push(task_layer(task));
    }

    let text = parts.join("\n\n");
    let estimated_tokens = estimate_tokens(&text);

    AssembledPrompt {
        text,
        estimated_tokens,
    }
}

fn identity_layer(tools: &ToolRegistry, role: Option<&Role>) -> String {
    let mut s = String::from("You are imp, a coding agent.\n\nAvailable tools:\n");

    let defs = match role {
        Some(r) if r.readonly => tools.readonly_definitions(),
        _ => tools.definitions(),
    };

    for def in &defs {
        s.push_str(&format!("- {}: {}\n", def.name, def.description));
    }

    // Append role instructions after identity layer
    if let Some(role) = role {
        if let Some(ref instructions) = role.instructions {
            s.push('\n');
            s.push_str(instructions);
            s.push('\n');
        }
    }

    s
}

fn agents_md_layer(agents: &[AgentsMd]) -> String {
    let mut s = String::from("# Project Context\n\n");
    for agent in agents {
        s.push_str(&agent.content);
        s.push('\n');
    }
    s
}

fn skills_layer(skills: &[Skill]) -> String {
    let mut s = String::from(
        "Available skills (use read to load when relevant):\n",
    );
    for skill in skills {
        s.push_str(&format!(
            "- {}: {} [{}]\n",
            skill.name,
            skill.description,
            skill.path.display()
        ));
    }
    s
}

fn facts_layer(facts: &[Fact]) -> String {
    let mut s = String::from("Project facts:\n");
    for fact in facts {
        s.push_str(&format!(
            "- \"{}\" [verified {}]\n",
            fact.text, fact.verified_ago
        ));
    }
    s
}

fn task_layer(task: &TaskContext) -> String {
    let mut s = String::from("## Task\n");
    s.push_str(&format!("Title: {}\n", task.title));
    s.push_str(&format!("Description: {}\n", task.description));
    if let Some(ref verify) = task.verify {
        s.push_str(&format!("Verify: {}\n", verify));
    }

    if !task.attempts.is_empty() {
        s.push_str("\n## Previous attempts\n");
        for attempt in &task.attempts {
            s.push_str(&format!(
                "Attempt {} ({}): {}\n",
                attempt.number, attempt.outcome, attempt.summary
            ));
        }
    }

    if !task.dependencies.is_empty() {
        s.push_str("\n## Dependencies\n");
        for dep in &task.dependencies {
            s.push_str(&format!(
                "- {} ({}): {}\n",
                dep.name, dep.status, dep.detail
            ));
        }
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use crate::tools::{Tool, ToolContext, ToolOutput};

    // -- Test tool helpers --

    struct FakeTool {
        name: &'static str,
        description: &'static str,
        readonly: bool,
    }

    #[async_trait]
    impl Tool for FakeTool {
        fn name(&self) -> &str { self.name }
        fn label(&self) -> &str { self.name }
        fn description(&self) -> &str { self.description }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn is_readonly(&self) -> bool { self.readonly }
        async fn execute(
            &self, _: &str, _: serde_json::Value, _: ToolContext,
        ) -> crate::Result<ToolOutput> {
            Ok(ToolOutput::text("ok"))
        }
    }

    fn make_registry() -> ToolRegistry {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(FakeTool {
            name: "read",
            description: "Read file contents",
            readonly: true,
        }));
        reg.register(Arc::new(FakeTool {
            name: "write",
            description: "Write content to a file",
            readonly: false,
        }));
        reg.register(Arc::new(FakeTool {
            name: "edit",
            description: "Edit a file by replacing exact text",
            readonly: false,
        }));
        reg.register(Arc::new(FakeTool {
            name: "grep",
            description: "Search file contents for a pattern",
            readonly: true,
        }));
        reg
    }

    fn make_skill(name: &str, desc: &str, path: &str) -> Skill {
        Skill {
            name: name.into(),
            description: desc.into(),
            path: PathBuf::from(path),
        }
    }

    fn make_agents_md(content: &str) -> AgentsMd {
        AgentsMd {
            path: PathBuf::from("/project/AGENTS.md"),
            content: content.into(),
        }
    }

    fn make_readonly_role() -> Role {
        use crate::roles::ToolSet;
        Role {
            name: "reviewer".into(),
            model: None,
            thinking_level: None,
            tool_set: ToolSet::All,
            readonly: true,
            instructions: Some("Review code carefully. Do not modify files.".into()),
            max_turns: Some(10),
        }
    }

    fn make_worker_role() -> Role {
        use crate::roles::ToolSet;
        Role {
            name: "worker".into(),
            model: None,
            thinking_level: None,
            tool_set: ToolSet::All,
            readonly: false,
            instructions: None,
            max_turns: None,
        }
    }

    // -- Layer 1: Identity --

    #[test]
    fn system_prompt_identity_includes_all_tools() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        assert!(result.text.contains("You are imp, a coding agent."));
        assert!(result.text.contains("- read: Read file contents"));
        assert!(result.text.contains("- write: Write content to a file"));
        assert!(result.text.contains("- edit: Edit a file by replacing exact text"));
        assert!(result.text.contains("- grep: Search file contents for a pattern"));
    }

    #[test]
    fn system_prompt_identity_only_when_all_layers_empty() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        // Should have identity but no section headers for missing layers
        assert!(result.text.contains("You are imp"));
        assert!(!result.text.contains("# Project Context"));
        assert!(!result.text.contains("Available skills"));
        assert!(!result.text.contains("Project facts"));
        assert!(!result.text.contains("## Task"));
    }

    // -- Layer 2: AGENTS.md --

    #[test]
    fn system_prompt_agents_md_included_verbatim() {
        let reg = make_registry();
        let agents = vec![make_agents_md("# Rules\n\nUse snake_case everywhere.")];
        let result = assemble(&reg, &agents, &[], &[], None, None);
        assert!(result.text.contains("# Project Context"));
        assert!(result.text.contains("# Rules\n\nUse snake_case everywhere."));
    }

    #[test]
    fn system_prompt_multiple_agents_md_concatenated() {
        let reg = make_registry();
        let agents = vec![
            make_agents_md("Global rules here."),
            make_agents_md("Project rules here."),
        ];
        let result = assemble(&reg, &agents, &[], &[], None, None);
        assert!(result.text.contains("Global rules here."));
        assert!(result.text.contains("Project rules here."));
    }

    #[test]
    fn system_prompt_empty_agents_md_skipped() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        assert!(!result.text.contains("# Project Context"));
    }

    // -- Layer 3: Skills --

    #[test]
    fn system_prompt_skills_listed_with_paths() {
        let reg = make_registry();
        let skills = vec![
            make_skill("rust", "Conventions for Rust code", "/home/.imp/skills/rust/SKILL.md"),
            make_skill("testing", "Write and review tests", "/home/.imp/skills/testing/SKILL.md"),
        ];
        let result = assemble(&reg, &[], &skills, &[], None, None);
        assert!(result.text.contains("Available skills (use read to load when relevant):"));
        assert!(result.text.contains("- rust: Conventions for Rust code [/home/.imp/skills/rust/SKILL.md]"));
        assert!(result.text.contains("- testing: Write and review tests [/home/.imp/skills/testing/SKILL.md]"));
    }

    #[test]
    fn system_prompt_empty_skills_skipped() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        assert!(!result.text.contains("Available skills"));
    }

    // -- Layer 4: Mana facts --

    #[test]
    fn system_prompt_facts_included() {
        let reg = make_registry();
        let facts = vec![
            Fact { text: "Uses JWT for auth".into(), verified_ago: "2h ago".into() },
            Fact { text: "Test suite requires Docker".into(), verified_ago: "1d ago".into() },
        ];
        let result = assemble(&reg, &[], &[], &facts, None, None);
        assert!(result.text.contains("Project facts:"));
        assert!(result.text.contains("\"Uses JWT for auth\" [verified 2h ago]"));
        assert!(result.text.contains("\"Test suite requires Docker\" [verified 1d ago]"));
    }

    #[test]
    fn system_prompt_empty_facts_skipped() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        assert!(!result.text.contains("Project facts"));
    }

    // -- Layer 5: Task context --

    #[test]
    fn system_prompt_task_context_included() {
        let reg = make_registry();
        let task = TaskContext {
            title: "Fix the failing auth test".into(),
            description: "The JWT validation test panics on expired tokens".into(),
            verify: Some("cargo test auth::jwt_test".into()),
            attempts: vec![],
            dependencies: vec![],
        };
        let result = assemble(&reg, &[], &[], &[], Some(&task), None);
        assert!(result.text.contains("## Task"));
        assert!(result.text.contains("Title: Fix the failing auth test"));
        assert!(result.text.contains("Description: The JWT validation test panics"));
        assert!(result.text.contains("Verify: cargo test auth::jwt_test"));
    }

    #[test]
    fn system_prompt_task_with_attempts() {
        let reg = make_registry();
        let task = TaskContext {
            title: "Fix bug".into(),
            description: "Something is broken".into(),
            verify: None,
            attempts: vec![
                Attempt { number: 1, outcome: "failed".into(), summary: "Tried X, got error Y".into() },
                Attempt { number: 2, outcome: "failed".into(), summary: "Tried Z, still broken".into() },
            ],
            dependencies: vec![],
        };
        let result = assemble(&reg, &[], &[], &[], Some(&task), None);
        assert!(result.text.contains("## Previous attempts"));
        assert!(result.text.contains("Attempt 1 (failed): Tried X, got error Y"));
        assert!(result.text.contains("Attempt 2 (failed): Tried Z, still broken"));
    }

    #[test]
    fn system_prompt_task_with_dependencies() {
        let reg = make_registry();
        let task = TaskContext {
            title: "Implement feature".into(),
            description: "New feature".into(),
            verify: None,
            attempts: vec![],
            dependencies: vec![
                Dependency { name: "Schema types".into(), status: "completed".into(), detail: "defined in src/schema.rs".into() },
            ],
        };
        let result = assemble(&reg, &[], &[], &[], Some(&task), None);
        assert!(result.text.contains("## Dependencies"));
        assert!(result.text.contains("- Schema types (completed): defined in src/schema.rs"));
    }

    #[test]
    fn system_prompt_no_task_skips_layer5() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        assert!(!result.text.contains("## Task"));
    }

    #[test]
    fn system_prompt_task_without_verify_omits_verify_line() {
        let reg = make_registry();
        let task = TaskContext {
            title: "Do something".into(),
            description: "Details here".into(),
            verify: None,
            attempts: vec![],
            dependencies: vec![],
        };
        let result = assemble(&reg, &[], &[], &[], Some(&task), None);
        assert!(result.text.contains("Title: Do something"));
        assert!(!result.text.contains("Verify:"));
    }

    // -- Role-aware assembly --

    #[test]
    fn system_prompt_readonly_role_filters_tools() {
        let reg = make_registry();
        let role = make_readonly_role();
        let result = assemble(&reg, &[], &[], &[], None, Some(&role));
        // Should include readonly tools
        assert!(result.text.contains("- read:"));
        assert!(result.text.contains("- grep:"));
        // Should NOT include write tools
        assert!(!result.text.contains("- write:"));
        assert!(!result.text.contains("- edit:"));
    }

    #[test]
    fn system_prompt_role_instructions_appended() {
        let reg = make_registry();
        let role = make_readonly_role();
        let result = assemble(&reg, &[], &[], &[], None, Some(&role));
        assert!(result.text.contains("Review code carefully. Do not modify files."));
    }

    #[test]
    fn system_prompt_worker_role_includes_all_tools() {
        let reg = make_registry();
        let role = make_worker_role();
        let result = assemble(&reg, &[], &[], &[], None, Some(&role));
        assert!(result.text.contains("- read:"));
        assert!(result.text.contains("- write:"));
        assert!(result.text.contains("- edit:"));
        assert!(result.text.contains("- grep:"));
    }

    #[test]
    fn system_prompt_no_role_instructions_when_none() {
        let reg = make_registry();
        let role = make_worker_role();
        let result = assemble(&reg, &[], &[], &[], None, Some(&role));
        // Worker has no instructions, so the prompt shouldn't have extra instruction text
        let lines: Vec<&str> = result.text.lines().collect();
        let after_tools = lines.iter()
            .position(|l| l.starts_with("- grep:"))
            .unwrap();
        // Next non-empty line after the last tool should be end of identity layer
        // (no instructions appended)
        let remaining = &lines[after_tools + 1..];
        let next_content = remaining.iter().find(|l| !l.is_empty());
        assert!(next_content.is_none() || !next_content.unwrap().contains("Review"));
    }

    // -- Size tracking --

    #[test]
    fn system_prompt_tracks_estimated_tokens() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        assert!(result.estimated_tokens > 0);
        // Rough check: the text is at least ~100 chars, so >= 25 tokens
        assert!(result.estimated_tokens >= 10);
    }

    #[test]
    fn system_prompt_more_layers_means_more_tokens() {
        let reg = make_registry();

        let minimal = assemble(&reg, &[], &[], &[], None, None);

        let agents = vec![make_agents_md("Lots of project context here with many words.")];
        let skills = vec![make_skill("rust", "Rust conventions", "/skills/rust/SKILL.md")];
        let facts = vec![Fact { text: "Uses Postgres".into(), verified_ago: "1h ago".into() }];

        let full = assemble(&reg, &agents, &skills, &facts, None, None);

        assert!(full.estimated_tokens > minimal.estimated_tokens,
            "full ({}) should have more tokens than minimal ({})",
            full.estimated_tokens, minimal.estimated_tokens);
    }

    // -- Full assembly --

    #[test]
    fn system_prompt_all_layers_present() {
        let reg = make_registry();
        let agents = vec![make_agents_md("Be concise.")];
        let skills = vec![make_skill("rust", "Rust code conventions", "/skills/rust/SKILL.md")];
        let facts = vec![Fact { text: "Uses SQLite".into(), verified_ago: "30m ago".into() }];
        let task = TaskContext {
            title: "Add caching".into(),
            description: "Add Redis caching layer".into(),
            verify: Some("cargo test cache".into()),
            attempts: vec![
                Attempt { number: 1, outcome: "failed".into(), summary: "Wrong key format".into() },
            ],
            dependencies: vec![
                Dependency { name: "Config".into(), status: "done".into(), detail: "src/config.rs".into() },
            ],
        };

        let result = assemble(&reg, &agents, &skills, &facts, Some(&task), None);

        // All layers present in order
        let identity_pos = result.text.find("You are imp").unwrap();
        let context_pos = result.text.find("# Project Context").unwrap();
        let skills_pos = result.text.find("Available skills").unwrap();
        let facts_pos = result.text.find("Project facts").unwrap();
        let task_pos = result.text.find("## Task").unwrap();

        assert!(identity_pos < context_pos, "identity before context");
        assert!(context_pos < skills_pos, "context before skills");
        assert!(skills_pos < facts_pos, "skills before facts");
        assert!(facts_pos < task_pos, "facts before task");
    }

    #[test]
    fn system_prompt_display_impl() {
        let reg = make_registry();
        let result = assemble(&reg, &[], &[], &[], None, None);
        let displayed = format!("{result}");
        assert_eq!(displayed, result.text);
    }
}
