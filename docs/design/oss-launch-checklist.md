# imp OSS launch checklist

Status: draft
Date: 2026-05-22

Purpose: prepare imp for a credible open-source launch with enough polish, documentation, trust, and onboarding that developers can try it, understand it, and believe the project is worth following.

## Launch bar

imp should launch as an open-source developer tool, not as a perfect commercial product. The bar is:

- a new developer can install it successfully
- a new developer can run a useful first task in under 10 minutes
- the README clearly explains what imp is and why it exists
- the docs answer the first wave of questions without needing the author
- core safety/trust boundaries are understandable
- obvious rough edges are documented, not hidden
- contributors can build, test, and make a small change

## 1. Positioning and messaging

- [ ] Choose the primary tagline.
  - Candidate: "A local-first coding agent with durable work built in."
  - Candidate: "The inspectable coding agent for real software work."
  - Candidate: "An open-source agent runtime for coding, verification, and durable work."
- [ ] Write a one-paragraph product description for README, package pages, and launch posts.
- [ ] Clearly state who imp is for.
  - Individual developers who want a local-first coding agent.
  - Agent/runtime builders who care about inspectability and policy.
  - Teams experimenting with durable agent workflows before adopting hosted platforms.
- [ ] Clearly state what imp is not yet.
  - Not a hosted enterprise platform.
  - Not a polished Slack/Jira/IDE suite yet.
  - Not a marketplace-first product.
- [ ] Define the comparison frame.
  - imp is not trying to out-Factory Factory on SaaS platform breadth.
  - imp should own local-first, open, inspectable, durable agent work.
- [ ] Add concise differentiators to README:
  - durable local sessions
  - native imp-work tasks/memory/runs
  - evidence and verification
  - policy-aware tools
  - extensibility through hooks/skills/agents
  - broad provider support / BYOK

## 2. README launch polish

- [ ] Make the top of README immediately compelling.
  - One-line tagline.
  - Short explanation.
  - Install command.
  - 2-3 screenshots or GIFs if available.
  - Quick start.
- [ ] Add a "Why imp" section that is crisp and non-defensive.
- [ ] Add a "What works today" section.
- [ ] Add a "What is experimental" section.
- [ ] Add a "Quick start in 5 minutes" flow:
  - install
  - authenticate or set API key
  - run `imp`
  - ask it to inspect the repo
  - run a one-shot prompt
  - run a small edit/test loop
- [ ] Add a minimal examples section:
  - `imp -p "summarize this repo"`
  - `imp chat`
  - `imp @src/file.rs "explain this"`
  - `imp review --staged` if available by launch
  - `imp work ...` if native imp-work CLI is launch-ready
- [ ] Add a provider/auth matrix.
- [ ] Add a safety/trust section.
- [ ] Add contribution links.
- [ ] Add roadmap links.
- [ ] Make sure old mana language is removed or clearly marked legacy.

## 3. Documentation library

Create a docs library that feels intentional, not like a pile of design notes.

### Proposed docs structure

```text
docs/
  index.md
  getting-started/
    install.md
    quickstart.md
    authentication.md
    first-task.md
    troubleshooting.md
  using-imp/
    tui.md
    cli.md
    chat.md
    file-context.md
    sessions.md
    models-and-providers.md
  work/
    overview.md
    tasks.md
    memory.md
    context-packs.md
    runs-and-outcomes.md
    prototypes.md
    verification.md
  safety/
    overview.md
    autonomy-modes.md
    policy.md
    hooks.md
    secrets.md
    evidence-and-traces.md
  extensibility/
    overview.md
    skills.md
    agents.md
    hooks.md
    lua-extensions.md
    mcp.md
  reference/
    cli.md
    config.md
    environment-variables.md
    file-layout.md
    provider-config.md
  contributing/
    setup.md
    architecture.md
    testing.md
    release-process.md
  roadmap.md
```

### Required docs for launch

