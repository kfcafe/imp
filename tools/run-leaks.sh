#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

export IMP_SHELL=sh

echo "== imp-core leak checks (macOS leaks) =="

cargo test -p imp-core --no-run --message-format=json > /tmp/imp_core_test_artifacts.jsonl 2>/dev/null
TEST_BIN=$(python3 - <<'PY'
import json
with open('/tmp/imp_core_test_artifacts.jsonl') as f:
    for line in f:
        try:
            obj = json.loads(line)
        except Exception:
            continue
        if obj.get('reason') == 'compiler-artifact' and obj.get('profile', {}).get('test') and obj.get('target', {}).get('name') == 'imp_core' and obj.get('executable'):
            print(obj['executable'])
            break
PY
)

if [[ -z "${TEST_BIN:-}" ]]; then
  echo "failed to find imp-core test binary" >&2
  exit 1
fi

echo
echo "-- leaks: bash timeout test --"
leaks --atExit -- "$TEST_BIN" tools::bash::tests::bash_timeout --exact --nocapture

echo
echo "-- leaks: bash streaming test --"
leaks --atExit -- "$TEST_BIN" tools::bash::tests::bash_streaming_output --exact --nocapture

echo
echo "-- leaks: context masking test --"
leaks --atExit -- "$TEST_BIN" context::tests::context_usage_masked_vs_unmasked --exact --nocapture

echo
echo "-- leaks: session listing test --"
leaks --atExit -- "$TEST_BIN" session::tests::session_list --exact --nocapture
