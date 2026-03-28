#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

export IMP_SHELL=sh

echo "== imp-core stress suite =="

echo
echo "-- repeated benchmark binary --"
for i in {1..5}; do
  echo "run $i"
  cargo bench -p imp-core --bench core_hot_paths
done

echo
echo "-- targeted tool/session stress tests --"
TESTS=(
  tools::bash::tests::bash_timeout
  tools::bash::tests::bash_streaming_output
  session::tests::session_list
  context::tests::context_usage_masked_vs_unmasked
)

for test_name in "${TESTS[@]}"; do
  echo
  echo "stress test: $test_name"
  cargo test -p imp-core "$test_name" -- --exact --nocapture
done
