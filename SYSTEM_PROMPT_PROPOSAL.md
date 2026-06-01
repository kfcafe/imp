# System Prompt Redesign — Final Proposal

## Changes completed

All stripping changes have been applied and tests pass (32/32 system_prompt tests green).

### Files modified

1. **`crates/imp-core/src/system_prompt.rs`**
   - Deleted `MANA_DELEGATION_GUIDANCE` const (~50 lines)
   - Removed mana-vs-bash guidance block (6 lines)
   - Removed delegation guidance injection block (12 lines)
   - Consolidated 5 deleted tests into 1 replacement test (`system_prompt_no_mana_guidance_or_delegation_in_prompt`)

2. **`crates/imp-core/src/learning.rs`**
   - Trimmed `LEARNING_INSTRUCTIONS` from 14 lines to 2 lines:
     `"You have persistent memory and can author skills. Use them to save durable knowledge and reduce repeat work."`

3. **`~/.imp/skills/mana/SKILL.md`**
   - Added "Tool usage" section with mana-native-tool preference guidance (moved from system prompt)

### Token savings

~950 tokens removed from every API request. Over a 30-turn session, that's significant cost and context-window savings.

---

## Comprehensive review of 25+ agent system prompts

Studied **25 agents**: Claude Code, Claude Cowork, Claude Sonnet 4.6, Codex (OpenAI), Hermes Agent, Pi, Cursor, Windsurf/Cascade, Augment Code (GPT-5 + Sonnet 4), Aider, Devin, Manus, Replit, Cline, Junie, Kiro, Lovable, v0, Amp (Sourcegraph), Zed, Warp, Trae, Factory/Droid, Roo Code, Same.dev, Qoder, GitHub Copilot (VS Code), Bolt, Gordon, Leap, Emergent/E1.

### Ideas worth considering for imp

#### 1. "Ambition vs. precision" framing (from Codex)

Codex has a brilliant distinction:
> "For tasks that have no prior context (starting something brand new), feel free to be ambitious and demonstrate creativity. If you're operating in an existing codebase, make sure you do exactly what the user asks with surgical precision."

**For imp:** This could be a `/personality` slider or just part of the autonomy/caution axis. Right now imp's caution slider partially covers this, but the "be creative on greenfield, be surgical on existing code" framing is sharper. Could be a sixth slider: `creativity` — from "surgical/conservative" to "ambitious/creative."

**Priority:** Low — the existing sliders mostly cover this.

#### 2. "Recovering from difficulty" nudge (from Augment/Amp)

Augment has:
> "If you notice yourself going in circles or down a rabbit hole (e.g., calling the same tool repeatedly without progress), ask the user for help."

Amp (Sourcegraph) reinforces this from a cost perspective:
> "MINIMIZE REASONING: Avoid verbose reasoning blocks throughout the entire session. Think efficiently and act quickly."

**For imp:** This is genuinely useful and absent from imp's prompt. It's a one-liner that prevents token waste. Could go in the system prompt or as a working-style line driven by the autonomy slider.

**Priority:** Medium — real practical value, tiny cost.

#### 3. Simple-first / reuse-first guardrails (from Amp)

Amp has excellent guardrails that are concise and behavioral:
> - **Simple-first**: prefer the smallest, local fix over a cross-file "architecture change"
> - **Reuse-first**: search for existing patterns; mirror naming, error handling, I/O, typing, tests
> - **No surprise edits**: if changes affect >3 files or multiple subsystems, show a short plan first
> - **No new deps** without explicit user approval

**For imp:** These are exactly the kind of engineering principles that belong in guardrail profiles, not the system prompt. The caution slider partially covers "no surprise edits" and the planning_depth slider covers "show a plan first." But "simple-first" and "reuse-first" are coding principles that could be excellent guardrail entries.

**Priority:** Medium — worth adding to guardrail profiles.

#### 4. Parallel tool execution (from Amp/Cursor/Same.dev)

Amp is the most explicit:
> "Default to **parallel** for all independent work: reads, searches, diagnostics, writes and subagents. Serialize only when there is a strict dependency."

Same.dev reinforces this aggressively:
> "CRITICAL INSTRUCTION: For maximum efficiency, whenever you perform multiple operations, invoke all relevant tools simultaneously rather than sequentially."

