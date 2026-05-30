# imp Output Mode Contract

This contract defines how imp separates human-readable output from machine-readable output across the CLI-first shell, `imp run`, RPC, `imp view`, and TUI surfaces.

It is grounded in current repo reality:

- planning notes in `.imp/workflows/50.16.1` document duplicated headless/RPC JSON encoders and the target split;
- `.imp/workflows/50.17` captures the follow-on output-contract requirement;
- `docs/rebuild/imp-cli-interactive-shell.md` defines transcript-oriented shell behavior;
- `docs/rebuild/imp-shared-ui-event-seam.md` defines shared UI request/runtime-event ownership;

The referenced historical docs (`imp-cli-affordance-sequence`, `imp-shared-runtime-startup-map`, `imp-command-grammar`, and shell transcript UX) are not present in this worktree, so this contract uses their durable workflow summaries plus currently restored rebuild docs.

## Problem: fragmented output handling

Current imp output has grown by surface:

- human shell and headless paths render terminal text;
- headless/print paths serialize JSON events through CLI-local helpers;
- RPC serializes similar runtime events through a separate conversion path;
- TUI consumes runtime events into fullscreen state;
- `imp view` is intended for browse-heavy human inspection.

The result is semantically similar events with different field names, envelopes, and responsibilities. For example, tool-call or tool-result events can differ between headless JSON and RPC. That duplication makes detached execution, approval pause/resume, structured reattach/status, and future worker protocols harder because callers cannot depend on one stable event/result family.

Target model: one shared runtime event/result substrate owned by `imp-core`, with thin human and machine adapters at the product surfaces.

## Stable modes

### Human transcript mode

Intended consumers:

- humans in `imp chat`;
- humans running `imp run` in a terminal;
- humans inspecting sessions through `imp view`;
- humans using `imp tui` as a fullscreen adapter.

Guarantees:

- output is readable and compact, not schema-stable;
- assistant text streams naturally;
- tool activity appears as terse notices and summaries;
- asks/confirms/selects/inputs appear as inline prompts or fullscreen widgets;
- failures explain consequence and next action;
- large artifacts route to `imp view` or artifact references instead of dumping raw payloads;
- turn summaries are visible when relevant.

Human transcript mode may change wording, spacing, glyphs, and formatting. It is not for scripts.

### Machine event mode

Intended consumers:

- scripts and wrappers;
- `imp run` worker/protocol mode;
- RPC clients;
- future detached/background execution monitors;
- future reattach/status tooling.

Guarantees:

- output is structured JSON/JSONL or RPC-framed JSON over the same canonical event family;
- event names and field meanings are versioned/stable;
- every event has a type, run/session/turn context when available, and bounded payloads;
- tool calls/results use the same field names across headless, worker, and RPC profiles;
- UI requests are explicit machine events requiring structured responses;
- final result events carry status, summary, artifacts, verification outcome, and error details where applicable.

Machine event mode is for protocol consumers. It should not rely on terminal glyphs, prose parsing, ANSI styling, or shell prompts.

## Surface defaults

- `imp chat`: human transcript mode by default.
- `imp print` / prompt-like one-shot use: human text by default when stdout is a TTY; machine mode only with explicit JSON/protocol flags.
- `imp run`: human transcript mode when directly attached to a TTY; machine event mode when stdout is non-TTY or an explicit worker/protocol/json mode is selected.
- RPC: machine event mode by default with transport/control envelopes.
- `imp view`: human browse mode by default; it may export structured artifacts separately but is not the baseline machine stream.
- `imp tui`: human fullscreen mode over shared events and UI requests.

## Shared runtime substrate

The shared substrate belongs in `imp-core` and should include these stable categories:

1. **Session lifecycle**
   - session loaded/created/resumed/forked;
   - run or turn started/completed/cancelled/failed;
   - compacted/resumed state when relevant.
2. **Model stream**
   - message start/end;
   - assistant text delta;
   - thinking/reasoning delta if exposed by policy;
   - model/provider error.
3. **Tool lifecycle**
   - tool call started;
   - bounded progress/update;
   - tool call completed/failed/cancelled;
   - artifact references and summarized outputs.
