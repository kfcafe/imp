#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

export IMP_SHELL=sh

echo "== imp-core Miri checks =="
rustup run nightly cargo miri setup

# Keep this list to tests that are pure/in-process enough for Miri.
# Tokio process/runtime paths used by the bash tool hit unsupported macOS
# kqueue operations under Miri and are better checked via leaks/ASan instead.
TESTS=(
  context::tests::context_usage_masked_vs_unmasked
  context::tests::mask_observations_20_turns_keeps_last_10
  session::tests::session_branch
  session::tests::session_list
  tools::tests::suggest_similar_levenshtein_transposition
)

for test_name in "${TESTS[@]}"; do
  echo
  echo "-- miri: $test_name --"
  MIRIFLAGS="-Zmiri-disable-isolation" \
    rustup run nightly cargo miri test -p imp-core "$test_name" -- --exact --nocapture
done