**For imp:** Imp already supports parallel tool calls. The question is whether to add a nudge. Tool-level behavior should be driven by the model's natural capability, not system prompt instructions. The model knows when calls are independent. Not needed in the prompt.

**Priority:** Not needed — model handles this naturally.

#### 5. Verification gates (from Amp/Factory)

Amp mandates: "Order: Typecheck → Lint → Tests → Build."

Factory/Droid goes further with mandatory environment bootstrapping before any code changes.

**For imp:** This is handled by mana's verify gates for delegated work. For interactive work, guardrail profiles could add "run tests after changes" as a post-edit hook. Not a system prompt concern.

**Priority:** Future guardrail / hook work.

#### 6. "No explanation unless asked" (from Amp/Cursor/Same.dev)

Amp: "Do not add explanations unless asked. After edits, stop."
Cursor: "Bias towards being direct and to the point"
Same.dev: "Do not add additional code explanation summary unless requested by the user. After working on a file, just stop."

**For imp:** The verbosity slider already controls this. At VeryLow/Low: "Keep responses terse/brief." The explicit "after edits, stop" framing is interesting but potentially too restrictive — sometimes a brief note about what changed is valuable. Well-covered by the existing slider.

**Priority:** Already covered by verbosity slider.

#### 7. Proactivity balance (from Windsurf/Amp/Same.dev)

Multiple agents articulate the same balance:
- Windsurf: "Strike a balance between doing the right thing when asked and not surprising the user."
- Amp Sonnet: "Take initiative when asked, but balance: doing the right thing vs. not surprising the user."

**For imp:** The autonomy slider already handles this. At high autonomy: "Act independently by default." At low: "Prefer confirmation." This is well-covered.

**Priority:** Already covered.

#### 8. Personality voice guidance (from Kiro)

Kiro has the most evocative personality description of any agent:
> "Speak like a dev — when necessary. Be decisive, precise, and clear. Lose the fluff when you can. We are supportive, not authoritative. Stay warm and friendly. Keep the cadence quick and easy."

**For imp:** The `/personality` sliders generate working-style lines that are functional but less evocative. The identity sentence ("thorough, clear, general collaborator") does some of this work. Could consider richer voice descriptions as an option in `/personality` — perhaps a `voice_description` freeform field for users who want more control.

**Priority:** Low — current approach works, this is polish.

#### 9. Skill loading strategy (from Claude Cowork / Hermes)

Both Cowork and Hermes are aggressive about skill loading:
- Cowork: "Your first order of business should always be to think about available skills and decide which are relevant."
- Hermes: "Before replying, scan the skills below. If one clearly matches your task, load it."

**For imp:** Currently the skills index says "use read to load when relevant." This is lighter and trusts the model. The aggressive approach burns tokens on skill reads for simple tasks. Imp's approach is better for token efficiency.

**Priority:** Low — monitor skill utilization first.

#### 10. Context file injection security (from Hermes)

Hermes scans AGENTS.md / context files for prompt injection patterns before loading them.

**For imp:** This is a security hardening measure. Imp currently loads AGENTS.md files verbatim. Worth considering as a guardrail, especially if imp is used in untrusted repos.

**Priority:** Medium for security — separate from system prompt work.

#### 11. "Keep going until done" directive (from Cursor/Windsurf/Augment/Claude Code/Amp/Same.dev/Roo)

Near-universal across coding agents:
- Cursor: "Please keep going until the query is completely resolved, before ending your turn"
- Amp: "Do the task end to end. Don't hand back half-baked work. FULLY resolve the user's request."
- Same.dev: "keep going until the user's query is completely resolved"

**For imp:** This is partially covered by the autonomy slider ("Act independently by default"), but the explicit "keep going until done" framing is different — it's about task completion, not just decision-making. Worth adding as a one-liner. Could be tied to the autonomy slider — at High/VeryHigh.

**Priority:** Medium — real behavioral impact.

#### 12. "No surprise edits" threshold (from Amp)

Amp: "if changes affect >3 files or multiple subsystems, show a short plan first"

**For imp:** This is a practical rule that aligns with imp's caution slider. At high caution, this behavior should be natural. Could be a guardrail entry or a caution-slider enhancement.

**Priority:** Low — caution slider partially covers this.

#### 13. Oracle/senior-review subagent pattern (from Amp)

Amp has a dedicated "Oracle" tool — a senior engineering advisor (o3 reasoning model) for reviews, architecture decisions, debugging. Separate from task execution workers.

