# Provider/auth smoke plan

Purpose: verify provider/auth setup surfaces and no-key behavior without live model calls or exposing secret values.

Commands:

1. Use the freshly installed smoke binary where possible:

```sh
BIN=/tmp/imp-install-smoke-cargo-home/bin/imp
$BIN --list-models
$BIN login --help
$BIN login openai --help
$BIN secrets --help
$BIN secrets list
$BIN secrets doctor
```

2. Verify no-key one-shot behavior is clear and does not panic. Run with an isolated home/config/auth root and no provider keys:

```sh
TMP_HOME=/tmp/imp-provider-auth-smoke-home
rm -rf "$TMP_HOME"
mkdir -p "$TMP_HOME"
HOME="$TMP_HOME" XDG_CONFIG_HOME="$TMP_HOME/.config" \
  ANTHROPIC_API_KEY= OPENAI_API_KEY= GOOGLE_API_KEY= OPENROUTER_API_KEY= \
  $BIN -p "hello" --provider anthropic --model claude-sonnet-4 --no-tools --no-session --max-turns 1
```

Expected: command fails before a live model call with a clear missing-auth/configuration error, and output contains no secret values.

3. Record output in `provider-auth-smoke-output.log` with any expected non-zero no-key result documented.

Non-goals:

- No OAuth browser login.
- No live paid provider/API calls.
- No secret value printing.
