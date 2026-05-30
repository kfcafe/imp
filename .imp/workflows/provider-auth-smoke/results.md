# Provider/auth smoke results

Status: done

## Anthropic setup path checked

- `imp --list-models` listed Anthropic models, including `claude-sonnet-4-6`, `claude-haiku-4-5-20251001`, and `claude-opus-4-6`.
- `imp login --help` documented Anthropic as the default login provider and showed the provider argument shape.
- Isolated no-key one-shot run for Anthropic exited with status 1 and a clear setup error:

```text
No readable secret field 'api_key' found for anthropic. Set ANTHROPIC_API_KEY or run `imp secrets anthropic` to save it again.
```

No live model call was made.

## OpenAI-compatible setup path checked

- `imp --list-models` listed OpenAI models and OpenAI-compatible/provider-family models.
- `imp login openai --help` documented the provider argument shape for OpenAI OAuth setup.
- `imp secrets --help` documented generic provider/service secret setup, including custom services.

## Secrets commands checked

- `imp secrets list` completed and displayed provider names/status/field names only.
- `imp secrets doctor` completed and displayed field readability status only.
- Output artifact was scanned for common API-key patterns; no secret values were found.

## Verification

Output artifact: `artifacts/provider-auth-smoke-output.log`.

Post-run checks:

```sh
git diff --check -- .imp/workflows/provider-auth-smoke
rg -n 'sk-[A-Za-z0-9]|AIza[0-9A-Za-z_-]|ANTHROPIC_API_KEY=[^ ]+|OPENAI_API_KEY=[^ ]+' \
  .imp/workflows/provider-auth-smoke/artifacts/provider-auth-smoke-output.log -S
```

Result: `provider auth artifact secret-pattern scan clean`.
