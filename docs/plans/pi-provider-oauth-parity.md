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
