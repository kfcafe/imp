---
id: '275'
title: Port Pi provider and OAuth compatibility into imp
slug: assess-and-sequence-next-llm-oauth-provider-integr
status: open
priority: 2
created_at: '2026-04-27T18:02:24.705703Z'
updated_at: '2026-04-27T19:21:29.038962Z'
acceptance: 'imp reaches rough parity with Pi''s practical provider/OAuth compatibility for high-value local-agent providers: Google Gemini CLI OAuth / Cloud Code Assist, Google Antigravity if accepted after review, Z.AI API-key/Coding Plan support, GitHub Copilot OAuth, and exploratory unofficial Cursor support if a stable user-consented credential route is found. Each shipped provider has setup/model-picker visibility, auth refresh or secret handling, targeted tests, docs/help updates, and a clear risk note for unofficial integrations.'
notes: |-
  ---
  2026-04-27T18:02:49.235309+00:00
  Initial decomposition from discussion: rank likely candidates as Google Gemini first (most plausible official OAuth path, but verify Google app/scopes and whether OAuth unlocks subscription-backed Gemini access), Zai second (investigate whether OAuth exists or API-key/OpenAI-compatible route is the right path), Cursor third/highest risk (only consider if official or stable user-consented credential import exists; avoid brittle/private session scraping).

  ---
  2026-04-27T18:11:22.212216+00:00
  Research direction updated from user: goal should be matching Pi's provider and OAuth compatibility rather than evaluating only isolated provider requests. Source repo discovered at /Users/asher/pi-mono (not ~/pi-core). Pi's AI package includes OAuth providers for Anthropic, GitHub Copilot, Google Gemini CLI / Cloud Code Assist, Google Antigravity, and OpenAI Codex; Z.AI appears API-key/JWT based in docs, not OAuth. Cursor has official CLI browser login/API key docs but no inspected provider implementation in Pi yet.
labels:
- imp
- oauth
- providers
- planning
kind: epic
decisions:
- 'Scope target: imp should aim for Pi provider/OAuth compatibility where safe, using /Users/asher/pi-mono/packages/ai as the reference implementation rather than inventing provider behavior from scratch.'
- 'Cursor policy: unofficial Cursor support is allowed to be researched and potentially implemented, but only with user-consented credentials from an official CLI/API-key/browser-login flow or a stable local credential file. Do not implement silent browser cookie scraping or brittle hidden-token extraction.'
---

Port Pi's practical provider and OAuth compatibility into imp while preserving imp's runtime/auth boundaries.

Reference source of truth:
- Pi repo: `/Users/asher/pi-mono/packages/ai`
- imp repo inside Tower: `/Users/asher/imp`

Target provider/auth areas:
- Google Gemini CLI / Cloud Code Assist OAuth (`google-gemini-cli`) from Pi.
- Google Antigravity route (`google-antigravity`) only if risk review accepts it.
- Z.AI API-key/JWT/Coding Plan provider support (`zai`, possibly coding endpoint) based on official docs and Pi model metadata.
- GitHub Copilot OAuth/device flow (`github-copilot`) from Pi.
- Unofficial Cursor support if possible, but only through user-consented official CLI/API-key/stable local credential paths.

Important decisions already made:
1. Pi parity is the target, not isolated provider one-offs.
2. Cursor unofficial support is allowed to be researched and potentially implemented, but no silent browser-cookie scraping, hidden token extraction, token printing, or credential exfiltration.
3. Older broad units 275.1-275.4 are superseded and should not be dispatched. The active entry points are 275.10 then 275.11, followed by implementation/research units 275.5-275.9.

Execution model for future agents:
- Start with 275.10 to produce factual inventory.
- Then run 275.11 to produce sequence/risk plan.
- Then run implementation/research units according to 275.11 dispatch order.
- Each worker must inspect named files in its unit; do not rely only on this epic description.
- Keep provider implementations small, tested, and model-first. Setup/model-picker UX and docs are part of acceptance for shipped providers.