- [ ] `docs/index.md`: docs landing page and navigation.
- [ ] `docs/getting-started/install.md`: install paths for Homebrew, archives, source.
- [ ] `docs/getting-started/quickstart.md`: 5-minute first useful run.
- [ ] `docs/getting-started/authentication.md`: API keys, OAuth, secrets storage.
- [ ] `docs/getting-started/troubleshooting.md`: common install/auth/provider issues.
- [ ] `docs/using-imp/tui.md`: core TUI controls and mental model.
- [ ] `docs/using-imp/cli.md`: one-shot, chat, file attachment, continuation.
- [ ] `docs/using-imp/sessions.md`: durable sessions, compaction, continuation.
- [ ] `docs/work/overview.md`: native imp-work overview, no mana terminology.
- [ ] `docs/work/tasks.md`: tasks, epics, dependencies, checks, outcomes.
- [ ] `docs/work/memory.md`: what memory is and when imp records it.
- [ ] `docs/work/prototypes.md`: disposable prototype workflow and evidence.
- [ ] `docs/safety/overview.md`: trust model and safe usage.
- [ ] `docs/safety/policy.md`: what policy means in practice.
- [ ] `docs/safety/hooks.md`: hooks, risks, examples.
- [ ] `docs/safety/secrets.md`: secret storage and redaction expectations.
- [ ] `docs/safety/evidence-and-traces.md`: run evidence, traces, verification.
- [ ] `docs/extensibility/skills.md`: skill format and examples.
- [ ] `docs/extensibility/agents.md`: `.imp/agents` if available or planned.
- [ ] `docs/extensibility/lua-extensions.md`: current stable extension path.
- [ ] `docs/reference/cli.md`: command reference.
- [ ] `docs/reference/config.md`: config precedence and examples.
- [ ] `docs/contributing/setup.md`: build from source.
- [ ] `docs/contributing/testing.md`: how to run checks.
- [ ] `docs/roadmap.md`: honest near-term roadmap.

### Docs quality bar

- [ ] Every page starts with "What this is" and "When to use it".
- [ ] Every workflow doc includes copy-paste commands.
- [ ] Every advanced concept has a beginner explanation first.
- [ ] Safety docs distinguish model instructions, hooks, policy, and user approval.
- [ ] Docs avoid stale future-tense promises unless clearly marked planned.
- [ ] Docs use native imp-work vocabulary, not mana, except in migration/legacy notes.
- [ ] Docs link back to the next useful page.
- [ ] Docs include expected outputs where helpful.
- [ ] Docs include troubleshooting notes for failure-prone flows.

## 4. Install and packaging

- [ ] Verify Homebrew install path works from a clean machine or container.
- [ ] Verify Linux archive install instructions for x86_64.
- [ ] Verify Linux archive install instructions for aarch64.
- [ ] Verify source install instructions.
- [ ] Verify macOS signing/quarantine workaround is documented if still needed.
- [ ] Add `imp --version` output that includes version, commit if available, and build target.
- [ ] Add `imp doctor` or improve it if present.
- [ ] Ensure failed install/auth states produce helpful errors.
- [ ] Ensure uninstall/update instructions exist.
- [ ] Confirm package metadata points to the OSS repo, docs, license, and issue tracker.

## 5. First-run experience

- [ ] First launch explains what imp needs next: API key, OAuth, or provider choice.
- [ ] Missing provider credentials produce clear next-step commands.
- [ ] First-run TUI has a short, useful welcome screen.
- [ ] Include starter prompts:
  - "Summarize this repo"
  - "Find the test command"
  - "Review my uncommitted changes"
  - "Create a task for this bug"
- [ ] Make it obvious how to quit.
- [ ] Make it obvious how to change model.
- [ ] Make it obvious when tools are about to run.
- [ ] Make approval prompts understandable.
- [ ] Ensure the first successful run leaves a positive artifact: summary, diff, task, or evidence.

## 6. Auth, providers, and secrets

- [ ] Document supported provider families accurately.
- [ ] Verify Anthropic API key path.
- [ ] Verify Anthropic OAuth path if launch-supported.
- [ ] Verify OpenAI API key path.
- [ ] Verify OpenAI/ChatGPT OAuth path if launch-supported.
- [ ] Verify Google path if launch-supported.
- [ ] Verify OpenRouter path.
- [ ] Verify OpenAI-compatible provider config.
- [ ] Document secret storage by platform.
- [ ] Ensure secret values are not printed in normal diagnostics.
- [ ] Add `secrets doctor` docs.
- [ ] Add troubleshooting for invalid key, missing key, model unavailable, and quota/rate-limit errors.

