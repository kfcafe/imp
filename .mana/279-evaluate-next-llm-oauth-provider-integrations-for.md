---
id: '279'
title: Evaluate next LLM OAuth provider integrations for imp
slug: evaluate-next-llm-oauth-provider-integrations-for
status: open
priority: 3
created_at: '2026-04-27T18:02:09.049717Z'
updated_at: '2026-04-27T18:02:09.049717Z'
acceptance: A concise recommendation exists in task notes or a linked doc ranking Google Gemini, Zai, and Cursor by user value and implementation risk, with a go/no-go call for each and a proposed first implementation target if any.
labels:
- planning
- oauth
- providers
verify: rg -n "Google Gemini|Zai|Cursor|OAuth" .mana docs crates || true
kind: job
---

Evaluate whether Google Gemini, Zai, and Cursor are worth adding as OAuth-backed provider integrations in imp. Compare implementation difficulty, auth flow availability/device-code support, subscription-model value, runtime/provider compatibility, refresh/storage requirements, and setup/model-picker UX impact. Do not implement providers in this task; produce a recommendation with a ranked short list and risks.
