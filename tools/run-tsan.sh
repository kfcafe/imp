#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

export IMP_SHELL=sh

TESTS=(
  tools::bash::tests::bash_streaming_output
  agent::tests::agent_concurrent_readonly
)

for test_name in "${TESTS[@]}"; do
  echo
  echo "-- tsan: $test_name --"
  RUSTFLAGS="-Zsanitizer=thread" \
  RUSTDOCFLAGS="-Zsanitizer=thread" \
  TSAN_OPTIONS="halt_on_error=1:abort_on_error=1" \
  rustup run nightly cargo test -Zbuild-std --target aarch64-apple-darwin -p imp-core \
    "$test_name" -- --exact --nocapture
done