**For imp:** Interesting pattern for mana. Could be a role — `mana create --role=reviewer` that uses a thinking model. Not a system prompt change, but a potential mana/imp feature.

**Priority:** Future feature exploration.

---

### Ideas NOT worth adding (and why)

| Idea | Source | Why not for imp |
|------|--------|----------------|
| Formatting rules (2000 tokens) | Codex | Let the model use natural judgment. /personality is the control. |
| Security/refusal blocks | Claude Code, Cowork, Claude 4.6 | Imp isn't a hosted service. Not needed. |
| File citation format rules | Codex, Cursor, Trae, Warp | IDE-specific. Imp is terminal-native. |
| Browser preview hooks | Windsurf, Lovable, Same.dev | Not applicable to imp's domain. |
| Deployment instructions | v0, Replit, Windsurf, Emergent | Domain-specific, not general agent work. |
| Task state tracking (todo.md) | Manus, Cursor, Codex, Amp, Same.dev | Mana handles this. |
| Preamble message style guide | Codex | Verbosity slider covers this. |
| "Don't use emojis" | Cowork, Codex, Amp, Claude 4.6 | Model default is fine. /personality could control if desired. |
| Package manager guidance | Augment, Factory, Trae | Engineering practice, could be guardrail. |
| SEARCH/REPLACE edit format | Aider, Codex, Cursor, Windsurf, Zed, Warp | Imp has native edit/diff tools with their own schemas. |
| Parallel tool call nudge | Amp, Same.dev, Cursor | Model handles this naturally. |
| Notebook/Jupyter editing | Cursor, VS Code Copilot | Not in imp's domain. |
| Web citation format | Trae, Warp | IDE-specific rendering. |
| Dynamic tool guidelines | Pi | Tools describe themselves. Redundant. |
| Git safety rules | Codex, Augment, Factory | Future guardrail profile, not system prompt. |
| Mandatory env bootstrap | Factory/Droid | Too rigid for general use. Skills can handle specific workflows. |
| Memory liberal preservation | Windsurf | Causes noise. Imp's end-of-session nudge is better. |

---

## Concrete next steps

### Now (this session — done ✓)
- [x] Remove `MANA_DELEGATION_GUIDANCE` from system_prompt.rs
- [x] Remove mana-vs-bash guidance block
- [x] Trim `LEARNING_INSTRUCTIONS` to one line
- [x] Add tool-usage section to mana skill
- [x] Update tests (consolidated 5 → 1)

### Improvements applied (this session — done ✓)
- [x] **"Keep going" in autonomy slider** — High/VeryHigh autonomy now includes "Keep working until the task is fully resolved before yielding."
- [x] **"Recovering from difficulty" line** — Added to working style (all profiles): "If you find yourself repeating the same action without progress, step back and try a different approach or ask the user for guidance."
- [x] **Simple-first/reuse-first guardrails** — Updated `GUIDANCE_GENERIC` with "Prefer the smallest, local fix", "Search for existing patterns first", and "Don't add new dependencies without explicit user approval"

### Soon (separate tasks)
1. **Consider `creativity` slider** — "surgical on existing code, ambitious on greenfield" (from Codex's ambition-vs-precision insight)

### Later (separate projects)
2. **Context file injection scanning** (from Hermes) — sanitize AGENTS.md before injection
3. **Git safety guardrail profile** (from Codex/Augment/Factory) — "never revert changes you didn't make" etc.
4. **Oracle/reviewer subagent role** (from Amp) — a mana role that uses a thinking model for review/architecture

---

## Design principles (confirmed by studying 13+ agents)

1. **Identity is one sentence.** Every agent does this. `/personality` controls it.
2. **Tools describe themselves.** No agent duplicates tool guidance in the system prompt when the tool has a good description.
3. **Skills are on-demand prompts.** Detailed workflow guidance belongs in skills loaded when relevant.
4. **Memory guidance = one nudge.** The model learns fast. Windsurf's "liberal memory" approach causes noise.
5. **No formatting rules by design.** Codex's 2000 tokens of formatting is the most bloated part of any prompt studied. Let the model use natural judgment. `/personality` is the control surface.
6. **The system prompt is for identity and environment, not tactics.** Tactics go in skills, tool descriptions, AGENTS.md, and guardrails.
