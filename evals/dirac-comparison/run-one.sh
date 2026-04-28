#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: run-one.sh --task TASK [--provider PROVIDER] [--model MODEL] [--dry-run]

Creates a stable result directory for one Dirac-derived imp eval. In --dry-run
mode it writes scaffolding only and does not clone repos or invoke imp.

Task specs live at evals/dirac-comparison/tasks/<task>.json.
USAGE
}

TASK=""
PROVIDER=""
MODEL=""
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task)
      TASK="${2:-}"
      shift 2
      ;;
    --provider)
      PROVIDER="${2:-}"
      shift 2
      ;;
    --model)
      MODEL="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$TASK" ]]; then
  echo "--task is required" >&2
  usage >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SPEC="$ROOT/tasks/$TASK.json"
RESULTS="$ROOT/results"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SAFE_PROVIDER="${PROVIDER:-dryrun}"
SAFE_MODEL="${MODEL:-dryrun}"
SAFE_PROVIDER="${SAFE_PROVIDER//[^A-Za-z0-9._-]/_}"
SAFE_MODEL="${SAFE_MODEL//[^A-Za-z0-9._-]/_}"
OUT="$RESULTS/${STAMP}-${TASK}-${SAFE_PROVIDER}-${SAFE_MODEL}"

mkdir -p "$OUT"

if [[ ! -f "$SPEC" ]]; then
  cat > "$OUT/result.json" <<JSON
{
  "task": "$TASK",
  "status": "missing_spec",
  "error": "Task spec not found: $SPEC",
  "dry_run": $([[ "$DRY_RUN" -eq 1 ]] && echo true || echo false)
}
JSON
  echo "Task spec not found: $SPEC" >&2
  echo "Created $OUT/result.json"
  exit 1
fi

python3 - "$SPEC" "$OUT" "$PROVIDER" "$MODEL" "$DRY_RUN" <<'PY'
import json
import pathlib
import sys

spec_path = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
provider = sys.argv[3] or None
model = sys.argv[4] or None
dry_run = sys.argv[5] == "1"
spec = json.loads(spec_path.read_text())

(out / "prompt.md").write_text(spec.get("prompt", ""))
for name in ["transcript.txt", "diff.patch", "verifier.txt"]:
    (out / name).write_text("dry-run placeholder\n" if dry_run else "")

result = {
    "task": spec.get("id", spec_path.stem),
    "source": {
        "repo": spec.get("repo"),
        "commit": spec.get("commit"),
    },
    "agent": {
        "command": None if dry_run else spec.get("imp_command"),
        "provider": provider,
        "model": model,
    },
    "verifier": {
        "command": spec.get("verifier"),
        "exit_code": None,
        "passed": None,
    },
    "usage": {
        "input_tokens": None,
        "output_tokens": None,
        "cost_usd": None,
    },
    "artifacts": {
        "diff": "diff.patch",
        "transcript": "transcript.txt",
        "verifier": "verifier.txt",
    },
    "dry_run": dry_run,
}
(out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
print(out)
PY

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "Dry-run result scaffold created: $OUT"
  exit 0
fi

echo "Real execution is intentionally staged behind task specs."
echo "Next implementation step: clone pinned repo, reset, run imp, capture diff, run verifier."
echo "Result scaffold created: $OUT"
