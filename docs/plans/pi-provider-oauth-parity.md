# Pi provider/OAuth parity plan

## Factual inventory

This section inventories Pi provider/OAuth support against imp's current provider/auth surface. It is factual mapping only; sequencing and implementation decisions belong to follow-up work.

### Pi support inventory

| Provider | Auth method | Source file(s) | Endpoint/scopes/env vars | Model families | Risk notes |
|---|---|---|---|---|---|
| Anthropic | OAuth browser/loopback callback plus refresh | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/anthropic.ts`, `index.ts` | authorize `https://claude.ai/oauth/authorize`, token `https://platform.claude.com/v1/oauth/token`, loopback `127.0.0.1:53692/callback`, scopes include `org:create_api_key`, `user:profile`, `user:inference`, Claude Code/session/MCP/file upload scopes | Anthropic/Claude models in generated registry | OAuth subscription route; Node/Bun callback server only. |
| OpenAI/ChatGPT Codex | OAuth browser/loopback callback plus refresh | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/openai-codex.ts`, `index.ts` | auth `https://auth.openai.com/oauth/authorize`, token `https://auth.openai.com/oauth/token`, redirect `http://localhost:1455/auth/callback`, scope `openid profile email offline_access` | OpenAI Codex/ChatGPT-backed model route | Uses ChatGPT/Codex backend, not plain OpenAI API key path. |
| Kimi Code/Moonshot | API key for Moonshot; separate Kimi coding provider shape in generated models/env mapping | `/Users/asher/pi-mono/packages/ai/src/env-api-keys.ts`, `/Users/asher/pi-mono/packages/ai/src/models.generated.ts` | `KIMI_API_KEY` for `kimi-coding`; Moonshot/Kimi generated model IDs such as `moonshot.kimi-k2-thinking`, `moonshotai.kimi-k2.5` | Kimi K2 / Moonshot / Kimi coding models | OAuth evidence not found in inspected Pi OAuth registry for Kimi; appears API-key/env based in inspected files. |
| Google Gemini CLI | Google OAuth loopback callback plus refresh and project discovery/onboarding | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/google-gemini-cli.ts`, `index.ts`, `models.generated.ts` | redirect `http://localhost:8085/oauth2callback`; scopes `cloud-platform`, `userinfo.email`, `userinfo.profile`; token `https://oauth2.googleapis.com/token`; Code Assist endpoint `https://cloudcode-pa.googleapis.com` | Gemini CLI / Google Cloud Code Assist models, standard Gemini 2.x families | Project provisioning/discovery and VPC-SC handling are part of auth path. |
| Google Antigravity | Google OAuth loopback callback with Antigravity credentials/project discovery | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/google-antigravity.ts`, `index.ts` | redirect `http://localhost:51121/oauth-callback`; scopes `cloud-platform`, `userinfo.email`, `userinfo.profile`, `cclog`, `experimentsandconfigs`; endpoints include `cloudcode-pa.googleapis.com` and sandbox; fallback project `rising-fact-p41fc` | Gemini 3, Claude, GPT-OSS via Google Cloud route per file header | Higher-risk route: special scopes, Google internal-ish Code Assist headers/endpoints, fallback project behavior. |
| Z.AI | API key/env var, not OAuth in inspected evidence | `/Users/asher/pi-mono/packages/ai/src/env-api-keys.ts`, `/Users/asher/pi-mono/packages/ai/src/models.generated.ts`, `/Users/asher/pi-mono/packages/ai/src/types.ts` | `ZAI_API_KEY`; types mention `zai` thinking format and `zaiToolStream` | Generated models include `zai.glm-4.7`, `zai.glm-4.7-flash`, `zai.glm-5`, `zai-glm-4.7` | Do not claim official OAuth; inspected evidence says env/API-key/JWT-like route only. |
| GitHub Copilot | GitHub device flow, token exchange, Copilot internal token refresh/base URL derivation | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/github-copilot.ts`, `index.ts`, `models.generated.ts` | device code `https://{domain}/login/device/code`, access token `https://{domain}/login/oauth/access_token`, Copilot token `https://api.{domain}/copilot_internal/v2/token`; scope `read:user`; headers mimic Copilot Chat/VS Code | Generated registry has many `github-copilot` models across Anthropic/OpenAI-style APIs | Device polling/slow_down behavior, enterprise domain handling, proxy endpoint parsing. |
| Cursor | No implementation found in inspected Pi OAuth registry or env mapping | OAuth registry `index.ts`, env mapping `env-api-keys.ts`, generated model searches | No exact source evidence found in inspected Pi files | Unknown | Treat as unknown/provider-route not implemented until evidence proves otherwise. |

