#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

export IMP_SHELL=sh

echo "== imp-core benchmark suite =="
echo

echo "-- cargo bench: grep_vs_probe --"
cargo bench -p imp-core --bench grep_vs_probe

echo
echo "-- cargo bench: core_hot_paths --"
cargo bench -p imp-core --bench core_hot_paths
