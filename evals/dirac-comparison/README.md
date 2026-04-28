# Dirac-derived imp eval harness

This directory is for repeatable comparison runs derived from the public Dirac eval tasks.

Source: `github.com/dirac-run/dirac`, `evals/README.md` on `master`, read 2026-04-28.

The goal is not to clone Dirac's IDE architecture. The goal is to turn the qualitative comparison into reproducible evidence for imp's code-intelligence and edit tooling: command, provider/model, prompt, diff, verifier output, pass/fail, and cost/usage when available.

## Task catalog

Initial upstream task set:

| id | name | repo | summary | verifier |
| --- | --- | --- | --- | --- |
| 1 | `extensionswb_service` | `microsoft/vscode` | Split `extensionsWorkbenchService.ts` into smaller modules, creating `extension.ts` and `extensions.ts`. | repo lint/build command to be specified after pinning commit |
| 2 | `sendRequest` | `microsoft/vscode` | Refactor chat service `sendRequest` to take one parameter object and update call sites. | repo type/lint command to be specified |
| 3 | `IOverlayWidget` | `microsoft/vscode` | Add mandatory `getName(): string` to `IOverlayWidget` and all implementations. | repo type/lint command to be specified |
| 4 | `addLogging` | `microsoft/vscode` | Add guaranteed entry/exit logging around exact `runCommand` definitions. | repo type/lint command to be specified |
| 5 | `DynamicCache` | `huggingface/transformers` | Add `DynamicCache.is_stale` and update selected model attention cache handling. | `.venv` ruff check on modified files or relevant tree |
| 6 | `stoppingcriteria` | `huggingface/transformers` | Add entropy-based generation stopping config and criterion. | verifier to be specified after pinning commit |
| 7 | `latency` | `huggingface/transformers` | Add pipeline latency telemetry and run ruff for pipelines. | `.venv` ruff check across `src/transformers/pipelines/` |
| 8 | `datadict` | `django/django` | Rename `value_from_datadict` to `extract_value_from_request` in code only. | `.venv` ruff on modified files; no tests |

Before running any task for score, pin an upstream repo commit in `tasks/<task>.json`. Do not depend on moving default branches.

## Result schema

Each run writes a directory under `results/<timestamp>-<task>-<provider>-<model>/` containing:

- `result.json` — machine-readable run metadata and pass/fail.
- `prompt.md` — exact task prompt sent to imp.
- `transcript.txt` — stdout/stderr from imp, with secrets redacted by the runner environment.
- `diff.patch` — final `git diff` of the task repo.
- `verifier.txt` — verifier command output and exit code.

`result.json` fields:

```json
{
  "task": "datadict",
  "source": { "repo": "django/django", "commit": "..." },
  "agent": { "command": "imp ...", "provider": "...", "model": "..." },
  "verifier": { "command": "...", "exit_code": 0, "passed": true },
  "usage": { "input_tokens": null, "output_tokens": null, "cost_usd": null },
  "artifacts": { "diff": "diff.patch", "transcript": "transcript.txt", "verifier": "verifier.txt" }
}
```

## Dirac reference patches

Import the published Dirac output patches and metadata with:

```sh
evals/dirac-comparison/import-reference.py --agent dirac
```

This caches raw patches under `reference/dirac/`, writes per-patch metadata under `reference/metadata/dirac/`, and updates `reference/manifest.json` with changed files, line counts, and patch `index` blob IDs. The reference diffs are comparison evidence, not proof of the upstream starting commit by themselves.

## Running one task

Use `run-one.sh` for the smallest reproducible path. It is dry-run friendly and can initialize a result directory without invoking an LLM.

```sh
# Creates result scaffolding only.
evals/dirac-comparison/run-one.sh --task datadict --dry-run

# Real run, after task JSON contains repo/commit/prompt/verifier.
evals/dirac-comparison/run-one.sh \
  --task datadict \
  --provider openrouter \
  --model anthropic/claude-sonnet-4.5
```

The script intentionally does not embed secrets. Configure provider auth through imp's normal auth/secrets mechanism or environment outside committed files.

## Benchmark discipline

- Reset task repos before every run: `git reset --hard && git clean -fd`.
- Run exactly one agent at a time in an isolated checkout.
- Capture the initial commit, final diff, verifier command, verifier exit code, provider, model, and imp command.
- Do not claim a pass without verifier evidence.
- Do not commit API keys, provider request payloads with credentials, or unredacted secret-bearing logs.
- Start with one smoke task before running a matrix.