### imp current support

| Provider | Auth method | Source file(s) | Equivalent/missing pieces | Setup/model-picker status |
|---|---|---|---|---|
| Anthropic | API key plus OAuth credential support in auth store/refresh path | `crates/imp-llm/src/model.rs`, `crates/imp-llm/src/auth.rs`, `crates/imp-cli/src/lib.rs` | Provider exists; OAuth display/refresh concepts exist. Need compare exact Claude subscription OAuth scopes/callback against Pi if parity required. | `run_login` supports `anthropic`; setup includes Anthropic. |
| OpenAI/ChatGPT Codex | OpenAI API key and `openai-codex` OAuth/backend provider | `crates/imp-llm/src/model.rs`, `crates/imp-llm/src/providers/openai_codex.rs`, `crates/imp-llm/src/auth.rs`, `crates/imp-cli/src/lib.rs` | `openai-codex` provider and OAuth preference exist; needs detailed parity against Pi Codex callback/token handling if gaps remain. | `run_login` supports OpenAI/Codex path; model aliases include Codex. |
| Kimi Code/Moonshot | Moonshot API key and Kimi Code OAuth/import path | `crates/imp-llm/src/model.rs`, `crates/imp-llm/src/auth.rs`, `crates/imp-llm/src/providers/mod.rs`, `crates/imp-cli/src/lib.rs` | imp has `moonshot` API-key provider and `kimi-code` provider with default headers; CLI can import Kimi CLI credentials. Env names differ from Pi (`MOONSHOT_API_KEY`/`KIMI_API_KEY` vs Pi `KIMI_API_KEY`/`kimi-coding`). | Setup hides `kimi-code` but includes Moonshot/Kimi API setup; `imp login kimi` maps to Kimi Code flow. |
| Google Gemini CLI | Google API-key provider only in inspected imp core provider registry | `crates/imp-llm/src/model.rs`, `crates/imp-llm/src/providers/google.rs`, CLI setup/auth files | Missing Gemini CLI OAuth loopback/project discovery/onboarding route. Existing provider uses `GOOGLE_API_KEY`/native Google API style. | Gemini models exist and aliases include Gemini; setup is API-key oriented. |
| Google Antigravity | No dedicated provider/OAuth route found | `crates/imp-llm/src/model.rs`, provider modules | Missing Antigravity OAuth credentials/scopes/endpoints/project discovery and model routing. | Not present in setup/model picker as a dedicated provider. |
| Z.AI | No built-in imp provider found in inspected registry | `crates/imp-llm/src/model.rs`, `crates/imp-llm/src/providers/mod.rs`, `crates/imp-cli/src/lib.rs` | Missing Z.AI provider/env handling. Pi evidence says `ZAI_API_KEY`, not OAuth. | Not present in imp setup/provider registry. |
| GitHub Copilot | No dedicated provider/OAuth route found | `crates/imp-llm/src/model.rs`, provider modules, `crates/imp-cli/src/lib.rs` | Missing device flow, Copilot token exchange/refresh, enterprise domain/base URL routing, model enablement. | Not present in setup/model picker. |
| Cursor | No provider route found | inspected imp provider/auth/model files | Missing/unknown; do not treat as implemented. | Not present. |

### Evidence inspected

- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/index.ts`
- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/types.ts`
- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/anthropic.ts`
- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/openai-codex.ts`
- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/google-gemini-cli.ts`
- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/google-antigravity.ts`
- `/Users/asher/pi-mono/packages/ai/src/utils/oauth/github-copilot.ts`
- `/Users/asher/pi-mono/packages/ai/src/env-api-keys.ts`
- `/Users/asher/pi-mono/packages/ai/src/models.generated.ts`
- `/Users/asher/pi-mono/packages/ai/src/types.ts`
- `crates/imp-llm/src/model.rs`
- `crates/imp-llm/src/auth.rs`
- `crates/imp-llm/src/providers/mod.rs`
- `crates/imp-llm/src/providers/openai_codex.rs` via references/search
- `crates/imp-cli/src/lib.rs`
- `crates/imp-tui/src/views/login_picker.rs` and `crates/imp-tui/src/views/welcome.rs` via search context

### Notes for follow-up sequencing

- Z.AI should be treated as API-key/env based unless later source evidence proves OAuth.
- Cursor is unknown/missing in the inspected Pi and imp provider registries.
- Google Gemini CLI, Antigravity, and GitHub Copilot are the largest missing imp provider/OAuth surfaces.
- Kimi/Moonshot requires careful naming split: Moonshot API key vs Kimi Code OAuth/coding route.

## Implementation sequence and risk

This section sequences implementation work from the factual inventory. It does not implement providers.