## 7. Core product readiness

- [ ] TUI basic chat works.
- [ ] `imp chat` works.
- [ ] `imp -p` works.
- [ ] File attachment with `@` works or is documented accurately.
- [ ] Continue/resume works.
- [ ] Compaction works or rough edges are documented.
- [ ] Read/write/edit tools work on normal files.
- [ ] Bash tool handles timeout/cancellation.
- [ ] Git status/diff/log work.
- [ ] Web search/read behavior is documented with provider requirements.
- [ ] Structural scan works on common languages or limitations are documented.
- [ ] Errors from failed tool calls are visible and recoverable.
- [ ] Runs do not silently swallow failed checks.

## 8. Native imp-work readiness

- [ ] Remove or clearly deprecate mana-first flows from launch docs.
- [ ] Document imp-work storage layout.
- [ ] Verify task create/list/show/update/claim flows.
- [ ] Verify dependencies and ready queue behavior.
- [ ] Verify context pack creation/refresh/stale detection.
- [ ] Verify run/outcome recording.
- [ ] Verify prototype run and recording behavior.
- [ ] Verify memory capture/search behavior.
- [ ] Add examples for:
  - create a task from chat
  - work on the next task
  - record a decision
  - run a prototype
  - close a task with checks
- [ ] Make task outcome states user-facing and understandable:
  - done
  - done with concerns
  - blocked
  - needs context
  - failed

## 9. Safety and policy readiness

- [ ] Document autonomy modes and defaults.
- [ ] Document what actions require approval by default.
- [ ] Document how tools are allowed/denied.
- [ ] Document how path/command restrictions work if available.
- [ ] Document hooks as programmable guardrails, including risks.
- [ ] Ensure blocking hook errors are understandable.
- [ ] Ensure policy denials include a reason and next action.
- [ ] Ensure dangerous commands are gated or clearly surfaced.
- [ ] Ensure file writes are visible before/after.
- [ ] Ensure secrets are redacted from tool output and traces where expected.
- [ ] Add a short "Security model" doc that is honest about local execution risks.

## 10. Evidence, traces, and verification

- [ ] Document when evidence is created.
- [ ] Document where run artifacts live.
- [ ] Document how to inspect a run.
- [ ] Document verification gates and check behavior.
- [ ] Ensure final responses say what was verified and what was not.
- [ ] Ensure failed verification is treated as a blocker or concern.
- [ ] Add an example evidence packet to docs.
- [ ] Add a review/handoff example showing evidence in practice.

## 11. Extensibility readiness

- [ ] Document skills as supported today, with examples.
- [ ] Document hooks as supported today, with examples.
- [ ] Document Lua extensions as the stable extension path.
- [ ] Avoid claiming TypeScript extensions are shipped unless they are actually implemented.
- [ ] If `.imp/agents` is not implemented by launch, mark it as roadmap, not current capability.
- [ ] If MCP is not implemented by launch, mark it as roadmap, not current capability.
- [ ] Provide one minimal Lua tool example.
- [ ] Provide one minimal hook example.
- [ ] Provide one minimal skill example.

## 12. Developer/contributor readiness

- [ ] `CONTRIBUTING.md` exists.
- [ ] Build instructions work on a fresh checkout.
- [ ] Test instructions work.
- [ ] Formatting/linting instructions are clear.
- [ ] Architecture overview exists for new contributors.
- [ ] Crate/module map exists.
- [ ] Issue templates exist.
- [ ] PR template exists.
- [ ] Code of conduct exists if desired.
- [ ] Security policy exists with vulnerability reporting instructions.
- [ ] License file is correct and intentional.
- [ ] Governance/maintainer expectations are clear enough for early contributors.

## 13. Repo hygiene

