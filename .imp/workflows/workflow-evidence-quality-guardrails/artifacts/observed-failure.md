# Observed failure

`simplify-agent-runtime-policy` was marked done by pre-existing passing test filters even though `git diff` showed no changes to `system_prompt.rs`, `agent/mod.rs`, or `turn_assessment.rs`.

This workflow exists to dogfood guardrails that distinguish implementation evidence from broad verification commands.
