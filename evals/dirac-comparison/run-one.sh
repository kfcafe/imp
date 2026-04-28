#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: run-one.sh --task TASK [--provider PROVIDER] [--model MODEL] [--dry-run] [--prepare-only]

Creates a stable result directory for one Dirac-derived imp eval.

Task specs live at evals/dirac-comparison/tasks/<task>.json.
Real runs clone or reuse an isolated checkout under evals/dirac-comparison/worktrees/.
USAGE
}

TASK=""
PROVIDER=""
MODEL=""
DRY_RUN=0
PREPARE_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task) TASK="${2:-}"; shift 2 ;;
    --provider) PROVIDER="${2:-}"; shift 2 ;;
    --model) MODEL="${2:-}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --prepare-only) PREPARE_ONLY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
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
WORKTREES="$ROOT/worktrees"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SAFE_PROVIDER="${PROVIDER:-dryrun}"
SAFE_MODEL="${MODEL:-dryrun}"
SAFE_PROVIDER="${SAFE_PROVIDER//[^A-Za-z0-9._-]/_}"
SAFE_MODEL="${SAFE_MODEL//[^A-Za-z0-9._-]/_}"
OUT="$RESULTS/${STAMP}-${TASK}-${SAFE_PROVIDER}-${SAFE_MODEL}"
mkdir -p "$OUT" "$WORKTREES"

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

json_field() {
  python3 - "$SPEC" "$1" <<'PY'
import json, sys
spec=json.load(open(sys.argv[1]))
value=spec.get(sys.argv[2])
print("" if value is None else value)
PY
}

ID="$(json_field id)"
REPO="$(json_field repo)"
COMMIT="$(json_field commit)"
PROMPT="$(json_field prompt)"
VERIFIER="$(json_field verifier)"
IMP_COMMAND_TEMPLATE="$(json_field imp_command)"
CHECKOUT="$WORKTREES/$TASK"

printf '%s' "$PROMPT" > "$OUT/prompt.md"
: > "$OUT/transcript.txt"
: > "$OUT/diff.patch"
: > "$OUT/verifier.txt"

write_result() {
  local status="$1"
  local verifier_exit="$2"
  local passed="$3"
  local error_json="$4"
  python3 - "$SPEC" "$OUT" "$PROVIDER" "$MODEL" "$status" "$verifier_exit" "$passed" "$error_json" "$CHECKOUT" <<'PY'
import json, pathlib, sys
spec_path=pathlib.Path(sys.argv[1])
out=pathlib.Path(sys.argv[2])
provider=sys.argv[3] or None
model=sys.argv[4] or None
status=sys.argv[5]
verifier_exit=None if sys.argv[6] == "" else int(sys.argv[6])
passed=None if sys.argv[7] == "null" else (sys.argv[7] == "true")
error=None if sys.argv[8] == "null" else sys.argv[8]
checkout=sys.argv[9]
spec=json.loads(spec_path.read_text())
result={
  "task": spec.get("id", spec_path.stem),
  "status": status,
  "source": {"repo": spec.get("repo"), "commit": spec.get("commit")},
  "checkout": checkout,
  "agent": {"command": None if status == "dry_run" else spec.get("imp_command"), "provider": provider, "model": model},
  "verifier": {"command": spec.get("verifier"), "exit_code": verifier_exit, "passed": passed},
  "usage": {"input_tokens": None, "output_tokens": None, "cost_usd": None},
  "artifacts": {"diff": "diff.patch", "transcript": "transcript.txt", "verifier": "verifier.txt"},
  "dry_run": status == "dry_run",
}
if error:
  result["error"] = error
(out/"result.json").write_text(json.dumps(result, indent=2)+"\n")
PY
}

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "dry-run placeholder" > "$OUT/transcript.txt"
  echo "dry-run placeholder" > "$OUT/diff.patch"
  echo "dry-run placeholder" > "$OUT/verifier.txt"
  write_result dry_run "" null null
  echo "Dry-run result scaffold created: $OUT"
  exit 0
fi

if [[ -z "$REPO" ]]; then
  echo "Task spec missing repo" | tee "$OUT/transcript.txt" >&2
  write_result blocked "" null "missing repo"
  exit 1
fi

if [[ -z "$COMMIT" ]]; then
  echo "Task $TASK has no pinned commit; refusing comparable benchmark run." | tee "$OUT/transcript.txt" >&2
  write_result blocked "" null "missing pinned commit"
  exit 1
fi

if [[ ! -d "$CHECKOUT/.git" ]]; then
  git clone "$REPO" "$CHECKOUT" |& tee -a "$OUT/transcript.txt"
fi

(
  cd "$CHECKOUT"
  git fetch --all --tags --prune
  git checkout "$COMMIT"
  git reset --hard
  git clean -fd
) |& tee -a "$OUT/transcript.txt"

if [[ "$PREPARE_ONLY" -eq 1 ]]; then
  write_result prepared "" null null
  echo "Prepared checkout and result scaffold: $OUT"
  exit 0
fi

if [[ -z "$PROVIDER" || -z "$MODEL" ]]; then
  echo "--provider and --model are required for real agent execution" | tee -a "$OUT/transcript.txt" >&2
  write_result blocked "" null "missing provider/model"
  exit 1
fi

if ! command -v imp >/dev/null 2>&1; then
  echo "imp binary not found on PATH" | tee -a "$OUT/transcript.txt" >&2
  write_result blocked "" null "imp binary not found"
  exit 1
fi

set +e
(
  cd "$CHECKOUT"
  PROVIDER="$PROVIDER" MODEL="$MODEL" bash -lc "$IMP_COMMAND_TEMPLATE"
) < "$OUT/prompt.md" |& tee -a "$OUT/transcript.txt"
AGENT_EXIT=${PIPESTATUS[0]}
set -e

(
  cd "$CHECKOUT"
  git diff -- . > "$OUT/diff.patch"
)

if [[ "$AGENT_EXIT" -ne 0 ]]; then
  echo "Agent command exited $AGENT_EXIT" | tee -a "$OUT/transcript.txt" >&2
fi

MODIFIED_FILES="$(cd "$CHECKOUT" && git diff --name-only --diff-filter=ACMR | tr '\n' ' ')"
VERIFIER_CMD="$VERIFIER"
if [[ "$VERIFIER_CMD" == *"<modified-files>"* ]]; then
  VERIFIER_CMD="${VERIFIER_CMD//<modified-files>/$MODIFIED_FILES}"
fi

set +e
(
  cd "$CHECKOUT"
  if [[ -z "$VERIFIER_CMD" || "$VERIFIER_CMD" == *TBD* ]]; then
    echo "Verifier unresolved: $VERIFIER"
    exit 125
  fi
  bash -lc "$VERIFIER_CMD"
) > "$OUT/verifier.txt" 2>&1
VERIFIER_EXIT=$?
set -e

if [[ "$AGENT_EXIT" -eq 0 && "$VERIFIER_EXIT" -eq 0 ]]; then
  write_result completed "$VERIFIER_EXIT" true null
else
  write_result failed "$VERIFIER_EXIT" false "agent_exit=$AGENT_EXIT"
fi

echo "Result written: $OUT"
exit 0
