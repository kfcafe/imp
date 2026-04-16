---
id: '53'
title: Audit model cutoff / mid-task truncation paths in imp runtime
slug: audit-model-cutoff-mid-task-truncation-paths-in-im
status: in_progress
priority: 2
created_at: '2026-04-16T04:15:31.814142Z'
updated_at: '2026-04-16T05:17:01.957397Z'
acceptance: Durable audit notes exist in mana with concrete file paths and failure modes. Follow-up implementation should either (a) transparently recover from max-token truncation, or (b) clearly surface truncation/incomplete-stream endings to the user instead of presenting them as clean completions.
notes: |-
  ---
  2026-04-16T04:55:22.907889+00:00
  User hypothesis refinement from follow-up: likely not token exhaustion in practice; stronger suspicion is incomplete stream termination being treated as success. Additional evidence from code inspection:

  - `crates/imp-llm/src/providers/openai.rs` streaming loop ends silently when `bytes_stream()` ends; there is no EOF validation, no processing of leftover buffer, and no synthetic error if `response.completed` / `MessageEnd` was never seen. If the connection closes after some deltas, the provider can return a stream with partial text and no terminal event.
  - `crates/imp-llm/src/providers/google.rs` has the same pattern: raw stream EOF just exits the task with no validation that a final `finishReason` / `MessageEnd` arrived.
  - `crates/imp-llm/src/providers/openai_compat.rs` is worse for this failure mode: after EOF it *always* emits `MessageEnd` from accumulated partial content, even if no SSE finish signal or `finish_reason` was ever received. That can convert an incomplete stream into an apparent clean completion.
  - `crates/imp-core/src/agent.rs` then treats missing provider `MessageEnd` as normal by synthesizing a message from accumulated deltas (`build_assistant_message(...)`), defaulting stop reason to `EndTurn`/`ToolUse`. So provider EOF without terminal completion can still become a normal-looking assistant turn.

  Likely fix direction:
  1. Track whether a terminal completion event was observed in each provider stream implementation.
  2. On EOF without terminal completion, emit `Err(Error::Stream(...))` or explicit incomplete termination instead of silent success.
  3. In `imp-core/src/agent.rs`, distinguish provider-complete turns from synthesized partial fallbacks; do not treat fallback synthesis as clean completion unless the provider explicitly finished.

  ---
  2026-04-16T05:17:01.957380+00:00
  Implemented stream-completion hardening.

  Code changes:
  - `crates/imp-llm/src/providers/openai.rs`
    - track `finished` in stream state
    - mark finished only on `response.completed`
    - process leftover buffered SSE line on EOF
    - emit `Error::Stream("OpenAI stream ended before response.completed")` when EOF occurs without terminal completion
  - `crates/imp-llm/src/providers/openai_compat.rs`
    - require at least one terminal `finish_reason` before synthesizing final `MessageEnd`
    - process leftover buffered SSE line on EOF
    - emit `Error::Stream("OpenAI-compatible stream ended before finish_reason")` on silent EOF instead of fabricating completion
  - `crates/imp-llm/src/providers/google.rs`
    - process leftover buffered SSE line on EOF
    - if stream ends without `state.finished` / terminal `finishReason`, emit `Error::Stream("Google stream ended before terminal finishReason")`
  - `crates/imp-llm/src/providers/anthropic.rs`
    - track `finished` in stream state
    - mark finished on `message_stop`
    - process leftover buffered SSE line on EOF
    - emit `Error::Stream("Anthropic stream ended before message_stop")` on silent EOF
  - `crates/imp-core/src/agent.rs`
    - track whether a provider-supplied `MessageEnd` was actually received
    - stop silently synthesizing a normal assistant completion when the stream ends without terminal completion
    - now emit an error and fail the run instead
    - added regression test `agent_treats_silent_eof_without_message_end_as_error`

  Verification run:
  - `cargo test -p imp-core agent_treats_silent_eof_without_message_end_as_error -- --nocapture`
  - `cargo test -p imp-core agent_surfaces_error_after_partial_stream_without_retrying -- --nocapture`
  - `cargo test -p imp-llm openai -- --nocapture`
  - `cargo test -p imp-llm openai_compat -- --nocapture`
  - `cargo test -p imp-llm google -- --nocapture`
  - `cargo test -p imp-llm anthropic -- --nocapture`

  Result: all passed.

  Residual note:
  - Current targeted provider tests are mostly parser/unit tests, not full mocked network EOF tests. The runtime behavior is now guarded both in providers and in `imp-core`, which materially closes the silent-cutoff path even before dedicated transport-level EOF fixtures are added.
