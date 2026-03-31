---
id: '22'
title: 'bug: compaction_resume tests hang due to mock provider response ordering'
slug: bug-compactionresume-tests-hang-due-to-mock-provid
status: closed
priority: 2
created_at: '2026-03-23T18:43:29.660196Z'
updated_at: '2026-03-30T09:38:15.480819Z'
notes: |2-

  ## Attempt 1 — 2026-03-24T06:26:57Z
  Exit code: 1

  ```

  ```


  ---
  2026-03-30T09:38:15.480226+00:00
  Stale: compaction was removed from the agent loop entirely. The tests this bug referenced no longer exist. Closing as obsolete.
labels:
- bug
- imp-core
verify: cd /Users/asher/tower && timeout 30 cargo test -p imp-core compaction_resume 2>&1 | grep "test result:" | grep -v "0 passed"
fail_first: true
attempts: 1
history:
- attempt: 1
  started_at: '2026-03-24T06:26:56.760343Z'
  finished_at: '2026-03-24T06:26:57.184868Z'
  duration_secs: 0.424
  result: fail
  exit_code: 1
kind: job
---

The three compaction_resume agent tests in imp-core/src/agent.rs hang indefinitely. The tests verify that after context compaction fires mid-task, a resume message containing the original prompt is injected.

Root cause: The mock provider (CapturingMockProvider) shares a single Mutex of Vec of responses between the agent loop and the compaction code. The agent loop now uses run_with_retry which buffers all stream events. The response ordering between compaction calls and agent LLM calls depends on exactly when context exceeds the threshold — getting this wrong means compaction consumes the wrong response (e.g. gets a tool_call_response when it expects a text summary), causing the agent to loop or hang.

The actual feature works — original_prompt is tracked, resume message is injected after compaction, turn-zero is skipped. The implementation in agent.rs is correct. Only the tests are broken.

Fix approach: Either (a) use a smarter mock that dispatches responses based on the context/system prompt (compaction has a distinct system prompt), or (b) test the resume logic with unit tests that call the injection logic directly without running the full agent loop, or (c) use separate mock providers for compaction and agent calls.

Files: imp-core/src/agent.rs (search for compaction_resume, make_compaction_agent_turn1)
