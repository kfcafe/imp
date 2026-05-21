use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::model::{
    ContextBlock, ContextBlockStability, ContextPack, ContextPackStatus, MemoryItem, SourceRef,
    Task, WorkId,
};
use crate::prototype::Prototype;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCompileRequest {
    pub work_id: WorkId,
    pub version: u32,
    pub token_budget: Option<u32>,
    pub objective: String,
    pub non_goals: Vec<String>,
    pub acceptance: Vec<String>,
    pub memory: Vec<MemoryItem>,
    pub checks: Vec<String>,
    pub prior_attempts: Vec<String>,
    pub source_refs: Vec<SourceRef>,
    pub launch_kind: ContextLaunchKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextLaunchKind {
    Task,
    Prototype,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedContextPack {
    pub pack_id: WorkId,
    pub work_id: WorkId,
    pub version: u32,
    pub blocks: Vec<RenderedContextBlock>,
    pub stable_prefix_hash: String,
    pub full_hash: String,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedContextBlock {
    pub title: String,
    pub body: String,
    pub stability: ContextBlockStability,
    pub hash: String,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextFreshness {
    pub stale: bool,
    pub stale_sources: Vec<SourceRef>,
}

pub struct ContextCompiler;

impl ContextCompiler {
    pub fn compile(request: ContextCompileRequest) -> ContextPack {
        let mut blocks = Vec::new();

        blocks.push(ContextBlock {
            title: "Objective".into(),
            body: request.objective,
            stability: ContextBlockStability::TaskVersionStable,
            source_refs: request.source_refs.clone(),
        });

        push_list_block(
            &mut blocks,
            "Non-goals",
            &request.non_goals,
            ContextBlockStability::TaskVersionStable,
            Vec::new(),
        );
        push_list_block(
            &mut blocks,
            "Acceptance",
            &request.acceptance,
            ContextBlockStability::TaskVersionStable,
            Vec::new(),
        );

        let mut memory_lines = request
            .memory
            .iter()
            .map(|memory| format!("- {}", memory.text.trim()))
            .collect::<Vec<_>>();
        memory_lines.sort();
        push_list_block(
            &mut blocks,
            "Relevant memory",
            &memory_lines,
            ContextBlockStability::ProjectStable,
            request
                .memory
                .iter()
                .flat_map(|memory| memory.source_refs.clone())
                .collect(),
        );

        push_list_block(
            &mut blocks,
            "Checks",
            &request.checks,
            ContextBlockStability::TaskVersionStable,
            Vec::new(),
        );
        push_list_block(
            &mut blocks,
            "Prior attempts",
            &request.prior_attempts,
            ContextBlockStability::TaskVersionStable,
            Vec::new(),
        );

        let output_contract = match request.launch_kind {
            ContextLaunchKind::Task => TASK_OUTPUT_CONTRACT,
            ContextLaunchKind::Prototype => PROTOTYPE_OUTPUT_CONTRACT,
        };
        blocks.push(ContextBlock {
            title: "Expected output".into(),
            body: output_contract.into(),
            stability: ContextBlockStability::TaskVersionStable,
            source_refs: Vec::new(),
        });

        ContextPack {
            id: WorkId(format!("CTX-{}-v{}", request.work_id, request.version)),
            work_id: request.work_id,
            version: request.version,
            status: ContextPackStatus::Ready,
            token_budget: request.token_budget,
            blocks,
            source_refs: request.source_refs,
        }
    }

    pub fn compile_task(
        task: &Task,
        memory: Vec<MemoryItem>,
        prior_attempts: Vec<String>,
    ) -> ContextPack {
        Self::compile(ContextCompileRequest {
            work_id: task.id.clone(),
            version: 1,
            token_budget: None,
            objective: task.title.clone(),
            non_goals: Vec::new(),
            acceptance: task.acceptance.clone(),
            memory,
            checks: task
                .checks
                .iter()
                .map(|check| {
                    check
                        .command
                        .clone()
                        .unwrap_or_else(|| check.description.clone())
                })
                .collect(),
            prior_attempts,
            source_refs: task.source_refs.clone(),
            launch_kind: ContextLaunchKind::Task,
        })
    }

    pub fn compile_prototype(prototype: &Prototype, memory: Vec<MemoryItem>) -> ContextPack {
        let mut acceptance = prototype.evidence_required.clone();
        if acceptance.is_empty() {
            acceptance.push(
                "Produce evidence, learnings, and a promote/discard/iterate recommendation.".into(),
            );
        }
        let mut non_goals = vec![
            "Do not treat prototype code as production code.".into(),
            "Do not wire prototype code into production modules unless explicitly promoted.".into(),
        ];
        if prototype.decision.is_some() {
            non_goals
                .push("Do not rerun a decided prototype unless creating a new iteration.".into());
        }
        Self::compile(ContextCompileRequest {
            work_id: WorkId::from(prototype.id.as_str()),
            version: 1,
            token_budget: None,
            objective: prototype.question.clone(),
            non_goals,
            acceptance,
            memory,
            checks: Vec::new(),
            prior_attempts: prototype.learnings.clone(),
            source_refs: Vec::new(),
            launch_kind: ContextLaunchKind::Prototype,
        })
    }
}

pub struct ContextRenderer;

impl ContextRenderer {
    pub fn render(pack: &ContextPack) -> RenderedContextPack {
        let blocks = pack
            .blocks
            .iter()
            .map(|block| RenderedContextBlock {
                title: block.title.clone(),
                body: block.body.clone(),
                stability: block.stability,
                hash: stable_hash(&render_block(block)),
                source_refs: block.source_refs.clone(),
            })
            .collect::<Vec<_>>();
        let stable_prefix = blocks
            .iter()
            .filter(|block| block.stability != ContextBlockStability::RunDynamic)
            .map(|block| block.hash.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let full = blocks
            .iter()
            .map(|block| block.hash.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let stale = Self::freshness(pack).stale;

        RenderedContextPack {
            pack_id: pack.id.clone(),
            work_id: pack.work_id.clone(),
            version: pack.version,
            blocks,
            stable_prefix_hash: stable_hash(&stable_prefix),
            full_hash: stable_hash(&full),
            stale,
        }
    }

    pub fn render_markdown(pack: &ContextPack) -> String {
        let mut output = String::new();
        output.push_str(&format!("# Context Pack {}\n\n", pack.id));
        output.push_str(&format!("work_id: {}\n", pack.work_id));
        output.push_str(&format!("version: {}\n", pack.version));
        output.push_str(&format!("status: {:?}\n\n", pack.status));
        for block in &pack.blocks {
            output.push_str(&render_block(block));
            output.push('\n');
        }
        output
    }

    pub fn freshness(pack: &ContextPack) -> ContextFreshness {
        let stale_sources = pack
            .blocks
            .iter()
            .flat_map(|block| block.source_refs.iter())
            .chain(pack.source_refs.iter())
            .filter(|source| source.fingerprint.as_deref() == Some("stale"))
            .cloned()
            .collect::<Vec<_>>();
        ContextFreshness {
            stale: !stale_sources.is_empty() || pack.status == ContextPackStatus::Stale,
            stale_sources,
        }
    }
}

fn push_list_block(
    blocks: &mut Vec<ContextBlock>,
    title: &str,
    items: &[String],
    stability: ContextBlockStability,
    source_refs: Vec<SourceRef>,
) {
    if items.is_empty() {
        return;
    }
    let mut sorted = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>();
    sorted.sort();
    blocks.push(ContextBlock {
        title: title.into(),
        body: sorted.join("\n"),
        stability,
        source_refs,
    });
}

fn render_block(block: &ContextBlock) -> String {
    format!("## {}\n\n{}\n", block.title.trim(), block.body.trim())
}

fn stable_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

const TASK_OUTPUT_CONTRACT: &str = "Return a structured outcome with status, summary, changed paths, checks run, memory updates, and follow-up tasks. Do not include a full transcript unless requested.";

const PROTOTYPE_OUTPUT_CONTRACT: &str = "Return hypothesis_result, evidence, learnings, followups, recommended_action, and whether the sandbox can be deleted. Prototype code is disposable unless explicitly promoted.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MemoryKind, SourceKind, Task};

    #[test]
    fn context_pack_rendering_is_cache_stable_for_same_inputs() {
        let mut task = Task::new("Build deterministic renderer");
        task.acceptance.push("same inputs produce same hash".into());
        let mut memory = MemoryItem::new(
            MemoryKind::Decision,
            "Keep dynamic lease data out of stable blocks.",
        );
        memory.source_refs.push(SourceRef {
            kind: SourceKind::Conversation,
            reference: "session:imp-work".into(),
            fingerprint: Some("abc123".into()),
        });

        let first = ContextCompiler::compile_task(&task, vec![memory.clone()], Vec::new());
        let second = ContextCompiler::compile_task(&task, vec![memory], Vec::new());

        assert_eq!(
            ContextRenderer::render(&first).stable_prefix_hash,
            ContextRenderer::render(&second).stable_prefix_hash
        );
    }

    #[test]
    fn dynamic_blocks_do_not_change_stable_prefix_hash() {
        let task = Task::new("Launch worker from context pack");
        let mut pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let base_hash = ContextRenderer::render(&pack).stable_prefix_hash;
        pack.blocks.push(ContextBlock {
            title: "Launch".into(),
            body: "lease_id: L-1".into(),
            stability: ContextBlockStability::RunDynamic,
            source_refs: Vec::new(),
        });
        let with_dynamic = ContextRenderer::render(&pack).stable_prefix_hash;

        assert_eq!(base_hash, with_dynamic);
    }

    #[test]
    fn stale_source_marks_context_pack_stale() {
        let mut task = Task::new("Detect stale context");
        task.source_refs.push(SourceRef {
            kind: SourceKind::FileRange,
            reference: "src/lib.rs:1-20".into(),
            fingerprint: Some("stale".into()),
        });
        let pack = ContextCompiler::compile_task(&task, Vec::new(), Vec::new());
        let freshness = ContextRenderer::freshness(&pack);

        assert!(freshness.stale);
        assert_eq!(freshness.stale_sources.len(), 2);
    }

    #[test]
    fn prototype_context_pack_uses_prototype_output_contract() {
        let mut prototype = Prototype::new(
            "Prototype context renderer",
            "Can context rendering be deterministic?",
            ".tmp/imp-prototypes/P-context",
        )
        .with_evidence_required(vec!["stable hash".into()]);
        prototype.record_learning("Sort list blocks before rendering.");

        let pack = ContextCompiler::compile_prototype(&prototype, Vec::new());
        let rendered = ContextRenderer::render_markdown(&pack);

        assert!(rendered.contains("Prototype code is disposable"));
        assert!(rendered.contains("stable hash"));
        assert!(rendered.contains("Sort list blocks"));
    }
}
