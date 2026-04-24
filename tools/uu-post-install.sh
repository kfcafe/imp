#!/usr/bin/env bash
set -euo pipefail

if command -v imp >/dev/null 2>&1; then
  imp install-local
else
  "$HOME/.cargo/bin/imp" install-local
fi