- [ ] Remove temporary files from repo root or move them under appropriate docs/scratch paths.
- [ ] Audit root directory for stale plans, experiments, and duplicate docs.
- [ ] Make docs discoverable from README.
- [ ] Make launch docs distinguish stable, experimental, and legacy.
- [ ] Run formatting.
- [ ] Run narrow test suite.
- [ ] Run full relevant test suite before tagging.
- [ ] Run secret scan.
- [ ] Run dependency audit if practical.
- [ ] Check binary size and release artifact contents.
- [ ] Verify `.gitignore` excludes local sessions, secrets, run artifacts, and prototype scratch output as intended.

## 14. Screenshots, demos, and examples

- [ ] Capture clean TUI screenshot.
- [ ] Capture tool-call/activity screenshot.
- [ ] Capture evidence/run summary screenshot if visually useful.
- [ ] Record a short GIF or terminal cast:
  - install/start
  - inspect repo
  - make a small edit
  - run verification
  - summarize outcome
- [ ] Add example workflows:
  - fix a failing Rust test
  - review a staged diff
  - create and complete an imp-work task
  - use a hook to format after writes
  - run a prototype to answer uncertainty
- [ ] Create a small demo repo or scripted demo if needed.

## 15. Launch issue backlog

Create GitHub issues for the work that should be visible to contributors:

- [ ] Good first issue: docs typo/small docs page.
- [ ] Good first issue: add provider troubleshooting entry.
- [ ] Good first issue: improve an error message.
- [ ] Help wanted: docs examples for non-Rust projects.
- [ ] Help wanted: MCP design/implementation if not started.
- [ ] Help wanted: ACP adapter research.
- [ ] Roadmap: `.imp/agents` custom agents.
- [ ] Roadmap: Work Control over imp-work.
- [ ] Roadmap: `/review` workflow.
- [ ] Roadmap: Slack adapter.

## 16. Release process

- [ ] Decide launch version number.
- [ ] Create release checklist for each platform.
- [ ] Update changelog.
- [ ] Tag release.
- [ ] Build release artifacts.
- [ ] Verify checksums if publishing archives.
- [ ] Publish Homebrew update.
- [ ] Publish GitHub release notes.
- [ ] Smoke-test install from published artifacts.
- [ ] Announce only after install path is verified.

## 17. Launch communications

- [ ] Write GitHub release announcement.
- [ ] Write short X/Mastodon/Bluesky post.
- [ ] Write longer launch post if desired.
- [ ] Prepare Hacker News / Reddit framing if posting.
- [ ] Prepare a concise comparison statement:
  - "imp is local-first and inspectable; hosted platforms are broader team products."
- [ ] Prepare honest limitations section.
- [ ] Invite contributors around specific areas:
  - docs
  - providers
  - MCP
  - editor adapters
  - safety/policy
  - imp-work UX

## 18. Post-launch feedback loop

- [ ] Track install failures.
- [ ] Track auth/provider confusion.
- [ ] Track first-run abandonment points.
- [ ] Track most requested integrations.
- [ ] Track docs pages that need expansion.
- [ ] Create a weekly triage rhythm for first month.
- [ ] Convert repeated questions into docs.
- [ ] Convert repeated bugs into tests.
- [ ] Keep roadmap visible but not overpromised.

## Minimum viable OSS launch checklist

If scope needs to shrink, do these first:

- [ ] README top section is excellent.
- [ ] Install works on macOS from Homebrew or documented source path.
- [ ] One provider path works reliably and is documented.
- [ ] `imp`, `imp chat`, and `imp -p` work.
- [ ] First-run errors are understandable.
- [ ] Basic safety/secrets docs exist.
- [ ] imp-work is documented or hidden if not ready.
- [ ] CONTRIBUTING/build/test docs exist.
- [ ] License and security policy exist.
- [ ] Launch release artifacts are smoke-tested.

## Recommended near-term documentation sprint

1. Create `docs/index.md` and docs navigation.
2. Rewrite README for launch positioning.
3. Write quickstart/install/auth/troubleshooting docs.
4. Write imp-work overview with native vocabulary.
5. Write safety/policy/secrets/evidence docs.
6. Write hooks/skills/Lua extension docs.
7. Add contributor setup/testing docs.
8. Add roadmap with `.imp/agents`, MCP, Work Control, `/review`, ACP, and Slack.