labels:
- bug
- llm
- runtime
claimed_by: imp
claimed_at: '2026-04-16T05:02:50.493473Z'
kind: job
attempt_log:
- num: 1
  outcome: abandoned
  agent: imp
  started_at: '2026-04-16T05:02:50.493473Z'
---

Investigate why imp often returns assistant messages that appear cut off mid-thought or mid-task. Audit provider request defaults, stream completion handling, and agent-loop retry/follow-up behavior for truncation sources. Current findings from this audit:

1. Provider defaults cap output aggressively when `config.max_tokens` is unset:
   - `crates/imp-llm/src/providers/openai.rs`: `default_max_output_tokens(model) = min(model.max_output_tokens, 8192)` and `build_request()` uses `options.max_tokens.or(Some(default_max_output_tokens(model)))`.
   - `crates/imp-llm/src/providers/openai_compat.rs`: same `min(..., 8192)` cap via `default_max_tokens()`.
   - `crates/imp-llm/src/providers/google.rs`: `default_max_output_tokens()` uses `min(model.max_output_tokens, 8192)` unless thinking budget raises it.
   - `crates/imp-llm/src/providers/anthropic.rs`: non-adaptive path also uses `min(model.max_output_tokens, 8192)` unless thinking budget raises it.

2. Provider stop reasons correctly map truncation to `StopReason::MaxTokens`:
   - OpenAI Responses: `response.completed` with `status=incomplete` => `StopReason::MaxTokens`.
   - OpenAI-compatible chat completions: `finish_reason=length` => `StopReason::MaxTokens`.
   - Google: `finish_reason=MAX_TOKENS` => `StopReason::MaxTokens`.
   - Anthropic: `stop_reason=max_tokens` => `StopReason::MaxTokens`.

3. Agent loop does not appear to recover when a turn ends with `StopReason::MaxTokens`:
   - `crates/imp-core/src/agent.rs` accepts `StreamEvent::MessageEnd { message }`, stores it as `assistant_msg`, pushes it into history, and continues as if the response were complete.
   - `build_assistant_message()` only defaults to `EndTurn`/`ToolUse` when no provider message arrives, so provider-truncation metadata is preserved, but there is no branch that retries or auto-continues on `StopReason::MaxTokens`.

4. Existing mana unit `41.5` claims max-token escalation should exist in the agent loop, but the repo currently only contains Anthropic constants (`DEFAULT_MAX_TOKENS`, `ESCALATED_MAX_TOKENS`) and tests for those constants. No active agent-loop retry/escalation logic was found.

5. Google streaming has an additional edge case: in `crates/imp-llm/src/providers/google.rs`, if the stream ends without a candidate `finishReason`, the provider emits no `MessageEnd`; `imp-core/src/agent.rs` then synthesizes a message from partial deltas via `build_assistant_message()`, marking it as `EndTurn`. That can make an incomplete/truncated stream look like a clean completion.

6. User config on this machine currently sets `max_tokens = 13568` in `/Users/asher/.config/imp/config.toml`, so the common 8192 default cap is not the main cause for this user right now. Mid-task cutoffs may still come from provider-side truncation above 13568 or from incomplete streams being treated as clean completions.

Possible fixes to evaluate:
- agent-loop retry/escalation when provider stop reason is `MaxTokens`
- explicit surfaced warning/error when a turn ended due to `MaxTokens`
- Google/provider incomplete-stream detection so missing `MessageEnd` does not silently become `EndTurn`
- optional auto-follow-up/resume prompt when the stop reason is truncation rather than genuine completion
