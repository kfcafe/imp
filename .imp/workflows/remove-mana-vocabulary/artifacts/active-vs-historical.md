# Mana active-vs-historical classification

Decision pending user review after inventory.

## Likely active release blockers
docs/autonomy-modes.md:125:- Existing bash-equivalent mana blocking remains in force.
docs/autonomy-modes.md:218:constraints, repeated-call protection, bash-equivalent mana blocking, schema
docs/mana-next-compatibility-adapter.md:1:# mana-next Compatibility Adapter
docs/mana-next-compatibility-adapter.md:4:Parent: mana `394.3` / child `394.3.3`
docs/mana-next-compatibility-adapter.md:8:The compatibility adapter lets imp-next view existing mana units as workflow-ledger records without breaking current mana commands or requiring a migration.
docs/mana-next-compatibility-adapter.md:13:current mana markdown/frontmatter + optional sidecars + imp run artifacts
docs/mana-next-compatibility-adapter.md:23:- existing `.mana/*.md` unit files
docs/mana-next-compatibility-adapter.md:25:- current mana decisions/notes where available
docs/mana-next-compatibility-adapter.md:26:- optional sidecars under `~/.mana/ledger/`
docs/mana-next-compatibility-adapter.md:40:These views are not necessarily new storage records. They are the compatibility interface used by imp workflow runtime, evidence, TUI summaries, and future mana-next commands.
docs/mana-next-compatibility-adapter.md:81:If `mana verify` has a recorded result in future metadata/sidecars, that result should populate status. Without result metadata, status is `pending` or `unknown` depending on context.
docs/mana-next-compatibility-adapter.md:93:The adapter should preserve direction exactly as existing mana commands interpret it.
docs/mana-next-compatibility-adapter.md:103:Current mana decisions map directly to DecisionView.
docs/mana-next-compatibility-adapter.md:138:2. Sidecars under `~/.mana/ledger/` for structured repeated records.
docs/mana-next-compatibility-adapter.md:154:If imp is asked to run mana unit `394.2.1`:
docs/mana-next-compatibility-adapter.md:178:Current `mana close` remains authoritative for old status updates. imp-next closeout should call compatible update APIs rather than bypass mana status semantics.
docs/mana-next-compatibility-adapter.md:194:struct ManaLedgerAdapter;
docs/mana-next-compatibility-adapter.md:196:impl ManaLedgerAdapter {
docs/mana-next-compatibility-adapter.md:209:This API can live in imp-core initially and use current mana tool/CLI/file APIs under the hood.
docs/mana-next-compatibility-adapter.md:213:### Current mana task
docs/mana-next-compatibility-adapter.md:243:### Current mana epic
docs/mana-next-compatibility-adapter.md:268:| user-created mana unit | durable_mana_record |
docs/mana-next-compatibility-adapter.md:272:| old fact with TTL | mana_fact_staleable |
docs/mana-next-compatibility-adapter.md:279:- Should sidecars be addressed by mana unit ID, run ID, or both?
docs/mana-next-compatibility-adapter.md:281:- How should current `mana verify` results be persisted so VerificationView can show status?
docs/mana-next-examples.md:1:# mana-next Workflow Ledger Examples
docs/mana-next-examples.md:4:Parent: mana `394.3` / child `394.3.7`
docs/mana-next-examples.md:6:These examples demonstrate the streamlined mana-next vocabulary while preserving compatibility with current mana concepts.
docs/mana-next-examples.md:146:## Current mana-compatible markdown sketch
docs/mana-next-examples.md:148:A current mana task can remain markdown/frontmatter and gain workflow-ledger refs later.
docs/mana-next-examples.md:184:~/.mana/ledger/
docs/mana-next-examples.md:191:The current mana markdown unit remains readable without these sidecars.
docs/reference-monitor-policy.md:23:6. Bash calls are checked for mana-equivalent commands with
docs/reference-monitor-policy.md:24:   `mana_bash_equivalent_hint` and can be blocked with a native mana-tool hint.
docs/reference-monitor-policy.md:25:7. Mana calls are checked with `evaluate_mana_policy`, which applies
docs/reference-monitor-policy.md:26:   `AgentMode::allows_mana_action` and records mana action class details.
docs/reference-monitor-policy.md:52:- `Full`: all tools and mana actions
docs/reference-monitor-policy.md:53:- `Worker`: implementation tools and limited progress-checkpoint mana actions
docs/reference-monitor-policy.md:54:- `Orchestrator`: mana orchestration with no direct file write tool
docs/reference-monitor-policy.md:56:- `Reviewer`: read-only inspection; no mana actions
docs/reference-monitor-policy.md:57:- `Auditor`: read/report-oriented code and mana inspection
docs/reference-monitor-policy.md:59:`AgentMode::allows_tool` and `AgentMode::allows_mana_action` are currently called
docs/reference-monitor-policy.md:69:### Mana loop policy
docs/reference-monitor-policy.md:71:`agent/mana_loop.rs` classifies mana actions (`inspect`, `lifecycle`,
docs/reference-monitor-policy.md:73:current `AgentMode`. That decision is already structured as `ManaPolicyDecision`;
docs/reference-monitor-policy.md:79:`mana_bash_equivalent_hint` blocks shell commands that should use the native
docs/reference-monitor-policy.md:80:mana tool. This is a policy check, not a bash implementation detail. The monitor
docs/reference-monitor-policy.md:118:       mana action policy
docs/reference-monitor-policy.md:147:- mana action and action class for mana calls
docs/reference-monitor-policy.md:162:(`bash`, `git`, `mana`), and schema/path extraction helpers. Later extension work
docs/reference-monitor-policy.md:194:    ManaLoop,
docs/reference-monitor-policy.md:223:artifact refs. Mana should receive durable summaries/refs, not raw policy logs.
docs/reference-monitor-policy.md:235:- keep mana bash-equivalent blocking and mana action class details
docs/reference-monitor-policy.md:246:   `AgentMode`, `RunPolicy`, mana policy, bash-equivalent, repeated-call, schema,
docs/reference-monitor-policy.md:270:  `edit`, `multi_edit`, `bash`, `git`, `worktree`, `mana`, `web`, `ask`,
docs/reference-monitor-policy.md:273:  mana action, and network host.
docs/reference-monitor-policy.md:277:- Adapter records for hook blocking, mana policy decisions, bash-equivalent
docs/reference-monitor-policy.md:321:- `action` for mana actions
docs/reference-monitor-policy.md:355:- `mana_policy_allowed`
docs/reference-monitor-policy.md:356:- `mana_policy_blocked`
docs/reference-monitor-policy.md:399:- Hook, mana, bash-equivalent, repeated-call, schema, and guardrail outcomes have
docs/reference-monitor-policy.md:426:  `edit`, `multi_edit`, `bash`, `git`, `worktree`, `mana`, `web`, `ask`,
docs/reference-monitor-policy.md:429:  mana action, and network host.
docs/reference-monitor-policy.md:433:- Adapter records for hook blocking, mana policy decisions, bash-equivalent
docs/reference-monitor-policy.md:477:- `action` for mana actions
docs/reference-monitor-policy.md:511:- `mana_policy_allowed`
docs/reference-monitor-policy.md:512:- `mana_policy_blocked`
docs/reference-monitor-policy.md:555:- Hook, mana, bash-equivalent, repeated-call, schema, and guardrail outcomes have
Cargo.toml:97:mana-core = "0.3.2"
README.md:248:| `/secrets` | credential manager |
README.md:416:- legacy `mana` integration is optional and compatibility-oriented
docs/imp-next-workflow-runtime.md:4:Audience: imp maintainers, mana maintainers, future TUI/GUI/runtime implementers
docs/imp-next-workflow-runtime.md:22:        -> mana workflow ledger
docs/imp-next-workflow-runtime.md:32:4. Make mana the streamlined durable workflow/evidence ledger, not a noisy project-management UI.
docs/imp-next-workflow-runtime.md:45:- Do not turn mana into Jira. Mana should be a workflow ledger and evidence index.
docs/imp-next-workflow-runtime.md:59:- Mana task execution for durable work.
docs/imp-next-workflow-runtime.md:68:- **mana** = platform / durable graph layer
docs/imp-next-workflow-runtime.md:69:- **imp** = agent + default human-facing environment on mana
docs/imp-next-workflow-runtime.md:125:│ Mana Workflow Ledger + Run Artifacts          │
docs/imp-next-workflow-runtime.md:156:- Mana-backed or task-backed runs can attach a `mana_unit_ref`, but no mana ledger writes are implemented by this slice.
docs/imp-next-workflow-runtime.md:164:- mana workflow-ledger writes
docs/imp-next-workflow-runtime.md:189:The TUI should not force users to manually manage workflow IDs for simple requests.
docs/imp-next-workflow-runtime.md:227:A workflow contract should be created implicitly for normal TUI runs and explicitly for mana/task/CI runs.
docs/imp-next-workflow-runtime.md:246:  mana_unit_ref?
docs/imp-next-workflow-runtime.md:298:## Mana as workflow/evidence ledger
docs/imp-next-workflow-runtime.md:300:Mana should become the durable graph and evidence index for workflows.
docs/imp-next-workflow-runtime.md:311:Current mana epics/tasks/facts/decisions can remain compatible, but the new runtime should write more structured workflow records.
docs/imp-next-workflow-runtime.md:313:Mana should record:
docs/imp-next-workflow-runtime.md:325:Mana should not store raw transcript spam. Raw event traces belong in run artifacts; mana stores pointers and durable summaries.
docs/imp-next-workflow-runtime.md:348:a complete privacy boundary. Mana should store stable summaries and artifact
docs/imp-next-workflow-runtime.md:355:Mana can point to these artifacts from workflow/task records.
docs/imp-next-workflow-runtime.md:422:- hooks/mana policy
docs/imp-next-workflow-runtime.md:469:Durable memory/mana writes derived from low-trust content should be scoped to the workflow or require review.
docs/imp-next-workflow-runtime.md:638:mana.updated
docs/imp-next-workflow-runtime.md:653:- mana unit refs
docs/imp-next-workflow-runtime.md:669:- Add mana workflow/evidence ledger adapter.
docs/imp-next-workflow-runtime.md:707:2. How should mana workflow records map to existing epic/task/fact files without breaking old mana usage?
docs/imp-next-workflow-runtime.md:720:- Mana stores durable workflow/evidence summaries, not transcript noise.
docs/role-registry.md:276:tools = ["read", "scan", "edit", "write", "bash", "git", "mana"]
docs/child-workflow-delegation.md:4:Audience: imp maintainers, mana maintainers, TUI/runtime implementers
docs/child-workflow-delegation.md:10:by OMO-style parallel agent work, but the imp core abstraction is workflow + mana
docs/child-workflow-delegation.md:29:5. Persist lifecycle state in mana and run artifacts.
docs/child-workflow-delegation.md:65:- parent mana unit ref when available
docs/child-workflow-delegation.md:80:    pub parent_mana_unit_ref: Option<String>,
docs/child-workflow-delegation.md:117:Mana relationship:
docs/child-workflow-delegation.md:127:Do not create noisy mana tasks for every trivial child unless the child needs
docs/child-workflow-delegation.md:252:## Mana records and evidence integration
docs/child-workflow-delegation.md:289:- mana notes/facts when durable
docs/child-workflow-delegation.md:424:- mana ledger adapters for child refs, child task records, and evidence records
docs/child-workflow-delegation.md:432:- full parallel child scheduling/resource management
docs/child-workflow-delegation.md:442:4. Sequential child workflow manager for verifier/reviewer/researcher.
docs/child-workflow-delegation.md:443:5. Mana ledger persistence and evidence refs.
docs/child-workflow-delegation.md:458:- mana ledger: durable truth
docs/child-workflow-delegation.md:461:This keeps child delegation usable from CLI, TUI, GUI, CI, and mana without
docs/mana-next-workflow-ledger.md:1:# mana-next Workflow Ledger Schema
docs/mana-next-workflow-ledger.md:4:Parent: mana `394.3` / child `394.3.1`
docs/mana-next-workflow-ledger.md:8:mana-next is the durable workflow ledger for imp-next. It records the stable state and reviewable evidence of agent work without becoming a project-management UI or a transcript dump.
docs/mana-next-workflow-ledger.md:19:Existing mana units remain compatible. This document defines how the new vocabulary maps onto the current file-backed mana concepts: epic, task, fact, decision, notes, dependencies, verify commands, acceptance criteria, and artifacts.
docs/mana-next-workflow-ledger.md:24:- No removal of existing mana primitives.
docs/mana-next-workflow-ledger.md:26:- No transcript storage in mana records.
docs/mana-next-workflow-ledger.md:32:1. **Workflow ledger, not Jira.** mana records execution truth and evidence, not a noisy planning bureaucracy.
docs/mana-next-workflow-ledger.md:33:2. **Summaries in mana, bulk in artifacts.** Raw traces, logs, diffs, and transcripts live under imp run artifacts; mana stores refs and durable summaries.
docs/mana-next-workflow-ledger.md:34:3. **Compatibility first.** Existing `mana list/show/create/update/verify/close/decision/notes/deps` should continue to work.
docs/mana-next-workflow-ledger.md:92:| mana-next Workflow | Current mana |
docs/mana-next-workflow-ledger.md:104:A Task is a decomposed execution unit within a Workflow. Current mana `task` maps naturally here.
docs/mana-next-workflow-ledger.md:129:| mana-next Task | Current mana |
docs/mana-next-workflow-ledger.md:163:| mana-next Decision | Current mana |
docs/mana-next-workflow-ledger.md:204:| mana-next Verification | Current mana |
docs/mana-next-workflow-ledger.md:207:| status/result | current `mana verify` outcome plus notes/logs |
docs/mana-next-workflow-ledger.md:246:| mana-next Evidence | Current mana |
docs/mana-next-workflow-ledger.md:272:| mana-next Note | Current mana |
docs/mana-next-workflow-ledger.md:306:  └─ may be summarized in mana
docs/mana-next-workflow-ledger.md:371:Existing mana compatibility can model child runs as child tasks initially.
docs/mana-next-workflow-ledger.md:375:1. Existing file-backed mana units under `~/.mana` must remain readable.
docs/mana-next-workflow-ledger.md:385:<thead><tr><th>mana-next</th><th>Current mana</th><th>Compatibility behavior</th></tr></thead>
docs/mana-next-workflow-ledger.md:401:- Should workflow IDs always equal mana unit IDs, or should imp run workflows have separate run IDs linked to mana units?
docs/mana-next-workflow-ledger.md:402:- How much workflow metadata belongs in `~/.mana` versus repo `.imp/runs` artifacts?
docs/dependency-audit.md:17:- `serde_yml 0.0.12` (`RUSTSEC-2025-0068`) and `libyml 0.0.5` (`RUSTSEC-2025-0067`) come from `mana-core 0.3.2`.
docs/dependency-audit.md:18:  - No newer `mana-core` version was published at the time of review.
docs/dependency-audit.md:19:  - Mitigation requires an upstream `mana-core` release or reducing/removing mana-core integration.
docs/mana-next-ux.md:1:# mana-next UX and Progressive Disclosure
docs/mana-next-ux.md:4:Parent: mana `394.3` / child `394.3.8`
docs/mana-next-ux.md:8:mana-next should feel invisible for routine TUI work and invaluable when work becomes durable, blocked, verified, resumed, delegated, or reviewed.
docs/mana-next-ux.md:10:The user should not need to understand mana to ask imp to fix a small issue. But when the task matters, mana should provide the durable workflow ledger: status, blockers, decisions, verification, evidence, and closeout.
docs/mana-next-ux.md:15:2. **Progressive disclosure.** Show workflow/mana details only when useful.
docs/mana-next-ux.md:16:3. **Automatic bookkeeping.** The agent writes durable summaries/evidence refs; users should not manually update mana after every step.
docs/mana-next-ux.md:17:4. **No transcript spam.** mana stores workflow summaries and artifact refs, not raw logs.
docs/mana-next-ux.md:18:5. **Recovery-oriented.** A user should be able to resume, inspect blockers, and find evidence from mana.
docs/mana-next-ux.md:30:- no visible mana ceremony
docs/mana-next-ux.md:51:- mana stores durable summary/evidence refs
docs/mana-next-ux.md:53:### Mana-backed work
docs/mana-next-ux.md:61:- mana unit is the workflow/task anchor
docs/mana-next-ux.md:63:- verification and evidence refs attach to the mana ledger
docs/mana-next-ux.md:84:mana list
docs/mana-next-ux.md:85:mana show 394.2
docs/mana-next-ux.md:86:mana verify 394.2.1
docs/mana-next-ux.md:87:mana close 394.2.1
docs/mana-next-ux.md:100:These should read the mana ledger, not replace it.
docs/mana-next-ux.md:122:Mana: updated 394.2.1 -> done
docs/mana-next-ux.md:135:## What mana records automatically
docs/mana-next-ux.md:160:- mana workflow/task show
docs/mana-next-ux.md:172:mana stores refs and summaries.
docs/mana-next-ux.md:190:## Compatibility with current mana
docs/mana-next-ux.md:192:Current mana remains valid:
docs/mana-next-ux.md:204:### Do I need to create a mana workflow before using imp?
docs/mana-next-ux.md:206:No. Routine TUI use should work without manual mana setup.
docs/mana-next-ux.md:208:### When does mana matter?
docs/mana-next-ux.md:212:### Does mana store every agent message?
docs/mana-next-ux.md:214:No. Raw traces live in run artifacts. mana stores durable summaries and references.
docs/mana-next-ux.md:216:### Can I still use current mana commands?
docs/mana-next-ux.md:218:Yes. mana-next is designed as a compatibility layer first.
docs/mana-next-ux.md:220:### Is this a project-management system?
docs/trace-and-evidence-format.md:4:Parent: mana `394.4` / child `394.4.1`
docs/trace-and-evidence-format.md:81:| `workflow_id` | mana/workflow contract id when available |
docs/trace-and-evidence-format.md:161:<tr><td><code>TurnEnd { index, message, mana_review }</code></td><td><code>turn.end</code></td><td>assistant message summary, mana review summary</td><td>Do not inline huge message content in future if artifact exists.</td></tr>
docs/trace-and-evidence-format.md:313:- workflow id / mana id when present
docs/trace-and-evidence-format.md:408:| mana refs | mana workflow ledger adapter |
docs/trace-and-evidence-format.md:481:enabled. Autonomy can reduce prompts; it must not remove auditability. Mana
docs/trace-and-evidence-format.md:499:- No mana ledger write path for evidence refs yet.
docs/mana-next-runtime-event-mapping.md:1:# mana-next Runtime Event Mapping
docs/mana-next-runtime-event-mapping.md:4:Parent: mana `394.3` / child `394.3.6`
docs/mana-next-runtime-event-mapping.md:8:This document defines which imp runtime events should create durable mana-next workflow ledger updates.
docs/mana-next-runtime-event-mapping.md:12:- **mana stores durable summaries and artifact refs**
docs/mana-next-runtime-event-mapping.md:16:Do not write every event to mana. Most runtime events belong only in `trace.jsonl`.
docs/mana-next-runtime-event-mapping.md:21:<thead><tr><th>Runtime event</th><th>mana update</th><th>Write policy</th><th>Notes</th></tr></thead>
docs/mana-next-runtime-event-mapping.md:23:<tr><td><code>workflow.started</code></td><td>Create/update WorkflowRecord status = executing; attach workflow contract summary and run_id.</td><td>Automatic for mana-backed or meaningful workflows.</td><td>Trivial chat may remain artifact-only.</td></tr>
docs/mana-next-runtime-event-mapping.md:28:<tr><td><code>contract.created</code></td><td>Attach workflow_contract_ref and summary fields.</td><td>Automatic for workflows with mana record.</td><td>Full contract lives in run artifacts.</td></tr>
docs/mana-next-runtime-event-mapping.md:29:<tr><td><code>tool.started</code></td><td>No mana write.</td><td>Never by default.</td><td>Trace only.</td></tr>
docs/mana-next-runtime-event-mapping.md:30:<tr><td><code>tool.completed</code></td><td>No mana write unless it produced an artifact/evidence ref.</td><td>Artifact summary only.</td><td>Example: diff.patch, evidence.md, verify.log.</td></tr>
docs/mana-next-runtime-event-mapping.md:37:<tr><td><code>mana.updated</code></td><td>No recursive write.</td><td>Never.</td><td>Trace only to avoid loops.</td></tr>
docs/mana-next-runtime-event-mapping.md:62:No mana writes:
docs/mana-next-runtime-event-mapping.md:88:mana update:
docs/mana-next-runtime-event-mapping.md:119:mana update:
docs/mana-next-runtime-event-mapping.md:141:mana update:
docs/mana-next-runtime-event-mapping.md:156:Runtime should batch/coalesce mana updates where possible:
docs/mana-next-runtime-event-mapping.md:162:This prevents `.mana` churn.
docs/mana-next-runtime-event-mapping.md:166:If mana update fails:
docs/mana-next-runtime-event-mapping.md:171:4. Keep run artifacts intact so a later repair can reconstruct mana refs.
docs/mana-next-runtime-event-mapping.md:178:- Should trivial TUI sessions ever create mana records automatically?
docs/tui-workflow-wireframes.md:5:Related epic: `394 Evolve imp into workflow-first agent runtime with mana ledger and extension support`
docs/tui-workflow-wireframes.md:25:  - It has bottom-left labels: mana scope, mana run, build loop/loop state.
docs/tui-workflow-wireframes.md:33:  - It can show mana run detail or thinking when no tool is selected.
docs/tui-workflow-wireframes.md:62:The current imp TUI should remain conversation-first. Workflow-first features should make the existing TUI more legible, not turn it into a project-management dashboard.
docs/tui-workflow-wireframes.md:64:A future GUI may make sense for richer evidence browsing, diff review, worktree management, and child workflow supervision. But the GUI should consume the same runtime state as the TUI:
docs/tui-workflow-wireframes.md:82:8. **Closeout should be satisfying.** Final summaries should make evidence, verification, diff, and mana status easy to inspect.
docs/tui-workflow-wireframes.md:99:│                                              │ - mana run detail            ││
docs/tui-workflow-wireframes.md:104:│ bottom-left: mana/build/loop labels                                           │
docs/tui-workflow-wireframes.md:210:Mana detail       current mana run detail pattern
docs/tui-workflow-wireframes.md:401:│ e open evidence · d diff · m mana · gpt-5.1-codex · main                    │
docs/tui-workflow-wireframes.md:440:## 14. Mana ledger through existing sidebar/detail
docs/tui-workflow-wireframes.md:442:Current `SidebarView` can show mana run detail when no tool is selected. Build on that.
docs/tui-workflow-wireframes.md:446:│ Mana                         │
docs/tui-workflow-wireframes.md:466:Do not put mana in the main chat unless the user asked to inspect/update it.
docs/tui-workflow-wireframes.md:613:│ [Tools] [Verify] [Evidence] [Mana] │
docs/tui-workflow-wireframes.md:683:- mana ledger
docs/tui-workflow-wireframes.md:702:- Add mana detail using current mana run detail pattern.
docs/trust-labels-and-provenance.md:36:    ManaLedger,
docs/trust-labels-and-provenance.md:48:    ManaFact,
docs/trust-labels-and-provenance.md:49:    ManaNote,
docs/trust-labels-and-provenance.md:50:    ManaDecision,
docs/trust-labels-and-provenance.md:96:Tool outputs from read/search/bash/git/mana/web/etc. Trust depends on the tool,
docs/trust-labels-and-provenance.md:119:### Mana fact/note/decision
docs/trust-labels-and-provenance.md:121:Durable mana ledger records. They are structured and reviewable, but still need
docs/trust-labels-and-provenance.md:128:Mana records can be trusted as workflow state when verified or explicitly adopted,
docs/trust-labels-and-provenance.md:174:   memory persistence, mana ledger writes, network/secrets/destructive access, or
docs/trust-labels-and-provenance.md:192:- mana record -> mana source type plus unit id
docs/trust-labels-and-provenance.md:218:- `mana show` -> mana ledger provenance
docs/trust-labels-and-provenance.md:226:Before writing durable memory, mana facts/notes/decisions, eval candidates, or
docs/trust-labels-and-provenance.md:244:[context id=ctx_22 source=mana-fact trust=mana-ledger unit=394.7 verified=true]
docs/trust-labels-and-provenance.md:274:- mana fact/note/decision writes as adopted truth
docs/trust-labels-and-provenance.md:324:mana: 394.7       mana-ledger
docs/trust-labels-and-provenance.md:342:2. Attach provenance during context assembly for user, workspace, web, mana,
docs/trust-labels-and-provenance.md:347:6. Gate durable memory/mana writes by provenance.
docs/trust-labels-and-provenance.md:369:  - `ManaRecordKind`
docs/trust-labels-and-provenance.md:373:- mana prompt-context provenance for facts and project memory status
docs/trust-labels-and-provenance.md:435:Future prompt context can add labels for web, mana, verifier, generated summaries,
docs/trust-labels-and-provenance.md:461:## Durable memory and mana writes
docs/trust-labels-and-provenance.md:483:Mana durable writes should follow the same rule as they are wired: low-trust
docs/trust-labels-and-provenance.md:485:not as adopted fact, policy, or decision. Mana facts should carry verification
docs/trust-labels-and-provenance.md:532:  mana facts, autonomy escalation, or dangerous grants
docs/trust-labels-and-provenance.md:549:- `mana show` should produce mana-ledger provenance
docs/trust-labels-and-provenance.md:585:- mana write gating is specified but not fully enforced everywhere yet
docs/trust-labels-and-provenance.md:607:- Will this content be written to memory/mana/eval artifacts?
docs/trust-labels-and-provenance.md:636:  - `ManaRecordKind`
docs/trust-labels-and-provenance.md:640:- mana prompt-context provenance for facts and project memory status
docs/trust-labels-and-provenance.md:702:Future prompt context can add labels for web, mana, verifier, generated summaries,
docs/trust-labels-and-provenance.md:728:## Durable memory and mana writes
docs/trust-labels-and-provenance.md:750:Mana durable writes should follow the same rule as they are wired: low-trust
docs/trust-labels-and-provenance.md:752:not as adopted fact, policy, or decision. Mana facts should carry verification
docs/trust-labels-and-provenance.md:799:  mana facts, autonomy escalation, or dangerous grants
docs/trust-labels-and-provenance.md:816:- `mana show` should produce mana-ledger provenance
docs/trust-labels-and-provenance.md:852:- mana write gating is specified but not fully enforced everywhere yet
docs/trust-labels-and-provenance.md:874:- Will this content be written to memory/mana/eval artifacts?
docs/workflow-profiles.md:74:Workflow profiles are backend-neutral. They should use imp-native-ready concepts such as plan, task, evidence, decision, verification, artifact, and closeout. Current mana compatibility should stay behind adapters; normal users should not need to understand mana terminology.
docs/mana-next-migration-test-plan.md:1:# mana-next Compatibility and Migration Test Plan
docs/mana-next-migration-test-plan.md:4:Parent: mana `394.3` / child `394.3.9`
docs/mana-next-migration-test-plan.md:8:Before changing mana internals, we need a compatibility test plan that protects current mana behavior while adding workflow-ledger views, sidecars, evidence refs, and imp adapters.
docs/mana-next-migration-test-plan.md:12:Use temporary mana roots whenever possible:
docs/mana-next-migration-test-plan.md:15:/tmp/imp-mana-test/.mana
docs/mana-next-migration-test-plan.md:18:Avoid mutating the developer's real `~/.mana` in automated tests.
docs/mana-next-migration-test-plan.md:22:- clean empty mana root
docs/mana-next-migration-test-plan.md:23:- existing mana root with old epic/task/fact/decision files
docs/mana-next-migration-test-plan.md:24:- mana root with new `ledger/` sidecars
docs/mana-next-migration-test-plan.md:33:mana template kind=task
docs/mana-next-migration-test-plan.md:34:mana list --count 1
docs/mana-next-migration-test-plan.md:35:mana show <id>
docs/mana-next-migration-test-plan.md:36:mana create --kind task --title "..."
docs/mana-next-migration-test-plan.md:37:mana update <id> --notes "..."
docs/mana-next-migration-test-plan.md:38:mana verify <id>
docs/mana-next-migration-test-plan.md:39:mana close <id>
docs/mana-next-migration-test-plan.md:40:mana notes_append <id> --notes "..."
docs/mana-next-migration-test-plan.md:41:mana decision_add <id> --title "..."
docs/mana-next-migration-test-plan.md:42:mana decision_resolve <id> --resolve_decisions "..."
docs/mana-next-migration-test-plan.md:43:mana dep_add --from-id <id> --dep-id <dep>
docs/mana-next-migration-test-plan.md:44:mana dep_remove --from-id <id> --dep-id <dep>
docs/mana-next-migration-test-plan.md:47:If exact CLI syntax differs, use native mana tool equivalents in integration tests.
docs/mana-next-migration-test-plan.md:54:fixtures/mana-old/
docs/mana-next-migration-test-plan.md:74:fixtures/mana-next/
docs/mana-next-migration-test-plan.md:85:- existing mana list/show still sees `200-workflow.md`
docs/mana-next-migration-test-plan.md:89:- child run refs do not break old mana commands
docs/mana-next-migration-test-plan.md:109:- artifact content is not inlined into mana record
docs/mana-next-migration-test-plan.md:146:Input: unresolved current mana decision.
docs/mana-next-migration-test-plan.md:165:- no raw trace content written to mana markdown
docs/mana-next-migration-test-plan.md:198:- current mana status maps consistently
docs/mana-next-migration-test-plan.md:206:Given old mana files only:
docs/mana-next-migration-test-plan.md:214:Given old mana file, after imp writes evidence ref:
docs/mana-next-migration-test-plan.md:226:- Delete `~/.mana/ledger/` sidecars: old mana list/show still works.
docs/mana-next-migration-test-plan.md:227:- Delete `.imp/runs/<run-id>/` artifact: mana shows broken evidence ref gracefully.
docs/mana-next-migration-test-plan.md:228:- Disable mana-next adapter: current mana commands remain usable.
docs/mana-next-migration-test-plan.md:239:## CI gates for mana-next implementation
docs/mana-next-migration-test-plan.md:244:cargo test -p imp-core mana_workflow_ledger
docs/mana-next-migration-test-plan.md:245:cargo test -p imp-core mana_workflow_ledger_adapter
docs/mana-next-migration-test-plan.md:252:cargo test -p imp-core mana_next_fixtures
docs/mana-next-migration-test-plan.md:255:Manual smoke with native mana tool:
docs/mana-next-migration-test-plan.md:258:mana template kind=task
docs/mana-next-migration-test-plan.md:259:mana list --count 1
docs/mana-next-migration-test-plan.md:260:mana show <known-id>
docs/mana-next-migration-test-plan.md:265:The mana-next implementation is migration-ready only when:
docs/mana-next-migration-test-plan.md:272:- evidence/log content is not inlined into mana records
CHANGELOG.md:11:- Removed unused direct `mana-core` dependency from the experimental `imp-gui` crate.
CHANGELOG.md:43:- Prefer native workflow policy, prompt, and tool surfaces over legacy mana/prototype guidance.
CHANGELOG.md:86:- Made `imp` standalone by default again: the default `imp-cli` dependency tree no longer pulls in `imp-work`, `mana-core`, or `mana-cli`.
CHANGELOG.md:87:- Moved mana-facing CLI/TUI/tool integration behind optional `mana-ui` / `mana-tool` feature gates while keeping default chat/run usage independent from mana.
CHANGELOG.md:88:- Split `imp-core` workflow integration away from the core agent loop into explicit runtime layers, separating recipe/runtime support from mana compatibility code.
CHANGELOG.md:89:- Reworked mana-related runtime hooks so mana API support and the heavier mana tool/CLI integration are separate optional feature surfaces.
CHANGELOG.md:90:- Updated the TUI to compile in a standalone default configuration, with mana navigator/run UI available only when the optional mana UI feature is enabled.
CHANGELOG.md:101:- Fixed feature gating so `imp-core --features mana-api` builds without pulling in the heavier mana tool integration.
CHANGELOG.md:102:- Fixed default dependency hygiene so `cargo tree -p imp-cli` has no `imp-work`, `mana-core`, or `mana-cli` entries unless optional mana integration is enabled.
CHANGELOG.md:103:- Fixed standalone TUI/CLI build paths that previously assumed mana run state or mana navigator types were always available.
CHANGELOG.md:137:- Folded worktree management into the native `git` tool as `worktree_list`, `worktree_add`, and `worktree_remove`; removed the standalone `worktree` tool.
CHANGELOG.md:152:- Added the native `work` tool as imp's durable work system, replacing mana as the default agent workflow for tasks, lifecycle state, context, verification, and handoff.
CHANGELOG.md:156:- Added an offline `scripts/migrate-mana-to-imp-work` migration command for importing existing `.mana` units into global imp-work storage.
CHANGELOG.md:161:- Switched system prompt, mode guidance, and agent workflow nudges from mana-first durable work to native imp-work.
CHANGELOG.md:173:- Fixed over-eager runtime stopping after tool observations so `read`, `edit`, failed `bash`, and mana close results are interpreted by the agent instead of ending work prematurely.
CHANGELOG.md:183:- Improved native mana run responsiveness during direct run orchestration.
CHANGELOG.md:190:- Removed defunct `ask_agent` helper-agent tool; use mana units/runs for durable delegation.
CHANGELOG.md:192:- Added mana `guide` and `template` actions so agents can inspect task/epic/decision/verify/orchestration guidance from the native mana tool.
CHANGELOG.md:193:- Added validation hints to mana actions to make schema errors and missing arguments easier for agents to recover from.
CHANGELOG.md:204:- Strengthened mana worker context assembly and improved mana unit closure reliability after implementation.
CHANGELOG.md:223:- Continued schema hardening across core tools, including `read`, `write`, `edit`, `scan`, `git`, `bash`, `web`, `ask_user`, and mana.
CHANGELOG.md:251:- Published `imp-core`, the agent runtime with tools, sessions, context, hooks, mana integration, and early SDK surface.
CHANGELOG.md:259:- Native tools for file I/O, editing, shell commands, git, structural code scanning, web reading/search, user prompts, memory, and mana coordination.
CHANGELOG.md:265:- Direct mana task execution through `imp run <unit-id>`.
CHANGELOG.md:270:- Added plain-language documentation for mana as included durable task coordination.
AGENTS.md:10:- `mana` = platform
AGENTS.md:11:- `imp` = agent + default human-facing environment on mana
AGENTS.md:30:- embedding/hostability for apps built on mana;
AGENTS.md:50:- `../docs/architecture/mana-platform-target-architecture.md`
AGENTS.md:94:Put work in `imp/` when it concerns agent behavior, runtime execution, context assembly, tool registration/UX, provider/model integration, session behavior, execution policy, agent-facing interfaces, or embedding surfaces for apps built on mana.
AGENTS.md:96:Escalate to root architecture work when a change affects the mana/imp split, runtime vs graph boundaries, extension contracts, cross-app platform APIs, or Tower-wide naming/ontology.
docs/worktree-auto.md:57:`mana_core::worktree::detect_worktree(cwd)` in list output to distinguish a
docs/worktree-auto.md:221:Evidence/mana should record:
docs/worktree-auto.md:287:## Mana/evidence refs
docs/worktree-auto.md:289:Mana should store durable refs, not huge diffs:
docs/worktree-auto.md:337:- 394.9.7: trace/evidence/mana metadata refs
docs/run-evidence.md:3:Status: nightly slice for mana epic 432
docs/run-evidence.md:43:The schema is intentionally compact for nightly. Workflow-branch work can extend it with richer policy, verification, diff, and mana references.
docs/runtime-event-state-api.md:71:- `mana_updated`
docs/runtime-event-state-api.md:116:    pub mana_refs: Vec<RuntimeManaRef>,
docs/runtime-event-state-api.md:133:- mana refs
docs/runtime-event-state-api.md:154:mana refs, warnings/errors, timing/recovery status, and unknown future events.
docs/mana-next-storage-strategy.md:1:# mana-next Storage and Artifact Reference Strategy
docs/mana-next-storage-strategy.md:4:Parent: mana `394.3` / child `394.3.2`
docs/mana-next-storage-strategy.md:10:1. Keep existing mana units as markdown files under `~/.mana` as the source of truth for workflow/task/decision/note compatibility.
docs/mana-next-storage-strategy.md:12:3. Store bulky run artifacts in imp run directories, not mana files.
docs/mana-next-storage-strategy.md:15:This preserves current mana behavior while giving imp-next stable references for evidence, verification, workflow contracts, and child runs.
docs/mana-next-storage-strategy.md:19:Current mana uses file-backed markdown units such as:
docs/mana-next-storage-strategy.md:22:~/.mana/394.3.1-specify-mana-next-workflow-ledger-schema-and-compa.md
docs/mana-next-storage-strategy.md:23:~/.mana/394.7.8-accept-user-and-mana-provided-verification-gates.md
docs/mana-next-storage-strategy.md:28:Therefore v1 mana-next must not require:
docs/mana-next-storage-strategy.md:31:- moving units out of `~/.mana`
docs/mana-next-storage-strategy.md:37:### Layer 1: mana unit markdown
docs/mana-next-storage-strategy.md:48:- notes/decisions in current mana format
docs/mana-next-storage-strategy.md:68:This can be added as optional frontmatter later. Old mana ignores unknown fields.
docs/mana-next-storage-strategy.md:77:~/.mana/ledger/
docs/mana-next-storage-strategy.md:158:- Do not inline large outputs in mana records.
docs/mana-next-storage-strategy.md:164:For a workflow backed by an existing mana unit:
docs/mana-next-storage-strategy.md:167:~/.mana/394.3-streamline-mana-into-workflow-and-evidence-ledger.md
docs/mana-next-storage-strategy.md:172:For a workflow not explicitly backed by mana:
docs/mana-next-storage-strategy.md:179:| Run type | mana record? | artifacts? |
docs/mana-next-storage-strategy.md:183:| mana task run | yes, existing unit | yes |
docs/mana-next-storage-strategy.md:209:The command may also remain in the current mana `verify` field for compatibility.
docs/mana-next-storage-strategy.md:215:Do not write large evidence content into `.mana/*.md` frontmatter.
docs/mana-next-storage-strategy.md:237:- `mana list`
docs/mana-next-storage-strategy.md:238:- `mana show`
docs/mana-next-storage-strategy.md:239:- `mana create`
docs/mana-next-storage-strategy.md:240:- `mana update`
docs/mana-next-storage-strategy.md:241:- `mana verify`
docs/mana-next-storage-strategy.md:242:- `mana close`
docs/mana-next-storage-strategy.md:243:- `mana notes_append`
docs/mana-next-storage-strategy.md:244:- `mana decision_*`
docs/mana-next-storage-strategy.md:245:- `mana dep_*`
docs/mana-next-storage-strategy.md:264:- Add optional workflow/evidence frontmatter fields only when current mana preserves unknown fields safely.
docs/mana-next-storage-strategy.md:276:- Old mana can ignore `~/.mana/ledger` sidecars.
docs/mana-next-storage-strategy.md:278:- Run artifacts can remain as historical evidence even if mana-next is disabled.
docs/mana-next-storage-strategy.md:325:- harder to inspect from `mana show`
docs/mana-next-storage-strategy.md:333:~/.mana/
docs/mana-next-storage-strategy.md:334:  394.3-streamline-mana-into-workflow-and-evidence-ledger.md
docs/workflow-first-ux.md:15:4. **Review and audit** — evidence packets, traces, mana ledger, runtime state.
docs/workflow-first-ux.md:32:still work. Routine work does not require manually creating mana tasks, choosing
docs/workflow-first-ux.md:51:evidence packets, ReferenceMonitor policy records, verification gates, and mana
docs/workflow-first-ux.md:52:ledger notes. You do not need to manage those pieces for small tasks; they are
docs/workflow-first-ux.md:67:The goal is not to turn the TUI into a project-management dashboard. The chat is
docs/workflow-first-ux.md:111:## 6. Mana workflow ledger
docs/workflow-first-ux.md:113:Mana is imp’s durable workflow ledger. It is useful when work has acceptance
docs/workflow-first-ux.md:125:Use mana when the work should survive beyond one chat turn. Ignore it for simple
docs/workflow-first-ux.md:226:### Do I need to learn mana?
docs/workflow-first-ux.md:228:No. Use mana when work is durable, multi-step, blocked, delegated, or needs
docs/eval-candidates.md:146:    "kind": "mana-unit",
docs/eval-candidates.md:148:    "path": ".mana"
docs/eval-candidates.md:210:- workflow id / mana unit id
docs/verification-gates.md:170:- mana task verify command
docs/verification-gates.md:261:5. Accept user/mana-provided gate declarations.
docs/verification-gates.md:268:from the user, mana task, workflow contract, or trusted config. Inference is a
docs/verification-gates.md:318:7. There is no explicit user/mana/workflow gate that already covers the same
docs/verification-gates.md:393:Explicit user/mana/workflow gates take precedence over inferred gates. Inference
docs/verification-gates.md:564:## Mana usage
docs/verification-gates.md:566:Mana tasks already have a verify-command concept. The workflow runtime should
docs/verification-gates.md:567:translate mana-provided verification into `VerificationGate` records with source
docs/verification-gates.md:568:`ManaTask`. A mana verify gate should behave like any other required gate:
docs/verification-gates.md:575:When closing mana work, store artifact refs or evidence summaries rather than
docs/verification-gates.md:576:inlining full logs. The durable mana record should point to the run evidence and
crates/imp-llm/src/auth.rs:14:const LEGACY_KEYRING_SERVICES: &[&str] = &["imp-cli", "impeccable", "mana"];
crates/imp-llm/src/auth.rs:262:/// Manages API keys and OAuth credentials.
crates/imp-llm/src/provider.rs:84:    ProviderManagedId,
crates/imp-llm/src/providers/openai.rs:213:            continuation: ContinuationMode::ProviderManagedId,
crates/imp-llm/src/providers/openai.rs:1464:            ContinuationMode::ProviderManagedId
crates/imp-llm/src/providers/openai_codex.rs:66:        "context_management",
crates/imp-core/src/mana_run_state.rs:7:pub struct ManaRunAgentSummary {
crates/imp-core/src/mana_run_state.rs:15:pub struct ManaRunSummary {
crates/imp-core/src/mana_run_state.rs:25:    pub agents: Vec<ManaRunAgentSummary>,
crates/imp-core/src/mana_run_state.rs:62:pub fn mana_run_summary(run_id: &str) -> Result<Option<ManaRunSummary>, String> {
crates/imp-core/src/mana_run_state.rs:71:pub fn stop_mana_run(run_id: &str) -> Result<Option<ManaRunSummary>, String> {
crates/imp-core/src/mana_run_state.rs:147:    fn into_summary(self) -> ManaRunSummary {
crates/imp-core/src/mana_run_state.rs:155:        ManaRunSummary {
crates/imp-core/src/mana_run_state.rs:180:fn load_agent_summaries() -> Result<Vec<ManaRunAgentSummary>, String> {
crates/imp-core/src/mana_run_state.rs:197:            ManaRunAgentSummary {
crates/imp-core/src/mana_run_state.rs:226:        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp").join("mana"));
crates/imp-core/src/guardrails.rs:563:            "pluginManagement {}\n",
crates/imp-core/src/mana_worker.rs:1://! Canonical single-unit mana worker runtime.
crates/imp-core/src/mana_worker.rs:3://! Provides the reusable substrate for executing one mana unit:
crates/imp-core/src/mana_worker.rs:4://! loading the unit via canonical mana-core APIs, assembling execution
crates/imp-core/src/mana_worker.rs:10://! - legacy `mana run` compatibility flows — transitional dispatch into imp workers
crates/imp-core/src/mana_worker.rs:11://! - imp's native mana tool — the first-class orchestration UX
crates/imp-core/src/mana_worker.rs:16://! imp native mana tool        = first-class orchestration UX
crates/imp-core/src/mana_worker.rs:18://! legacy mana run compatibility = transitional parallel dispatch into imp workers
crates/imp-core/src/mana_worker.rs:26:use mana_core::api;
crates/imp-core/src/mana_worker.rs:27:use mana_core::ops::close::{CloseOpts, CloseOutcome, VerifyFailureResult};
crates/imp-core/src/mana_worker.rs:28:use mana_core::ops::verify as mana_verify;
crates/imp-core/src/mana_worker.rs:32:use crate::mana_prompt_context;
crates/imp-core/src/mana_worker.rs:42:// until mana also needs a versioned shared protocol crate.
crates/imp-core/src/mana_worker.rs:49:/// Load a worker assignment from a mana unit using canonical mana-core APIs.
crates/imp-core/src/mana_worker.rs:51:/// This replaces the ad hoc markdown-scanning `load_mana_unit()` that lived
crates/imp-core/src/mana_worker.rs:52:/// in imp-cli. It uses `mana_core::api::get_unit()` for canonical resolution
crates/imp-core/src/mana_worker.rs:53:/// and `mana_core::discovery::find_mana_dir()` for `.mana/` discovery.
crates/imp-core/src/mana_worker.rs:58:    load_assignment_with_mana_dir(cwd, unit_id, None)
crates/imp-core/src/mana_worker.rs:61:/// Load a worker assignment with an explicit mana dir override.
crates/imp-core/src/mana_worker.rs:62:pub fn load_assignment_with_mana_dir(
crates/imp-core/src/mana_worker.rs:65:    mana_dir_override: Option<&Path>,
crates/imp-core/src/mana_worker.rs:67:    let mana_dir = match mana_dir_override {
crates/imp-core/src/mana_worker.rs:69:        None => mana_core::discovery::find_mana_dir(cwd).map_err(|e| {
crates/imp-core/src/mana_worker.rs:71:                "Could not find .mana directory while walking up from {}: {e}",
crates/imp-core/src/mana_worker.rs:77:    let workspace_root = mana_dir
crates/imp-core/src/mana_worker.rs:82:    let unit = api::get_unit(&mana_dir, unit_id)
crates/imp-core/src/mana_worker.rs:83:        .map_err(|e| format!("Failed to load mana unit {unit_id}: {e}"))?;
crates/imp-core/src/mana_worker.rs:86:    // after the frontmatter, which mana-core merges into unit.description).
crates/imp-core/src/mana_worker.rs:87:    // mana-core's Unit already handles frontmatter+body merging in from_file(),
crates/imp-core/src/mana_worker.rs:92:    // after the frontmatter that mana-core stores separately.
crates/imp-core/src/mana_worker.rs:93:    let unit_path = mana_core::discovery::find_unit_file(&mana_dir, unit_id).ok();
crates/imp-core/src/mana_worker.rs:123:    let files: Vec<String> = Vec::new(); // Unit doesn't have a separate `files` field in mana-core
crates/imp-core/src/mana_worker.rs:133:            mana_core::config::Config::load_with_extends(&mana_dir)
crates/imp-core/src/mana_worker.rs:213:        let mana_dir = assignment.workspace_root.join(".mana");
crates/imp-core/src/mana_worker.rs:214:        match api::load_index(&mana_dir) {
crates/imp-core/src/mana_worker.rs:289:    pub mana_dir_override: Option<PathBuf>,
crates/imp-core/src/mana_worker.rs:334:        .mana_dir_override
crates/imp-core/src/mana_worker.rs:336:        .or_else(|| mana_prompt_context::nearest_mana_dir(&options.cwd))
crates/imp-core/src/mana_worker.rs:337:        .map(|mana_dir| {
crates/imp-core/src/mana_worker.rs:338:            mana_prompt_context::load_task_prompt_context(&mana_dir, &task_context.context_paths)
crates/imp-core/src/mana_worker.rs:398:        .session_manager()
crates/imp-core/src/mana_worker.rs:404:        .session_manager()
crates/imp-core/src/mana_worker.rs:410:    let batch_verify = defer_verify || std::env::var("MANA_BATCH_VERIFY").is_ok();
crates/imp-core/src/mana_worker.rs:512:    let mana_dir = assignment.workspace_root.join(".mana");
crates/imp-core/src/mana_worker.rs:514:        &mana_dir,
crates/imp-core/src/mana_worker.rs:690:    let mana_dir = cwd.join(".mana");
crates/imp-core/src/mana_worker.rs:691:    let timeout_secs = if mana_dir.exists() {
crates/imp-core/src/mana_worker.rs:692:        match api::get_unit(&mana_dir, unit_id) {
crates/imp-core/src/mana_worker.rs:694:                let config = mana_core::config::Config::load_with_extends(&mana_dir).ok();
crates/imp-core/src/mana_worker.rs:704:        mana_verify::run_verify_command(&verify_cmd, &working_dir, timeout_secs)
crates/imp-core/src/mana_worker.rs:794:        "# Mana worker assignment\n\nUnit: {}\nTitle: {}\nWorkspace: {}",
crates/imp-core/src/mana_worker.rs:882:        "Stay inside this unit's scope. Update mana notes with discoveries or blockers. Run the verify command or equivalent focused checks before claiming completion. If the stored verify is stale/invalid, record equivalent evidence and close with force only with an explicit reason. Do not retry a failed approach unchanged.",
crates/imp-core/src/mana_worker.rs:1006:        assert!(prompt.contains("# Mana worker assignment"));
crates/imp-core/src/mana_worker.rs:1185:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/mana_worker.rs:1186:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-core/src/mana_worker.rs:1187:        let unit = mana_core::unit::Unit {
crates/imp-core/src/mana_worker.rs:1190:            ..mana_core::unit::Unit::new("11", "Slow verify")
crates/imp-core/src/mana_worker.rs:1192:        unit.to_file(mana_dir.join("11-slow-verify.md")).unwrap();
crates/imp-core/src/mana_worker.rs:1213:        let outcome = CloseOutcome::Closed(mana_core::ops::close::CloseResult {
crates/imp-core/src/mana_worker.rs:1214:            unit: mana_core::unit::Unit::new("1", "Task"),
crates/imp-core/src/mana_worker.rs:1260:        let mut unit = mana_core::unit::Unit::new("9", "Verify fail");
crates/imp-core/src/mana_worker.rs:1292:                unit: mana_core::unit::Unit::new("11", "Verify fail"),
crates/imp-core/src/lib.rs:21:#[cfg(feature = "mana-api")]
crates/imp-core/src/lib.rs:22:pub mod mana_next;
crates/imp-core/src/lib.rs:23:#[cfg(feature = "mana-api")]
crates/imp-core/src/lib.rs:24:pub mod mana_prompt_context;
crates/imp-core/src/lib.rs:25:pub mod mana_review;
crates/imp-core/src/lib.rs:26:#[cfg(feature = "mana-api")]
crates/imp-core/src/lib.rs:27:pub mod mana_run_state;
crates/imp-core/src/lib.rs:28:#[cfg(feature = "mana-api")]
crates/imp-core/src/lib.rs:29:pub mod mana_worker;
crates/imp-core/src/lib.rs:60:pub use mana_review::{ManaReviewState, ManaReviewUnitKind, ManaUnitRef, TurnManaReview};
crates/imp-core/src/lib.rs:61:#[cfg(feature = "mana-api")]
crates/imp-core/src/lib.rs:62:pub use mana_run_state::{mana_run_summary, stop_mana_run, ManaRunSummary};
crates/imp-core/src/context_prefill.rs:1://! Context prefill assembly for mana dispatch.
crates/imp-core/README.md:5:It contains the agent loop, tool registry, session persistence, context assembly, hooks, policy/mode enforcement, mana integration, and the early Rust SDK surface used by hosts that want to embed imp.
crates/imp-core/README.md:14:- mana task execution support
crates/imp-core/src/typescript_extensions/mod.rs:846:        use crate::mana_review::TurnManaReviewAccumulator;
crates/imp-core/src/typescript_extensions/mod.rs:868:            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
crates/imp-core/src/roles.rs:563:            | "mana"
crates/imp-core/src/roles.rs:656:                vec!["read".into(), "scan".into(), "edit".into(), "write".into(), "bash".into(), "git".into(), "mana".into()],
crates/imp-core/src/roles.rs:728:                vec!["read".into(), "scan".into(), "edit".into(), "write".into(), "bash".into(), "git".into(), "mana".into()],
crates/imp-core/src/sdk.rs:9://! - create and manage an [`ImpSession`]
crates/imp-core/src/sdk.rs:53:pub use crate::mana_review::{ManaReviewState, TurnManaReview};
crates/imp-core/src/mana_prompt_context.rs:4:use mana_core::{index::Index, unit::Status};
crates/imp-core/src/mana_prompt_context.rs:17:/// Session-start mana-backed prompt context owned by imp runtime assembly.
crates/imp-core/src/mana_prompt_context.rs:30:    let Some(mana_dir) = nearest_mana_dir(cwd) else {
crates/imp-core/src/mana_prompt_context.rs:34:    load_session_prompt_context_from_mana_dir(&mana_dir).unwrap_or_default()
crates/imp-core/src/mana_prompt_context.rs:37:pub fn load_task_prompt_context(mana_dir: &Path, task_paths: &[String]) -> SessionPromptContext {
crates/imp-core/src/mana_prompt_context.rs:38:    load_task_prompt_context_from_mana_dir(mana_dir, task_paths).unwrap_or_default()
crates/imp-core/src/mana_prompt_context.rs:41:pub fn nearest_mana_dir(cwd: &Path) -> Option<PathBuf> {
crates/imp-core/src/mana_prompt_context.rs:42:    mana_core::api::find_mana_dir(cwd).ok()
crates/imp-core/src/mana_prompt_context.rs:45:fn load_session_prompt_context_from_mana_dir(
crates/imp-core/src/mana_prompt_context.rs:46:    mana_dir: &Path,
crates/imp-core/src/mana_prompt_context.rs:48:    let memory = load_fast_session_memory_context(mana_dir)?;
crates/imp-core/src/mana_prompt_context.rs:56:                Provenance::mana_record(crate::trust::ManaRecordKind::Fact, "relevant-fact"),
crates/imp-core/src/mana_prompt_context.rs:64:            Provenance::mana_record(crate::trust::ManaRecordKind::Note, "project-memory-status"),
crates/imp-core/src/mana_prompt_context.rs:76:fn load_task_prompt_context_from_mana_dir(
crates/imp-core/src/mana_prompt_context.rs:77:    mana_dir: &Path,
crates/imp-core/src/mana_prompt_context.rs:80:    let memory = mana_core::api::memory_context(mana_dir).map_err(|err| err.to_string())?;
crates/imp-core/src/mana_prompt_context.rs:88:                Provenance::mana_record(crate::trust::ManaRecordKind::Fact, "task-relevant-fact"),
crates/imp-core/src/mana_prompt_context.rs:100:fn unit_from_index_entry(entry: &mana_core::index::IndexEntry) -> mana_core::unit::Unit {
crates/imp-core/src/mana_prompt_context.rs:101:    let mut unit = mana_core::unit::Unit::new(entry.id.clone(), entry.title.clone());
crates/imp-core/src/mana_prompt_context.rs:118:        mana_core::unit::UnitType::Fact => "fact",
crates/imp-core/src/mana_prompt_context.rs:119:        mana_core::unit::UnitType::Task => "task",
crates/imp-core/src/mana_prompt_context.rs:120:        mana_core::unit::UnitType::Epic => "epic",
crates/imp-core/src/mana_prompt_context.rs:124:        unit.kind = mana_core::unit::UnitType::Fact;
crates/imp-core/src/mana_prompt_context.rs:132:    mana_dir: &Path,
crates/imp-core/src/mana_prompt_context.rs:133:) -> Result<mana_core::api::MemoryContext, String> {
crates/imp-core/src/mana_prompt_context.rs:135:    let index = Index::build(mana_dir).map_err(|err| err.to_string())?;
crates/imp-core/src/mana_prompt_context.rs:136:    let archived = Index::collect_archived(mana_dir).unwrap_or_default();
crates/imp-core/src/mana_prompt_context.rs:146:            working_on.push(mana_core::api::WorkingUnit {
crates/imp-core/src/mana_prompt_context.rs:153:        if entry.kind == mana_core::unit::UnitType::Fact
crates/imp-core/src/mana_prompt_context.rs:167:            relevant_facts.push(mana_core::api::RelevantFact { unit, score: 1 });
crates/imp-core/src/mana_prompt_context.rs:172:        if entry.kind == mana_core::unit::UnitType::Fact
crates/imp-core/src/mana_prompt_context.rs:186:            relevant_facts.push(mana_core::api::RelevantFact { unit, score: 1 });
crates/imp-core/src/mana_prompt_context.rs:191:            recent_work.push(mana_core::api::RecentWork {
crates/imp-core/src/mana_prompt_context.rs:201:    Ok(mana_core::api::MemoryContext {
crates/imp-core/src/mana_prompt_context.rs:209:fn map_relevant_facts(memory: &mana_core::api::MemoryContext) -> Vec<Fact> {
crates/imp-core/src/mana_prompt_context.rs:222:    memory: &mana_core::api::MemoryContext,
crates/imp-core/src/mana_prompt_context.rs:250:fn format_project_memory_status(memory: &mana_core::api::MemoryContext) -> Option<String> {
crates/imp-core/src/mana_prompt_context.rs:276:fn format_warning_lines(memory: &mana_core::api::MemoryContext) -> Vec<String> {
crates/imp-core/src/mana_prompt_context.rs:285:fn format_working_on_lines(memory: &mana_core::api::MemoryContext) -> Vec<String> {
crates/imp-core/src/mana_prompt_context.rs:310:fn format_recent_work_lines(memory: &mana_core::api::MemoryContext) -> Vec<String> {
crates/imp-core/src/mana_prompt_context.rs:363:    use mana_core::config::Config;
crates/imp-core/src/mana_prompt_context.rs:364:    use mana_core::ops::memory_context::MemoryContext;
crates/imp-core/src/mana_prompt_context.rs:365:    use mana_core::unit::{Status, Unit};
crates/imp-core/src/mana_prompt_context.rs:368:    fn setup_mana_dir() -> (TempDir, std::path::PathBuf) {
crates/imp-core/src/mana_prompt_context.rs:370:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/mana_prompt_context.rs:371:        std::fs::create_dir(&mana_dir).unwrap();
crates/imp-core/src/mana_prompt_context.rs:377:        config.save(&mana_dir).unwrap();
crates/imp-core/src/mana_prompt_context.rs:379:        (dir, mana_dir)
crates/imp-core/src/mana_prompt_context.rs:382:    fn write_unit(mana_dir: &Path, unit: &Unit) {
crates/imp-core/src/mana_prompt_context.rs:383:        let slug = mana_core::util::title_to_slug(&unit.title);
crates/imp-core/src/mana_prompt_context.rs:384:        unit.to_file(mana_dir.join(format!("{}-{}.md", unit.id, slug)))
crates/imp-core/src/mana_prompt_context.rs:389:    fn finds_nearest_mana_dir_from_nested_cwd() {
crates/imp-core/src/mana_prompt_context.rs:390:        let (dir, mana_dir) = setup_mana_dir();
crates/imp-core/src/mana_prompt_context.rs:394:        assert_eq!(nearest_mana_dir(&nested), Some(mana_dir));
crates/imp-core/src/mana_prompt_context.rs:398:    fn missing_mana_dir_yields_empty_prompt_context() {
crates/imp-core/src/mana_prompt_context.rs:406:    fn invalid_mana_dir_load_yields_empty_prompt_context() {
crates/imp-core/src/mana_prompt_context.rs:408:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/mana_prompt_context.rs:409:        std::fs::create_dir(&mana_dir).unwrap();
crates/imp-core/src/mana_prompt_context.rs:431:                mana_core::ops::memory_context::RelevantFact {
crates/imp-core/src/mana_prompt_context.rs:435:                mana_core::ops::memory_context::RelevantFact {
crates/imp-core/src/mana_prompt_context.rs:453:    fn loads_relevant_facts_from_mana_memory_context() {
crates/imp-core/src/mana_prompt_context.rs:454:        let (_dir, mana_dir) = setup_mana_dir();
crates/imp-core/src/mana_prompt_context.rs:460:        write_unit(&mana_dir, &working);
crates/imp-core/src/mana_prompt_context.rs:463:        fact.kind = mana_core::unit::UnitType::Fact;
crates/imp-core/src/mana_prompt_context.rs:468:        write_unit(&mana_dir, &fact);
crates/imp-core/src/mana_prompt_context.rs:469:        Index::build(&mana_dir).unwrap().save(&mana_dir).unwrap();
crates/imp-core/src/mana_prompt_context.rs:471:        let context = load_session_prompt_context_from_mana_dir(&mana_dir).unwrap();
crates/imp-core/src/mana_prompt_context.rs:485:        let (_dir, mana_dir) = setup_mana_dir();
crates/imp-core/src/mana_prompt_context.rs:491:        write_unit(&mana_dir, &fact_auth);
crates/imp-core/src/mana_prompt_context.rs:497:        write_unit(&mana_dir, &fact_cache);
crates/imp-core/src/mana_prompt_context.rs:500:            &mana_dir,
crates/imp-core/src/mana_prompt_context.rs:526:            working_on: vec![mana_core::ops::memory_context::WorkingUnit {
crates/imp-core/src/mana_prompt_context.rs:532:            recent_work: vec![mana_core::ops::memory_context::RecentWork { unit: recent }],
crates/imp-core/src/mana_prompt_context.rs:555:                mana_core::ops::memory_context::RelevantFact {
crates/imp-core/src/workflow/contract.rs:17:    pub mana_unit_ref: Option<String>,
crates/imp-core/src/workflow/contract.rs:48:    pub fn mana_unit_ref(mut self, mana_unit_ref: impl Into<String>) -> Self {
crates/imp-core/src/workflow/contract.rs:49:        self.mana_unit_ref = Some(mana_unit_ref.into());
crates/imp-core/src/workflow/contract.rs:71:    pub mana_unit_ref: Option<String>,
crates/imp-core/src/workflow/contract.rs:95:            mana_unit_ref: input.mana_unit_ref,
crates/imp-core/src/workflow/contract.rs:110:    pub fn with_mana_unit_ref(mut self, mana_unit_ref: impl Into<String>) -> Self {
crates/imp-core/src/workflow/contract.rs:111:        self.mana_unit_ref = Some(mana_unit_ref.into());
crates/imp-core/src/workflow/contract.rs:131:            mana_unit_ref: None,
crates/imp-core/src/workflow/contract.rs:462:    fn implicit_workflow_contract_records_mana_unit_ref() {
crates/imp-core/src/workflow/contract.rs:464:            ImplicitWorkflowContractInput::prompt("Implement mana task").mana_unit_ref("394.2.2"),
crates/imp-core/src/workflow/contract.rs:467:        assert_eq!(contract.mana_unit_ref.as_deref(), Some("394.2.2"));
crates/imp-core/src/workflow/contract.rs:468:        assert_eq!(contract.objective, "Implement mana task");
crates/imp-core/src/system_prompt.rs:11:/// A project fact from mana-core.
crates/imp-core/src/system_prompt.rs:94:/// - Layer 4: Mana facts (skipped if empty)
crates/imp-core/src/system_prompt.rs:139:    // Layer 4: Mana facts
crates/imp-core/src/system_prompt.rs:227:    s.push_str("- Use `bash` for shell-native search, file discovery, builds, tests, scripts, and package managers; prefer `scan` when code structure or symbols matter.\n");
crates/imp-core/src/system_prompt.rs:799:            .contains("between-turn mana update before the substantive reply"));
crates/imp-core/src/system_prompt.rs:802:            .contains("include a concise mana delta summary in the response"));
crates/imp-core/src/system_prompt.rs:823:            description: "Manage imp-native workflows",
crates/imp-core/src/system_prompt.rs:834:        assert!(!result.text.contains("Use mana when durable work"));
crates/imp-core/src/system_prompt.rs:847:    fn system_prompt_no_legacy_mana_guidance_or_delegation_in_prompt() {
crates/imp-core/src/system_prompt.rs:857:            name: "mana",
crates/imp-core/src/system_prompt.rs:858:            description: "Manage mana work",
crates/imp-core/src/system_prompt.rs:864:            !result.text.contains("Mana guidance:"),
crates/imp-core/src/system_prompt.rs:865:            "mana guidance block should not appear in system prompt"
crates/imp-core/src/system_prompt.rs:868:            !result.text.contains("## Mana delegation"),
crates/imp-core/src/system_prompt.rs:1022:    fn system_prompt_does_not_add_mode_aware_mana_skill_trigger() {
crates/imp-core/src/system_prompt.rs:1025:            "mana",
crates/imp-core/src/system_prompt.rs:1026:            "Coordinate explicit work through mana",
crates/imp-core/src/system_prompt.rs:1027:            "/home/.imp/skills/mana/SKILL.md",
crates/imp-core/src/system_prompt.rs:1049:        assert!(!result.text.contains("Load `mana`"));
crates/imp-core/src/system_prompt.rs:1053:    fn system_prompt_orchestrator_does_not_add_mana_skill_trigger() {
crates/imp-core/src/system_prompt.rs:1056:            "mana",
crates/imp-core/src/system_prompt.rs:1057:            "Coordinate explicit work through mana",
crates/imp-core/src/system_prompt.rs:1058:            "/home/.imp/skills/mana/SKILL.md",
crates/imp-core/src/system_prompt.rs:1080:        assert!(!result.text.contains("Load `mana`"));
crates/imp-core/src/system_prompt.rs:1084:    fn system_prompt_worker_does_not_add_mana_basics_trigger() {
crates/imp-core/src/system_prompt.rs:1088:                "mana",
crates/imp-core/src/system_prompt.rs:1089:                "Coordinate multi-step work through mana",
crates/imp-core/src/system_prompt.rs:1090:                "/home/.imp/skills/mana/SKILL.md",
crates/imp-core/src/system_prompt.rs:1093:                "mana-basics",
crates/imp-core/src/system_prompt.rs:1094:                "Use native mana actions safely and efficiently",
crates/imp-core/src/system_prompt.rs:1095:                "/home/.imp/skills/mana-basics/SKILL.md",
crates/imp-core/src/system_prompt.rs:1118:        assert!(!result.text.contains("Load `mana-basics`"));
crates/imp-core/src/system_prompt.rs:1122:    fn system_prompt_omits_mana_trigger_without_mana_skill() {
crates/imp-core/src/system_prompt.rs:1152:    fn system_prompt_reviewer_mode_omits_mana_trigger() {
crates/imp-core/src/system_prompt.rs:1155:            "mana",
crates/imp-core/src/system_prompt.rs:1156:            "Coordinate multi-step work through mana",
crates/imp-core/src/system_prompt.rs:1157:            "/home/.imp/skills/mana/SKILL.md",
crates/imp-core/src/system_prompt.rs:1188:    // -- Layer 4: Mana facts --
crates/imp-core/Cargo.toml:15:mana-core = { workspace = true, optional = true }
crates/imp-core/Cargo.toml:16:mana = { version = "0.3.2", package = "mana-cli", optional = true }
crates/imp-core/Cargo.toml:79:mana-api = ["dep:mana-core"]
crates/imp-core/Cargo.toml:80:mana-tool = ["mana-api", "dep:mana"]
crates/imp-core/src/builder.rs:65:    /// Preloaded mana prompt context; avoids duplicate mana reads for worker mode.
crates/imp-core/src/builder.rs:189:    /// Use preloaded mana prompt context instead of loading it during build.
crates/imp-core/src/builder.rs:370:                .set_workflow_mana_skill_available(skills.iter().any(|skill| skill.name == "mana"));
crates/imp-core/src/builder.rs:371:            agent.set_workflow_mana_basics_skill_available(
crates/imp-core/src/builder.rs:372:                skills.iter().any(|skill| skill.name == "mana-basics"),
crates/imp-core/src/builder.rs:374:            agent.set_workflow_mana_delegation_skill_available(
crates/imp-core/src/builder.rs:375:                skills.iter().any(|skill| skill.name == "mana-delegation"),
crates/imp-core/src/builder.rs:386:            trace_phase("mana_prompt_context", prompt_context_started);
crates/imp-core/src/builder.rs:705:            "Project lives at /Users/asher/tower and uses root mana.",
crates/imp-core/src/builder.rs:710:            "User prefers root mana in /tower for Tower work.",
crates/imp-core/src/builder.rs:746:            "Project lives at /Users/asher/tower and uses root mana.",
crates/imp-core/src/builder.rs:772:    #[cfg(feature = "mana-api")]
crates/imp-core/src/builder.rs:773:    fn builder_injects_mana_facts_into_system_prompt_when_available() {
crates/imp-core/src/builder.rs:775:        let mana_dir = temp.path().join(".mana");
crates/imp-core/src/builder.rs:776:        std::fs::create_dir(&mana_dir).unwrap();
crates/imp-core/src/builder.rs:778:        let mana_config = mana_core::config::Config {
crates/imp-core/src/builder.rs:782:        mana_config.save(&mana_dir).unwrap();
crates/imp-core/src/builder.rs:784:        let mut working = mana_core::unit::Unit::new("1", "Implement auth flow");
crates/imp-core/src/builder.rs:785:        working.status = mana_core::unit::Status::InProgress;
crates/imp-core/src/builder.rs:788:        let working_slug = mana_core::util::title_to_slug(&working.title);
crates/imp-core/src/builder.rs:790:            .to_file(mana_dir.join(format!("1-{}.md", working_slug)))
crates/imp-core/src/builder.rs:793:        let mut fact = mana_core::unit::Unit::new("2", "Auth uses RS256 signing");
crates/imp-core/src/builder.rs:794:        fact.kind = mana_core::unit::UnitType::Fact;
crates/imp-core/src/builder.rs:799:        let fact_slug = mana_core::util::title_to_slug(&fact.title);
crates/imp-core/src/builder.rs:800:        fact.to_file(mana_dir.join(format!("2-{}.md", fact_slug)))
crates/imp-core/src/builder.rs:822:    #[cfg(feature = "mana-api")]
crates/imp-core/src/builder.rs:825:        let mana_dir = temp.path().join(".mana");
crates/imp-core/src/builder.rs:826:        std::fs::create_dir(&mana_dir).unwrap();
crates/imp-core/src/builder.rs:828:        let mana_config = mana_core::config::Config {
crates/imp-core/src/builder.rs:832:        mana_config.save(&mana_dir).unwrap();
crates/imp-core/src/builder.rs:834:        let mut working = mana_core::unit::Unit::new("1", "Implement auth flow");
crates/imp-core/src/builder.rs:835:        working.status = mana_core::unit::Status::InProgress;
crates/imp-core/src/builder.rs:837:        let working_slug = mana_core::util::title_to_slug(&working.title);
crates/imp-core/src/builder.rs:839:            .to_file(mana_dir.join(format!("1-{}.md", working_slug)))
crates/imp-core/src/builder.rs:842:        let mut recent = mana_core::unit::Unit::new("3", "Recently closed cleanup");
crates/imp-core/src/builder.rs:843:        recent.status = mana_core::unit::Status::Closed;
crates/imp-core/src/builder.rs:845:        let recent_slug = mana_core::util::title_to_slug(&recent.title);
crates/imp-core/src/builder.rs:846:        let archive_dir = mana_dir.join("archive").join("2026").join("05");
crates/imp-core/src/config.rs:16:/// Agent mode — controls which tools and mana actions the agent may use.
crates/imp-core/src/config.rs:48:// Legacy mana action lists are retained for compatibility with older code paths
crates/imp-core/src/config.rs:49:// that still route through the mana tool. New native orchestration should use
crates/imp-core/src/config.rs:51:const WORKER_MANA_ACTIONS: &[&str] = &[
crates/imp-core/src/config.rs:61:const ORCHESTRATOR_MANA_ACTIONS: &[&str] = &[
crates/imp-core/src/config.rs:89:const PLANNER_MANA_ACTIONS: &[&str] = &[
crates/imp-core/src/config.rs:104:const AUDITOR_MANA_ACTIONS: &[&str] = &[
crates/imp-core/src/config.rs:157:    /// Mana sub-actions this mode permits. An empty slice means "allow all" (Full).
crates/imp-core/src/config.rs:158:    pub fn allowed_mana_actions(&self) -> &'static [&'static str] {
crates/imp-core/src/config.rs:161:            AgentMode::Worker => WORKER_MANA_ACTIONS,
crates/imp-core/src/config.rs:162:            AgentMode::Orchestrator => ORCHESTRATOR_MANA_ACTIONS,
crates/imp-core/src/config.rs:163:            AgentMode::Planner => PLANNER_MANA_ACTIONS,
crates/imp-core/src/config.rs:164:            AgentMode::Auditor => AUDITOR_MANA_ACTIONS,
crates/imp-core/src/config.rs:168:    /// Returns true if the mode allows the named mana action.
crates/imp-core/src/config.rs:169:    pub fn allows_mana_action(&self, action: &str) -> bool {
crates/imp-core/src/config.rs:173:            _ => self.allowed_mana_actions().contains(&action),
crates/imp-core/src/config.rs:387:pub enum ManaScopePreference {
crates/imp-core/src/config.rs:394:pub struct ManaRunConfig {
crates/imp-core/src/config.rs:395:    /// Whether native mana runs should return immediately by default.
crates/imp-core/src/config.rs:399:    #[serde(default = "default_mana_run_jobs")]
crates/imp-core/src/config.rs:409:impl Default for ManaRunConfig {
crates/imp-core/src/config.rs:420:impl ManaRunConfig {
crates/imp-core/src/config.rs:430:fn default_mana_run_jobs() -> u32 {
crates/imp-core/src/config.rs:435:pub struct ManaConfig {
crates/imp-core/src/config.rs:436:    /// Default mana graph selection for native mana calls.
crates/imp-core/src/config.rs:438:    pub scope: ManaScopePreference,
crates/imp-core/src/config.rs:439:    /// Whether successful mana close operations should auto-commit mana changes.
crates/imp-core/src/config.rs:442:    /// Whether mana should close completed parent units after closing a child.
crates/imp-core/src/config.rs:448:    /// Native mana run defaults.
crates/imp-core/src/config.rs:449:    #[serde(default, skip_serializing_if = "ManaRunConfig::is_default")]
crates/imp-core/src/config.rs:450:    pub run: ManaRunConfig,
crates/imp-core/src/config.rs:453:impl Default for ManaConfig {
crates/imp-core/src/config.rs:456:            scope: ManaScopePreference::Project,
crates/imp-core/src/config.rs:460:            run: ManaRunConfig::default(),
crates/imp-core/src/config.rs:494:    /// Context management settings.
crates/imp-core/src/config.rs:510:    /// Agent mode — controls tool and mana action access.
crates/imp-core/src/config.rs:534:    /// Mana tool settings.
crates/imp-core/src/config.rs:536:    pub mana: ManaConfig,
crates/imp-core/src/config.rs:615:    /// Auto-continue on clear, visible, mana-backed next steps.
crates/imp-core/src/config.rs:999:        if other.mana != ManaConfig::default() {
crates/imp-core/src/config.rs:1000:            self.mana = other.mana;
crates/imp-core/src/config.rs:1697:        assert!(!mode.allows_tool("mana"));
crates/imp-core/src/config.rs:1726:    fn agent_mode_planner_allows_mana_create() {
crates/imp-core/src/config.rs:1728:        assert!(mode.allows_mana_action("create"));
crates/imp-core/src/config.rs:1729:        assert!(mode.allows_mana_action("status"));
crates/imp-core/src/config.rs:1730:        assert!(mode.allows_mana_action("list"));
crates/imp-core/src/config.rs:1731:        assert!(mode.allows_mana_action("show"));
crates/imp-core/src/config.rs:1733:        assert!(!mode.allows_tool("mana"));
crates/imp-core/src/config.rs:1737:    fn agent_mode_planner_blocks_mana_close_and_run() {
crates/imp-core/src/config.rs:1739:        assert!(!mode.allows_mana_action("close"));
crates/imp-core/src/config.rs:1740:        assert!(!mode.allows_mana_action("run"));
crates/imp-core/src/config.rs:1741:        assert!(mode.allows_mana_action("update"));
crates/imp-core/src/config.rs:1769:    fn agent_mode_worker_blocks_mana_create() {
crates/imp-core/src/config.rs:1771:        assert!(!mode.allows_mana_action("create"));
crates/imp-core/src/config.rs:1772:        assert!(!mode.allows_mana_action("run"));
crates/imp-core/src/config.rs:1773:        assert!(!mode.allows_mana_action("close"));
crates/imp-core/src/config.rs:1778:    fn agent_mode_worker_allows_mana_update() {
crates/imp-core/src/config.rs:1780:        assert!(mode.allows_mana_action("update"));
crates/imp-core/src/config.rs:1781:        assert!(mode.allows_mana_action("show"));
crates/imp-core/src/config.rs:1782:        assert!(mode.allows_mana_action("status"));
crates/imp-core/src/config.rs:1783:        assert!(mode.allows_mana_action("list"));
crates/imp-core/src/config.rs:1787:    fn agent_mode_reviewer_no_mana() {
crates/imp-core/src/config.rs:1789:        assert!(!mode.allows_mana_action("status"));
crates/imp-core/src/config.rs:1790:        assert!(!mode.allows_mana_action("list"));
crates/imp-core/src/config.rs:1791:        assert!(!mode.allows_mana_action("show"));
crates/imp-core/src/config.rs:1792:        assert!(!mode.allows_mana_action("create"));
crates/imp-core/src/config.rs:1793:        assert!(!mode.allows_mana_action("run"));
crates/imp-core/src/config.rs:1795:        assert!(!mode.allows_tool("mana"));
crates/imp-core/src/config.rs:1800:    fn agent_mode_auditor_mana_readonly() {
crates/imp-core/src/config.rs:1802:        assert!(mode.allows_mana_action("status"));
crates/imp-core/src/config.rs:1803:        assert!(mode.allows_mana_action("list"));
crates/imp-core/src/config.rs:1804:        assert!(mode.allows_mana_action("show"));
crates/imp-core/src/config.rs:1805:        assert!(!mode.allows_mana_action("create"));
crates/imp-core/src/config.rs:1806:        assert!(!mode.allows_mana_action("close"));
crates/imp-core/src/config.rs:1807:        assert!(!mode.allows_mana_action("run"));
crates/imp-core/src/config.rs:1808:        assert!(!mode.allows_mana_action("update"));
crates/imp-core/benches/core_hot_paths.rs:17:use imp_core::session::{sanitize_messages, SessionEntry, SessionManager};
crates/imp-core/benches/core_hot_paths.rs:212:    let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/benches/core_hot_paths.rs:227:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/benches/core_hot_paths.rs:311:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/benches/core_hot_paths.rs:326:        let mgr = SessionManager::open(&path).unwrap();
crates/imp-core/benches/core_hot_paths.rs:334:        let listed = SessionManager::list(&tmp.path().join("sessions"))
crates/imp-core/src/workflow/bootstrap.rs:131:    pub mana_root_id: String,
crates/imp-core/src/workflow/bootstrap.rs:183:#[cfg(feature = "mana-api")]
crates/imp-core/src/workflow/bootstrap.rs:185:    mana_dir: &Path,
crates/imp-core/src/workflow/bootstrap.rs:192:        let result = mana_core::api::create_unit(
crates/imp-core/src/workflow/bootstrap.rs:193:            mana_dir,
crates/imp-core/src/workflow/bootstrap.rs:194:            mana_core::ops::create::CreateParams {
crates/imp-core/src/workflow/bootstrap.rs:213:                kind: Some(mana_core::unit::UnitType::Task),
crates/imp-core/src/workflow/bootstrap.rs:237:#[cfg(feature = "mana-api")]
crates/imp-core/src/workflow/bootstrap.rs:238:pub fn create_native_mana_root(
crates/imp-core/src/workflow/bootstrap.rs:239:    mana_dir: &Path,
crates/imp-core/src/workflow/bootstrap.rs:242:    let result = mana_core::api::create_unit(
crates/imp-core/src/workflow/bootstrap.rs:243:        mana_dir,
crates/imp-core/src/workflow/bootstrap.rs:244:        mana_core::ops::create::CreateParams {
crates/imp-core/src/workflow/bootstrap.rs:265:            kind: Some(mana_core::unit::UnitType::Task),
crates/imp-core/src/workflow/bootstrap.rs:274:        mana_root_id: result.unit.id,
crates/imp-core/src/workflow/bootstrap.rs:338:        let mut controller = WorkflowRunController::new().with_mana_root_id("28.1");
crates/imp-core/src/imp_session.rs:47:use crate::session::{SessionCheckpointRecord, SessionEntry, SessionManager};
crates/imp-core/src/imp_session.rs:237:/// Manages the lifecycle of a single agent: config resolution, model
crates/imp-core/src/imp_session.rs:242:    session_mgr: SessionManager,
crates/imp-core/src/imp_session.rs:417:            SessionChoice::New => SessionManager::new(&cwd, &session_dir)?,
crates/imp-core/src/imp_session.rs:418:            SessionChoice::InMemory => SessionManager::in_memory(),
crates/imp-core/src/imp_session.rs:419:            SessionChoice::Continue => SessionManager::continue_recent(&cwd, &session_dir)?
crates/imp-core/src/imp_session.rs:420:                .unwrap_or_else(|| SessionManager::new(&cwd, &session_dir).unwrap()),
crates/imp-core/src/imp_session.rs:421:            SessionChoice::Open(ref path) => SessionManager::open(path)?,
crates/imp-core/src/imp_session.rs:690:    /// The session manager (tree, entries, persistence).
crates/imp-core/src/imp_session.rs:691:    pub fn session_manager(&self) -> &SessionManager {
crates/imp-core/src/imp_session.rs:695:    /// Mutable access to the session manager.
crates/imp-core/src/imp_session.rs:696:    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
crates/imp-core/src/imp_session.rs:895:    session_mgr: &mut SessionManager,
crates/imp-core/src/imp_session.rs:1384:            session_mgr: SessionManager::in_memory(),
crates/imp-core/src/imp_session.rs:1441:            session_mgr: SessionManager::in_memory(),
crates/imp-core/src/imp_session.rs:1502:        let mut session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/imp_session.rs:1576:        let mut session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/imp_session.rs:1672:        let session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/imp_session.rs:1704:            mana_review: crate::mana_review::TurnManaReview::no_change(2),
crates/imp-core/src/imp_session.rs:1731:        let session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/imp_session.rs:1753:            mana_review: crate::mana_review::TurnManaReview::no_change(0),
crates/imp-core/src/imp_session.rs:1766:        let session_mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/usage.rs:5:use crate::session::{SessionEntry, SessionManager};
crates/imp-core/src/usage.rs:291:    session: &SessionManager,
crates/imp-core/src/usage.rs:308:    session: &SessionManager,
crates/imp-core/src/usage.rs:399:/// Read usage rows from a session manager, attaching the session path when known.
crates/imp-core/src/usage.rs:400:pub fn usage_records_from_session(session: &SessionManager) -> Vec<SessionUsageRecord> {
crates/imp-core/src/mana_next/mod.rs:1://! mana-next workflow ledger compatibility types.
crates/imp-core/src/mana_next/mod.rs:3://! These are additive data structures used by imp-next to view current mana
crates/imp-core/src/mana_next/mod.rs:4://! units as workflow/evidence ledger records without changing mana storage.
crates/imp-core/src/compaction.rs:7:use crate::session::{sanitize_messages, SessionEntry, SessionManager};
crates/imp-core/src/compaction.rs:27:/// remote/provider-managed compaction or context-editing APIs.
crates/imp-core/src/compaction.rs:395:    session: &mut SessionManager,
crates/imp-core/src/compaction.rs:488:    session: &mut SessionManager,
crates/imp-core/src/compaction.rs:512:    use crate::session::SessionManager;
crates/imp-core/src/compaction.rs:715:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/compaction.rs:775:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/compaction.rs:788:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/compaction.rs:812:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session_index.rs:7:use crate::session::{SessionEntry, SessionManager};
crates/imp-core/src/session_index.rs:55:    pub fn index_session(&self, session: &SessionManager) -> Result<()> {
crates/imp-core/src/session_index.rs:205:    use crate::session::SessionManager;
crates/imp-core/src/session_index.rs:208:    fn make_session_with_messages(dir: &std::path::Path, texts: &[&str]) -> SessionManager {
crates/imp-core/src/session_index.rs:211:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/mana_next/ledger.rs:636:                mana_unit_ref: Some("394.13".into()),
crates/imp-core/src/mana_next/ledger.rs:656:    fn mana_child_workflow_ledger_updates_parent_and_child_refs() {
crates/imp-core/src/mana_next/ledger.rs:683:    fn mana_child_workflow_required_blocker_updates_parent_status() {
crates/imp-core/src/mana_next/ledger.rs:701:    fn mana_child_workflow_task_and_evidence_records_are_derived() {
crates/imp-core/src/mana_next/ledger.rs:730:    fn mana_workflow_ledger_adapter_builds_record_from_contract() {
crates/imp-core/src/mana_next/ledger.rs:733:            .with_mana_unit_ref("394.3.5");
crates/imp-core/src/mana_next/ledger.rs:767:    fn mana_workflow_ledger_adapter_updates_record_without_duplicate_refs() {
crates/imp-core/src/mana_next/ledger.rs:799:    fn mana_workflow_ledger_round_trips_workflow_record() {
crates/imp-core/src/mana_next/ledger.rs:802:            title: "Streamline mana".into(),
crates/imp-core/src/mana_next/ledger.rs:826:    fn mana_workflow_ledger_represents_task_decision_verification_evidence_and_note() {
crates/imp-core/src/mana_next/ledger.rs:837:                "cargo test -p imp-core mana_workflow_ledger",
crates/imp-core/src/contracts.rs:3://! These DTOs define the boundary between imp's mana worker runtime,
crates/imp-core/src/contracts.rs:52:    /// Workspace root (parent of .mana/).
crates/imp-core/src/contracts.rs:160:/// Minimal verifier result lineage shared across imp and mana.
crates/imp-core/src/contracts.rs:175:/// Reference-first evidence bundle shape; storage stays owned by mana.
crates/imp-core/src/contracts.rs:219:                locator: "mana://units/9/artifacts/verify-output".to_string(),
crates/imp-core/src/hooks.rs:153:/// Manages and executes hooks.
crates/imp-core/src/agent/turn_assessment.rs:27:pub(super) struct ManaEvidence {
crates/imp-core/src/agent/turn_assessment.rs:46:    pub mana: NextActionManaEvidence,
crates/imp-core/src/agent/turn_assessment.rs:64:pub struct NextActionManaEvidence {
crates/imp-core/src/agent/turn_assessment.rs:89:    pub(super) mana: ManaEvidence,
crates/imp-core/src/agent/turn_assessment.rs:112:        if let Some(reason) = self.mana.stop_reason {
crates/imp-core/src/agent/turn_assessment.rs:173:            mana: NextActionManaEvidence {
crates/imp-core/src/agent/turn_assessment.rs:175:                    .mana
crates/imp-core/src/tools/mana.rs:7:use mana::commands::agents::{agents_file_path, load_agents};
crates/imp-core/src/tools/mana.rs:8:use mana::commands::logs::find_all_logs;
crates/imp-core/src/tools/mana.rs:9:use mana::commands::next::ScoredUnit;
crates/imp-core/src/tools/mana.rs:10:use mana::commands::run::{NativeRunParams, RunSummary, RunTarget, RunUnitStatus, RunView};
crates/imp-core/src/tools/mana.rs:11:use mana::stream::{self, StreamEvent};
crates/imp-core/src/tools/mana.rs:12:use mana_core::ops::claim::ClaimParams;
crates/imp-core/src/tools/mana.rs:13:use mana_core::ops::run as mana_run_core;
crates/imp-core/src/tools/mana.rs:14:use mana_core::unit::{OnFailAction, UnitType};
crates/imp-core/src/tools/mana.rs:20:use crate::mana_worker::{self, WorkerRunOptions, WorkerStatus};
crates/imp-core/src/tools/mana.rs:31:pub fn mana_executable_available() -> bool {
crates/imp-core/src/tools/mana.rs:33:        .map(|paths| env::split_paths(&paths).any(|dir| dir.join("mana").is_file()))
crates/imp-core/src/tools/mana.rs:37:fn find_mana_dir(cwd: &Path) -> std::result::Result<std::path::PathBuf, String> {
crates/imp-core/src/tools/mana.rs:38:    mana_core::discovery::find_mana_dir(cwd).map_err(|e| e.to_string())
crates/imp-core/src/tools/mana.rs:41:fn resolve_mana_dir(
crates/imp-core/src/tools/mana.rs:50:        .or_else(|| params.get("mana_scope").and_then(|v| v.as_str()))
crates/imp-core/src/tools/mana.rs:56:        .or_else(|| params.get("mana_dir").and_then(|v| v.as_str()))
crates/imp-core/src/tools/mana.rs:65:            if resolved.file_name().and_then(|name| name.to_str()) == Some(".mana") {
crates/imp-core/src/tools/mana.rs:68:                resolved.join(".mana")
crates/imp-core/src/tools/mana.rs:74:        "auto" | "project" => find_mana_dir(cwd),
crates/imp-core/src/tools/mana.rs:75:        "root" => mana_core::discovery::find_outermost_mana_dir(cwd).map_err(|e| e.to_string()),
crates/imp-core/src/tools/mana.rs:77:            "Unknown mana scope '{other}'. Use auto, project, or root."
crates/imp-core/src/tools/mana.rs:353:struct ManaRunStore {
crates/imp-core/src/tools/mana.rs:358:impl ManaRunStore {
crates/imp-core/src/tools/mana.rs:509:        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp").join("mana"));
crates/imp-core/src/tools/mana.rs:554:            "Mana run started: {total_units} jobs across {total_rounds} waves"
crates/imp-core/src/tools/mana.rs:628:            "Mana run finished: {total_closed} done · {total_failed} failed · {duration_secs}s"
crates/imp-core/src/tools/mana.rs:674:    store: &std::sync::Mutex<ManaRunStore>,
crates/imp-core/src/tools/mana.rs:684:fn finish_run_in_store(store: &std::sync::Mutex<ManaRunStore>, run_id: &str, view: &RunView) {
crates/imp-core/src/tools/mana.rs:691:fn fail_run_in_store(store: &std::sync::Mutex<ManaRunStore>, run_id: &str, error: String) {
crates/imp-core/src/tools/mana.rs:698:fn mana_widget_lines(summary: impl Into<String>, detail: Option<String>) -> WidgetContent {
crates/imp-core/src/tools/mana.rs:706:fn mana_run_widget_lines(
crates/imp-core/src/tools/mana.rs:712:        return mana_widget_lines(
crates/imp-core/src/tools/mana.rs:713:            format!("mana {run_id}: starting"),
crates/imp-core/src/tools/mana.rs:719:        "mana {}: {} · {}/{} done · {} failed",
crates/imp-core/src/tools/mana.rs:740:    mana_widget_lines(summary, Some(detail.join(" · ")))
crates/imp-core/src/tools/mana.rs:743:async fn set_mana_delta_widget(
crates/imp-core/src/tools/mana.rs:749:        .set_widget("mana", Some(mana_widget_lines(summary, detail)))
crates/imp-core/src/tools/mana.rs:770:                "mana run targets must contain at least one string id".into(),
crates/imp-core/src/tools/mana.rs:802:            "Native mana orchestration finished for {scope}: {} done, {} failed, {} candidate complete / awaiting verify.",
crates/imp-core/src/tools/mana.rs:809:            "Native mana orchestration finished for {scope}: {} done, {} candidate complete / awaiting verify.",
crates/imp-core/src/tools/mana.rs:814:            "Native mana orchestration finished for {scope}: {} done, 0 failed.",
crates/imp-core/src/tools/mana.rs:823:            " Orchestration ran through mana; worker runtime: {agent} · {model}."
crates/imp-core/src/tools/mana.rs:827:    summary.push_str(" Inspect with mana(action=\"run_state\") or mana(action=\"evaluate\").");
crates/imp-core/src/tools/mana.rs:872:        return mana_core::ops::create::parse_on_fail(raw)
crates/imp-core/src/tools/mana.rs:936:fn mana_close_force_reason_error(id: &str) -> ToolOutput {
crates/imp-core/src/tools/mana.rs:940:                "mana close {id} with force=true requires reason with equivalent verify evidence"
crates/imp-core/src/tools/mana.rs:954:                "reason": "Equivalent verify passed: cargo test -p imp-core mana -- --nocapture; commit abc123"
crates/imp-core/src/tools/mana.rs:961:fn mana_close_error_output(id: &str, error: String) -> ToolOutput {
crates/imp-core/src/tools/mana.rs:998:fn mana_validation_error(
crates/imp-core/src/tools/mana.rs:1007:            text: format!("mana {action} validation failed: {hint}"),
crates/imp-core/src/tools/mana.rs:1031:fn validate_mana_action(action: &str, params: &serde_json::Value) -> Option<ToolOutput> {
crates/imp-core/src/tools/mana.rs:1033:        Some(mana_validation_error(
crates/imp-core/src/tools/mana.rs:1045:                return Some(mana_validation_error(
crates/imp-core/src/tools/mana.rs:1049:                    "Use path for project/.mana location; use paths to attach relevant files to units.",
crates/imp-core/src/tools/mana.rs:1190:                    "run_state/evaluate requires run_id from mana action=run.",
crates/imp-core/src/tools/mana.rs:1197:                return Some(mana_validation_error(
crates/imp-core/src/tools/mana.rs:1230:    let text = format!("Started native mana orchestration for {scope} as {run_id}.");
crates/imp-core/src/tools/mana.rs:1250:                "state": format!("mana(action=\\\"run_state\\\", run_id=\\\"{run_id}\\\")"),
crates/imp-core/src/tools/mana.rs:1251:                "logs": format!("mana(action=\\\"logs\\\", run_id=\\\"{run_id}\\\")")
crates/imp-core/src/tools/mana.rs:1261:) -> mana::commands::run::RunRuntimeInfo {
crates/imp-core/src/tools/mana.rs:1262:    mana::commands::run::RunRuntimeInfo {
crates/imp-core/src/tools/mana.rs:1275:fn core_target_from_native(target: &RunTarget) -> mana_core::api::RunTarget {
crates/imp-core/src/tools/mana.rs:1277:        RunTarget::AllReady => mana_core::api::RunTarget::AllReady,
crates/imp-core/src/tools/mana.rs:1278:        RunTarget::Unit(id) => mana_core::api::RunTarget::Unit(id.clone()),
crates/imp-core/src/tools/mana.rs:1279:        RunTarget::Explicit(ids) => mana_core::api::RunTarget::Explicit(ids.clone()),
crates/imp-core/src/tools/mana.rs:1286:            "native mana run does not yet support multiple explicit targets; run one target at a time"
crates/imp-core/src/tools/mana.rs:1293:fn mana_workspace_root(mana_dir: &Path, fallback: &Path) -> PathBuf {
crates/imp-core/src/tools/mana.rs:1294:    mana_dir
crates/imp-core/src/tools/mana.rs:1307:    unit: &mana_run_core::ReadyUnit,
crates/imp-core/src/tools/mana.rs:1310:    mana_dir: PathBuf,
crates/imp-core/src/tools/mana.rs:1327:        mana_dir_override: Some(mana_dir),
crates/imp-core/src/tools/mana.rs:1334:    unit: mana_run_core::ReadyUnit,
crates/imp-core/src/tools/mana.rs:1336:    mana_dir: PathBuf,
crates/imp-core/src/tools/mana.rs:1342:    let assignment = match mana_worker::load_assignment_with_mana_dir(
crates/imp-core/src/tools/mana.rs:1345:        Some(&mana_dir),
crates/imp-core/src/tools/mana.rs:1378:    let options = worker_options_for_native_unit(&unit, &run_args, workspace_root, mana_dir, &ctx);
crates/imp-core/src/tools/mana.rs:1379:    let outcome = mana_worker::run_worker_assignment(assignment, options).await;
crates/imp-core/src/tools/mana.rs:1431:    mana_dir: PathBuf,
crates/imp-core/src/tools/mana.rs:1434:    run_store: Arc<std::sync::Mutex<ManaRunStore>>,
crates/imp-core/src/tools/mana.rs:1442:    let plan = mana_run_core::compute_run_plan(&mana_dir, &core_target, run_args.dry_run)
crates/imp-core/src/tools/mana.rs:1485:    let workspace_root = mana_workspace_root(&mana_dir, &ctx.cwd);
crates/imp-core/src/tools/mana.rs:1507:                return Err("native mana run cancelled".to_string());
crates/imp-core/src/tools/mana.rs:1528:                    mana_dir.clone(),
crates/imp-core/src/tools/mana.rs:1537:                    .map_err(|error| format!("native mana worker task failed: {error}"))?;
crates/imp-core/src/tools/mana.rs:1600:    mana_dir: std::path::PathBuf,
crates/imp-core/src/tools/mana.rs:1603:    run_store: Arc<std::sync::Mutex<ManaRunStore>>,
crates/imp-core/src/tools/mana.rs:1612:            "mana",
crates/imp-core/src/tools/mana.rs:1613:            Some(&format!("mana orchestration: running {scope}")),
crates/imp-core/src/tools/mana.rs:1616:        ui.set_widget("mana", Some(mana_run_widget_lines(&run_id, &scope, None)))
crates/imp-core/src/tools/mana.rs:1631:                        "mana {}: {} · {}/{} done · {} failed",
crates/imp-core/src/tools/mana.rs:1638:                    ui_for_progress.set_status("mana", Some(&status)).await;
crates/imp-core/src/tools/mana.rs:1641:                            "mana",
crates/imp-core/src/tools/mana.rs:1642:                            Some(mana_run_widget_lines(
crates/imp-core/src/tools/mana.rs:1657:                            "mana",
crates/imp-core/src/tools/mana.rs:1658:                            Some(&format!("mana {run_id_for_progress}: waiting for events")),
crates/imp-core/src/tools/mana.rs:1666:            mana_dir,
crates/imp-core/src/tools/mana.rs:1679:                    "mana orchestration: {scope} finished · {} done · {} failed",
crates/imp-core/src/tools/mana.rs:1689:                            "native mana tool → mana orchestration → {agent} workers · {scope} · {model}"
crates/imp-core/src/tools/mana.rs:1693:                ui.set_status("mana", Some(&summary)).await;
crates/imp-core/src/tools/mana.rs:1695:                    "mana",
crates/imp-core/src/tools/mana.rs:1696:                    Some(mana_widget_lines(summary.clone(), Some(runtime_detail))),
crates/imp-core/src/tools/mana.rs:1710:                    ui_clear.set_widget("mana", None).await;
crates/imp-core/src/tools/mana.rs:1711:                    ui_clear.set_status("mana", None).await;
crates/imp-core/src/tools/mana.rs:1715:                let message = format!("mana orchestration: {scope} failed: {err}");
crates/imp-core/src/tools/mana.rs:1717:                ui.set_status("mana", Some(&message)).await;
crates/imp-core/src/tools/mana.rs:1718:                ui.set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
crates/imp-core/src/tools/mana.rs:1724:                            "Native mana orchestration failed for {scope}: {err}. Inspect with mana(action=\"run_state\") or mana(action=\"logs\", run_id=\"{run_id}\")."
crates/imp-core/src/tools/mana.rs:1804:            "Invalid mana guide topic '{topic}'. Use overview, task, epic, decision, notes, verify, orchestrate, or worker_context."
crates/imp-core/src/tools/mana.rs:1813:                "Invalid mana template topic '{topic}'. Use overview, task, epic, decision, notes, verify, orchestrate, or worker_context."
crates/imp-core/src/tools/mana.rs:1824:            "Invalid mana template kind '{kind}'. Use epic, task, or fact."
crates/imp-core/src/tools/mana.rs:1832:            "Use mana when work needs durable scope, verification, dependencies, retries, or handoff; use direct edits for small one-pass changes.",
crates/imp-core/src/tools/mana.rs:1924:fn mana_guide_output(topic: GuideTopic) -> ToolOutput {
crates/imp-core/src/tools/mana.rs:1928:            "mana guide: {}\n{}\n- {}\nnext: {}",
crates/imp-core/src/tools/mana.rs:1973:fn mana_template_output(kind: TemplateKind, topic: Option<GuideTopic>) -> ToolOutput {
crates/imp-core/src/tools/mana.rs:1983:            "mana template: {} ({})\n{}\n{}",
crates/imp-core/src/tools/mana.rs:2009:    run_store: &Arc<std::sync::Mutex<ManaRunStore>>,
crates/imp-core/src/tools/mana.rs:2019:    mana_dir: &Path,
crates/imp-core/src/tools/mana.rs:2024:        let Ok(result) = mana_core::ops::show::get(mana_dir, id) else {
crates/imp-core/src/tools/mana.rs:2033:            mana_core::unit::types::AttemptOutcome::Failed
crates/imp-core/src/tools/mana.rs:2059:            "Inspect failed unit with mana action=show id=<unit>",
crates/imp-core/src/tools/mana.rs:2109:        next_actions.push("Inspect failed units with mana action=show id=<unit>".to_string());
crates/imp-core/src/tools/mana.rs:2149:        "Native mana orchestration {}: {} · {}",
crates/imp-core/src/tools/mana.rs:2199:                "Native mana orchestration run {} is still running for {}.",
crates/imp-core/src/tools/mana.rs:2204:            "Native mana orchestration run {} failed for {}.",
crates/imp-core/src/tools/mana.rs:2208:            "Native mana orchestration run {} finished with {} failed unit(s).",
crates/imp-core/src/tools/mana.rs:2212:            "Native mana orchestration run {} finished with {} unit(s) candidate complete / awaiting verify.",
crates/imp-core/src/tools/mana.rs:2216:            "Native mana orchestration run {} finished successfully: {} unit(s) done.",
crates/imp-core/src/tools/mana.rs:2253:fn claim_output(result: &mana_core::ops::claim::ClaimResult) -> ToolOutput {
crates/imp-core/src/tools/mana.rs:2275:fn release_output(result: &mana_core::ops::claim::ReleaseResult) -> ToolOutput {
crates/imp-core/src/tools/mana.rs:2316:    entries: &[mana_core::index::IndexEntry],
crates/imp-core/src/tools/mana.rs:2345:        "mana list: showing {shown} of {} units. Prefer `show` + `update`/`notes_append` on an existing relevant unit before `create`.",
crates/imp-core/src/tools/mana.rs:2387:        return "No ready units. Create one with: mana create \"task\" --verify \"cmd\""
crates/imp-core/src/tools/mana.rs:2414:fn tree_lines(node: &mana_core::api::TreeNode, indent: usize, out: &mut Vec<String>) {
crates/imp-core/src/tools/mana.rs:2426:pub struct ManaTool {
crates/imp-core/src/tools/mana.rs:2427:    run_store: Arc<std::sync::Mutex<ManaRunStore>>,
crates/imp-core/src/tools/mana.rs:2430:impl Default for ManaTool {
crates/imp-core/src/tools/mana.rs:2433:            run_store: Arc::new(std::sync::Mutex::new(ManaRunStore::load_persisted())),
crates/imp-core/src/tools/mana.rs:2439:impl Tool for ManaTool {
crates/imp-core/src/tools/mana.rs:2441:        "mana"
crates/imp-core/src/tools/mana.rs:2444:        "Mana"
crates/imp-core/src/tools/mana.rs:2447:        "Mana work graph operations. Use guide/template for detailed usage."
crates/imp-core/src/tools/mana.rs:2471:            json!({ "type": "string", "description": "Project/.mana path" }),
crates/imp-core/src/tools/mana.rs:2616:            "guide" => return Ok(mana_guide_output(parse_guide_topic(&params)?)),
crates/imp-core/src/tools/mana.rs:2618:                return Ok(mana_template_output(
crates/imp-core/src/tools/mana.rs:2626:        if !mode.allows_mana_action(action) {
crates/imp-core/src/tools/mana.rs:2629:                "Mana action '{action}' is not available in {mode_name} mode"
crates/imp-core/src/tools/mana.rs:2633:        if let Some(validation_error) = validate_mana_action(action, &params) {
crates/imp-core/src/tools/mana.rs:2637:        let mana_dir = resolve_mana_dir(&ctx.cwd, &params).map_err(crate::error::Error::Tool)?;
crates/imp-core/src/tools/mana.rs:2640:            "status" => match mana_core::api::get_status(&mana_dir) {
crates/imp-core/src/tools/mana.rs:2645:                let list_params = mana_core::ops::list::ListParams {
crates/imp-core/src/tools/mana.rs:2654:                match mana_core::api::list_units(&mana_dir, &list_params) {
crates/imp-core/src/tools/mana.rs:2660:                        let message = format!("mana run failed: {e}");
crates/imp-core/src/tools/mana.rs:2662:                            .set_widget("mana", Some(mana_widget_lines(message.clone(), None)))
crates/imp-core/src/tools/mana.rs:2664:                        ctx.ui.set_status("mana", Some(&message)).await;
crates/imp-core/src/tools/mana.rs:2673:                match mana_core::ops::show::get(&mana_dir, id) {
crates/imp-core/src/tools/mana.rs:2691:                let create_params = mana_core::ops::create::CreateParams {
crates/imp-core/src/tools/mana.rs:2715:                match mana_core::api::create_unit(&mana_dir, create_params) {
crates/imp-core/src/tools/mana.rs:2720:                            .map(|label| format!("mana delta: created {label}"))
crates/imp-core/src/tools/mana.rs:2721:                            .unwrap_or_else(|| "mana delta: created unit".to_string());
crates/imp-core/src/tools/mana.rs:2730:                        set_mana_delta_widget(&ctx, summary.clone(), detail).await;
crates/imp-core/src/tools/mana.rs:2760:                match mana_core::api::claim_unit(&mana_dir, id, claim_params) {
crates/imp-core/src/tools/mana.rs:2769:                match mana_core::api::release_unit(&mana_dir, id) {
crates/imp-core/src/tools/mana.rs:2787:                    return Ok(mana_close_force_reason_error(id));
crates/imp-core/src/tools/mana.rs:2789:                let opts = mana_core::ops::close::CloseOpts {
crates/imp-core/src/tools/mana.rs:2794:                match mana_core::api::close_unit(&mana_dir, id, opts) {
crates/imp-core/src/tools/mana.rs:2800:                                .map(|label| format!("mana delta: closed {label}"))
crates/imp-core/src/tools/mana.rs:2801:                                .unwrap_or_else(|| format!("mana delta: closed {id}"));
crates/imp-core/src/tools/mana.rs:2802:                            set_mana_delta_widget(&ctx, summary, reason.clone()).await;
crates/imp-core/src/tools/mana.rs:2819:                    Err(e) => Ok(mana_close_error_output(id, e.to_string())),
crates/imp-core/src/tools/mana.rs:2830:                let update_params = mana_core::ops::update::UpdateParams {
crates/imp-core/src/tools/mana.rs:2847:                match mana_core::api::update_unit(&mana_dir, id, update_params) {
crates/imp-core/src/tools/mana.rs:2852:                            .map(|label| format!("mana delta: updated {label}"))
crates/imp-core/src/tools/mana.rs:2853:                            .unwrap_or_else(|| format!("mana delta: updated {id}"));
crates/imp-core/src/tools/mana.rs:2854:                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
crates/imp-core/src/tools/mana.rs:2885:                let update_params = mana_core::ops::update::UpdateParams {
crates/imp-core/src/tools/mana.rs:2899:                match mana_core::api::update_unit(&mana_dir, id, update_params) {
crates/imp-core/src/tools/mana.rs:2904:                            .map(|label| format!("mana delta: notes appended on {label}"))
crates/imp-core/src/tools/mana.rs:2905:                            .unwrap_or_else(|| format!("mana delta: notes appended on {id}"));
crates/imp-core/src/tools/mana.rs:2906:                        set_mana_delta_widget(&ctx, summary.clone(), Some("notes appended".into()))
crates/imp-core/src/tools/mana.rs:2938:                let update_params = mana_core::ops::update::UpdateParams {
crates/imp-core/src/tools/mana.rs:2952:                match mana_core::api::update_unit(&mana_dir, id, update_params) {
crates/imp-core/src/tools/mana.rs:2957:                            .map(|label| format!("mana delta: decision added on {label}"))
crates/imp-core/src/tools/mana.rs:2958:                            .unwrap_or_else(|| format!("mana delta: decision added on {id}"));
crates/imp-core/src/tools/mana.rs:2959:                        set_mana_delta_widget(&ctx, summary.clone(), Some("decision added".into()))
crates/imp-core/src/tools/mana.rs:2986:                let update_params = mana_core::ops::update::UpdateParams {
crates/imp-core/src/tools/mana.rs:3000:                match mana_core::api::update_unit(&mana_dir, id, update_params) {
crates/imp-core/src/tools/mana.rs:3005:                            .map(|label| format!("mana delta: decision resolved on {label}"))
crates/imp-core/src/tools/mana.rs:3006:                            .unwrap_or_else(|| format!("mana delta: decision resolved on {id}"));
crates/imp-core/src/tools/mana.rs:3007:                        set_mana_delta_widget(
crates/imp-core/src/tools/mana.rs:3031:                match mana_core::api::reopen_unit(&mana_dir, id) {
crates/imp-core/src/tools/mana.rs:3034:                            "mana delta: reopened {} ({})",
crates/imp-core/src/tools/mana.rs:3037:                        set_mana_delta_widget(&ctx, summary, Some("status=open".into())).await;
crates/imp-core/src/tools/mana.rs:3058:                match mana_core::api::run_verify(&mana_dir, id) {
crates/imp-core/src/tools/mana.rs:3089:                match mana_core::api::fail_unit(
crates/imp-core/src/tools/mana.rs:3090:                    &mana_dir,
crates/imp-core/src/tools/mana.rs:3098:                            .map(|label| format!("mana delta: marked failed {label}"))
crates/imp-core/src/tools/mana.rs:3099:                            .unwrap_or_else(|| format!("mana delta: marked failed {id}"));
crates/imp-core/src/tools/mana.rs:3100:                        set_mana_delta_widget(
crates/imp-core/src/tools/mana.rs:3123:                match mana_core::api::delete_unit(&mana_dir, id) {
crates/imp-core/src/tools/mana.rs:3126:                            format!("mana delta: deleted {} ({})", result.id, result.title);
crates/imp-core/src/tools/mana.rs:3127:                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
crates/imp-core/src/tools/mana.rs:3143:                match mana_core::api::add_dep(&mana_dir, from_id, dep_id) {
crates/imp-core/src/tools/mana.rs:3146:                            "mana delta: dependency added {} -> {}",
crates/imp-core/src/tools/mana.rs:3149:                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
crates/imp-core/src/tools/mana.rs:3168:                match mana_core::api::remove_dep(&mana_dir, from_id, dep_id) {
crates/imp-core/src/tools/mana.rs:3171:                            "mana delta: dependency removed {} -> {}",
crates/imp-core/src/tools/mana.rs:3174:                        set_mana_delta_widget(&ctx, summary.clone(), None).await;
crates/imp-core/src/tools/mana.rs:3205:                let fact_params = mana_core::ops::fact::FactParams {
crates/imp-core/src/tools/mana.rs:3213:                match mana_core::api::create_fact(&mana_dir, fact_params) {
crates/imp-core/src/tools/mana.rs:3216:                            "mana delta: created fact {} ({})",
crates/imp-core/src/tools/mana.rs:3219:                        set_mana_delta_widget(&ctx, summary.clone(), Some("fact".into())).await;
crates/imp-core/src/tools/mana.rs:3239:            "fact_verify" => match mana_core::api::verify_facts(&mana_dir) {
crates/imp-core/src/tools/mana.rs:3276:                        "Unknown native mana run_id: {run_id}"
crates/imp-core/src/tools/mana.rs:3285:                        "No logs for unit {id}. Has it been dispatched with mana run?"
crates/imp-core/src/tools/mana.rs:3316:                let unit_path = mana_core::discovery::find_unit_file(&mana_dir, id)
crates/imp-core/src/tools/mana.rs:3318:                let mut unit = mana_core::unit::Unit::from_file(&unit_path)
crates/imp-core/src/tools/mana.rs:3326:                    "mana delta: reparented {id} from {} to {}",
crates/imp-core/src/tools/mana.rs:3330:                set_mana_delta_widget(&ctx, summary.clone(), reason.clone()).await;
crates/imp-core/src/tools/mana.rs:3355:                            "No native mana run state available for {which}. Start one with mana(action=\"run\")."
crates/imp-core/src/tools/mana.rs:3362:                match mana_core::api::load_index(&mana_dir) {
crates/imp-core/src/tools/mana.rs:3364:                        let ready: Vec<&mana_core::index::IndexEntry> = index
crates/imp-core/src/tools/mana.rs:3368:                                e.status == mana_core::unit::Status::Open
crates/imp-core/src/tools/mana.rs:3371:                                    && mana_core::blocking::check_blocked(e, &index).is_none()
crates/imp-core/src/tools/mana.rs:3374:                                            && child.status != mana_core::unit::Status::Closed
crates/imp-core/src/tools/mana.rs:3409:                            entry: &mana_core::index::IndexEntry,
crates/imp-core/src/tools/mana.rs:3471:                    match mana_core::api::get_tree(&mana_dir, root_id) {
crates/imp-core/src/tools/mana.rs:3477:                        Err(tree_err) => match mana_core::ops::show::get(&mana_dir, root_id) {
crates/imp-core/src/tools/mana.rs:3487:                    match mana_core::api::load_index(&mana_dir) {
crates/imp-core/src/tools/mana.rs:3497:                                match mana_core::api::get_tree(&mana_dir, root_id) {
crates/imp-core/src/tools/mana.rs:3536:                    if let Some(guardrail) = retry_guardrail_for_targets(&mana_dir, &target_ids)? {
crates/imp-core/src/tools/mana.rs:3539:                                text: "mana run blocked: failed units must be updated before retry"
crates/imp-core/src/tools/mana.rs:3550:                        crate::error::Error::Tool("mana run state lock poisoned".into())
crates/imp-core/src/tools/mana.rs:3559:                    mana_dir.clone(),
crates/imp-core/src/tools/mana.rs:3583:        compact_list_output, evaluate_run_output, mana_close_error_output,
crates/imp-core/src/tools/mana.rs:3584:        mana_close_force_reason_error, mana_guide_output, mana_run_core, mana_template_output,
crates/imp-core/src/tools/mana.rs:3587:        target_ids_from_run_target, unix_time_ms, validate_mana_action, validate_native_run_target,
crates/imp-core/src/tools/mana.rs:3588:        worker_options_for_native_unit, GuideTopic, ManaRunStore, ManaTool, NativeRunState,
crates/imp-core/src/tools/mana.rs:3594:    enum ManaResult {
crates/imp-core/src/tools/mana.rs:3651:    async fn run_with_mode(mode_name: &str, action: &str) -> ManaResult {
crates/imp-core/src/tools/mana.rs:3660:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:3661:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:3662:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:3664:            mana_dir.join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:3684:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:3685:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:3692:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:3710:                ManaResult::Attempted(crate::tools::ToolOutput::error(msg))
crates/imp-core/src/tools/mana.rs:3712:            Err(e) => ManaResult::Attempted(crate::tools::ToolOutput::error(e.to_string())),
crates/imp-core/src/tools/mana.rs:3717:                            return ManaResult::ModeBlocked(text.to_string());
crates/imp-core/src/tools/mana.rs:3721:                ManaResult::Attempted(output)
crates/imp-core/src/tools/mana.rs:3730:        let mana_dir = dir.join(".mana");
crates/imp-core/src/tools/mana.rs:3731:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:3732:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:3734:            mana_dir.join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:3753:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:3754:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:3767:        let mana_dir = dir.join(".mana");
crates/imp-core/src/tools/mana.rs:3768:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:3769:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:3771:            mana_dir.join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:3793:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:3794:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:3803:    async fn run_with_ctx_mode(mode: crate::config::AgentMode, action: &str) -> ManaResult {
crates/imp-core/src/tools/mana.rs:3806:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:3812:                ManaResult::Attempted(crate::tools::ToolOutput::error(msg))
crates/imp-core/src/tools/mana.rs:3814:            Err(e) => ManaResult::Attempted(crate::tools::ToolOutput::error(e.to_string())),
crates/imp-core/src/tools/mana.rs:3819:                            return ManaResult::ModeBlocked(text.to_string());
crates/imp-core/src/tools/mana.rs:3823:                ManaResult::Attempted(output)
crates/imp-core/src/tools/mana.rs:3829:    async fn create_sets_mana_delta_widget_and_action_details() {
crates/imp-core/src/tools/mana.rs:3832:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:3846:            key == "mana"
crates/imp-core/src/tools/mana.rs:3847:                && matches!(content, Some(WidgetContent::Lines(lines)) if lines.iter().any(|line| line.contains("mana delta: created 2 · Widget unit")))
crates/imp-core/src/tools/mana.rs:3852:    async fn decision_add_sets_mana_delta_widget_and_action_details() {
crates/imp-core/src/tools/mana.rs:3855:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:3869:            key == "mana"
crates/imp-core/src/tools/mana.rs:3870:                && matches!(content, Some(WidgetContent::Lines(lines)) if lines.iter().any(|line| line.contains("mana delta: decision added on 1 · Test unit")))
crates/imp-core/src/tools/mana.rs:3877:            ManaResult::ModeBlocked(_) => {}
crates/imp-core/src/tools/mana.rs:3878:            ManaResult::Attempted(out) => {
crates/imp-core/src/tools/mana.rs:3891:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:3962:        let output = mana_close_force_reason_error("313.2");
crates/imp-core/src/tools/mana.rs:3975:        let output = mana_close_error_output(
crates/imp-core/src/tools/mana.rs:3990:        let output = mana_close_error_output("313.2", "Unit not found".to_string());
crates/imp-core/src/tools/mana.rs:3999:    fn mana_guide_outputs_concise_structured_topic() {
crates/imp-core/src/tools/mana.rs:4000:        let output = mana_guide_output(GuideTopic::Verify);
crates/imp-core/src/tools/mana.rs:4009:            .contains("mana guide: verify"));
crates/imp-core/src/tools/mana.rs:4013:    fn mana_overview_guide_prefers_update_before_create() {
crates/imp-core/src/tools/mana.rs:4014:        let output = mana_guide_output(GuideTopic::Overview);
crates/imp-core/src/tools/mana.rs:4025:        let entries: Vec<mana_core::index::IndexEntry> = (0..60)
crates/imp-core/src/tools/mana.rs:4026:            .map(|i| mana_core::index::IndexEntry {
crates/imp-core/src/tools/mana.rs:4030:                status: mana_core::unit::Status::Open,
crates/imp-core/src/tools/mana.rs:4034:                labels: vec!["mana".to_string()],
crates/imp-core/src/tools/mana.rs:4044:                paths: vec!["crates/imp-core/src/tools/mana.rs".to_string()],
crates/imp-core/src/tools/mana.rs:4045:                kind: mana_core::unit::UnitType::Task,
crates/imp-core/src/tools/mana.rs:4067:        let output = validate_mana_action("create", &json!({}))
crates/imp-core/src/tools/mana.rs:4078:    fn mana_template_outputs_task_template() {
crates/imp-core/src/tools/mana.rs:4079:        let output = mana_template_output(TemplateKind::Task, Some(GuideTopic::Verify));
crates/imp-core/src/tools/mana.rs:4089:    fn mana_guide_and_template_validate_topic_and_kind() {
crates/imp-core/src/tools/mana.rs:4094:            .contains("Invalid mana guide topic"));
crates/imp-core/src/tools/mana.rs:4099:            .contains("Invalid mana template kind"));
crates/imp-core/src/tools/mana.rs:4103:    fn mana_schema_uses_canonical_fields_only() {
crates/imp-core/src/tools/mana.rs:4104:        let schema = ManaTool::default().parameters();
crates/imp-core/src/tools/mana.rs:4126:        assert!(!properties.contains_key("mana_scope"));
crates/imp-core/src/tools/mana.rs:4127:        assert!(!properties.contains_key("mana_dir"));
crates/imp-core/src/tools/mana.rs:4139:    fn mana_validation_teaches_required_fields() {
crates/imp-core/src/tools/mana.rs:4140:        let output = validate_mana_action("notes_append", &json!({ "id": "304" }))
crates/imp-core/src/tools/mana.rs:4154:    fn mana_validation_rejects_path_when_paths_is_intended() {
crates/imp-core/src/tools/mana.rs:4155:        let output = validate_mana_action(
crates/imp-core/src/tools/mana.rs:4166:            .contains("Use path for project/.mana location"));
crates/imp-core/src/tools/mana.rs:4170:    fn mana_validation_allows_valid_create_and_decision_add() {
crates/imp-core/src/tools/mana.rs:4171:        assert!(validate_mana_action("create", &json!({ "title": "Build thing" })).is_none());
crates/imp-core/src/tools/mana.rs:4172:        assert!(validate_mana_action(
crates/imp-core/src/tools/mana.rs:4180:    fn mana_validation_requires_parent_for_reparent() {
crates/imp-core/src/tools/mana.rs:4181:        let output = validate_mana_action("reparent", &json!({ "id": "313.2" }))
crates/imp-core/src/tools/mana.rs:4196:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4260:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4273:            dir2.path().join(".mana").join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:4304:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4321:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4322:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:4323:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:4325:            mana_dir.join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:4343:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4344:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4350:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4358:        let mana_dir2 = dir2.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4359:        std::fs::create_dir_all(&mana_dir2).unwrap();
crates/imp-core/src/tools/mana.rs:4360:        std::fs::write(mana_dir2.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:4362:            mana_dir2.join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:4380:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4381:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4398:        let mana_dir3 = dir3.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4399:        std::fs::create_dir_all(&mana_dir3).unwrap();
crates/imp-core/src/tools/mana.rs:4400:        std::fs::write(mana_dir3.join("config.yaml"), "project: test\nnext_id: 1\n").unwrap();
crates/imp-core/src/tools/mana.rs:4416:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4417:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4423:        let fact = tool.execute("call_fact", json!({ "action": "fact_create", "title": "Auth fact", "verify": "test -d .mana", "description": "fact body", "ttl_days": 7 }), ctx3).await.unwrap();
crates/imp-core/src/tools/mana.rs:4431:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4456:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4474:            dir2.path().join(".mana").join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:4499:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4500:        std::fs::create_dir_all(mana_dir.join("archive/2026/04")).unwrap();
crates/imp-core/src/tools/mana.rs:4501:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:4503:            mana_dir.join("archive/2026/04/1-archived-unit.md"),
crates/imp-core/src/tools/mana.rs:4522:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4523:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4529:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4547:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4548:        std::fs::create_dir_all(mana_dir.join("archive/2026/04")).unwrap();
crates/imp-core/src/tools/mana.rs:4549:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:4551:            mana_dir.join("archive/2026/04/1-archived-unit.md"),
crates/imp-core/src/tools/mana.rs:4570:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4571:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4577:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4593:    async fn root_scope_targets_outermost_mana() {
crates/imp-core/src/tools/mana.rs:4595:        let root_mana = tower.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4596:        std::fs::create_dir_all(&root_mana).unwrap();
crates/imp-core/src/tools/mana.rs:4597:        std::fs::write(root_mana.join("config.yaml"), "project: root\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:4599:            root_mana.join("1-root-unit.md"),
crates/imp-core/src/tools/mana.rs:4603:        let project_mana = project.join(".mana");
crates/imp-core/src/tools/mana.rs:4604:        std::fs::create_dir_all(&project_mana).unwrap();
crates/imp-core/src/tools/mana.rs:4606:            project_mana.join("config.yaml"),
crates/imp-core/src/tools/mana.rs:4611:            project_mana.join("1-project-unit.md"),
crates/imp-core/src/tools/mana.rs:4631:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4632:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4638:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4654:        let target_mana = target_project.join(".mana");
crates/imp-core/src/tools/mana.rs:4655:        std::fs::create_dir_all(&target_mana).unwrap();
crates/imp-core/src/tools/mana.rs:4657:            target_mana.join("config.yaml"),
crates/imp-core/src/tools/mana.rs:4662:            target_mana.join("1-other-unit.md"),
crates/imp-core/src/tools/mana.rs:4685:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4686:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4692:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4707:            ManaResult::ModeBlocked(_) => {}
crates/imp-core/src/tools/mana.rs:4708:            ManaResult::Attempted(out) => {
crates/imp-core/src/tools/mana.rs:4720:            ManaResult::Attempted(_) => {}
crates/imp-core/src/tools/mana.rs:4721:            ManaResult::ModeBlocked(msg) => {
crates/imp-core/src/tools/mana.rs:4730:            ManaResult::Attempted(_) => {}
crates/imp-core/src/tools/mana.rs:4731:            ManaResult::ModeBlocked(msg) => {
crates/imp-core/src/tools/mana.rs:4740:            ManaResult::ModeBlocked(_) => {}
crates/imp-core/src/tools/mana.rs:4741:            ManaResult::Attempted(out) => {
crates/imp-core/src/tools/mana.rs:4753:            ManaResult::Attempted(_) => {}
crates/imp-core/src/tools/mana.rs:4754:            ManaResult::ModeBlocked(msg) => {
crates/imp-core/src/tools/mana.rs:4779:                ManaResult::Attempted(_) => {}
crates/imp-core/src/tools/mana.rs:4780:                ManaResult::ModeBlocked(msg) => {
crates/imp-core/src/tools/mana.rs:4810:            ManaResult::ModeBlocked(_) => {}
crates/imp-core/src/tools/mana.rs:4811:            ManaResult::Attempted(out) => {
crates/imp-core/src/tools/mana.rs:4823:            ManaResult::ModeBlocked(_) => {}
crates/imp-core/src/tools/mana.rs:4824:            ManaResult::Attempted(out) => {
crates/imp-core/src/tools/mana.rs:4853:                ManaResult::Attempted(_) => {}
crates/imp-core/src/tools/mana.rs:4854:                ManaResult::ModeBlocked(msg) => {
crates/imp-core/src/tools/mana.rs:4865:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4878:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4884:        assert!(text.contains("Started native mana orchestration"));
crates/imp-core/src/tools/mana.rs:4891:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:4892:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:4893:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-core/src/tools/mana.rs:4895:            mana_dir.join("1-test-unit.md"),
crates/imp-core/src/tools/mana.rs:4915:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/mana.rs:4916:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/mana.rs:4923:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4941:                    text.contains("Native mana orchestration finished"),
crates/imp-core/src/tools/mana.rs:4945:                    text.contains("Inspect with mana(action=\"run_state\")"),
crates/imp-core/src/tools/mana.rs:4957:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:4987:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:5005:        let tool = ManaTool::default();
crates/imp-core/src/tools/mana.rs:5028:            state_text.contains("Native mana orchestration "),
crates/imp-core/src/tools/mana.rs:5059:            eval_text.contains("Native mana orchestration run ")
crates/imp-core/src/tools/mana.rs:5071:        let mut store = ManaRunStore::default();
crates/imp-core/src/tools/mana.rs:5074:            &mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5075:                target: mana::commands::run::RunTarget::AllReady,
crates/imp-core/src/tools/mana.rs:5090:            &mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5091:                target: mana::commands::run::RunTarget::Unit("1".to_string()),
crates/imp-core/src/tools/mana.rs:5113:        let line = stream_event_line(&mana::stream::StreamEvent::UnitTool {
crates/imp-core/src/tools/mana.rs:5129:            &mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5130:                target: mana::commands::run::RunTarget::Unit("7".to_string()),
crates/imp-core/src/tools/mana.rs:5171:            &mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5172:                target: mana::commands::run::RunTarget::Unit("8".to_string()),
crates/imp-core/src/tools/mana.rs:5243:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:5244:        std::fs::create_dir(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:5245:        mana_core::config::Config {
crates/imp-core/src/tools/mana.rs:5275:        .save(&mana_dir)
crates/imp-core/src/tools/mana.rs:5278:        let created = mana_core::api::create_unit(
crates/imp-core/src/tools/mana.rs:5279:            &mana_dir,
crates/imp-core/src/tools/mana.rs:5280:            mana_core::ops::create::CreateParams {
crates/imp-core/src/tools/mana.rs:5289:        let mut failed_unit = mana_core::ops::show::get(&mana_dir, &id).unwrap().unit;
crates/imp-core/src/tools/mana.rs:5292:            .push(mana_core::unit::AttemptRecord {
crates/imp-core/src/tools/mana.rs:5294:                outcome: mana_core::unit::AttemptOutcome::Failed,
crates/imp-core/src/tools/mana.rs:5302:        let unit_path = mana_core::discovery::find_unit_file(&mana_dir, &id).unwrap();
crates/imp-core/src/tools/mana.rs:5305:        let guardrail = retry_guardrail_for_targets(&mana_dir, std::slice::from_ref(&id))
crates/imp-core/src/tools/mana.rs:5321:        let mana_dir = dir.path().join(".mana");
crates/imp-core/src/tools/mana.rs:5322:        std::fs::create_dir(&mana_dir).unwrap();
crates/imp-core/src/tools/mana.rs:5323:        mana_core::config::Config {
crates/imp-core/src/tools/mana.rs:5353:        .save(&mana_dir)
crates/imp-core/src/tools/mana.rs:5356:        let created = mana_core::api::create_unit(
crates/imp-core/src/tools/mana.rs:5357:            &mana_dir,
crates/imp-core/src/tools/mana.rs:5358:            mana_core::ops::create::CreateParams {
crates/imp-core/src/tools/mana.rs:5367:        let mut failed_unit = mana_core::ops::show::get(&mana_dir, &id).unwrap().unit;
crates/imp-core/src/tools/mana.rs:5370:            .push(mana_core::unit::AttemptRecord {
crates/imp-core/src/tools/mana.rs:5372:                outcome: mana_core::unit::AttemptOutcome::Failed,
crates/imp-core/src/tools/mana.rs:5380:        let unit_path = mana_core::discovery::find_unit_file(&mana_dir, &id).unwrap();
crates/imp-core/src/tools/mana.rs:5383:        mana_core::api::update_unit(
crates/imp-core/src/tools/mana.rs:5384:            &mana_dir,
crates/imp-core/src/tools/mana.rs:5386:            mana_core::ops::update::UpdateParams {
crates/imp-core/src/tools/mana.rs:5393:        let guardrail = retry_guardrail_for_targets(&mana_dir, &[id]).unwrap();
crates/imp-core/src/tools/mana.rs:5397:    fn native_run_params_for_test() -> mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5398:        mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5399:            target: mana::commands::run::RunTarget::AllReady,
crates/imp-core/src/tools/mana.rs:5413:    fn ready_unit_for_model_test(model: Option<&str>) -> mana_run_core::ReadyUnit {
crates/imp-core/src/tools/mana.rs:5414:        mana_run_core::ReadyUnit {
crates/imp-core/src/tools/mana.rs:5427:            retry: mana_core::ops::run::RunRetryContext {
crates/imp-core/src/tools/mana.rs:5445:        let run_args = mana::commands::run::NativeRunParams {
crates/imp-core/src/tools/mana.rs:5454:            dir.path().join(".mana"),
crates/imp-core/src/tools/mana.rs:5462:            options.mana_dir_override.as_deref(),
crates/imp-core/src/tools/mana.rs:5463:            Some(dir.path().join(".mana").as_path())
crates/imp-core/src/tools/mana.rs:5481:            dir.path().join(".mana"),
crates/imp-core/src/tools/mana.rs:5505:    fn mana_run_state_persists_material_events() {
crates/imp-core/src/tools/mana.rs:5506:        let mut store = ManaRunStore::default();
crates/imp-core/src/tools/mana.rs:5512:            &mana::stream::StreamEvent::UnitTool {
crates/imp-core/src/tools/mana.rs:5527:    fn mana_run_state_marks_stale_running_runs_interrupted() {
crates/imp-core/src/tools/mana.rs:5528:        let mut store = ManaRunStore::default();
crates/imp-core/src/workflow/verification.rs:251:    ManaTask { unit_id: Option<String> },
crates/imp-core/src/workflow/verification.rs:290:        gate.source = VerificationGateSource::ManaTask {
crates/imp-core/src/resources.rs:625:        let user_skill = user_dir.join("skills").join("mana");
crates/imp-core/src/resources.rs:627:        fs::write(user_skill.join("SKILL.md"), "# Mana\n\nUser version.\n").unwrap();
crates/imp-core/src/resources.rs:630:        let project_skill = project.join(".imp").join("skills").join("mana");
crates/imp-core/src/resources.rs:634:            "# Mana\n\nProject version.\n",
crates/imp-core/src/resources.rs:640:        assert_eq!(skills[0].name, "mana");
crates/imp-core/src/learning.rs:29:1. Is there anything worth capturing in mana facts, mana notes, or user profile context?
crates/imp-core/src/learning.rs:35:You can author skills and should use mana for durable project knowledge. \
crates/imp-tui/src/views/settings.rs:3:    ManaConfig, ManaRunConfig, ManaScopePreference, ShellBackend, SidebarStyle, ToolOutputDisplay,
crates/imp-tui/src/views/settings.rs:55:    ManaScope,
crates/imp-tui/src/views/settings.rs:56:    ManaAutoCommit,
crates/imp-tui/src/views/settings.rs:57:    ManaAutoCloseParent,
crates/imp-tui/src/views/settings.rs:58:    ManaVerifyTimeout,
crates/imp-tui/src/views/settings.rs:59:    ManaRunBackground,
crates/imp-tui/src/views/settings.rs:60:    ManaMaxWorkers,
crates/imp-tui/src/views/settings.rs:61:    ManaReviewAfterRun,
crates/imp-tui/src/views/settings.rs:62:    ManaContinueAfterFailure,
crates/imp-tui/src/views/settings.rs:73:    Mana,
crates/imp-tui/src/views/settings.rs:82:    SettingsTab::Mana,
crates/imp-tui/src/views/settings.rs:134:const MANA_FIELDS: &[SettingsField] = &[
crates/imp-tui/src/views/settings.rs:135:    SettingsField::ManaScope,
crates/imp-tui/src/views/settings.rs:136:    SettingsField::ManaAutoCommit,
crates/imp-tui/src/views/settings.rs:137:    SettingsField::ManaAutoCloseParent,
crates/imp-tui/src/views/settings.rs:138:    SettingsField::ManaVerifyTimeout,
crates/imp-tui/src/views/settings.rs:139:    SettingsField::ManaRunBackground,
crates/imp-tui/src/views/settings.rs:140:    SettingsField::ManaMaxWorkers,
crates/imp-tui/src/views/settings.rs:141:    SettingsField::ManaReviewAfterRun,
crates/imp-tui/src/views/settings.rs:142:    SettingsField::ManaContinueAfterFailure,
crates/imp-tui/src/views/settings.rs:180:    SettingsField::ManaScope,
crates/imp-tui/src/views/settings.rs:181:    SettingsField::ManaAutoCommit,
crates/imp-tui/src/views/settings.rs:182:    SettingsField::ManaAutoCloseParent,
crates/imp-tui/src/views/settings.rs:183:    SettingsField::ManaVerifyTimeout,
crates/imp-tui/src/views/settings.rs:184:    SettingsField::ManaRunBackground,
crates/imp-tui/src/views/settings.rs:185:    SettingsField::ManaMaxWorkers,
crates/imp-tui/src/views/settings.rs:186:    SettingsField::ManaReviewAfterRun,
crates/imp-tui/src/views/settings.rs:187:    SettingsField::ManaContinueAfterFailure,
crates/imp-tui/src/views/settings.rs:199:            SettingsTab::Mana => "Mana",
crates/imp-tui/src/views/settings.rs:210:            SettingsTab::Mana => MANA_FIELDS,
crates/imp-tui/src/views/settings.rs:217:            SettingsTab::Mana => None,
crates/imp-tui/src/views/settings.rs:273:    pub mana_scope: ManaScopePreference,
crates/imp-tui/src/views/settings.rs:274:    pub mana_auto_commit: bool,
crates/imp-tui/src/views/settings.rs:275:    pub mana_auto_close_parent: bool,
crates/imp-tui/src/views/settings.rs:276:    pub mana_verify_timeout: u64,
crates/imp-tui/src/views/settings.rs:277:    pub mana_run_background: bool,
crates/imp-tui/src/views/settings.rs:278:    pub mana_max_workers: u32,
crates/imp-tui/src/views/settings.rs:279:    pub mana_review_after_run: bool,
crates/imp-tui/src/views/settings.rs:280:    pub mana_continue_after_failure: bool,
crates/imp-tui/src/views/settings.rs:391:            mana_scope: config.mana.scope,
crates/imp-tui/src/views/settings.rs:392:            mana_auto_commit: config.mana.auto_commit,
crates/imp-tui/src/views/settings.rs:393:            mana_auto_close_parent: config.mana.auto_close_parent,
crates/imp-tui/src/views/settings.rs:394:            mana_verify_timeout: config.mana.verify_timeout.unwrap_or(0),
crates/imp-tui/src/views/settings.rs:395:            mana_run_background: config.mana.run.background,
crates/imp-tui/src/views/settings.rs:396:            mana_max_workers: config.mana.run.max_workers,
crates/imp-tui/src/views/settings.rs:397:            mana_review_after_run: config.mana.run.review_after_run,
crates/imp-tui/src/views/settings.rs:398:            mana_continue_after_failure: config.mana.run.continue_after_failure,
crates/imp-tui/src/views/settings.rs:592:            SettingsField::ManaScope => {
crates/imp-tui/src/views/settings.rs:593:                self.mana_scope = match self.mana_scope {
crates/imp-tui/src/views/settings.rs:594:                    ManaScopePreference::Project => ManaScopePreference::Root,
crates/imp-tui/src/views/settings.rs:595:                    ManaScopePreference::Root => ManaScopePreference::Project,
crates/imp-tui/src/views/settings.rs:598:            SettingsField::ManaAutoCommit => {
crates/imp-tui/src/views/settings.rs:599:                self.mana_auto_commit = !self.mana_auto_commit;
crates/imp-tui/src/views/settings.rs:601:            SettingsField::ManaAutoCloseParent => {
crates/imp-tui/src/views/settings.rs:602:                self.mana_auto_close_parent = !self.mana_auto_close_parent;
crates/imp-tui/src/views/settings.rs:604:            SettingsField::ManaVerifyTimeout => {
crates/imp-tui/src/views/settings.rs:605:                self.mana_verify_timeout = self.mana_verify_timeout.saturating_add(30).min(3600);
crates/imp-tui/src/views/settings.rs:607:            SettingsField::ManaRunBackground => {
crates/imp-tui/src/views/settings.rs:608:                self.mana_run_background = !self.mana_run_background;
crates/imp-tui/src/views/settings.rs:610:            SettingsField::ManaMaxWorkers => {
crates/imp-tui/src/views/settings.rs:611:                self.mana_max_workers = self.mana_max_workers.saturating_add(1).min(32);
crates/imp-tui/src/views/settings.rs:613:            SettingsField::ManaReviewAfterRun => {
crates/imp-tui/src/views/settings.rs:614:                self.mana_review_after_run = !self.mana_review_after_run;
crates/imp-tui/src/views/settings.rs:616:            SettingsField::ManaContinueAfterFailure => {
crates/imp-tui/src/views/settings.rs:617:                self.mana_continue_after_failure = !self.mana_continue_after_failure;
crates/imp-tui/src/views/settings.rs:771:            SettingsField::ManaScope => {
crates/imp-tui/src/views/settings.rs:772:                self.mana_scope = match self.mana_scope {
crates/imp-tui/src/views/settings.rs:773:                    ManaScopePreference::Project => ManaScopePreference::Root,
crates/imp-tui/src/views/settings.rs:774:                    ManaScopePreference::Root => ManaScopePreference::Project,
crates/imp-tui/src/views/settings.rs:777:            SettingsField::ManaAutoCommit => {
crates/imp-tui/src/views/settings.rs:778:                self.mana_auto_commit = !self.mana_auto_commit;
crates/imp-tui/src/views/settings.rs:780:            SettingsField::ManaAutoCloseParent => {
crates/imp-tui/src/views/settings.rs:781:                self.mana_auto_close_parent = !self.mana_auto_close_parent;
crates/imp-tui/src/views/settings.rs:783:            SettingsField::ManaVerifyTimeout => {
crates/imp-tui/src/views/settings.rs:784:                self.mana_verify_timeout = self.mana_verify_timeout.saturating_sub(30);
crates/imp-tui/src/views/settings.rs:786:            SettingsField::ManaRunBackground => {
crates/imp-tui/src/views/settings.rs:787:                self.mana_run_background = !self.mana_run_background;
crates/imp-tui/src/views/settings.rs:789:            SettingsField::ManaMaxWorkers => {
crates/imp-tui/src/views/settings.rs:790:                self.mana_max_workers = self.mana_max_workers.saturating_sub(1).max(1);
crates/imp-tui/src/views/settings.rs:792:            SettingsField::ManaReviewAfterRun => {
crates/imp-tui/src/views/settings.rs:793:                self.mana_review_after_run = !self.mana_review_after_run;
crates/imp-tui/src/views/settings.rs:795:            SettingsField::ManaContinueAfterFailure => {
crates/imp-tui/src/views/settings.rs:796:                self.mana_continue_after_failure = !self.mana_continue_after_failure;
crates/imp-tui/src/views/settings.rs:831:            SettingsField::ManaVerifyTimeout => {
crates/imp-tui/src/views/settings.rs:833:                self.edit_buffer = self.mana_verify_timeout.to_string();
crates/imp-tui/src/views/settings.rs:835:            SettingsField::ManaMaxWorkers => {
crates/imp-tui/src/views/settings.rs:837:                self.edit_buffer = self.mana_max_workers.to_string();
crates/imp-tui/src/views/settings.rs:948:            SettingsField::ManaVerifyTimeout => {
crates/imp-tui/src/views/settings.rs:950:                    self.mana_verify_timeout = v.min(3600);
crates/imp-tui/src/views/settings.rs:953:            SettingsField::ManaMaxWorkers => {
crates/imp-tui/src/views/settings.rs:955:                    self.mana_max_workers = v.clamp(1, 32);
crates/imp-tui/src/views/settings.rs:1023:        config.mana = ManaConfig {
crates/imp-tui/src/views/settings.rs:1024:            scope: self.mana_scope,
crates/imp-tui/src/views/settings.rs:1025:            auto_commit: self.mana_auto_commit,
crates/imp-tui/src/views/settings.rs:1026:            auto_close_parent: self.mana_auto_close_parent,
crates/imp-tui/src/views/settings.rs:1027:            verify_timeout: (self.mana_verify_timeout > 0).then_some(self.mana_verify_timeout),
crates/imp-tui/src/views/settings.rs:1028:            run: ManaRunConfig {
crates/imp-tui/src/views/settings.rs:1029:                background: self.mana_run_background,
crates/imp-tui/src/views/settings.rs:1030:                max_workers: self.mana_max_workers.max(1),
crates/imp-tui/src/views/settings.rs:1031:                continue_after_failure: self.mana_continue_after_failure,
crates/imp-tui/src/views/settings.rs:1032:                review_after_run: self.mana_review_after_run,
crates/imp-tui/src/views/settings.rs:1859:        SettingsField::ManaScope => render_field(
crates/imp-tui/src/views/settings.rs:1868:            match state.mana_scope {
crates/imp-tui/src/views/settings.rs:1869:                ManaScopePreference::Project => "project",
crates/imp-tui/src/views/settings.rs:1870:                ManaScopePreference::Root => "root",
crates/imp-tui/src/views/settings.rs:1874:        SettingsField::ManaAutoCommit => render_field(
crates/imp-tui/src/views/settings.rs:1883:            if state.mana_auto_commit { "on" } else { "off" },
crates/imp-tui/src/views/settings.rs:1886:        SettingsField::ManaAutoCloseParent => render_field(
crates/imp-tui/src/views/settings.rs:1895:            if state.mana_auto_close_parent {
crates/imp-tui/src/views/settings.rs:1902:        SettingsField::ManaVerifyTimeout => {
crates/imp-tui/src/views/settings.rs:1904:                && state.current_field() == SettingsField::ManaVerifyTimeout
crates/imp-tui/src/views/settings.rs:1907:            } else if state.mana_verify_timeout == 0 {
crates/imp-tui/src/views/settings.rs:1910:                format!("{}s", state.mana_verify_timeout)
crates/imp-tui/src/views/settings.rs:1925:        SettingsField::ManaRunBackground => render_field(
crates/imp-tui/src/views/settings.rs:1934:            if state.mana_run_background {
crates/imp-tui/src/views/settings.rs:1941:        SettingsField::ManaMaxWorkers => {
crates/imp-tui/src/views/settings.rs:1943:                if state.editing_number && state.current_field() == SettingsField::ManaMaxWorkers {
crates/imp-tui/src/views/settings.rs:1946:                    state.mana_max_workers.to_string()
crates/imp-tui/src/views/settings.rs:1961:        SettingsField::ManaReviewAfterRun => render_field(
crates/imp-tui/src/views/settings.rs:1970:            if state.mana_review_after_run {
crates/imp-tui/src/views/settings.rs:1977:        SettingsField::ManaContinueAfterFailure => render_field(
crates/imp-tui/src/views/settings.rs:1986:            if state.mana_continue_after_failure {
crates/imp-core/src/tools/prototype.rs:758:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/prototype.rs:759:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/workflow/controller.rs:7:/// Runtime-owned state for a mana-backed workflow run.
crates/imp-core/src/workflow/controller.rs:15:    pub mana_root_id: Option<String>,
crates/imp-core/src/workflow/controller.rs:38:    pub fn with_mana_root_id(mut self, mana_root_id: impl Into<String>) -> Self {
crates/imp-core/src/workflow/controller.rs:39:        let mana_root_id = mana_root_id.into();
crates/imp-core/src/workflow/controller.rs:40:        self.mana_root_id = Some(mana_root_id.clone());
crates/imp-core/src/workflow/controller.rs:41:        self.active_unit_id.get_or_insert(mana_root_id);
crates/imp-core/src/workflow/controller.rs:107:    pub fn bind_mana_root(&mut self, mana_root_id: impl Into<String>) {
crates/imp-core/src/workflow/controller.rs:108:        let mana_root_id = mana_root_id.into();
crates/imp-core/src/workflow/controller.rs:109:        self.mana_root_id = Some(mana_root_id.clone());
crates/imp-core/src/workflow/controller.rs:111:            .get_or_insert_with(|| mana_root_id.clone());
crates/imp-core/src/workflow/controller.rs:112:        self.bootstrap = WorkflowBootstrapState::Complete { mana_root_id };
crates/imp-core/src/workflow/controller.rs:148:        if self.active_unit_id.as_deref() == self.mana_root_id.as_deref() {
crates/imp-core/src/workflow/controller.rs:162:                self.active_unit_id = self.mana_root_id.clone();
crates/imp-core/src/workflow/controller.rs:177:            .or_else(|| self.mana_root_id.clone());
crates/imp-core/src/workflow/controller.rs:195:    pub fn record_mana_orchestration_started(&mut self, run_id: Option<String>) {
crates/imp-core/src/workflow/controller.rs:200:    pub fn record_mana_graph_changed(&mut self) {
crates/imp-core/src/workflow/controller.rs:257:            remaining.push("mana graph changed and needs closeout inspection".to_string());
crates/imp-core/src/workflow/controller.rs:266:            remaining.push("durable workflow bootstrap requires a bound mana root".to_string());
crates/imp-core/src/workflow/controller.rs:397:    pub fn from_mana_run_status(status: &str, total_failed: Option<u64>) -> Self {
crates/imp-core/src/workflow/controller.rs:428:    pub mana_root_id: Option<String>,
crates/imp-core/src/workflow/controller.rs:453:            mana_root_id: self.mana_root_id.clone(),
crates/imp-core/src/workflow/controller.rs:476:        mana_root_id: String,
crates/imp-core/src/workflow/controller.rs:569:    "Durable workflow bootstrap is required before presenting the work as complete. Create or bind the root mana work item for this goal, attach acceptance/verification, and continue from the graph; do not close out until the controller has a mana root or bootstrap is explicitly skipped by runtime policy.".to_string()
crates/imp-core/src/workflow/controller.rs:573:    "Orchestration has started, so continue supervising it instead of presenting the work as complete. Inspect mana run_state/logs for active child runs, coordinate ready work, retry or escalate failed units, and only stop when workflow closeout is verified, a concrete blocker exists, or no runnable work remains.".to_string()
crates/imp-core/src/workflow/controller.rs:577:    "This workflow was classified as needing decomposition. Create real child mana tasks for separable durable work products under the workflow root, then execute the first ready child. Do not create lifecycle-only tasks such as verify, closeout, or run tests.".to_string()
crates/imp-core/src/workflow/controller.rs:581:    "Mana graph state changed, so do not present the work as complete yet. Inspect the relevant mana units/tree/next state, verify acceptance and blockers, run required checks, then close or update units before final closeout.".to_string()
crates/imp-core/src/workflow/controller.rs:589:    "Workflow execution is not ready for closeout. Review remaining mana units, child runs, required verification, unresolved decisions, and evidence before presenting the work as complete.".to_string()
crates/imp-core/src/workflow/controller.rs:616:        let mut controller = WorkflowRunController::new().with_mana_root_id("28.1");
crates/imp-core/src/workflow/controller.rs:631:        let mut controller = WorkflowRunController::new().with_mana_root_id("28.1");
crates/imp-core/src/workflow/controller.rs:654:        let mut controller = WorkflowRunController::new().with_mana_root_id("28.1");
crates/imp-core/src/workflow/controller.rs:678:        let mut controller = WorkflowRunController::new().with_mana_root_id("28.1");
crates/imp-core/src/workflow/controller.rs:686:    fn required_bootstrap_continues_until_mana_root_bound() {
crates/imp-core/src/workflow/controller.rs:699:        controller.bind_mana_root("28.1.3.4");
crates/imp-core/src/workflow/controller.rs:700:        assert_eq!(controller.mana_root_id.as_deref(), Some("28.1.3.4"));
crates/imp-core/src/workflow/controller.rs:719:            .with_mana_root_id("28.1")
crates/imp-core/src/workflow/controller.rs:722:        assert_eq!(controller.mana_root_id.as_deref(), Some("28.1"));
crates/imp-core/src/workflow/controller.rs:743:        controller.record_mana_graph_changed();
crates/imp-core/src/workflow/controller.rs:744:        controller.record_mana_orchestration_started(Some("run-1".into()));
crates/imp-core/src/workflow/controller.rs:777:        controller.record_mana_orchestration_started(Some("run-1".into()));
crates/imp-core/src/workflow/controller.rs:792:    fn mana_run_status_maps_failures_to_failed_child_run() {
crates/imp-core/src/workflow/controller.rs:794:            WorkflowChildRunStatus::from_mana_run_status("done", Some(1)),
crates/imp-core/src/workflow/controller.rs:798:            WorkflowChildRunStatus::from_mana_run_status("running", None),
crates/imp-core/src/workflow/controller.rs:804:    fn mana_orchestration_creates_supervision_obligation() {
crates/imp-core/src/workflow/controller.rs:806:        controller.record_mana_orchestration_started(Some("run-1".into()));
crates/imp-core/src/workflow/controller.rs:832:    fn mana_graph_change_requires_graph_closeout_without_mana_run() {
crates/imp-core/src/workflow/controller.rs:834:        controller.record_mana_graph_changed();
crates/imp-core/src/workflow/controller.rs:848:            .any(|item| item.contains("mana graph changed")));
crates/imp-core/src/workflow/controller.rs:852:    fn direct_work_change_requires_direct_closeout_without_mana() {
crates/imp-core/src/workflow/controller.rs:868:        controller.record_mana_graph_changed();
crates/imp-core/src/workflow/controller.rs:878:        controller.record_mana_orchestration_started(Some("run-1".into()));
crates/imp-core/src/trust.rs:82:    pub fn mana_record(kind: ManaRecordKind, unit_id: impl Into<String>) -> Self {
crates/imp-core/src/trust.rs:84:        let mut provenance = Self::new(ProvenanceSource::ManaRecord {
crates/imp-core/src/trust.rs:163:    ManaLedger,
crates/imp-core/src/trust.rs:185:            Self::ManaLedger => 7,
crates/imp-core/src/trust.rs:211:    ManaRecord {
crates/imp-core/src/trust.rs:212:        kind: ManaRecordKind,
crates/imp-core/src/trust.rs:233:            Self::ManaRecord { .. } => TrustLabel::ManaLedger,
crates/imp-core/src/trust.rs:251:            Self::ManaRecord { .. } => vec![RiskLabel::DurableLedger],
crates/imp-core/src/trust.rs:312:pub enum ManaRecordKind {
crates/imp-core/src/trust.rs:327:    ManaLedger,
crates/imp-core/src/trust.rs:345:            ProvenanceSource::ManaRecord { .. } => Self::ManaLedger,
crates/imp-core/src/trust.rs:400:            Provenance::mana_record(ManaRecordKind::Fact, "394.8").trust,
crates/imp-core/src/trust.rs:401:            TrustLabel::ManaLedger
crates/imp-core/src/agent/loop_state.rs:96:    ManaWorkflowProgress,
crates/imp-core/src/agent/loop_state.rs:111:            Self::ManaWorkflowProgress => "mana_workflow_progress",
crates/imp-core/src/workflow_profiles.rs:283:                tools: Some(vec!["read".into(), "rg".into(), "mana".into()]),
crates/imp-core/src/workflow/child_workflow.rs:45:    pub mana_unit_ref: Option<String>,
crates/imp-core/src/workflow/child_workflow.rs:553:        mana_unit_ref: parent.mana_unit_ref.clone(),
crates/imp-core/src/workflow/child_workflow.rs:758:            mana_unit_ref: Some("394.13".into()),
crates/imp-core/src/workflow/child_workflow.rs:775:        assert_eq!(contract.mana_unit_ref.as_deref(), Some("394.13"));
crates/imp-core/src/workflow/child_workflow.rs:838:                mana_unit_ref: Some("394.13".into()),
crates/imp-core/src/workflow/child_workflow.rs:881:        assert_eq!(decoded.spec.parent.mana_unit_ref.as_deref(), Some("394.13"));
crates/imp-core/src/tools/workflow.rs:2250:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/workflow.rs:2251:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-tui/src/interactive.rs:6:use imp_core::session::SessionManager;
crates/imp-tui/src/interactive.rs:37:        session: SessionManager,
crates/imp-core/src/tools/web/mod.rs:436:            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/web/mod.rs:437:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/mana_review.rs:7:pub enum ManaReviewState {
crates/imp-core/src/mana_review.rs:13:impl ManaReviewState {
crates/imp-core/src/mana_review.rs:25:pub enum ManaReviewScopeKind {
crates/imp-core/src/mana_review.rs:34:pub struct ManaReviewScope {
crates/imp-core/src/mana_review.rs:35:    pub kind: ManaReviewScopeKind,
crates/imp-core/src/mana_review.rs:39:impl Default for ManaReviewScope {
crates/imp-core/src/mana_review.rs:42:            kind: ManaReviewScopeKind::None,
crates/imp-core/src/mana_review.rs:49:pub struct ManaUnitRef {
crates/imp-core/src/mana_review.rs:56:impl ManaUnitRef {
crates/imp-core/src/mana_review.rs:57:    pub fn from_snapshot(unit: &ManaUnitSnapshot) -> Self {
crates/imp-core/src/mana_review.rs:75:pub struct ManaUnitSnapshot {
crates/imp-core/src/mana_review.rs:78:    pub kind: ManaReviewUnitKind,
crates/imp-core/src/mana_review.rs:100:impl ManaUnitSnapshot {
crates/imp-core/src/mana_review.rs:101:    pub fn unit_ref(&self) -> ManaUnitRef {
crates/imp-core/src/mana_review.rs:102:        ManaUnitRef::from_snapshot(self)
crates/imp-core/src/mana_review.rs:108:pub enum ManaReviewUnitKind {
crates/imp-core/src/mana_review.rs:114:impl ManaReviewUnitKind {
crates/imp-core/src/mana_review.rs:134:pub enum ManaMutationAction {
crates/imp-core/src/mana_review.rs:149:impl ManaMutationAction {
crates/imp-core/src/mana_review.rs:170:pub enum ManaTouchKind {
crates/imp-core/src/mana_review.rs:182:pub enum ManaUnitOrigin {
crates/imp-core/src/mana_review.rs:189:pub enum ManaUnitRole {
crates/imp-core/src/mana_review.rs:199:pub enum ManaAnchorKind {
crates/imp-core/src/mana_review.rs:206:pub enum ManaAnchorReason {
crates/imp-core/src/mana_review.rs:214:pub struct TurnManaAnchorUnit {
crates/imp-core/src/mana_review.rs:215:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:216:    pub anchor_kind: ManaAnchorKind,
crates/imp-core/src/mana_review.rs:217:    pub reason: ManaAnchorReason,
crates/imp-core/src/mana_review.rs:221:pub struct TurnManaTouchedUnit {
crates/imp-core/src/mana_review.rs:222:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:223:    pub touch_kind: ManaTouchKind,
crates/imp-core/src/mana_review.rs:224:    pub unit_origin: ManaUnitOrigin,
crates/imp-core/src/mana_review.rs:226:    pub roles: Vec<ManaUnitRole>,
crates/imp-core/src/mana_review.rs:230:pub struct TurnManaProposedChild {
crates/imp-core/src/mana_review.rs:231:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:232:    pub parent: ManaUnitRef,
crates/imp-core/src/mana_review.rs:233:    pub child_kind: ManaReviewUnitKind,
crates/imp-core/src/mana_review.rs:234:    pub child_origin: ManaUnitOrigin,
crates/imp-core/src/mana_review.rs:239:pub enum ManaFieldChangeKind {
crates/imp-core/src/mana_review.rs:247:pub struct TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:248:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:250:    pub change_kind: ManaFieldChangeKind,
crates/imp-core/src/mana_review.rs:259:pub struct TurnManaNoteAppend {
crates/imp-core/src/mana_review.rs:260:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:267:pub enum ManaDecisionEventKind {
crates/imp-core/src/mana_review.rs:273:pub struct TurnManaDecisionEvent {
crates/imp-core/src/mana_review.rs:274:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:275:    pub event_kind: ManaDecisionEventKind,
crates/imp-core/src/mana_review.rs:282:pub enum ManaConsequentialChoiceCategory {
crates/imp-core/src/mana_review.rs:292:pub struct TurnManaConsequentialChoice {
crates/imp-core/src/mana_review.rs:293:    pub unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:295:    pub category: ManaConsequentialChoiceCategory,
crates/imp-core/src/mana_review.rs:302:pub struct TurnManaReview {
crates/imp-core/src/mana_review.rs:304:    pub state: ManaReviewState,
crates/imp-core/src/mana_review.rs:305:    pub scope: ManaReviewScope,
crates/imp-core/src/mana_review.rs:307:    pub anchor_unit: Option<TurnManaAnchorUnit>,
crates/imp-core/src/mana_review.rs:309:    pub touched_units: Vec<TurnManaTouchedUnit>,
crates/imp-core/src/mana_review.rs:311:    pub proposed_children: Vec<TurnManaProposedChild>,
crates/imp-core/src/mana_review.rs:313:    pub material_field_changes: Vec<TurnManaFieldChange>,
crates/imp-core/src/mana_review.rs:315:    pub notes_appended: Vec<TurnManaNoteAppend>,
crates/imp-core/src/mana_review.rs:317:    pub decision_events: Vec<TurnManaDecisionEvent>,
crates/imp-core/src/mana_review.rs:319:    pub unresolved_consequential_choices: Vec<TurnManaConsequentialChoice>,
crates/imp-core/src/mana_review.rs:324:impl TurnManaReview {
crates/imp-core/src/mana_review.rs:328:            state: ManaReviewState::NoChange,
crates/imp-core/src/mana_review.rs:329:            scope: ManaReviewScope::default(),
crates/imp-core/src/mana_review.rs:343:pub struct ManaMutationRecord {
crates/imp-core/src/mana_review.rs:344:    pub action: ManaMutationAction,
crates/imp-core/src/mana_review.rs:345:    pub scope: ManaReviewScope,
crates/imp-core/src/mana_review.rs:347:    pub before_unit: Option<ManaUnitSnapshot>,
crates/imp-core/src/mana_review.rs:349:    pub after_unit: Option<ManaUnitSnapshot>,
crates/imp-core/src/mana_review.rs:351:    pub deleted_unit: Option<ManaUnitRef>,
crates/imp-core/src/mana_review.rs:353:    pub parent_unit: Option<ManaUnitRef>,
crates/imp-core/src/mana_review.rs:355:    pub related_unit: Option<ManaUnitRef>,
crates/imp-core/src/mana_review.rs:357:    pub field_changes: Vec<TurnManaFieldChange>,
crates/imp-core/src/mana_review.rs:359:    pub notes_appended: Vec<TurnManaNoteAppend>,
crates/imp-core/src/mana_review.rs:361:    pub decision_events: Vec<TurnManaDecisionEvent>,
crates/imp-core/src/mana_review.rs:365:pub struct TurnManaReviewAccumulator {
crates/imp-core/src/mana_review.rs:367:    mutations: Vec<ManaMutationRecord>,
crates/imp-core/src/mana_review.rs:370:impl TurnManaReviewAccumulator {
crates/imp-core/src/mana_review.rs:376:    pub fn push(&mut self, record: ManaMutationRecord) {
crates/imp-core/src/mana_review.rs:380:    pub fn finalize(&self) -> TurnManaReview {
crates/imp-core/src/mana_review.rs:382:            return TurnManaReview::no_change(self.turn_index);
crates/imp-core/src/mana_review.rs:390:                ManaMutationAction::Delete => {
crates/imp-core/src/mana_review.rs:396:                        aggregate.touch_actions.insert(ManaTouchKind::Deleted);
crates/imp-core/src/mana_review.rs:397:                        aggregate.roles.insert(ManaUnitRole::DirectTarget);
crates/imp-core/src/mana_review.rs:425:                    aggregate.roles.insert(ManaUnitRole::DirectTarget);
crates/imp-core/src/mana_review.rs:426:                    if matches!(unit.kind, ManaReviewUnitKind::Fact) {
crates/imp-core/src/mana_review.rs:427:                        aggregate.roles.insert(ManaUnitRole::Fact);
crates/imp-core/src/mana_review.rs:431:                        aggregate.roles.insert(ManaUnitRole::Child);
crates/imp-core/src/mana_review.rs:434:                            ManaUnitRef::new(
crates/imp-core/src/mana_review.rs:440:                        aggregate.roles.insert(ManaUnitRole::Child);
crates/imp-core/src/mana_review.rs:444:                        ManaMutationAction::Create => {
crates/imp-core/src/mana_review.rs:446:                            aggregate.touch_actions.insert(ManaTouchKind::Created);
crates/imp-core/src/mana_review.rs:448:                        ManaMutationAction::FactCreate => {
crates/imp-core/src/mana_review.rs:450:                            aggregate.touch_actions.insert(ManaTouchKind::FactCreated);
crates/imp-core/src/mana_review.rs:451:                            aggregate.roles.insert(ManaUnitRole::Fact);
crates/imp-core/src/mana_review.rs:453:                        ManaMutationAction::Close => {
crates/imp-core/src/mana_review.rs:454:                            aggregate.touch_actions.insert(ManaTouchKind::Closed);
crates/imp-core/src/mana_review.rs:456:                        ManaMutationAction::Reopen => {
crates/imp-core/src/mana_review.rs:457:                            aggregate.touch_actions.insert(ManaTouchKind::Reopened);
crates/imp-core/src/mana_review.rs:459:                        ManaMutationAction::Fail => {
crates/imp-core/src/mana_review.rs:460:                            aggregate.touch_actions.insert(ManaTouchKind::Failed);
crates/imp-core/src/mana_review.rs:463:                            aggregate.touch_actions.insert(ManaTouchKind::Updated);
crates/imp-core/src/mana_review.rs:473:                aggregate.roles.insert(ManaUnitRole::Related);
crates/imp-core/src/mana_review.rs:527:                touched_units.push(TurnManaTouchedUnit {
crates/imp-core/src/mana_review.rs:531:                        ManaUnitOrigin::CreatedInTurn
crates/imp-core/src/mana_review.rs:533:                        ManaUnitOrigin::Preexisting
crates/imp-core/src/mana_review.rs:542:                            ManaReviewUnitKind::Epic | ManaReviewUnitKind::Job
crates/imp-core/src/mana_review.rs:544:                            proposed_children.push(TurnManaProposedChild {
crates/imp-core/src/mana_review.rs:548:                                child_origin: ManaUnitOrigin::CreatedInTurn,
crates/imp-core/src/mana_review.rs:606:            ManaReviewState::NoChange
crates/imp-core/src/mana_review.rs:608:            ManaReviewState::Changed
crates/imp-core/src/mana_review.rs:610:            ManaReviewState::NeedsDecision
crates/imp-core/src/mana_review.rs:613:        TurnManaReview {
crates/imp-core/src/mana_review.rs:631:    unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:633:    change_kind: ManaFieldChangeKind,
crates/imp-core/src/mana_review.rs:641:    display_unit: ManaUnitRef,
crates/imp-core/src/mana_review.rs:642:    first_before: Option<ManaUnitSnapshot>,
crates/imp-core/src/mana_review.rs:643:    latest_after: Option<ManaUnitSnapshot>,
crates/imp-core/src/mana_review.rs:644:    parent_unit: Option<ManaUnitRef>,
crates/imp-core/src/mana_review.rs:647:    touch_actions: BTreeSet<ManaTouchKind>,
crates/imp-core/src/mana_review.rs:648:    roles: BTreeSet<ManaUnitRole>,
crates/imp-core/src/mana_review.rs:652:    notes: Vec<TurnManaNoteAppend>,
crates/imp-core/src/mana_review.rs:655:    decision_event_order: Vec<TurnManaDecisionEvent>,
crates/imp-core/src/mana_review.rs:659:    fn new(display_unit: ManaUnitRef) -> Self {
crates/imp-core/src/mana_review.rs:679:    fn display_unit_ref(&self) -> ManaUnitRef {
crates/imp-core/src/mana_review.rs:682:            .map(ManaUnitSnapshot::unit_ref)
crates/imp-core/src/mana_review.rs:686:    fn record_field_change(&mut self, change: TurnManaFieldChange) {
crates/imp-core/src/mana_review.rs:691:                        ManaFieldChangeKind::Added => 1,
crates/imp-core/src/mana_review.rs:692:                        ManaFieldChangeKind::Removed => -1,
crates/imp-core/src/mana_review.rs:701:                        ManaFieldChangeKind::Added => 1,
crates/imp-core/src/mana_review.rs:702:                        ManaFieldChangeKind::Removed => -1,
crates/imp-core/src/mana_review.rs:729:    fn record_decision_event(&mut self, event: TurnManaDecisionEvent) {
crates/imp-core/src/mana_review.rs:731:            ManaDecisionEventKind::Added => {
crates/imp-core/src/mana_review.rs:737:            ManaDecisionEventKind::Resolved => {
crates/imp-core/src/mana_review.rs:747:    fn surviving_decision_events(&self) -> Vec<TurnManaDecisionEvent> {
crates/imp-core/src/mana_review.rs:764:                ManaDecisionEventKind::Added => {
crates/imp-core/src/mana_review.rs:771:                ManaDecisionEventKind::Resolved => {
crates/imp-core/src/mana_review.rs:783:    fn surviving_field_changes(&self) -> Vec<TurnManaFieldChange> {
crates/imp-core/src/mana_review.rs:790:            out.push(TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:802:                out.push(TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:805:                    change_kind: ManaFieldChangeKind::Added,
crates/imp-core/src/mana_review.rs:808:                    source_action: ManaMutationAction::Update.as_str().to_string(),
crates/imp-core/src/mana_review.rs:811:                out.push(TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:814:                    change_kind: ManaFieldChangeKind::Removed,
crates/imp-core/src/mana_review.rs:817:                    source_action: ManaMutationAction::Update.as_str().to_string(),
crates/imp-core/src/mana_review.rs:824:                out.push(TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:827:                    change_kind: ManaFieldChangeKind::Added,
crates/imp-core/src/mana_review.rs:830:                    source_action: ManaMutationAction::DepAdd.as_str().to_string(),
crates/imp-core/src/mana_review.rs:833:                out.push(TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:836:                    change_kind: ManaFieldChangeKind::Removed,
crates/imp-core/src/mana_review.rs:839:                    source_action: ManaMutationAction::DepRemove.as_str().to_string(),
crates/imp-core/src/mana_review.rs:847:    fn touch_kind(&self) -> ManaTouchKind {
crates/imp-core/src/mana_review.rs:849:            ManaTouchKind::Deleted
crates/imp-core/src/mana_review.rs:852:                Some(ManaReviewUnitKind::Fact) => ManaTouchKind::FactCreated,
crates/imp-core/src/mana_review.rs:853:                _ => ManaTouchKind::Created,
crates/imp-core/src/mana_review.rs:855:        } else if self.touch_actions.contains(&ManaTouchKind::Failed) {
crates/imp-core/src/mana_review.rs:856:            ManaTouchKind::Failed
crates/imp-core/src/mana_review.rs:857:        } else if self.touch_actions.contains(&ManaTouchKind::Reopened) {
crates/imp-core/src/mana_review.rs:858:            ManaTouchKind::Reopened
crates/imp-core/src/mana_review.rs:859:        } else if self.touch_actions.contains(&ManaTouchKind::Closed) {
crates/imp-core/src/mana_review.rs:860:            ManaTouchKind::Closed
crates/imp-core/src/mana_review.rs:862:            ManaTouchKind::Updated
crates/imp-core/src/mana_review.rs:867:fn summarize_scope(mutations: &[ManaMutationRecord]) -> ManaReviewScope {
crates/imp-core/src/mana_review.rs:868:    let unique: BTreeSet<(ManaReviewScopeKind, String)> = mutations
crates/imp-core/src/mana_review.rs:874:        return ManaReviewScope::default();
crates/imp-core/src/mana_review.rs:878:        return ManaReviewScope { kind, display };
crates/imp-core/src/mana_review.rs:881:    ManaReviewScope {
crates/imp-core/src/mana_review.rs:882:        kind: ManaReviewScopeKind::Mixed,
crates/imp-core/src/mana_review.rs:890:    proposed_children: &[TurnManaProposedChild],
crates/imp-core/src/mana_review.rs:891:    touched_units: &[TurnManaTouchedUnit],
crates/imp-core/src/mana_review.rs:892:) -> Option<TurnManaAnchorUnit> {
crates/imp-core/src/mana_review.rs:894:        let mut parent_counts: BTreeMap<String, (&ManaUnitRef, usize)> = BTreeMap::new();
crates/imp-core/src/mana_review.rs:911:            return Some(TurnManaAnchorUnit {
crates/imp-core/src/mana_review.rs:914:                    ManaAnchorKind::CreatedInTurn
crates/imp-core/src/mana_review.rs:916:                    ManaAnchorKind::ReusedExisting
crates/imp-core/src/mana_review.rs:919:                    ManaAnchorReason::CreatedParent
crates/imp-core/src/mana_review.rs:921:                    ManaAnchorReason::AttachedParent
crates/imp-core/src/mana_review.rs:929:        let reason = if touched.roles.contains(&ManaUnitRole::Fact) {
crates/imp-core/src/mana_review.rs:930:            ManaAnchorReason::PrimaryFact
crates/imp-core/src/mana_review.rs:932:            ManaAnchorReason::PrimaryTarget
crates/imp-core/src/mana_review.rs:934:        let anchor_kind = if matches!(touched.unit_origin, ManaUnitOrigin::CreatedInTurn) {
crates/imp-core/src/mana_review.rs:935:            ManaAnchorKind::CreatedInTurn
crates/imp-core/src/mana_review.rs:937:            ManaAnchorKind::ReusedExisting
crates/imp-core/src/mana_review.rs:939:        return Some(TurnManaAnchorUnit {
crates/imp-core/src/mana_review.rs:949:fn classify_unresolved_choices(unit: &ManaUnitSnapshot) -> Vec<TurnManaConsequentialChoice> {
crates/imp-core/src/mana_review.rs:957:    unit: &ManaUnitRef,
crates/imp-core/src/mana_review.rs:959:) -> Option<TurnManaConsequentialChoice> {
crates/imp-core/src/mana_review.rs:964:        &["mana", "imp", "root", "project", "ownership", "boundary"],
crates/imp-core/src/mana_review.rs:967:            ManaConsequentialChoiceCategory::OwnershipBoundary,
crates/imp-core/src/mana_review.rs:975:            ManaConsequentialChoiceCategory::Architecture,
crates/imp-core/src/mana_review.rs:983:            ManaConsequentialChoiceCategory::ExecutionLaunch,
crates/imp-core/src/mana_review.rs:988:            ManaConsequentialChoiceCategory::ScopeChange,
crates/imp-core/src/mana_review.rs:993:            ManaConsequentialChoiceCategory::PruneOrDelete,
crates/imp-core/src/mana_review.rs:1001:            ManaConsequentialChoiceCategory::Other,
crates/imp-core/src/mana_review.rs:1008:    Some(TurnManaConsequentialChoice {
crates/imp-core/src/mana_review.rs:1025:    fn scope() -> ManaReviewScope {
crates/imp-core/src/mana_review.rs:1026:        ManaReviewScope {
crates/imp-core/src/mana_review.rs:1027:            kind: ManaReviewScopeKind::Project,
crates/imp-core/src/mana_review.rs:1032:    fn unit(id: &str, title: &str, kind: ManaReviewUnitKind) -> ManaUnitSnapshot {
crates/imp-core/src/mana_review.rs:1033:        ManaUnitSnapshot {
crates/imp-core/src/mana_review.rs:1053:        let mut acc = TurnManaReviewAccumulator::default();
crates/imp-core/src/mana_review.rs:1056:        assert_eq!(review.state, ManaReviewState::NoChange);
crates/imp-core/src/mana_review.rs:1062:        let mut acc = TurnManaReviewAccumulator::default();
crates/imp-core/src/mana_review.rs:1064:        let created = unit("28.5", "child", ManaReviewUnitKind::Job);
crates/imp-core/src/mana_review.rs:1065:        acc.push(ManaMutationRecord {
crates/imp-core/src/mana_review.rs:1066:            action: ManaMutationAction::Create,
crates/imp-core/src/mana_review.rs:1071:            parent_unit: Some(ManaUnitRef::new("28", "parent", Some("epic".into()))),
crates/imp-core/src/mana_review.rs:1077:        acc.push(ManaMutationRecord {
crates/imp-core/src/mana_review.rs:1078:            action: ManaMutationAction::Delete,
crates/imp-core/src/mana_review.rs:1085:            field_changes: vec![TurnManaFieldChange {
crates/imp-core/src/mana_review.rs:1088:                change_kind: ManaFieldChangeKind::Set,
crates/imp-core/src/mana_review.rs:1097:        assert_eq!(review.state, ManaReviewState::NoChange);
crates/imp-core/src/mana_review.rs:1103:        let mut acc = TurnManaReviewAccumulator::default();
crates/imp-core/src/mana_review.rs:1105:        let mut after = unit("28", "boundary work", ManaReviewUnitKind::Epic);
crates/imp-core/src/mana_review.rs:1106:        after.decisions = vec!["Choose architecture boundary between mana and imp".into()];
crates/imp-core/src/mana_review.rs:1107:        acc.push(ManaMutationRecord {
crates/imp-core/src/mana_review.rs:1108:            action: ManaMutationAction::DecisionAdd,
crates/imp-core/src/mana_review.rs:1110:            before_unit: Some(unit("28", "boundary work", ManaReviewUnitKind::Epic)),
crates/imp-core/src/mana_review.rs:1117:            decision_events: vec![TurnManaDecisionEvent {
crates/imp-core/src/mana_review.rs:1119:                event_kind: ManaDecisionEventKind::Added,
crates/imp-core/src/mana_review.rs:1120:                decision_text: "Choose architecture boundary between mana and imp".into(),
crates/imp-core/src/mana_review.rs:1125:        assert_eq!(review.state, ManaReviewState::NeedsDecision);
crates/imp-core/src/agent/tool_execution.rs:17:    extract_file_path, mana_bash_equivalent_hint,
crates/imp-core/src/agent/tool_execution.rs:18:    mana_loop::{enrich_mana_result_details, evaluate_mana_policy},
crates/imp-core/src/agent/tool_execution.rs:74:        "mana" => args
crates/imp-core/src/agent/tool_execution.rs:77:            .map(|action| Provenance::mana_record(crate::trust::ManaRecordKind::Note, action))
crates/imp-core/src/agent/tool_execution.rs:133:                } else if matches!(name.as_str(), "bash" | "git" | "mana") {
crates/imp-core/src/agent/tool_execution.rs:343:            .or_else(|| self.workflow_contract().mana_unit_ref.clone());
crates/imp-core/src/agent/tool_execution.rs:416:                if let Some(hint) = mana_bash_equivalent_hint(command) {
crates/imp-core/src/agent/tool_execution.rs:440:        if tool_name == "mana" {
crates/imp-core/src/agent/tool_execution.rs:441:            let policy = evaluate_mana_policy(self.mode, &args);
crates/imp-core/src/agent/tool_execution.rs:446:                    .unwrap_or_else(|| "Mana action blocked by loop policy".to_string());
crates/imp-core/src/agent/tool_execution.rs:463:                    Some("mana_policy_blocked".to_string()),
crates/imp-core/src/agent/tool_execution.rs:513:                    turn_mana_review: self.turn_mana_review_accumulator(),
crates/imp-core/src/agent/tool_execution.rs:540:                        if tool_name == "mana" {
crates/imp-core/src/agent/tool_execution.rs:541:                            let policy = evaluate_mana_policy(self.mode, &args);
crates/imp-core/src/agent/tool_execution.rs:542:                            result.details = enrich_mana_result_details(result.details, &policy);
crates/imp-tui/src/views/editor.rs:40:/// Multi-line editor state with cursor management.
crates/imp-tui/src/views/editor.rs:421:    mana_scope_label: Option<String>,
crates/imp-tui/src/views/editor.rs:422:    mana_run_label: Option<String>,
crates/imp-tui/src/views/editor.rs:451:            mana_scope_label: None,
crates/imp-tui/src/views/editor.rs:452:            mana_run_label: None,
crates/imp-tui/src/views/editor.rs:525:    pub fn mana_scope_label(mut self, label: Option<String>) -> Self {
crates/imp-tui/src/views/editor.rs:526:        self.mana_scope_label = label;
crates/imp-tui/src/views/editor.rs:530:    pub fn mana_run_label(mut self, label: Option<String>) -> Self {
crates/imp-tui/src/views/editor.rs:531:        self.mana_run_label = label;
crates/imp-tui/src/views/editor.rs:574:            self.mana_scope_label.as_deref(),
crates/imp-tui/src/views/editor.rs:575:            self.mana_run_label.as_deref(),
crates/imp-tui/src/views/editor.rs:814:    mana_scope_label: Option<&str>,
crates/imp-tui/src/views/editor.rs:815:    mana_run_label: Option<&str>,
crates/imp-tui/src/views/editor.rs:819:    if let Some(scope) = mana_scope_label.filter(|scope| !scope.trim().is_empty()) {
crates/imp-tui/src/views/editor.rs:822:    if let Some(run) = mana_run_label.filter(|label| !label.trim().is_empty()) {
crates/imp-tui/Cargo.toml:15:mana-ui = ["dep:mana-core", "imp-core/mana-tool"]
crates/imp-tui/Cargo.toml:35:mana-core = { workspace = true, optional = true }
crates/imp-core/src/tools/read.rs:500:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/read.rs:501:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/agent/mana_loop.rs:2:pub enum ManaActionClass {
crates/imp-core/src/agent/mana_loop.rs:14:pub fn classify_mana_action(action: &str) -> ManaActionClass {
crates/imp-core/src/agent/mana_loop.rs:16:        "guide" | "template" => ManaActionClass::ReadHelp,
crates/imp-core/src/agent/mana_loop.rs:18:            ManaActionClass::Inspect
crates/imp-core/src/agent/mana_loop.rs:20:        "update" | "notes_append" => ManaActionClass::ProgressCheckpoint,
crates/imp-core/src/agent/mana_loop.rs:21:        "create" | "dep_add" | "dep_remove" | "reparent" => ManaActionClass::GraphMutation,
crates/imp-core/src/agent/mana_loop.rs:23:            ManaActionClass::DecisionFact
crates/imp-core/src/agent/mana_loop.rs:25:        "claim" | "release" | "verify" | "close" | "reopen" | "fail" => ManaActionClass::Lifecycle,
crates/imp-core/src/agent/mana_loop.rs:26:        "run" | "evaluate" | "run_state" => ManaActionClass::Orchestration,
crates/imp-core/src/agent/mana_loop.rs:27:        "delete" => ManaActionClass::Destructive,
crates/imp-core/src/agent/mana_loop.rs:28:        _ => ManaActionClass::Unknown,
crates/imp-core/src/agent/mana_loop.rs:33:pub struct ManaPolicyDecision {
crates/imp-core/src/agent/mana_loop.rs:35:    pub class: ManaActionClass,
crates/imp-core/src/agent/mana_loop.rs:40:impl ManaPolicyDecision {
crates/imp-core/src/agent/mana_loop.rs:43:            "mana_loop_policy": {
crates/imp-core/src/agent/mana_loop.rs:53:impl ManaActionClass {
crates/imp-core/src/agent/mana_loop.rs:69:pub fn evaluate_mana_policy(
crates/imp-core/src/agent/mana_loop.rs:72:) -> ManaPolicyDecision {
crates/imp-core/src/agent/mana_loop.rs:79:        .map(classify_mana_action)
crates/imp-core/src/agent/mana_loop.rs:80:        .unwrap_or(ManaActionClass::Unknown);
crates/imp-core/src/agent/mana_loop.rs:83:        if matches!(action_name, "guide" | "template") || mode.allows_mana_action(action_name) {
crates/imp-core/src/agent/mana_loop.rs:87:                "Mana action '{action_name}' is not available in {} mode",
crates/imp-core/src/agent/mana_loop.rs:93:    ManaPolicyDecision {
crates/imp-core/src/agent/mana_loop.rs:101:pub fn enrich_mana_result_details(
crates/imp-core/src/agent/mana_loop.rs:103:    policy: &ManaPolicyDecision,
crates/imp-core/src/agent/mana_loop.rs:107:        .get("mana_loop_policy")
crates/imp-core/src/agent/mana_loop.rs:114:                "mana_action_class".to_string(),
crates/imp-core/src/agent/mana_loop.rs:117:            object.insert("mana_loop_policy".to_string(), policy_value);
crates/imp-core/src/agent/mana_loop.rs:121:            "mana_action_class": policy.class.as_str(),
crates/imp-core/src/agent/mana_loop.rs:122:            "mana_loop_policy": policy_value,
crates/imp-core/src/agent/mana_loop.rs:126:            "mana_action_class": policy.class.as_str(),
crates/imp-core/src/agent/mana_loop.rs:127:            "mana_loop_policy": policy_value,
crates/imp-core/src/agent/mana_loop.rs:132:#[cfg(all(test, feature = "mana-tool"))]
crates/imp-core/src/agent/mana_loop.rs:135:    use crate::tools::{mana::ManaTool, Tool};
crates/imp-core/src/agent/mana_loop.rs:138:    fn mana_action_classification_covers_tool_schema_actions() {
crates/imp-core/src/agent/mana_loop.rs:139:        let schema = ManaTool::default().parameters();
crates/imp-core/src/agent/mana_loop.rs:145:            .expect("mana action enum should be present in schema");
crates/imp-core/src/agent/mana_loop.rs:149:            let action = action.as_str().expect("mana action should be a string");
crates/imp-core/src/agent/mana_loop.rs:150:            if classify_mana_action(action) == ManaActionClass::Unknown {
crates/imp-core/src/agent/mana_loop.rs:157:            "unclassified mana schema actions: {}",
crates/imp-core/src/agent/mana_loop.rs:163:    fn mana_action_classification_groups_actions_by_loop_purpose() {
crates/imp-core/src/agent/mana_loop.rs:165:            ("guide", ManaActionClass::ReadHelp),
crates/imp-core/src/agent/mana_loop.rs:166:            ("template", ManaActionClass::ReadHelp),
crates/imp-core/src/agent/mana_loop.rs:167:            ("show", ManaActionClass::Inspect),
crates/imp-core/src/agent/mana_loop.rs:168:            ("update", ManaActionClass::ProgressCheckpoint),
crates/imp-core/src/agent/mana_loop.rs:169:            ("notes_append", ManaActionClass::ProgressCheckpoint),
crates/imp-core/src/agent/mana_loop.rs:170:            ("create", ManaActionClass::GraphMutation),
crates/imp-core/src/agent/mana_loop.rs:171:            ("reparent", ManaActionClass::GraphMutation),
crates/imp-core/src/agent/mana_loop.rs:172:            ("decision_add", ManaActionClass::DecisionFact),
crates/imp-core/src/agent/mana_loop.rs:173:            ("fact_verify", ManaActionClass::DecisionFact),
crates/imp-core/src/agent/mana_loop.rs:174:            ("verify", ManaActionClass::Lifecycle),
crates/imp-core/src/agent/mana_loop.rs:175:            ("close", ManaActionClass::Lifecycle),
crates/imp-core/src/agent/mana_loop.rs:176:            ("run", ManaActionClass::Orchestration),
crates/imp-core/src/agent/mana_loop.rs:177:            ("evaluate", ManaActionClass::Orchestration),
crates/imp-core/src/agent/mana_loop.rs:178:            ("run_state", ManaActionClass::Orchestration),
crates/imp-core/src/agent/mana_loop.rs:179:            ("delete", ManaActionClass::Destructive),
crates/imp-core/src/agent/mana_loop.rs:180:            ("not_real", ManaActionClass::Unknown),
crates/imp-core/src/agent/mana_loop.rs:184:            assert_eq!(classify_mana_action(action), expected, "{action}");
crates/imp-tui/src/views/mana_navigator.rs:5:use mana_core::api::{self, Unit};
crates/imp-tui/src/views/mana_navigator.rs:15:pub struct ManaTreeNode {
crates/imp-tui/src/views/mana_navigator.rs:32:pub struct ManaNavigatorState {
crates/imp-tui/src/views/mana_navigator.rs:33:    pub mana_dir: Option<PathBuf>,
crates/imp-tui/src/views/mana_navigator.rs:34:    pub nodes: Vec<ManaTreeNode>,
crates/imp-tui/src/views/mana_navigator.rs:45:impl ManaNavigatorState {
crates/imp-tui/src/views/mana_navigator.rs:49:            Err((mana_dir, message)) => Self::error(mana_dir, message),
crates/imp-tui/src/views/mana_navigator.rs:55:            mana_dir: api::find_mana_dir(cwd).ok(),
crates/imp-tui/src/views/mana_navigator.rs:72:        match api::find_mana_dir(cwd) {
crates/imp-tui/src/views/mana_navigator.rs:73:            Ok(mana_dir) => match load_nodes(&mana_dir) {
crates/imp-tui/src/views/mana_navigator.rs:76:                        mana_dir: Some(mana_dir),
crates/imp-tui/src/views/mana_navigator.rs:97:                Err(error) => Err((Some(mana_dir), format!("Failed to load mana tree: {error}"))),
crates/imp-tui/src/views/mana_navigator.rs:101:                format!("No .mana directory found from {}: {error}", cwd.display()),
crates/imp-tui/src/views/mana_navigator.rs:106:    pub fn error(mana_dir: Option<PathBuf>, message: String) -> Self {
crates/imp-tui/src/views/mana_navigator.rs:108:            mana_dir,
crates/imp-tui/src/views/mana_navigator.rs:177:    pub fn selected_node(&self) -> Option<&ManaTreeNode> {
crates/imp-tui/src/views/mana_navigator.rs:182:    pub fn visible_nodes(&self) -> Vec<&ManaTreeNode> {
crates/imp-tui/src/views/mana_navigator.rs:285:    fn ancestors_expanded(&self, node: &ManaTreeNode) -> bool {
crates/imp-tui/src/views/mana_navigator.rs:320:        let (Some(mana_dir), Some(id)) = (self.mana_dir.as_deref(), self.selected_id.as_deref())
crates/imp-tui/src/views/mana_navigator.rs:324:        match api::get_unit(mana_dir, id) {
crates/imp-tui/src/views/mana_navigator.rs:331:fn load_nodes(mana_dir: &Path) -> Result<Vec<ManaTreeNode>, Box<dyn std::error::Error>> {
crates/imp-tui/src/views/mana_navigator.rs:333:        mana_dir,
crates/imp-tui/src/views/mana_navigator.rs:334:        &mana_core::ops::list::ListParams {
crates/imp-tui/src/views/mana_navigator.rs:355:    by_parent: &BTreeMap<Option<String>, Vec<&mana_core::api::IndexEntry>>,
crates/imp-tui/src/views/mana_navigator.rs:356:    out: &mut Vec<ManaTreeNode>,
crates/imp-tui/src/views/mana_navigator.rs:365:        out.push(ManaTreeNode {
crates/imp-tui/src/views/mana_navigator.rs:386:impl ManaTreeNode {
crates/imp-tui/src/views/mana_navigator.rs:410:pub struct ManaNavigatorView<'a> {
crates/imp-tui/src/views/mana_navigator.rs:411:    state: &'a ManaNavigatorState,
crates/imp-tui/src/views/mana_navigator.rs:415:impl<'a> ManaNavigatorView<'a> {
crates/imp-tui/src/views/mana_navigator.rs:416:    pub fn new(state: &'a ManaNavigatorState, theme: &'a Theme) -> Self {
crates/imp-tui/src/views/mana_navigator.rs:421:impl Widget for ManaNavigatorView<'_> {
crates/imp-tui/src/views/mana_navigator.rs:429:            .mana_dir
crates/imp-tui/src/views/mana_navigator.rs:431:            .map(|path| format!(" Mana {} ", path.display()))
crates/imp-tui/src/views/mana_navigator.rs:432:            .unwrap_or_else(|| " Mana ".to_string());
crates/imp-tui/src/views/mana_navigator.rs:444:                &Line::from(Span::styled("Loading mana…", self.theme.muted_style())),
crates/imp-tui/src/views/mana_navigator.rs:475:        render_mana_list(columns[0], self.state, buf, self.theme);
crates/imp-tui/src/views/mana_navigator.rs:477:            render_mana_detail(columns[1], self.state, buf, self.theme);
crates/imp-tui/src/views/mana_navigator.rs:482:fn render_filter_bar(area: Rect, state: &ManaNavigatorState, buf: &mut Buffer, theme: &Theme) {
crates/imp-tui/src/views/mana_navigator.rs:503:fn render_mana_list(area: Rect, state: &ManaNavigatorState, buf: &mut Buffer, theme: &Theme) {
crates/imp-tui/src/views/mana_navigator.rs:509:            &Line::from(Span::styled("No mana units", theme.muted_style())),
crates/imp-tui/src/views/mana_navigator.rs:565:fn render_mana_detail(area: Rect, state: &ManaNavigatorState, buf: &mut Buffer, theme: &Theme) {
crates/imp-tui/src/views/mana_navigator.rs:683:    fn node(id: &str, parent: Option<&str>, child_count: usize, depth: usize) -> ManaTreeNode {
crates/imp-tui/src/views/mana_navigator.rs:684:        ManaTreeNode {
crates/imp-tui/src/views/mana_navigator.rs:703:        let mut state = ManaNavigatorState {
crates/imp-tui/src/views/mana_navigator.rs:704:            mana_dir: None,
crates/imp-tui/src/views/mana_navigator.rs:724:        let mut state = ManaNavigatorState {
crates/imp-tui/src/views/mana_navigator.rs:725:            mana_dir: None,
crates/imp-tui/src/views/tool_output.rs:24:        "mana" => styled_mana_output(tc, theme),
crates/imp-tui/src/views/tool_output.rs:51:        "mana" => tool_card_output(
crates/imp-tui/src/views/tool_output.rs:52:            "Mana",
crates/imp-tui/src/views/tool_output.rs:55:            styled_plain_output_with(tc, theme, mana_line_style),
crates/imp-tui/src/views/tool_output.rs:788:fn styled_mana_output(tc: &DisplayToolCall, theme: &Theme) -> Vec<Line<'static>> {
crates/imp-tui/src/views/tool_output.rs:789:    styled_plain_output_with(tc, theme, mana_line_style)
crates/imp-tui/src/views/tool_output.rs:1311:fn mana_line_style(line: &str, theme: &Theme, is_error: bool) -> Style {
crates/imp-tui/src/views/tool_output.rs:1317:    if line.starts_with("mana delta") || trimmed.starts_with('✓') || trimmed.starts_with("done") {
crates/imp-core/src/tools/shell.rs:422:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/shell.rs:423:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-tui/src/views/sidebar.rs:761:        "mana" => summarize_named_fields(
crates/imp-tui/src/views/sidebar.rs:937:    if tc.name == "mana" {
crates/imp-tui/src/views/sidebar.rs:938:        let raw_lines = format_mana_output(tc);
crates/imp-tui/src/views/sidebar.rs:1050:fn format_mana_output(tc: &DisplayToolCall) -> Vec<String> {
crates/imp-tui/src/views/sidebar.rs:1063:            "create" => push_mana_request_fields(
crates/imp-tui/src/views/sidebar.rs:1076:            "update" => push_mana_request_fields(
crates/imp-tui/src/views/sidebar.rs:1081:            "run" => push_mana_request_fields(
crates/imp-tui/src/views/sidebar.rs:1099:                push_mana_request_fields(&mut lines, tc, &["id", "reason", "unit"])
crates/imp-tui/src/views/sidebar.rs:1101:            "notes_append" | "decision_add" | "decision_resolve" => push_mana_request_fields(
crates/imp-tui/src/views/sidebar.rs:1107:                push_mana_request_fields(&mut lines, tc, &["from_id", "dep_id"])
crates/imp-tui/src/views/sidebar.rs:1109:            "delete" => push_mana_request_fields(&mut lines, tc, &["id", "title"]),
crates/imp-tui/src/views/sidebar.rs:1110:            "fact_create" => push_mana_request_fields(&mut lines, tc, &["unit_id", "unit"]),
crates/imp-tui/src/views/sidebar.rs:1111:            _ => push_mana_request_fields(
crates/imp-tui/src/views/sidebar.rs:1119:    if has_live_mana_output(tc) {
crates/imp-tui/src/views/sidebar.rs:1133:            lines.push(format!("  {}", format_mana_summary(summary)));
crates/imp-tui/src/views/sidebar.rs:1142:                push_mana_unit_lines(&mut lines, unit);
crates/imp-tui/src/views/sidebar.rs:1160:fn has_live_mana_output(tc: &DisplayToolCall) -> bool {
crates/imp-tui/src/views/sidebar.rs:1170:fn push_mana_request_fields(lines: &mut Vec<String>, tc: &DisplayToolCall, keys: &[&str]) {
crates/imp-tui/src/views/sidebar.rs:1172:        push_mana_detail_line(lines, key, tc.details.get(*key));
crates/imp-tui/src/views/sidebar.rs:1176:fn format_mana_summary(summary: &Value) -> String {
crates/imp-tui/src/views/sidebar.rs:1214:fn push_mana_unit_lines(lines: &mut Vec<String>, unit: &Value) {
crates/imp-tui/src/views/sidebar.rs:1249:fn push_mana_detail_line(lines: &mut Vec<String>, key: &str, value: Option<&Value>) {
crates/imp-tui/src/views/sidebar.rs:1518:    fn format_mana_output_renders_summary_and_units() {
crates/imp-tui/src/views/sidebar.rs:1521:            name: "mana".into(),
crates/imp-tui/src/views/sidebar.rs:1548:        let lines = format_mana_output(&tc);
crates/imp-tui/src/views/sidebar.rs:1566:    fn format_mana_output_renders_scope_target_and_runtime() {
crates/imp-tui/src/views/sidebar.rs:1569:            name: "mana".into(),
crates/imp-tui/src/views/sidebar.rs:1595:        let lines = format_mana_output(&tc);
crates/imp-tui/src/views/sidebar.rs:1602:    fn format_mana_output_renders_delta_actions() {
crates/imp-tui/src/views/sidebar.rs:1605:            name: "mana".into(),
crates/imp-tui/src/views/sidebar.rs:1607:            output: Some("mana delta: decision added on 1 · Test unit".into()),
crates/imp-tui/src/views/sidebar.rs:1625:        let lines = format_mana_output(&tc);
crates/imp-tui/src/views/sidebar.rs:1634:            .any(|l| l.contains("mana delta: decision added on 1 · Test unit")));
crates/imp-core/src/tools/mod.rs:7:#[cfg(feature = "mana-tool")]
crates/imp-core/src/tools/mod.rs:8:pub mod mana;
crates/imp-core/src/tools/mod.rs:33:use crate::mana_review::TurnManaReviewAccumulator;
crates/imp-core/src/tools/mod.rs:226:    /// Turn-scoped runtime accumulator for between-turn mana review packets.
crates/imp-core/src/tools/mod.rs:227:    pub turn_mana_review: Arc<std::sync::Mutex<TurnManaReviewAccumulator>>,
crates/imp-core/src/agent/mod.rs:18:use crate::mana_review::TurnManaReview;
crates/imp-core/src/agent/mod.rs:29:mod mana_loop;
crates/imp-core/src/agent/mod.rs:33:pub(crate) use mana_loop::ManaPolicyDecision;
crates/imp-core/src/agent/mod.rs:35:pub(crate) use mana_loop::{evaluate_mana_policy, ManaPolicyDecision};
crates/imp-core/src/agent/mod.rs:71:    ContinueRecommendation, ManaEvidence, PostTurnAssessment, RuntimeEvidence, TextFallbackEvidence,
crates/imp-core/src/agent/mod.rs:91:    /// Context management thresholds (wired from Config via AgentBuilder).
crates/imp-core/src/agent/mod.rs:133:    /// Optional host/workflow runtime layer for mana-backed obligations.
crates/imp-core/src/agent/mod.rs:268:        mana_review: &TurnManaReview,
crates/imp-core/src/agent/mod.rs:274:        let workflow_signals = self.workflow_post_turn_signals(tool_results, mana_review);
crates/imp-core/src/agent/mod.rs:277:        let mana_stop_reason = workflow_signals.stop_reason;
crates/imp-core/src/agent/mod.rs:348:            mana: ManaEvidence {
crates/imp-core/src/agent/mod.rs:349:                stop_reason: mana_stop_reason,
crates/imp-core/src/agent/mod.rs:377:            | ContinueReason::ManaWorkflowProgress
crates/imp-core/src/agent/mod.rs:417:            .or(self.workflow_contract().mana_unit_ref.as_ref())
crates/imp-core/src/agent/mod.rs:527:fn assistant_message_contains_mana_tool_call(message: &AssistantMessage) -> bool {
crates/imp-core/src/agent/mod.rs:529:        ContentBlock::ToolCall { name, .. } => name == "mana",
crates/imp-core/src/agent/mod.rs:560:    if !assistant_message_contains_mana_tool_call(message) {
crates/imp-core/src/agent/mod.rs:767:        if result.tool_name == "mana" {
crates/imp-core/src/agent/mod.rs:831:fn mana_bash_equivalent_hint(command: &str) -> Option<&'static str> {
crates/imp-core/src/agent/mod.rs:833:    let rest = trimmed.strip_prefix("mana")?;
crates/imp-core/src/agent/mod.rs:843:                "Mana is retired from the default workflow. Use native workflow actions instead, starting with `workflow(action=\"list\")`, `workflow(action=\"show\")`, or `workflow(action=\"run\")` as appropriate.",
crates/imp-core/src/agent/mod.rs:1016:            ManaEvidence, PostTurnAssessment, RuntimeEvidence, TextFallbackEvidence,
crates/imp-core/src/agent/mod.rs:1030:            mana: ManaEvidence { stop_reason: None },
crates/imp-core/src/agent/mod.rs:1059:            .record_mana_graph_changed();
crates/imp-core/src/agent/mod.rs:1554:    fn agent_queues_mana_hint_for_planner_requests() {
crates/imp-core/src/agent/mod.rs:1556:            text_response("Loaded mana skill", 100, 20),
crates/imp-core/src/agent/mod.rs:1562:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:1590:    async fn agent_queues_mana_externalization_follow_up_after_planning_turn() {
crates/imp-core/src/agent/mod.rs:1597:            text_response("Externalized into mana.", 120, 25),
crates/imp-core/src/agent/mod.rs:1602:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:1637:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:1679:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:1716:                tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:1729:            &TurnManaReview::no_change(0),
crates/imp-core/src/agent/mod.rs:1757:            mana: ManaEvidence { stop_reason: None },
crates/imp-core/src/agent/mod.rs:2208:    #[cfg(feature = "mana-tool")]
crates/imp-core/src/agent/mod.rs:2218:                    name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2222:                    text: "Done. Updated mana and next step is ready to continue.".to_string(),
crates/imp-core/src/agent/mod.rs:2229:                                name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2233:                                text: "Done. Updated mana and next step is ready to continue."
crates/imp-core/src/agent/mod.rs:2248:            text_response("Stopped after visible mana turn.", 120, 25),
crates/imp-core/src/agent/mod.rs:2257:            .register(Arc::new(crate::tools::mana::ManaTool::default()));
crates/imp-core/src/agent/mod.rs:2293:            mana: ManaEvidence {
crates/imp-core/src/agent/mod.rs:2326:            mana: ManaEvidence { stop_reason: None },
crates/imp-core/src/agent/mod.rs:2358:            mana: ManaEvidence { stop_reason: None },
crates/imp-core/src/agent/mod.rs:2379:    fn mana_planning_without_execution_creates_execution_debt_follow_up() {
crates/imp-core/src/agent/mod.rs:2381:            tool_call_id: "call_mana".to_string(),
crates/imp-core/src/agent/mod.rs:2382:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2433:        controller.record_mana_graph_changed();
crates/imp-core/src/agent/mod.rs:2450:    fn mana_run_status_result_extracts_terminal_status() {
crates/imp-core/src/agent/mod.rs:2452:            tool_call_id: "call_mana".to_string(),
crates/imp-core/src/agent/mod.rs:2453:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2468:            crate::agent::workflow_integration::mana_run_status_from_result(&result),
crates/imp-core/src/agent/mod.rs:2496:    fn mana_run_result_extracts_run_id_for_supervision() {
crates/imp-core/src/agent/mod.rs:2498:            tool_call_id: "call_mana".to_string(),
crates/imp-core/src/agent/mod.rs:2499:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2501:                text: "Started native mana orchestration".to_string(),
crates/imp-core/src/agent/mod.rs:2523:    fn mana_run_assessment_prefers_supervision_over_work_completed() {
crates/imp-core/src/agent/mod.rs:2530:            tool_call_id: "call_mana".to_string(),
crates/imp-core/src/agent/mod.rs:2531:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2533:                text: "Started native mana orchestration".to_string(),
crates/imp-core/src/agent/mod.rs:2552:            &TurnManaReview::no_change(0),
crates/imp-core/src/agent/mod.rs:2562:                        &TurnManaReview::no_change(0),
crates/imp-core/src/agent/mod.rs:2599:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2793:    fn mana_close_is_workflow_progress_not_runtime_completion() {
crates/imp-core/src/agent/mod.rs:2795:            tool_call_id: "call_mana".to_string(),
crates/imp-core/src/agent/mod.rs:2796:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2858:            tool_name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:2883:    fn mana_review_needs_decision_maps_to_user_blocker() {
crates/imp-core/src/agent/mod.rs:2884:        let review = TurnManaReview {
crates/imp-core/src/agent/mod.rs:2886:            state: crate::mana_review::ManaReviewState::NeedsDecision,
crates/imp-core/src/agent/mod.rs:2887:            scope: crate::mana_review::ManaReviewScope::default(),
crates/imp-core/src/agent/mod.rs:2913:    fn mana_review_changed_with_planner_children_maps_to_decomposition_completed() {
crates/imp-core/src/agent/mod.rs:2914:        let review = TurnManaReview {
crates/imp-core/src/agent/mod.rs:2916:            state: crate::mana_review::ManaReviewState::Changed,
crates/imp-core/src/agent/mod.rs:2917:            scope: crate::mana_review::ManaReviewScope::default(),
crates/imp-core/src/agent/mod.rs:2920:            proposed_children: vec![crate::mana_review::TurnManaProposedChild {
crates/imp-core/src/agent/mod.rs:2921:                unit: crate::mana_review::ManaUnitRef::new(
crates/imp-core/src/agent/mod.rs:2926:                parent: crate::mana_review::ManaUnitRef::new(
crates/imp-core/src/agent/mod.rs:2931:                child_kind: crate::mana_review::ManaReviewUnitKind::Job,
crates/imp-core/src/agent/mod.rs:2932:                child_origin: crate::mana_review::ManaUnitOrigin::CreatedInTurn,
crates/imp-core/src/agent/mod.rs:2958:            "Externalized into mana. Plan is complete and ready for handoff.",
crates/imp-core/src/agent/mod.rs:2966:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:2996:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:3077:                    || text.contains("Mana graph state changed")
crates/imp-core/src/agent/mod.rs:3083:    #[cfg(feature = "mana-tool")]
crates/imp-core/src/agent/mod.rs:3085:    async fn agent_queues_confidence_continue_follow_up_after_visible_mana_turn() {
crates/imp-core/src/agent/mod.rs:3093:                    name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:3097:                    text: "Done. Updated mana and next step is ready to continue.".to_string(),
crates/imp-core/src/agent/mod.rs:3104:                                name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:3108:                                text: "Done. Updated mana and next step is ready to continue."
crates/imp-core/src/agent/mod.rs:3132:            .register(Arc::new(crate::tools::mana::ManaTool::default()));
crates/imp-core/src/agent/mod.rs:3152:    #[cfg(feature = "mana-tool")]
crates/imp-core/src/agent/mod.rs:3162:                    name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:3166:                    text: "Done. Updated mana and next step is ready to continue.".to_string(),
crates/imp-core/src/agent/mod.rs:3173:                                name: "mana".to_string(),
crates/imp-core/src/agent/mod.rs:3177:                                text: "Done. Updated mana and next step is ready to continue."
crates/imp-core/src/agent/mod.rs:3192:            text_response("Stopped after visible mana turn.", 120, 25),
crates/imp-core/src/agent/mod.rs:3201:            .register(Arc::new(crate::tools::mana::ManaTool::default()));
crates/imp-core/src/agent/mod.rs:3220:    #[cfg(feature = "mana-tool")]
crates/imp-core/src/agent/mod.rs:3222:    async fn agent_does_not_queue_externalization_follow_up_after_mana_tool_turn() {
crates/imp-core/src/agent/mod.rs:3226:                "mana",
crates/imp-core/src/agent/mod.rs:3231:            text_response("Done after mana", 120, 25),
crates/imp-core/src/agent/mod.rs:3236:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:3240:            .register(Arc::new(crate::tools::mana::ManaTool::default()));
crates/imp-core/src/agent/mod.rs:3260:    async fn agent_queues_mana_basics_hint_for_worker_mana_requests() {
crates/imp-core/src/agent/mod.rs:3268:        agent.set_workflow_mana_basics_skill_available(true);
crates/imp-core/src/agent/mod.rs:3272:            .run("Check mana status and logs for my unit".to_string())
crates/imp-core/src/agent/mod.rs:3289:        assert_eq!(user_texts[0], "Check mana status and logs for my unit");
crates/imp-core/src/agent/mod.rs:3293:    async fn agent_does_not_queue_mana_hint_without_matching_signal() {
crates/imp-core/src/agent/mod.rs:3298:        agent.set_workflow_mana_skill_available(true);
crates/imp-core/src/agent/mod.rs:3325:    async fn agent_does_not_queue_mana_basics_hint_when_no_tools_available() {
crates/imp-core/src/agent/mod.rs:3334:        agent.set_workflow_mana_basics_skill_available(true);
crates/imp-core/src/agent/mod.rs:3339:            .run("Check mana status and logs for my unit".to_string())
crates/imp-core/src/agent/mod.rs:3357:            vec!["Check mana status and logs for my unit".to_string()]
crates/imp-core/src/agent/mod.rs:3367:        agent.set_workflow_mana_basics_skill_available(true);
crates/imp-core/src/agent/mod.rs:3371:        let result = agent.run("Check mana status and finish".to_string()).await;
crates/imp-core/src/agent/mod.rs:3906:    #[cfg(feature = "mana-tool")]
crates/imp-core/src/agent/mod.rs:3912:                "mana",
crates/imp-core/src/agent/mod.rs:3925:            .register(Arc::new(crate::tools::mana::ManaTool::default()));
crates/imp-core/src/agent/mod.rs:3944:    #[cfg(feature = "mana-tool")]
crates/imp-core/src/agent/mod.rs:3946:    async fn execution_stops_after_mana_close_tool_result_without_done_text() {
crates/imp-core/src/agent/mod.rs:3950:                "mana",
crates/imp-core/src/agent/mod.rs:3963:            .register(Arc::new(crate::tools::mana::ManaTool::default()));
crates/imp-core/src/agent/mod.rs:4026:                    &TurnManaReview::no_change(0),
crates/imp-core/src/agent/mod.rs:4078:                    &TurnManaReview::no_change(0),
crates/imp-core/src/agent/mod.rs:4200:    fn mana_bash_equivalent_hint_handles_release_and_tree() {
crates/imp-core/src/agent/mod.rs:4201:        assert!(mana_bash_equivalent_hint("mana release 1").is_some());
crates/imp-core/src/agent/mod.rs:4202:        assert!(mana_bash_equivalent_hint("mana tree").is_some());
crates/imp-core/src/agent/mod.rs:4206:    fn mana_bash_equivalent_hint_ignores_non_mana_prefixes() {
crates/imp-core/src/agent/mod.rs:4207:        assert!(mana_bash_equivalent_hint("manatee status").is_none());
crates/imp-core/src/agent/mod.rs:4208:        assert!(mana_bash_equivalent_hint("./mana status").is_none());
crates/imp-core/src/agent/mod.rs:4212:    async fn agent_blocks_bash_mana_when_native_action_exists() {
crates/imp-core/src/agent/mod.rs:4217:                serde_json::json!({"command": "mana status", "timeout": 5}),
crates/imp-core/src/agent/mod.rs:4251:            text.contains("native workflow") || text.contains("Mana is retired"),
crates/imp-core/src/agent/mod.rs:4257:    async fn agent_allows_non_mana_bash_commands() {
crates/imp-tui/src/views/mod.rs:6:#[cfg(feature = "mana-ui")]
crates/imp-tui/src/views/mod.rs:7:pub mod mana_navigator;
crates/imp-tui/src/views/tools.rs:231:            "mana" => format_mana_args(args),
crates/imp-tui/src/views/tools.rs:488:fn format_mana_args(args: &Value) -> String {
crates/imp-tui/src/views/tools.rs:687:        "mana" => "•",
crates/imp-tui/src/views/tools.rs:938:    fn make_args_summary_formats_mana_compactly() {
crates/imp-tui/src/views/tools.rs:940:            "mana",
crates/imp-cli/src/acp/mod.rs:7:use imp_core::session::{SessionEntry, SessionManager};
crates/imp-cli/src/acp/mod.rs:89:    session: SessionManager,
crates/imp-cli/src/acp/mod.rs:380:fn create_session(cwd: &Path) -> imp_core::Result<(String, PathBuf, SessionManager)> {
crates/imp-cli/src/acp/mod.rs:381:    let session = SessionManager::new(cwd, &imp_core::storage::global_sessions_dir())?;
crates/imp-cli/src/acp/mod.rs:458:fn load_session_by_id(session_id: &str) -> Result<SessionManager, String> {
crates/imp-cli/src/acp/mod.rs:460:    let session = SessionManager::open(&path).map_err(|error| error.to_string())?;
crates/imp-core/src/tools/edit.rs:521:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/edit.rs:522:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/tools/bash.rs:762:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/bash.rs:763:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/agent/workflow_integration.rs:4://! recipe/runtime support from mana/work-graph compatibility.
crates/imp-core/src/agent/workflow_integration.rs:6:mod mana_compat;
crates/imp-core/src/agent/workflow_integration.rs:10:pub(super) use mana_compat::mana_run_status_from_result;
crates/imp-core/src/agent/workflow_integration.rs:11:pub(crate) use mana_compat::orchestration_follow_up_text;
crates/imp-core/src/session.rs:168:/// Manages a single session's entries and persistence.
crates/imp-core/src/session.rs:176:pub struct SessionManager {
crates/imp-core/src/session.rs:193:impl SessionManager {
crates/imp-core/src/session.rs:877:    pub fn fork(&self, entry_id: &str, new_path: &Path) -> Result<SessionManager> {
crates/imp-core/src/session.rs:956:        Ok(SessionManager {
crates/imp-core/src/session.rs:2048:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2113:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2141:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2185:        let mut mgr = SessionManager::new(Path::new("/tmp"), tmp.path()).unwrap();
crates/imp-core/src/session.rs:2209:        let mut mgr = SessionManager::new(Path::new("/tmp"), tmp.path()).unwrap();
crates/imp-core/src/session.rs:2234:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2243:        let reopened = SessionManager::open(&path).unwrap();
crates/imp-core/src/session.rs:2265:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2307:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2322:        let reopened = SessionManager::open(&fork_path).unwrap();
crates/imp-core/src/session.rs:2333:        let mut s1 = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2337:        let mut s2 = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2342:        let sessions = SessionManager::list(&session_dir).unwrap();
crates/imp-core/src/session.rs:2373:        let mut old = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2377:        let mut new = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2381:        let sessions = SessionManager::list_page(&session_dir, 0, 1, None).unwrap();
crates/imp-core/src/session.rs:2392:        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2400:        let sessions = SessionManager::list(&session_dir).unwrap();
crates/imp-core/src/session.rs:2413:        let mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2416:        let sessions = SessionManager::list(&session_dir).unwrap();
crates/imp-core/src/session.rs:2431:        let mut s1 = SessionManager::new(&cwd_a, &session_dir).unwrap();
crates/imp-core/src/session.rs:2435:        let mut s2 = SessionManager::new(&cwd_b, &session_dir).unwrap();
crates/imp-core/src/session.rs:2439:        let continued = SessionManager::continue_recent(&cwd_a, &session_dir)
crates/imp-core/src/session.rs:2446:            SessionManager::continue_recent(Path::new("/nonexistent"), &session_dir).unwrap();
crates/imp-core/src/session.rs:2456:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2462:        let reopened = SessionManager::open(&path).unwrap();
crates/imp-core/src/session.rs:2472:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2516:        let mgr = SessionManager::open(&path).unwrap();
crates/imp-core/src/session.rs:2524:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2557:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2601:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2641:        let mut mgr = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-core/src/session.rs:2664:        let mut mgr = SessionManager::in_memory();
crates/imp-core/src/session.rs:2705:        let mut session = SessionManager::in_memory();
crates/imp-core/src/session.rs:2739:        let mut session = SessionManager::in_memory();
crates/imp-core/src/tools/ask.rs:285:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/ask.rs:286:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-tui/src/app.rs:16:use imp_core::ManaUnitRef;
crates/imp-tui/src/app.rs:17:#[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:18:use imp_core::{mana_run_summary, stop_mana_run, ManaRunSummary};
crates/imp-tui/src/app.rs:19:#[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:21:struct ManaRunSummary {
crates/imp-tui/src/app.rs:31:    agents: Vec<ManaRunAgentSummary>,
crates/imp-tui/src/app.rs:33:#[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:35:struct ManaRunAgentSummary {
crates/imp-tui/src/app.rs:41:#[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:43:fn mana_run_summary(_id: &str) -> Result<Option<ManaRunSummary>, String> {
crates/imp-tui/src/app.rs:46:#[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:47:fn stop_mana_run(_id: &str) -> Result<Option<ManaRunSummary>, String> {
crates/imp-tui/src/app.rs:50:#[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:51:use mana_core::api;
crates/imp-tui/src/app.rs:65:use imp_core::session::{SessionEntry, SessionInfo, SessionManager};
crates/imp-tui/src/app.rs:110:#[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:111:use crate::views::mana_navigator::{ManaNavigatorState, ManaNavigatorView};
crates/imp-tui/src/app.rs:162:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:163:    ManaNavigator(ManaNavigatorState),
crates/imp-tui/src/app.rs:203:    session: SessionManager,
crates/imp-tui/src/app.rs:290:    session: SessionManager,
crates/imp-tui/src/app.rs:296:    active_mana_scope: Option<ManaUnitRef>,
crates/imp-tui/src/app.rs:346:        persisted_session: Option<SessionManager>,
crates/imp-tui/src/app.rs:355:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:356:    ManaNavigatorLoaded(ManaNavigatorState),
crates/imp-tui/src/app.rs:357:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:358:    ManaNavigatorLoadFailed {
crates/imp-tui/src/app.rs:359:        mana_dir: Option<PathBuf>,
crates/imp-tui/src/app.rs:477:#[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:478:fn mana_run_summary_cache_key(run: &ManaRunSummary) -> String {
crates/imp-tui/src/app.rs:493:#[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:494:fn mana_run_detail_render_data(run: &ManaRunSummary, theme: &Theme) -> SidebarDetailRenderData {
crates/imp-tui/src/app.rs:498:            " mana run ",
crates/imp-tui/src/app.rs:505:        "Mana run details are unavailable in this standalone build.".to_string(),
crates/imp-tui/src/app.rs:510:#[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:511:fn mana_run_detail_render_data(run: &ManaRunSummary, theme: &Theme) -> SidebarDetailRenderData {
crates/imp-tui/src/app.rs:515:            " mana run ",
crates/imp-tui/src/app.rs:1207:    pub session: SessionManager,
crates/imp-tui/src/app.rs:1242:    active_mana_scope: Option<ManaUnitRef>,
crates/imp-tui/src/app.rs:1243:    active_mana_run: Option<ManaRunSummary>,
crates/imp-tui/src/app.rs:1251:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:1252:    mana_navigator_task: Option<tokio::task::JoinHandle<()>>,
crates/imp-tui/src/app.rs:1345:        #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:1346:        RuntimeSignal::ManaNavigatorLoaded(_) => "mana_navigator_loaded",
crates/imp-tui/src/app.rs:1347:        #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:1348:        RuntimeSignal::ManaNavigatorLoadFailed { .. } => "mana_navigator_load_failed",
crates/imp-tui/src/app.rs:1742:    if let Some(scope) = request.active_mana_scope.as_ref() {
crates/imp-tui/src/app.rs:1748:            context.push_str(&format!(" Active mana scope: {}.", scope.id));
crates/imp-tui/src/app.rs:1750:            context.push_str(&format!(" Active mana scope: {} — {}.", scope.id, title));
crates/imp-tui/src/app.rs:2216:        session: SessionManager,
crates/imp-tui/src/app.rs:2300:            active_mana_scope: None,
crates/imp-tui/src/app.rs:2301:            active_mana_run: None,
crates/imp-tui/src/app.rs:2309:            #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:2310:            mana_navigator_task: None,
crates/imp-tui/src/app.rs:2844:            #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:2845:            RuntimeSignal::ManaNavigatorLoaded(state) => self.finish_mana_navigator_load(state),
crates/imp-tui/src/app.rs:2846:            #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:2847:            RuntimeSignal::ManaNavigatorLoadFailed { mana_dir, message } => {
crates/imp-tui/src/app.rs:2848:                self.fail_mana_navigator_load(mana_dir, message);
crates/imp-tui/src/app.rs:3199:        _run: Option<&ManaRunSummary>,
crates/imp-tui/src/app.rs:3207:                #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:3209:                    stable_hash(&_run.map(mana_run_summary_cache_key))
crates/imp-tui/src/app.rs:3211:                #[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:3254:        run: Option<&ManaRunSummary>,
crates/imp-tui/src/app.rs:3263:                mana_run_detail_render_data(run, &self.theme)
crates/imp-tui/src/app.rs:3685:                        #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:3687:                            self.active_mana_run.clone()
crates/imp-tui/src/app.rs:3689:                        #[cfg(not(feature = "mana-ui"))]
crates/imp-tui/src/app.rs:3789:                .mana_scope_label(self.active_mana_scope_label())
crates/imp-tui/src/app.rs:3790:                .mana_run_label(self.active_mana_run_label())
crates/imp-tui/src/app.rs:3837:            #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:3838:            UiMode::ManaNavigator(state) => {
crates/imp-tui/src/app.rs:3839:                let mana_area = centered_rect(88, 86, area);
crates/imp-tui/src/app.rs:3840:                let view = ManaNavigatorView::new(state, &self.theme);
crates/imp-tui/src/app.rs:3841:                frame.render_widget(view, mana_area);
crates/imp-tui/src/app.rs:4045:            #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:4046:            UiMode::ManaNavigator(_) => self.handle_mana_navigator_key(key),
crates/imp-tui/src/app.rs:4391:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:4392:    fn handle_mana_navigator_key(&mut self, key: KeyEvent) {
crates/imp-tui/src/app.rs:4398:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4403:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4412:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4417:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4426:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4431:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4440:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4445:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4454:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4459:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4464:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4469:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4474:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4479:                if let UiMode::ManaNavigator(ref mut state) = self.mode {
crates/imp-tui/src/app.rs:4990:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:4991:    fn handle_mana_navigator_mouse(&mut self, mouse: &crossterm::event::MouseEvent) -> bool {
crates/imp-tui/src/app.rs:4992:        let UiMode::ManaNavigator(ref mut state) = self.mode else {
crates/imp-tui/src/app.rs:5002:        let mana_area = centered_rect(88, 86, terminal_area);
crates/imp-tui/src/app.rs:5003:        if !point_in_rect(mouse.column, mouse.row, Some(mana_area)) {
crates/imp-tui/src/app.rs:5008:            x: mana_area.x.saturating_add(1),
crates/imp-tui/src/app.rs:5009:            y: mana_area.y.saturating_add(1),
crates/imp-tui/src/app.rs:5010:            width: mana_area.width.saturating_sub(2),
crates/imp-tui/src/app.rs:5011:            height: mana_area.height.saturating_sub(2),
crates/imp-tui/src/app.rs:5057:        #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:5058:        if self.handle_mana_navigator_mouse(&mouse) {
crates/imp-tui/src/app.rs:5238:        if let Some(run_id) = self.active_mana_run.as_ref().map(|run| run.run_id.clone()) {
crates/imp-tui/src/app.rs:5239:            match stop_mana_run(&run_id) {
crates/imp-tui/src/app.rs:5241:                    self.active_mana_run = Some(summary);
crates/imp-tui/src/app.rs:5243:                        "Stopped active mana run {run_id}. External workers may need manual cleanup."
crates/imp-tui/src/app.rs:5247:                    self.push_system_msg(&format!("Active mana run {run_id} was not found."))
crates/imp-tui/src/app.rs:5250:                    self.push_system_msg(&format!("Could not stop mana run {run_id}: {err}"))
crates/imp-tui/src/app.rs:5382:        if let Some(scope) = self.active_mana_scope.as_ref() {
crates/imp-tui/src/app.rs:5384:                "Continue working on active mana scope {} until the requested outcome is complete, blocked, or no runnable work remains.",
crates/imp-tui/src/app.rs:5388:        #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:5389:        if let Some(run) = self.active_mana_run.as_ref() {
crates/imp-tui/src/app.rs:5391:                "Continue supervising active mana run {} until it is complete, blocked, or no runnable work remains.",
crates/imp-tui/src/app.rs:5470:            active_mana_scope: self.active_mana_scope.clone(),
crates/imp-tui/src/app.rs:5747:        session: SessionManager,
crates/imp-tui/src/app.rs:5794:        persisted_session: Option<SessionManager>,
crates/imp-tui/src/app.rs:5937:    fn active_mana_run_label(&self) -> Option<String> {
crates/imp-tui/src/app.rs:5938:        self.active_mana_run
crates/imp-tui/src/app.rs:5943:    fn active_mana_scope_label(&self) -> Option<String> {
crates/imp-tui/src/app.rs:5944:        self.active_mana_scope.as_ref().map(|scope| {
crates/imp-tui/src/app.rs:5952:                format!("mana {}", scope.id)
crates/imp-tui/src/app.rs:5954:                format!("mana {} {}", scope.id, title)
crates/imp-tui/src/app.rs:5959:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:5961:    fn set_active_mana_run(&mut self, id: &str) {
crates/imp-tui/src/app.rs:5964:            let Some(active_id) = self.active_mana_run.as_ref().map(|run| run.run_id.clone())
crates/imp-tui/src/app.rs:5969:            self.refresh_active_mana_run(&active_id);
crates/imp-tui/src/app.rs:5973:            self.active_mana_run = None;
crates/imp-tui/src/app.rs:5974:            self.push_system_msg("Active mana run cleared");
crates/imp-tui/src/app.rs:5978:        self.refresh_active_mana_run(id);
crates/imp-tui/src/app.rs:5981:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:5983:    fn refresh_active_mana_run(&mut self, id: &str) {
crates/imp-tui/src/app.rs:5984:        match mana_run_summary(id) {
crates/imp-tui/src/app.rs:5987:                    "Active mana run: {} {} ({}/{}, failed {})",
crates/imp-tui/src/app.rs:5994:                self.active_mana_run = Some(summary);
crates/imp-tui/src/app.rs:5996:            Ok(None) => self.push_system_msg(&format!("Could not find mana run {id}")),
crates/imp-tui/src/app.rs:5997:            Err(err) => self.push_system_msg(&format!("Could not read mana run {id}: {err}")),
crates/imp-tui/src/app.rs:6323:                self.session = SessionManager::in_memory();
crates/imp-tui/src/app.rs:7068:                SessionManager::list_page(&session_dir, 0, SESSION_LIST_PAGE_SIZE, None)
crates/imp-tui/src/app.rs:7099:                SessionManager::list_page(
crates/imp-tui/src/app.rs:7183:                let session = SessionManager::open(&path)
crates/imp-tui/src/app.rs:8116:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:8117:    fn open_mana_navigator(&mut self, initial_id: Option<&str>) {
crates/imp-tui/src/app.rs:8118:        self.mode = UiMode::ManaNavigator(ManaNavigatorState::loading(&self.cwd));
crates/imp-tui/src/app.rs:8119:        if self.mana_navigator_task.is_some() {
crates/imp-tui/src/app.rs:8125:        self.mana_navigator_task = Some(tokio::spawn(async move {
crates/imp-tui/src/app.rs:8127:                ManaNavigatorState::try_load(&cwd, initial_id.as_deref())
crates/imp-tui/src/app.rs:8131:                Ok(Ok(state)) => RuntimeSignal::ManaNavigatorLoaded(state),
crates/imp-tui/src/app.rs:8132:                Ok(Err((mana_dir, message))) => {
crates/imp-tui/src/app.rs:8133:                    RuntimeSignal::ManaNavigatorLoadFailed { mana_dir, message }
crates/imp-tui/src/app.rs:8135:                Err(error) => RuntimeSignal::ManaNavigatorLoadFailed {
crates/imp-tui/src/app.rs:8136:                    mana_dir: None,
crates/imp-tui/src/app.rs:8137:                    message: format!("Mana navigator task failure: {error}"),
crates/imp-tui/src/app.rs:8144:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:8145:    fn finish_mana_navigator_load(&mut self, state: ManaNavigatorState) {
crates/imp-tui/src/app.rs:8146:        self.mana_navigator_task = None;
crates/imp-tui/src/app.rs:8147:        if matches!(self.mode, UiMode::ManaNavigator(_)) {
crates/imp-tui/src/app.rs:8148:            self.mode = UiMode::ManaNavigator(state);
crates/imp-tui/src/app.rs:8152:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:8153:    fn fail_mana_navigator_load(&mut self, mana_dir: Option<PathBuf>, message: String) {
crates/imp-tui/src/app.rs:8154:        self.mana_navigator_task = None;
crates/imp-tui/src/app.rs:8155:        if matches!(self.mode, UiMode::ManaNavigator(_)) {
crates/imp-tui/src/app.rs:8156:            self.mode = UiMode::ManaNavigator(ManaNavigatorState::error(mana_dir, message));
crates/imp-tui/src/app.rs:8355:                &mut SessionManager::in_memory_with_messages(active_messages),
crates/imp-tui/src/app.rs:8873:                mana_review: _,
crates/imp-tui/src/app.rs:9017:    use imp_core::session::{SessionEntry, SessionManager};
crates/imp-tui/src/app.rs:9030:        let session = SessionManager::in_memory();
crates/imp-tui/src/app.rs:9036:    fn make_app_with_session(session: SessionManager, cwd: PathBuf) -> App {
crates/imp-tui/src/app.rs:9046:        let session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:9291:        let session = SessionManager::in_memory();
crates/imp-tui/src/app.rs:9568:        let mut app = make_app_with_session(SessionManager::in_memory(), cwd.clone());
crates/imp-tui/src/app.rs:9609:        let session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:9633:        let app = make_app_with_session(SessionManager::in_memory(), cwd);
crates/imp-tui/src/app.rs:9688:        let reopened = SessionManager::open(&session_path).unwrap();
crates/imp-tui/src/app.rs:9694:    #[cfg(feature = "mana-ui")]
crates/imp-tui/src/app.rs:9696:    async fn tui_integration_slash_mana_opens_navigator() {
crates/imp-tui/src/app.rs:9698:        app.execute_command("mana");
crates/imp-tui/src/app.rs:9699:        assert!(matches!(app.mode, UiMode::ManaNavigator(_)));
crates/imp-tui/src/app.rs:9703:    fn command_palette_omits_mana_command() {
crates/imp-tui/src/app.rs:9705:        assert!(!commands.iter().any(|cmd| cmd.name == "mana"));
crates/imp-tui/src/app.rs:9792:        let mut app = make_app_with_session(SessionManager::in_memory(), temp.path().to_path_buf());
crates/imp-tui/src/app.rs:9888:        let mut app = make_app_with_session(SessionManager::in_memory(), temp.path().to_path_buf());
crates/imp-tui/src/app.rs:9905:        let mut app = make_app_with_session(SessionManager::in_memory(), temp.path().to_path_buf());
crates/imp-tui/src/app.rs:10077:        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:10101:        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:10539:        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:10557:        let reloaded_session = SessionManager::open(&session_path).unwrap();
crates/imp-tui/src/app.rs:10578:        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:10589:        let continued = SessionManager::continue_recent(&cwd, &session_dir)
crates/imp-tui/src/app.rs:10765:        let app = make_app_with_session(SessionManager::in_memory(), cwd.clone());
crates/imp-tui/src/app.rs:10821:        let mut app = make_app_with_session(SessionManager::in_memory(), cwd);
crates/imp-tui/src/app.rs:11552:        let mut session = SessionManager::new(&cwd, &session_dir).unwrap();
crates/imp-tui/src/app.rs:11557:            tool_name: "mana".into(),
crates/imp-tui/src/app.rs:11569:                    text: "Trying mana create".into(),
crates/imp-tui/src/app.rs:11573:                    name: "mana".into(),
crates/imp-tui/src/app.rs:11598:        let reopened = SessionManager::open(&session_path).unwrap();
crates/imp-tui/src/app.rs:11637:            mana_review: imp_core::mana_review::TurnManaReview::no_change(0),
crates/imp-tui/src/app.rs:11970:            key: "mana".into(),
crates/imp-tui/src/app.rs:11973:                "inspect with mana agents".into(),
crates/imp-tui/src/app.rs:11977:        assert!(app.widgets.contains_key("mana"));
crates/imp-tui/src/app.rs:11980:            key: "mana".into(),
crates/imp-tui/src/app.rs:11984:        assert!(!app.widgets.contains_key("mana"));
crates/imp-tui/src/app.rs:11993:                component_type: "mana-widget".into(),
crates/imp-core/src/tools/multi_edit.rs:319:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/multi_edit.rs:320:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/runtime.rs:103:    ManaUpdated {
crates/imp-core/src/runtime.rs:104:        mana_ref: RuntimeManaRef,
crates/imp-core/src/runtime.rs:151:    pub mana_refs: Vec<RuntimeManaRef>,
crates/imp-core/src/runtime.rs:362:            RuntimeEventKind::ManaUpdated { mana_ref } => {
crates/imp-core/src/runtime.rs:363:                upsert_mana_ref(&mut self.snapshot.mana_refs, mana_ref.clone());
crates/imp-core/src/runtime.rs:473:fn upsert_mana_ref(mana_refs: &mut Vec<RuntimeManaRef>, mana_ref: RuntimeManaRef) {
crates/imp-core/src/runtime.rs:474:    if let Some(existing) = mana_refs.iter_mut().find(|item| item.id == mana_ref.id) {
crates/imp-core/src/runtime.rs:475:        *existing = mana_ref;
crates/imp-core/src/runtime.rs:477:        mana_refs.push(mana_ref);
crates/imp-core/src/runtime.rs:515:            mana_refs: Vec::new(),
crates/imp-core/src/runtime.rs:682:pub struct RuntimeManaRef {
crates/imp-core/src/runtime.rs:739:        assert!(snapshot.mana_refs.is_empty());
crates/imp-core/src/runtime.rs:779:        snapshot.mana_refs.push(RuntimeManaRef {
crates/imp-core/src/tools/git.rs:1427:    use crate::mana_review::TurnManaReviewAccumulator;
crates/imp-core/src/tools/git.rs:1449:            turn_mana_review: Arc::new(std::sync::Mutex::new(TurnManaReviewAccumulator::default())),
crates/imp-core/src/tools/git.rs:1743:    async fn git_worktree_actions_manage_worktrees() {
crates/imp-cli/src/stats_report.rs:6:use imp_core::session::{SessionEntry, SessionManager};
crates/imp-cli/src/stats_report.rs:341:        let session = SessionManager::open(&path)?;
crates/imp-cli/src/stats_report.rs:382:        records.extend(SessionManager::open(&path)?.usage_records());
crates/imp-cli/src/stats_report.rs:387:fn project_from_session(session: &SessionManager, path: &Path) -> String {
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:3:use super::mana_compat::tool_results_indicate_execution_evidence;
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:17:use crate::mana_review::TurnManaReviewAccumulator;
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:31:    pub(crate) turn_mana_review: std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>>,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:32:    pub(crate) has_mana_skill: bool,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:33:    pub(crate) has_mana_basics_skill: bool,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:34:    pub(crate) has_mana_delegation_skill: bool,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:35:    pub(crate) queued_mana_externalization_nudge: bool,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:43:            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:44:                TurnManaReviewAccumulator::default(),
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:46:            has_mana_skill: false,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:47:            has_mana_basics_skill: false,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:48:            has_mana_delegation_skill: false,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:49:            queued_mana_externalization_nudge: false,
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:73:    pub(crate) fn turn_mana_review(
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:75:    ) -> std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>> {
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:76:        self.turn_mana_review.clone()
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:79:    pub(crate) fn set_mana_skill_available(&mut self, available: bool) {
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:80:        self.has_mana_skill = available;
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:83:    pub(crate) fn set_mana_basics_skill_available(&mut self, available: bool) {
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:84:        self.has_mana_basics_skill = available;
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:87:    pub(crate) fn set_mana_delegation_skill_available(&mut self, available: bool) {
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:88:        self.has_mana_delegation_skill = available;
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:92:        self.queued_mana_externalization_nudge = true;
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:326:                .or_else(|| self.workflow_contract().mana_unit_ref.clone()),
crates/imp-core/src/agent/workflow_integration/recipe_runtime.rs:384:            .or_else(|| self.workflow_contract().mana_unit_ref.clone());
crates/imp-core/src/agent/events.rs:7:use crate::mana_review::TurnManaReview;
crates/imp-core/src/agent/events.rs:163:        mana_review: TurnManaReview,
crates/imp-core/src/agent/events.rs:424:                mana_review,
crates/imp-core/src/agent/events.rs:431:                    "mana_review": format!("{mana_review:?}"),
crates/imp-core/src/tools/query.rs:295:        "ment",    // management → manage
crates/imp-core/src/agent/run_loop.rs:258:            self.begin_turn_mana_review(turn);
crates/imp-core/src/agent/run_loop.rs:308:            // Context management is observation-mask only. Full conversation
crates/imp-core/src/agent/run_loop.rs:531:                                let mana_review = self.finish_turn_mana_review(turn);
crates/imp-core/src/agent/run_loop.rs:535:                                    mana_review,
crates/imp-core/src/agent/run_loop.rs:619:                let mana_review = self.finish_turn_mana_review(turn);
crates/imp-core/src/agent/run_loop.rs:623:                    mana_review,
crates/imp-core/src/agent/run_loop.rs:682:                let mana_review = self.finish_turn_mana_review(turn);
crates/imp-core/src/agent/run_loop.rs:686:                    mana_review: mana_review.clone(),
crates/imp-core/src/agent/run_loop.rs:699:                let assessment = self.assess_post_turn(&msg, &[], false, &mana_review);
crates/imp-core/src/agent/run_loop.rs:777:            self.record_turn_mana_mutations(&results);
crates/imp-core/src/agent/run_loop.rs:780:            let mana_review = self.finish_turn_mana_review(turn);
crates/imp-core/src/agent/run_loop.rs:784:                mana_review: mana_review.clone(),
crates/imp-core/src/agent/run_loop.rs:797:            let assessment = self.assess_post_turn(&msg, &results, true, &mana_review);
crates/imp-core/src/tools/scan/mod.rs:2183:            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/scan/mod.rs:2184:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-cli/src/usage_report.rs:7:use imp_core::session::SessionManager;
crates/imp-cli/src/usage_report.rs:320:        let session = SessionManager::open(&path)?;
crates/imp-core/src/reference_monitor.rs:226:    pub fn mana_policy_record(
crates/imp-core/src/reference_monitor.rs:229:        decision: &crate::agent::ManaPolicyDecision,
crates/imp-core/src/reference_monitor.rs:234:                    PolicySource::ManaLoop,
crates/imp-core/src/reference_monitor.rs:235:                    "compat_mana_policy_allowed",
crates/imp-core/src/reference_monitor.rs:236:                    "Legacy mana action allowed by active compatibility policy",
crates/imp-core/src/reference_monitor.rs:242:                    PolicySource::ManaLoop,
crates/imp-core/src/reference_monitor.rs:243:                    "compat_mana_policy_blocked",
crates/imp-core/src/reference_monitor.rs:245:                        "Legacy mana action blocked by active compatibility policy".into()
crates/imp-core/src/reference_monitor.rs:264:            Some("Use the native workflow tool instead of shelling out to legacy mana".into());
crates/imp-core/src/reference_monitor.rs:640:            .or_else(|| contract.mana_unit_ref.clone());
crates/imp-core/src/reference_monitor.rs:670:                    | ToolActionKind::Mana
crates/imp-core/src/reference_monitor.rs:750:    Mana,
crates/imp-core/src/reference_monitor.rs:793:                    | ToolActionKind::Mana
crates/imp-core/src/reference_monitor.rs:841:        if self.action_kind == ToolActionKind::Mana {
crates/imp-core/src/reference_monitor.rs:842:            return ResourceScope::Mana {
crates/imp-core/src/reference_monitor.rs:889:            "mana" => {
crates/imp-core/src/reference_monitor.rs:893:                    .push(ResourceScope::Mana { action: None });
crates/imp-core/src/reference_monitor.rs:928:            "mana" => Self::Mana,
crates/imp-core/src/reference_monitor.rs:962:    Mana {
crates/imp-core/src/reference_monitor.rs:1058:    ManaLoop,
crates/imp-core/src/reference_monitor.rs:1186:            ResourceScope::Mana { action } => {
crates/imp-core/src/reference_monitor.rs:1187:                serde_json::json!({ "kind": "mana", "action": action })
crates/imp-core/src/reference_monitor.rs:1234:        let mana = ToolMetadata::for_tool_name("mana", false);
crates/imp-core/src/reference_monitor.rs:1235:        assert_eq!(mana.action_kind, ToolActionKind::Mana);
crates/imp-core/src/reference_monitor.rs:1236:        assert!(mana.external_side_effect);
crates/imp-core/src/reference_monitor.rs:1274:        let mana = ToolMetadata::for_tool_name("mana", false);
crates/imp-core/src/reference_monitor.rs:1276:            mana.resource_scope_for_args(None, &serde_json::json!({ "action": "close" })),
crates/imp-core/src/reference_monitor.rs:1277:            ResourceScope::Mana {
crates/imp-core/src/reference_monitor.rs:1464:    fn policy_trace_records_cover_legacy_mana_policy_outcomes() {
crates/imp-core/src/reference_monitor.rs:1466:        let mut context = ToolPolicyContext::new("mana", ToolActionKind::Mana);
crates/imp-core/src/reference_monitor.rs:1468:        let decision = crate::agent::evaluate_mana_policy(
crates/imp-core/src/reference_monitor.rs:1472:        let record = monitor.mana_policy_record(&context, &decision);
crates/imp-core/src/reference_monitor.rs:1473:        assert_policy_record(record, PolicySource::ManaLoop, "compat_mana_policy_blocked");
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:1://! Mana/work-graph compatibility support for imp workflow integration.
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:3:#[cfg(feature = "mana-api")]
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:7:    mana_loop::{self, ManaActionClass},
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:12:use crate::mana_review::{
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:13:    ManaMutationAction, ManaMutationRecord, ManaReviewScope, ManaReviewScopeKind, ManaReviewState,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:14:    ManaUnitSnapshot, TurnManaReview, TurnManaReviewAccumulator,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:20:    assistant_message_contains_mana_tool_call, assistant_message_text,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:24:#[cfg(feature = "mana-api")]
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:25:fn find_mana_dir(cwd: &Path) -> Option<PathBuf> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:28:        let candidate = current.join(".mana");
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:46:fn mana_review_scope_from_result(result: &imp_llm::ToolResultMessage) -> ManaReviewScope {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:50:        .or_else(|| result.details.get("mana_dir"))
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:55:    ManaReviewScope {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:57:            ManaReviewScopeKind::None
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:59:            ManaReviewScopeKind::ExplicitPath
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:65:fn unit_snapshot_from_value(value: &serde_json::Value) -> Option<ManaUnitSnapshot> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:69:fn unit_snapshot_from_result(result: &imp_llm::ToolResultMessage) -> Option<ManaUnitSnapshot> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:76:fn mana_mutation_action(action: &str) -> Option<ManaMutationAction> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:78:        "create" => Some(ManaMutationAction::Create),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:79:        "close" => Some(ManaMutationAction::Close),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:80:        "update" => Some(ManaMutationAction::Update),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:81:        "notes_append" => Some(ManaMutationAction::NotesAppend),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:82:        "decision_add" => Some(ManaMutationAction::DecisionAdd),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:83:        "decision_resolve" => Some(ManaMutationAction::DecisionResolve),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:84:        "reopen" => Some(ManaMutationAction::Reopen),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:85:        "fail" => Some(ManaMutationAction::Fail),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:86:        "delete" => Some(ManaMutationAction::Delete),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:87:        "dep_add" => Some(ManaMutationAction::DepAdd),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:88:        "dep_remove" => Some(ManaMutationAction::DepRemove),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:89:        "fact_create" => Some(ManaMutationAction::FactCreate),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:94:fn mutation_record_from_mana_result(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:96:) -> Option<ManaMutationRecord> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:97:    if result.is_error || result.tool_name != "mana" {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:101:    let action_name = mana_result_action(result)?;
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:102:    let action = mana_mutation_action(action_name)?;
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:104:    let deleted_unit = if action == ManaMutationAction::Delete {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:112:        Some(crate::mana_review::ManaUnitRef::new(id, title, None))
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:121:            ManaMutationAction::DepAdd | ManaMutationAction::DepRemove
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:127:    Some(ManaMutationRecord {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:129:        scope: mana_review_scope_from_result(result),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:141:fn mana_workflow_follow_up_text() -> &'static str {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:147:        "mana" | "work" => result
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:154:                    .get("mana_loop_policy")
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:162:fn durable_workflow_action_class(action: &str) -> mana_loop::ManaActionClass {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:164:        // Native imp-work names that do not exist in legacy mana but mutate or
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:167:            mana_loop::ManaActionClass::ProgressCheckpoint
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:170:        "search" | "runs" | "validate" | "scope" => mana_loop::ManaActionClass::Inspect,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:171:        _ => mana_loop::classify_mana_action(action),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:178:        mana_loop::ManaActionClass::ProgressCheckpoint
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:179:            | mana_loop::ManaActionClass::GraphMutation
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:180:            | mana_loop::ManaActionClass::DecisionFact
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:187:        mana_loop::ManaActionClass::Lifecycle | mana_loop::ManaActionClass::Orchestration
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:210:fn mana_result_action(result: &imp_llm::ToolResultMessage) -> Option<&str> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:214:fn mana_unit_id_from_result(result: &imp_llm::ToolResultMessage) -> Option<String> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:223:fn mana_result_parent_id(result: &imp_llm::ToolResultMessage) -> Option<String> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:232:pub(crate) fn mana_run_status_from_result(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:235:    if result.is_error || result.tool_name != "mana" {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:251:        crate::workflow::WorkflowChildRunStatus::from_mana_run_status(status, total_failed),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:268:            && matches!(result.tool_name.as_str(), "mana" | "work")
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:274:fn mana_orchestration_run_id(tool_results: &[imp_llm::ToolResultMessage]) -> Option<String> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:276:        if result.is_error || !matches!(result.tool_name.as_str(), "mana" | "work") {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:280:        if durable_workflow_action_class(action) != mana_loop::ManaActionClass::Orchestration {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:301:            && matches!(result.tool_name.as_str(), "mana" | "work")
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:303:                durable_workflow_action_class(action) == mana_loop::ManaActionClass::Orchestration
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:327:            "mana" | "work" => durable_workflow_action(result)
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:346:        if result.is_error || !matches!(result.tool_name.as_str(), "mana" | "work") {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:358:fn mana_review_stop_reason(mana_review: &TurnManaReview, mode: AgentMode) -> Option<StopReason> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:359:    match mana_review.state {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:360:        ManaReviewState::NeedsDecision => Some(StopReason::UserBlocker),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:361:        ManaReviewState::Changed if matches!(mode, AgentMode::Planner) => {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:362:            if !mana_review.proposed_children.is_empty()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:363:                || !mana_review.touched_units.is_empty()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:364:                || !mana_review.material_field_changes.is_empty()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:365:                || !mana_review.notes_appended.is_empty()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:366:                || !mana_review.decision_events.is_empty()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:377:fn should_queue_mana_externalization_follow_up(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:381:    _has_mana_skill: bool,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:395:    if assistant_message_contains_mana_tool_call(message) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:465:fn mana_externalization_follow_up_text() -> &'static str {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:469:fn mana_skill_follow_up_hint(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:473:    _has_mana_skill: bool,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:474:    _has_mana_basics_skill: bool,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:475:    _has_mana_delegation_skill: bool,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:539:    pub fn set_workflow_mana_skill_available(&mut self, available: bool) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:540:        self.workflow_layer.set_mana_skill_available(available);
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:543:    pub fn set_workflow_mana_basics_skill_available(&mut self, available: bool) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:545:            .set_mana_basics_skill_available(available);
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:548:    pub fn set_workflow_mana_delegation_skill_available(&mut self, available: bool) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:550:            .set_mana_delegation_skill_available(available);
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:562:        mana_skill_follow_up_hint(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:566:            self.workflow_layer.has_mana_skill,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:567:            self.workflow_layer.has_mana_basics_skill,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:568:            self.workflow_layer.has_mana_delegation_skill,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:576:        should_queue_mana_externalization_follow_up(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:580:            self.workflow_layer.has_mana_skill,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:581:            self.workflow_layer.queued_mana_externalization_nudge,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:583:        .then(mana_externalization_follow_up_text)
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:592:        should_queue_mana_externalization_follow_up(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:596:            self.workflow_layer.has_mana_skill,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:597:            self.workflow_layer.queued_mana_externalization_nudge,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:604:        mana_review: &TurnManaReview,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:606:        let orchestration_run_id = mana_orchestration_run_id(tool_results);
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:617:            stop_reason: mana_review_stop_reason(mana_review, self.mode),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:632:                prompt: mana_workflow_follow_up_text().to_string(),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:633:                reason: ContinueReason::ManaWorkflowProgress,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:661:        mana_orchestration_run_id(tool_results)
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:683:        mana_review: &TurnManaReview,
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:685:        mana_review_stop_reason(mana_review, self.mode)
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:688:    pub(in crate::agent) fn turn_mana_review_accumulator(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:690:    ) -> std::sync::Arc<std::sync::Mutex<TurnManaReviewAccumulator>> {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:691:        self.workflow_layer.turn_mana_review()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:694:    pub(in crate::agent) fn begin_turn_mana_review(&self, turn: u32) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:695:        if let Ok(mut review) = self.workflow_layer.turn_mana_review.lock() {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:700:    pub(in crate::agent) fn record_turn_mana_mutations(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:704:        let Ok(mut review) = self.workflow_layer.turn_mana_review.lock() else {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:709:            if let Some(record) = mutation_record_from_mana_result(result) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:715:    pub(in crate::agent) fn finish_turn_mana_review(&self, turn: u32) -> TurnManaReview {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:716:        match self.workflow_layer.turn_mana_review.lock() {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:722:                    TurnManaReview::no_change(turn)
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:725:            Err(_) => TurnManaReview::no_change(turn),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:790:        #[cfg(feature = "mana-api")]
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:792:            let Some(mana_dir) = find_mana_dir(&self.cwd) else {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:794:                    "mana bootstrap unavailable: no .mana found for {}",
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:801:            match crate::workflow::create_native_mana_root(&mana_dir, request) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:805:                        .bind_mana_root(root.mana_root_id);
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:808:                        .record_mana_graph_changed();
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:812:                        "workflow bootstrap failed to create mana root: {err}"
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:818:        #[cfg(not(feature = "mana-api"))]
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:820:            "mana bootstrap unavailable: imp-core built without mana-api".to_string(),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:860:                "mana" | "work" => self.record_workflow_mana_obligation(result),
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:878:    fn record_workflow_mana_obligation(&mut self, result: &imp_llm::ToolResultMessage) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:879:        let Some(action) = mana_result_action(result) else {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:884:            ManaActionClass::Orchestration => {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:886:                    if let Some((run_id, status)) = mana_run_status_from_result(result) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:894:                        .record_mana_orchestration_started(mana_orchestration_run_id(
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:899:            ManaActionClass::ProgressCheckpoint
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:900:            | ManaActionClass::GraphMutation
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:901:            | ManaActionClass::DecisionFact => {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:903:                    if let Some(unit_id) = mana_unit_id_from_result(result) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:904:                        if mana_result_parent_id(result).as_deref()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:905:                            == self.workflow_layer.controller().mana_root_id.as_deref()
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:911:                            self.workflow_layer.controller_mut().bind_mana_root(unit_id);
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:917:                    .record_mana_graph_changed();
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:919:            ManaActionClass::Lifecycle => {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:921:                    if let Some(unit_id) = mana_unit_id_from_result(result) {
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:928:                        .record_mana_graph_changed();
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:931:            ManaActionClass::ReadHelp
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:932:            | ManaActionClass::Inspect
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:933:            | ManaActionClass::Destructive
crates/imp-core/src/agent/workflow_integration/mana_compat.rs:934:            | ManaActionClass::Unknown => {}
crates/imp-core/src/tools/write.rs:328:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/write.rs:329:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-core/src/agent/loop_policy.rs:25:            .or_else(|| ManaStopRule.decide(assessment))
crates/imp-core/src/agent/loop_policy.rs:83:struct ManaStopRule;
crates/imp-core/src/agent/loop_policy.rs:85:impl LoopPolicyRule for ManaStopRule {
crates/imp-core/src/agent/loop_policy.rs:87:        assessment.mana.stop_reason.map(finish)
crates/imp-core/src/agent/loop_policy.rs:146:    use crate::agent::{ContinueReason, ManaEvidence, RuntimeEvidence, TextFallbackEvidence};
crates/imp-core/src/agent/loop_policy.rs:159:            mana: ManaEvidence { stop_reason: None },
crates/imp-core/src/agent/loop_policy.rs:210:    fn mana_stop_wins_over_text_fallback_and_continue() {
crates/imp-core/src/agent/loop_policy.rs:212:        assessment.mana.stop_reason = Some(StopReason::UserBlocker);
crates/imp-cli/README.md:5:It builds the `imp` binary and wires together the terminal UI, CLI chat shell, one-shot prompt mode, auth/setup commands, secrets commands, mana task execution, import helpers, and RPC/headless entrypoints.
crates/imp-cli/README.md:15:- direct mana task execution via `imp run <unit-id>`
crates/imp-cli/README.md:36:The CLI is an active user-facing surface. Some headless/RPC-oriented paths are still evolving; the normal terminal UI, CLI chat, auth/secrets, and direct mana execution workflows are the primary supported surfaces.
crates/imp-core/src/tools/memory.rs:26:        "Manage persistent memory across sessions. Use to save environment facts, \
crates/imp-core/src/tools/memory.rs:196:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-core/src/tools/memory.rs:197:                crate::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-cli/Cargo.toml:20:mana-ui = ["dep:mana-core", "imp-core/mana-tool", "imp-tui/mana-ui"]
crates/imp-cli/Cargo.toml:27:mana-core = { workspace = true, optional = true }
crates/imp-cli/src/lib.rs:50:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:106:use imp_core::session::{SessionEntry, SessionManager};
crates/imp-cli/src/lib.rs:243:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:245:struct HeadlessManaArgs {
crates/imp-cli/src/lib.rs:246:    /// Mana unit ID to run
crates/imp-cli/src/lib.rs:248:    /// Explicit path to the .mana directory for canonical unit loading
crates/imp-cli/src/lib.rs:250:    mana_dir: Option<PathBuf>,
crates/imp-cli/src/lib.rs:256:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:258:struct ManaNamespaceArgs {
crates/imp-cli/src/lib.rs:259:    /// Mana operator verb or unit ID
crates/imp-cli/src/lib.rs:261:    /// Additional arguments for reserved mana namespace verbs
crates/imp-cli/src/lib.rs:263:    /// Explicit path to the .mana directory for canonical unit loading
crates/imp-cli/src/lib.rs:265:    mana_dir: Option<PathBuf>,
crates/imp-cli/src/lib.rs:300:    /// Enter the mana-aware operator namespace. Use `imp mana <unit-id>` to run one unit.
crates/imp-cli/src/lib.rs:301:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:302:    Mana(ManaNamespaceArgs),
crates/imp-cli/src/lib.rs:928:            turn_mana_review: std::sync::Arc::new(std::sync::Mutex::new(
crates/imp-cli/src/lib.rs:929:                imp_core::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-cli/src/lib.rs:1034:            #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:1035:            Commands::Mana(ManaNamespaceArgs {
crates/imp-cli/src/lib.rs:1038:                mana_dir,
crates/imp-cli/src/lib.rs:1042:                    if let Err(e) = run_reserved_mana_namespace_command(target, args).await {
crates/imp-cli/src/lib.rs:1051:                            "Error: unexpected extra arguments after mana unit id `{target}`: {}",
crates/imp-cli/src/lib.rs:1056:                    match run_headless_mode(&cli, target, mana_dir.as_deref(), *defer_verify).await
crates/imp-cli/src/lib.rs:2165:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2166:fn worker_status_counts_as_success(status: imp_core::mana_worker::WorkerStatus) -> bool {
crates/imp-cli/src/lib.rs:2169:        imp_core::mana_worker::WorkerStatus::Completed
crates/imp-cli/src/lib.rs:2170:            | imp_core::mana_worker::WorkerStatus::AwaitingVerify
crates/imp-cli/src/lib.rs:2174:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2178:    mana_dir_override: Option<&Path>,
crates/imp-cli/src/lib.rs:2186:    // Load the unit via canonical mana-core APIs for the primary single-unit
crates/imp-cli/src/lib.rs:2189:        imp_core::mana_worker::load_assignment_with_mana_dir(&cwd, unit_id, mana_dir_override)?;
crates/imp-cli/src/lib.rs:2198:    let options = imp_core::mana_worker::WorkerRunOptions {
crates/imp-cli/src/lib.rs:2215:        mana_dir_override: mana_dir_override.map(Path::to_path_buf),
crates/imp-cli/src/lib.rs:2220:    let mut prepared = imp_core::mana_worker::prepare_worker_run(assignment, options).await?;
crates/imp-cli/src/lib.rs:2270:    let outcome = imp_core::mana_worker::finalize_worker_run(prepared).await?;
crates/imp-cli/src/lib.rs:2342:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2343:async fn run_reserved_mana_namespace_command(
crates/imp-cli/src/lib.rs:2353:        "`imp mana {target}{rendered_args}` is reserved for a future mana-aware operator command. For now, use `mana {target}{rendered_args}` directly or `imp mana <unit-id>` for single-unit worker execution."
crates/imp-cli/src/lib.rs:2368:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2377:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2463:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2467:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2508:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:2613:#[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:3371:        "mana": {
crates/imp-cli/src/lib.rs:3372:            "stop_reason": assessment.mana.stop_reason,
crates/imp-cli/src/lib.rs:3914:            let sessions = SessionManager::list(&session_dir)?;
crates/imp-cli/src/lib.rs:3943:                SessionManager::continue_recent(&cwd, &session_dir)?.ok_or_else(|| {
crates/imp-cli/src/lib.rs:3961:                SessionManager::continue_recent(&cwd, &session_dir)?.ok_or_else(|| {
crates/imp-cli/src/lib.rs:3975:                SessionManager::continue_recent(&cwd, &session_dir)?.ok_or_else(|| {
crates/imp-cli/src/lib.rs:4121:            SessionManager::in_memory()
crates/imp-cli/src/lib.rs:4124:            match SessionManager::continue_recent(&cwd, &imp_core::storage::global_sessions_dir())?
crates/imp-cli/src/lib.rs:4127:                None => SessionManager::new(&cwd, &imp_core::storage::global_sessions_dir())?,
crates/imp-cli/src/lib.rs:4130:            SessionManager::open(path)?
crates/imp-cli/src/lib.rs:4133:            SessionManager::new(&cwd, &imp_core::storage::global_sessions_dir())?
crates/imp-cli/src/lib.rs:4214:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4216:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4219:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4221:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4224:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4226:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4227:    use mana_core::unit::Unit;
crates/imp-cli/src/lib.rs:4229:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4231:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4292:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4297:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4340:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4358:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4359:    fn write_test_mana_unit(
crates/imp-cli/src/lib.rs:4367:        let mana_dir = root.join(".mana");
crates/imp-cli/src/lib.rs:4368:        std::fs::create_dir_all(&mana_dir).unwrap();
crates/imp-cli/src/lib.rs:4369:        std::fs::write(mana_dir.join("config.yaml"), "project: test\nnext_id: 2\n").unwrap();
crates/imp-cli/src/lib.rs:4375:        unit.to_file(mana_dir.join(format!("{id}-{title_slug}.md")))
crates/imp-cli/src/lib.rs:4420:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4422:    fn cli_parses_mana_namespace_unit_target_for_headless_worker() {
crates/imp-cli/src/lib.rs:4423:        let cli = Cli::try_parse_from(["imp", "mana", "5.1"]).expect("parse mana unit target");
crates/imp-cli/src/lib.rs:4425:            Some(Commands::Mana(args)) => {
crates/imp-cli/src/lib.rs:4428:                assert!(args.mana_dir.is_none());
crates/imp-cli/src/lib.rs:4431:            other => panic!("expected mana namespace subcommand, got {other:?}"),
crates/imp-cli/src/lib.rs:4490:    fn cli_treats_old_run_mana_flags_as_prompt_args() {
crates/imp-cli/src/lib.rs:4497:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4499:    fn cli_parses_reserved_mana_namespace_verb_with_passthrough_args() {
crates/imp-cli/src/lib.rs:4500:        let cli = Cli::try_parse_from(["imp", "mana", "show", "28.1"]).expect("parse mana show");
crates/imp-cli/src/lib.rs:4502:            Some(Commands::Mana(args)) => {
crates/imp-cli/src/lib.rs:4506:            other => panic!("expected mana namespace args, got {other:?}"),
crates/imp-cli/src/lib.rs:4510:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4512:    fn reserved_mana_namespace_commands_error_clearly() {
crates/imp-cli/src/lib.rs:4515:            .block_on(run_reserved_mana_namespace_command("status", &[]))
crates/imp-cli/src/lib.rs:4518:        assert!(text.contains("reserved for a future mana-aware operator command"));
crates/imp-cli/src/lib.rs:4519:        assert!(text.contains("use `mana status` directly"));
crates/imp-cli/src/lib.rs:4522:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4526:            imp_core::mana_worker::WorkerStatus::Completed
crates/imp-cli/src/lib.rs:4529:            imp_core::mana_worker::WorkerStatus::AwaitingVerify
crates/imp-cli/src/lib.rs:4532:            imp_core::mana_worker::WorkerStatus::Failed
crates/imp-cli/src/lib.rs:4535:            imp_core::mana_worker::WorkerStatus::Blocked
crates/imp-cli/src/lib.rs:4538:            imp_core::mana_worker::WorkerStatus::Cancelled
crates/imp-cli/src/lib.rs:4542:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4546:        write_test_mana_unit(
crates/imp-cli/src/lib.rs:4555:        let assignment = imp_core::mana_worker::load_assignment_with_mana_dir(
crates/imp-cli/src/lib.rs:4558:            Some(temp.path().join(".mana").as_path()),
crates/imp-cli/src/lib.rs:4562:        let options = imp_core::mana_worker::WorkerRunOptions {
crates/imp-cli/src/lib.rs:4576:            mana_dir_override: Some(temp.path().join(".mana")),
crates/imp-cli/src/lib.rs:4581:        let mut prepared = imp_core::mana_worker::prepare_worker_run(assignment, options)
crates/imp-cli/src/lib.rs:4590:        let outcome = imp_core::mana_worker::finalize_worker_run(prepared)
crates/imp-cli/src/lib.rs:4596:            imp_core::mana_worker::WorkerStatus::Completed
crates/imp-cli/src/lib.rs:4602:            mana_core::ops::show::get(&temp.path().join(".mana"), "1").expect("show closed unit");
crates/imp-cli/src/lib.rs:4607:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4611:        write_test_mana_unit(
crates/imp-cli/src/lib.rs:4616:            "Check mana status and finish.",
crates/imp-cli/src/lib.rs:4620:        let assignment = imp_core::mana_worker::load_assignment_with_mana_dir(
crates/imp-cli/src/lib.rs:4623:            Some(temp.path().join(".mana").as_path()),
crates/imp-cli/src/lib.rs:4627:        let options = imp_core::mana_worker::WorkerRunOptions {
crates/imp-cli/src/lib.rs:4641:            mana_dir_override: Some(temp.path().join(".mana")),
crates/imp-cli/src/lib.rs:4646:        let mut prepared = imp_core::mana_worker::prepare_worker_run(assignment, options)
crates/imp-cli/src/lib.rs:4686:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:4699:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:5190:    #[cfg(feature = "mana-ui")]
crates/imp-cli/src/lib.rs:5192:    fn imp_cli_uses_canonical_mana_worker_prompt_and_context_helpers() {
crates/imp-cli/src/lib.rs:5193:        let assignment = imp_core::mana_worker::WorkerAssignment {
crates/imp-cli/src/lib.rs:5207:            attempts: vec![imp_core::mana_worker::WorkerAttempt {
crates/imp-cli/src/lib.rs:5216:        let prompt = imp_core::mana_worker::build_task_prompt(&assignment);
crates/imp-cli/src/lib.rs:5217:        let context = imp_core::mana_worker::build_task_context(&assignment);
crates/imp-cli/src/lib.rs:5219:        assert!(prompt.starts_with("# Mana worker assignment"));
crates/imp-lua/src/sandbox.rs:86:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-lua/src/sandbox.rs:87:                imp_core::mana_review::TurnManaReviewAccumulator::default(),
crates/imp-lua/src/sandbox.rs:117:/// Manages the Lua state for extensions.
crates/imp-gui/README.md:5:This crate starts as a standalone shell for the imp workbench UI: project status, mana work navigation, execution timeline, inspector, diff preview, and terminal output panes. Runtime integration will be added incrementally after the layout and app boundary are stable.
crates/imp-lua/src/lib.rs:106:            turn_mana_review: Arc::new(std::sync::Mutex::new(
crates/imp-lua/src/lib.rs:107:                imp_core::mana_review::TurnManaReviewAccumulator::default(),

## Likely historical/archive candidates
- .mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md
- .mana/.6.6-enforce-lua-extension-capability-boundaries.md
- .mana/.6.7-propagate-cancellation-into-active-tool-execution.md
- .mana/.6.8-align-diff-tool-registration-with-mode-contracts.md
- .mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md
- .mana/.gitignore
- .mana/21-imp-efficiency-smarter-tool-output-truncation.md
- .mana/245.1-define-manaimp-contract-implications-of-file-nativ.md
- .mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md
- .mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md
- .mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md
- .mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md
- .mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md
- .mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md
- .mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md
- .mana/249-reduce-duplicate-verbose-mana-change-narration-in.md
- .mana/250-shape-getimpdev-landing-page-direction-and-impleme.md
- .mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md
- .mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md
- .mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md
- .mana/259-audit-panic-usage-and-detached-task-failure-policy.md
- .mana/263-audit-and-isolate-library-level-stderr-writes-that.md
- .mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md
- .mana/264-normalize-imp-storage-topology-for-sessions-config.md
- .mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md
- .mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md
- .mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md
- .mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md
- .mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md
- .mana/266.1-design-adoption-path-provider-resilience-and-auth.md
- .mana/266.2-design-adoption-path-session-recall-memory-and-con.md
- .mana/266.3-design-adoption-path-extension-seams-and-product-s.md
- .mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md
- .mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md
- .mana/267-adopt-highest-value-product-lessons-from-opencode.md
- .mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md
- .mana/27.14-define-attempt-scoped-autonomy-observation-record.md
- .mana/27.2-imp-ui-compact-mana-statusprogress-surface.md
- .mana/271-add-native-youtube-video-interpretation-support-to.md
- .mana/271.1-implement-pure-http-youtube-transcript-extraction.md
- .mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md
- .mana/272-add-native-video-context-ingestion-architecture-fo.md
- .mana/272.1-implement-pure-http-youtube-transcript-extraction.md
- .mana/272.2-design-richer-video-interpretation-beyond-transcri.md
- .mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md
- .mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md
- .mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md
- .mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md
- .mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md
- .mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md
- .mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md
- .mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md
- .mana/275.13-implement-real-google-gemini-cli-oauth-provider-pa.md
- .mana/275.14-implement-real-github-copilot-oauth-provider-parit.md
- .mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md
- .mana/275.9-research-unofficial-cursor-provider-support-for-im.md
- .mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md
- .mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md
- .mana/278-fix-inspector-mode-interaction-model.md
- .mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md
- .mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md
- .mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md
- .mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md
- .mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md
- .mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md
- .mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md
- .mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md
- .mana/280-review-project-gaps-that-would-make-imp-stronger-t.md
- .mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md
- .mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md
- .mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md
- .mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md
- .mana/280.2.4-implement-edit-transaction-batching-with-combined.md
- .mana/280.2.6-collapse-multiedit-into-edit-as-the-canonical-mode.md
- .mana/280.3-run-dirac-public-eval-task-set-with-imp-harness.md
- .mana/280.4.1-audit-codex-agent-loop-for-executioncompletion-dis.md
- .mana/280.4.2-audit-pi-agent-loop-for-completioncontinuation-dis.md
- .mana/282-design-native-scoped-secret-injection-for-command.md
- .mana/285-verify-installed-imp-binary-includes-latest-secret.md
- .mana/290-complete-imp-codebase-quality-audit.md
- .mana/290.1-split-imp-tui-apprs-by-responsibility.md
- .mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md
- .mana/290.3-split-imp-cli-librs-into-command-modules.md
- .mana/290.4-split-native-mana-tool-implementation-into-focused.md
- .mana/291-rewrite-imp-readme-around-product-features-mana-an.md
- .mana/296-commit-imp-settings-shell-cleanup-separately.md
- .mana/300.1-add-cratesio-ready-dependency-versions-and-package.md
- .mana/300.2-resolve-imp-license-metadata-consistency-for-publi.md
- .mana/300.3-classify-and-clean-imp-worktree-before-cratesio-pu.md
- .mana/300.4-replace-imp-local-mana-deps-with-publishedversione.md
- .mana/300.5-verify-imp-crates-package-cleanly-before-publish-a.md
- .mana/306-improve-imp-tui-tool-call-display-and-inspector-ta.md
- .mana/309-review-warp-open-source-repo-for-imp-ideas.md
- .mana/31.2-add-guardrail-config-types-and-profile-selection-t.md
- .mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md
- .mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md
- .mana/310-choose-open-source-license-for-imp.md
- .mana/310.1-audit-imp-dependency-license-compatibility-for-mpl.md
- .mana/310.2-inventory-imp-repo-components-for-mpl-vs-apache-li.md
- .mana/312-assess-article-relevance-to-imp-improvements.md
- .mana/315-audit-imp-codebase-for-candidate-013-follow-up-wor.md
- .mana/315.1-fix-stale-imp-cli-workerassignment-test-initialize.md
- .mana/315.2-remove-accidental-root-files-from-next-imp-release.md
- .mana/315.3-align-imp-version-metadata-for-013-release.md
- .mana/315.4-resolve-gitleaks-findings-before-imp-013-release.md
- .mana/316-audit-imp-ui-facing-runtime-exposure.md
- .mana/321-fix-local-imp-install-workflow-reliability.md
- .mana/322-stabilize-imp-and-aush-local-install-workflow.md
- .mana/322.2-design-lua-extension-for-impaush-install-maintenan.md
- .mana/322.3-prototype-read-only-install-verification-lua-exten.md
- .mana/324-format-imp-mana-tool-file-separately-from-lua-inte.md
- .mana/325-extend-is-intended-as-first-class-interface-for-sk.md
- .mana/326-explore-github-repository-search-and-open-source-c.md
- .mana/327-audit-premature-stop-after-writing-next-step-to-ma.md
- .mana/33-chat-view-replace-duplicated-animation-logic-with.md
- .mana/330-audit-and-design-first-class-mana-integration-in-t.md
- .mana/330.3-introduce-pre-execution-manalooppolicy-in-agent-to.md
- .mana/330.5-revise-post-turn-assessment-to-use-structured-mana.md
- .mana/331-fix-mana-run-dispatch-and-terminal-slowdown-regres.md
- .mana/331.1-diagnose-mana-run-queued-worker-dispatch-failure.md
- .mana/331.2-diagnose-mana-run-terminal-slowdown-and-update-sto.md
- .mana/331.3-reject-zero-effective-concurrency-in-mana-run.md
- .mana/331.4-fix-native-mana-run-zero-worker-dispatch-regressio.md
- .mana/332-improve-imp-web-search-and-provider-agnostic-resea.md
- .mana/332.1-design-imp-rss-registry-and-trusted-source-researc.md
- .mana/332.2-design-github-source-connector-for-websearch-webre.md
- .mana/338-land-cross-project-mana-run-shutdown-reset-fix.md
- .mana/339-replace-imp-native-mana-run-cli-embedding-with-cor.md
- .mana/339.3-adapt-imp-tui-mana-run-events-to-direct-runtime-or.md
- .mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md
- .mana/340-build-imp-gui-with-egui.md
- .mana/340.1-harden-imp-gui-parity-implementation-for-real-dail.md
- .mana/344-add-mana-tui-navigator-for-durable-work-graph-brow.md
- .mana/344.1-improve-native-git-commit-tool-for-targeted-commit.md
- .mana/348-queue-behavior-decomposition-reference.md
- .mana/349-rebuild-imp-agent-loop-around-explicit-turn-state.md
- .mana/349.1-extract-composable-loop-policy-stack-from-post-tur.md
- .mana/349.2-persist-and-reconcile-agent-recovery-checkpoints-a.md
- .mana/349.3-audit-and-remove-remaining-max-turn-autonomy-surfa.md
- .mana/35-editor-remove-dead-tick-and-animationlevel-params.md
- .mana/350-audit-and-improve-imp-tui-responsiveness.md
- .mana/350.1-make-file-finder-loading-non-blocking-or-cached.md
- .mana/350.2-make-resume-session-listingopening-non-blocking.md
- .mana/350.3-profile-and-reduce-per-tick-render-work-during-str.md
- .mana/350.4-redesign-tui-event-loop-around-wakeable-inputrunti.md
- .mana/350.4.4-phase-4-reduce-streaming-render-invalidation-and-l.md
- .mana/350.4.4.1-measured-large-transcript-render-optimization.md
- .mana/350.4.6-manual-tui-responsiveness-trace-run.md
- .mana/350.4.7-final-audit-of-remaining-sync-work-on-tui-inputren.md
- .mana/350.4.8-validate-post-send-tui-latency-after-async-message.md
- .mana/350.5-move-manastatusimprove-blocking-operations-off-the.md
- .mana/350.6-add-opt-in-tui-keyboard-input-diagnostics.md
- .mana/353-bring-nightly-current-before-continuing-unit-394-w.md
- .mana/354-harden-imp-ttft-and-startup-regression-guardrails.md
- .mana/354.1-guard-tui-agent-start-request-against-pre-redraw-p.md
- .mana/354.3-guard-chatgpt-oauth-fresh-token-fast-path.md
- .mana/355-audit-workflownightlyrelease-regressions-against-m.md
- .mana/36-animation-config-reconcile-minimal-namingdocs-afte.md
- .mana/364.1-extend-global-imp-work-store-to-all-durable-record.md
- .mana/364.2-wire-work-tool-to-global-project-scoped-imp-work-s.md
- .mana/364.3-migrate-local-imp-work-and-mana-stores-into-global.md
- .mana/365-recenter-work-orchestration-on-mana-and-slim-imp-i.md
- .mana/365.1-inventory-imp-work-concepts-and-map-them-to-mana-o.md
- .mana/365.2-define-the-mana-harness-target-architecture-and-ag.md
- .mana/365.4-specify-the-production-mana-gui-harness-product-sh.md
- .mana/365.5-define-migration-and-deprecation-path-from-imp-wor.md
- .mana/365.6-define-non-imp-agent-adapter-contract-for-the-mana.md
- .mana/365.7-define-mana-dataapi-surface-needed-by-the-gui-harn.md
- .mana/365.8-decide-mana-harness-product-identity-daemon-topolo.md
- .mana/37-add-first-class-usage-accounting-and-reporting-to.md
- .mana/37.5-test-and-document-imp-usage-accountingreporting.md
- .mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md
- .mana/44.1-author-guest-runtime-extension-substrate-proposal.md
- .mana/45-tower-rebuild-around-explicit-contracts-durable-le.md
- .mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md
- .mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md
- .mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md
- .mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md
- .mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md
- .mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md
- .mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md
- .mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md
- .mana/46-broaden-imp-attention-beyond-toolsprompting-under.md
- .mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md
- .mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md
- .mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md
- .mana/50-define-cli-first-operator-surface-for-imp-with-tui.md
- .mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md
- .mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md
- .mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md
- .mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md
- .mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md
- .mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md
- .mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md
- .mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md
- .mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md
- .mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md
- .mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md
- .mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md
- .mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md
- .mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md
- .mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md
- .mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md
- .mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md
- .mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md
- .mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md
- .mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md
- .mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md
- .mana/50.24-define-the-first-cli-first-approval-policy-surface.md
- .mana/50.25-specify-detachedbackground-local-execution-contrac.md
- .mana/50.26-define-the-first-local-detachedbackground-executio.md
- .mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md
- .mana/65-root-mana-currently-lists-child-513-but-direct-sho.md
- .mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md
- .mana/73-code-intelligence-outputs-are-transient-by-default.md
- .mana/81-design-imp-native-delegation-tool-around-imp-run-a.md
- .mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md
- .mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md
- .mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md
- .mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md
- .mana/agent_history.jsonl
- .mana/archive.yaml
- .mana/config.yaml
- .mana/index.lock
- .mana/index.sqlite
- .mana/index.yaml
- .mana/index.yaml.old
- .mana/RULES.md