4. **UI request/response**
   - notify;
   - confirm;
   - select/multi-select;
   - input;
   - status/widget update.
5. **Policy and approval**
   - approval required;
   - approval accepted/denied/cancelled;
   - policy blocked;
   - sandbox or capability degraded.
6. **Checkpoint and restore**
   - checkpoint created;
   - checkpoint available after failure;
   - restore requested/completed/failed.
7. **Verification and diagnostics**
   - check started;
   - check passed/failed;
   - compact diagnostic summary;
   - artifact/log references.
8. **Final result**
   - status;
   - summary;
   - changed/affected paths;
   - artifact references;
   - verification outcome;
   - error kind/message for failure.

Adapters may omit or reformat categories, but should not invent new canonical semantics without promoting them into the shared substrate.

## Human adapter contract

Human adapters render the shared substrate into terminal or fullscreen presentation.

Allowed adapter-owned details:

- wording;
- color/styling/glyphs;
- truncation layout;
- progress spinners;
- TUI panel placement;
- `imp view` filters and navigation;
- command prompt shape;
- grouping of adjacent events for readability.

Not adapter-owned:

- event meaning;
- final status semantics;
- approval state meaning;
- checkpoint identity;
- artifact identity;
- tool call/result identity;
- verification outcome semantics.

## Machine adapter contract

Machine adapters serialize the shared substrate with stable field names.

Required envelope fields:

- `version`;
- `type`;
- `timestamp` when available;
- `session_id` when available;
- `run_id` / `turn_id` / `tool_call_id` as applicable;
- `payload` object;
- optional `sequence` for ordered streams;
- optional `profile` such as `headless`, `worker`, or `rpc`.

Profile-specific transport may wrap the event, but must not rename canonical payload fields for the same event meaning.

RPC may add control envelopes such as `ui_request`, `ui_response`, `protocol_error`, and request ids. Those are transport concerns layered around the canonical machine event family.

## Approval and pause requirements

Approval handling depends on this contract because human and machine surfaces must agree on states:

- `approval_required` is a durable runtime state, not just a prompt string;
- machine clients need a structured request id and allowed responses;
- human clients need clear consequence text;
- detached/background execution must be able to report “paused awaiting approval” without replaying transcript text;
- reattach/status must show the same approval state across CLI, view, RPC, and TUI.

Therefore approval state belongs in shared runtime events, while prompt wording/layout belongs to adapters.

## Detached/background dependency

Detached execution cannot land cleanly before output modes are stable.

Required by detached/background work:

- a machine-readable lifecycle stream;
- final result/status records independent of terminal prose;
- structured approval-required state;
- checkpoint/artifact refs;
- status polling/reattach views that do not parse human transcript text.

Human transcript output remains useful for attached runs, but detached/reattach features must build on the machine event contract.

## Structured reattach/status dependency

Future `imp view`, `imp run --status`, RPC status, and reattach flows need the same canonical state:

- current run/turn status;
- pending approval or input request;
- active/last tool;
- recent diagnostics/verify outcome;
- artifact/checkpoint refs;
- final status and summary.

This state should be produced by the shared event/result substrate, not reconstructed from a TUI render cache or CLI transcript.

## Implementation direction

1. Define or consolidate canonical event/result/request types in `imp-core`.
2. Move duplicated headless JSON and RPC conversion behind a shared serializer layer.
3. Keep CLI/TUI/view rendering as thin adapters.
4. Add explicit output profile selection for `imp run` and protocol/RPC paths.
5. Keep human transcript wording flexible while versioning machine event fields.
6. Add regression tests for matching tool/error/result fields across headless JSON and RPC profiles.

## Non-goals

- This document does not implement the serializer.
- It does not freeze terminal wording.
- It does not require TUI layout changes.
- It does not define every final JSON field; it defines ownership, categories, and guarantees.
- It does not make `imp chat` machine-first.

## Decision

imp should standardize on **human transcript mode** and **machine event mode** over a shared `imp-core` runtime event/result substrate. Human-facing surfaces render the substrate; machine-facing surfaces serialize it. Detached execution, approval pause/resume, and structured reattach/status should depend on this contract before adding new surface-specific output paths.