| Rank | Provider | Status | Why | User value | Implementation risk | Auth/storage impact | Likely imp files | Verify strategy | Workflow units |
|---:|---|---|---|---|---|---|---|---|---|
| 1 | Z.AI | go | Inventory shows API-key/env based support (`ZAI_API_KEY`), not OAuth. This is the smallest missing provider surface. | Adds GLM/Z.AI model access with low auth complexity. | Low/medium: endpoint/model metadata and OpenAI-compatible routing need validation. | Add provider secret/env mapping; no OAuth credential store. | `imp-llm/src/model.rs`, provider registry, auth/env resolution, CLI/TUI setup lists. | provider registry/model tests, env-secret auth tests, provider construction/base URL tests. | Keep/update `275.7`; prior closed zero-test state should be treated as stale until real tests/code exist. |
| 2 | Google Gemini CLI | go | High-value OAuth route; Pi has concrete reference. imp already has Google API-key provider but lacks CLI/Code Assist OAuth. | Subscription/CLI-style Gemini access without API key. | Medium/high: project discovery/provisioning and Code Assist headers/endpoints must be ported carefully. | New OAuth credential kind/provider ID, token refresh, project/account metadata. | `imp-llm/src/oauth/*`, `auth.rs`, `model.rs`, Google provider routing, `imp-cli`, TUI login/setup. | mocked OAuth URL/scopes/refresh/project discovery tests; model/provider routing checks. | Keep/update `275.5`; prior closed zero-test state should be reopened or superseded by a real implementation unit. |
| 3 | GitHub Copilot | research then go | Pi has strong reference for device flow and Copilot token exchange, but request routing/model enablement is more complex. | Unlocks existing Copilot subscription models. | High: device polling, enterprise domains, Copilot internal token/base URL, model availability. | New OAuth/device credentials, Copilot token refresh, optional enterprise domain metadata. | OAuth module, auth store, provider routing/headers, model registry, CLI/TUI login. | device-flow parsing/polling tests, token/base URL extraction tests, small model routing fixture. | Keep/update `275.8`; old closed zero-test gate is stale. |
| 4 | Google Antigravity | defer pending risk acceptance | Pi reference exists but uses more brittle/special Code Assist/Antigravity scopes, endpoints, headers, and fallback project behavior. | Potentially unlocks Gemini 3 / Claude / GPT-OSS via Google route. | Very high: private-ish endpoint compatibility and stability/legal/product risk. | Separate provider/OAuth route if accepted; do not silently mix with Gemini CLI. | Research note first; provider code only after explicit approval. | risk assessment doc and maybe a disposable request-shape prototype without credentials. | Continue `275.6` research; do not implement until accepted. |
| 5 | Cursor | defer / research more | Inventory found no Pi Cursor implementation and no imp provider route. Official CLI/API docs must be researched safely first. | Cursor subscription/API compatibility if stable official credential route exists. | High/unknown: provider endpoint compatibility and credential storage unknown. | Only official `CURSOR_API_KEY` or user-consented CLI credentials allowed. | Research doc first; no provider files until route is proven. | docs/local metadata research only; no token printing or login mutation. | Continue `275.9`; create implementation only if recommendation becomes go. |

### Cursor safety boundary

Unofficial Cursor work may only use user-consented, stable, official credential paths such as documented API keys (`CURSOR_API_KEY`) or documented CLI browser-login state if it is explicitly intended for CLI/API use. It must not scrape browser cookies, print tokens, silently extract hidden credentials, mutate login/logout state during research, or claim subscription model routing works without proving endpoint/request compatibility.

### Dispatch order

1. Reopen or replace stale zero-test implementation units for Z.AI and Google Gemini CLI with real gates.
2. Implement Z.AI API-key provider first.
3. Implement Google Gemini CLI OAuth/provider route second.
4. Research/implement GitHub Copilot only after the simpler missing provider surfaces are stable.
5. Complete Google Antigravity risk assessment; defer implementation unless accepted.
6. Complete Cursor feasibility research; defer implementation unless a safe official credential/provider route is proven.

### Child unit decisions

- `275.5` Google Gemini CLI: keep as implementation target, but its current closed state is not trustworthy because the recorded verification filtered to zero tests. Reopen or create a replacement before implementation.
- `275.6` Antigravity: keep open as research/risk assessment.
- `275.7` Z.AI: keep as implementation target, but current closed zero-test state is stale. Reopen or create a replacement with real tests before implementation.
- `275.8` GitHub Copilot: keep as implementation target after research/sequence, but current closed zero-test state is stale.
- `275.9` Cursor: keep open as research with strict safety boundary.
