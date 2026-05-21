#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
PROMPT=${1:-${TB_PROMPT:-}}

if [[ -z "$PROMPT" ]]; then
  echo "usage: $0 <prompt>" >&2
  echo "or set TB_PROMPT" >&2
  exit 2
fi

exec python3 "$SCRIPT_DIR/harbor_imp_agent.py" "$PROMPT"
