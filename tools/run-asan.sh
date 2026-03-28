#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

export IMP_SHELL=sh

TESTS=(
  context::tests::context_usage_masked_vs_unmasked
  session::tests::session_list
  tools::bash::tests::bash_timeout
)

for test_name in "${TESTS[@]}"; do
  echo
  echo "-- asan: $test_name --"
  RUSTFLAGS="-Zsanitizer=address" \
  RUSTDOCFLAGS="-Zsanitizer=address" \
  ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:abort_on_error=1" \
  rustup run nightly cargo test -Zbuild-std --target aarch64-apple-darwin -p imp-core \
    "$test_name" -- --exact --nocapture
done
