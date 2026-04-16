#!/bin/sh
set -eu

BIN=${1:-/Users/asher/.local/imp-current/bin/imp}

if [ "$(uname -s)" != "Darwin" ]; then
  exit 0
fi

if [ ! -f "$BIN" ]; then
  echo "imp-fix-signature: missing binary: $BIN" >&2
  exit 1
fi

codesign --force --sign - "$BIN"
