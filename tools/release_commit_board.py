#!/usr/bin/env python3
"""Generate a local HTML commit board for curating imp stable releases.

The board is intentionally static: it embeds git-derived commit data and uses
browser localStorage for promotion classifications/notes. Re-run this script to
refresh commit data after branch changes.
"""
from __future__ import annotations

import argparse
import html
import json
import subprocess
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from pathlib import Path


RISKY_PATH_HINTS = {
    "crates/imp-core/src/agent": 4,
    "crates/imp-core/src/mana_worker": 4,
    "crates/imp-core/src/tools/mana": 4,
    "crates/imp-core/src/reference_monitor": 4,
    "crates/imp-core/src/workflow": 4,
    "crates/imp-tui/src/app": 4,
    "crates/imp-tui/src/event_source": 4,
    "crates/imp-llm/src/providers": 3,
    "Cargo.lock": 3,
    "Cargo.toml": 3,
    ".github/workflows": 3,
}

LOW_RISK_PATH_HINTS = {
    "README": -1,
    "CHANGELOG": -1,
    "docs/": -2,
}

SAFE_SUBJECT_HINTS = ("format", "readme", "docs", "changelog", "version", "typo")
RISKY_SUBJECT_HINTS = (
    "runtime",
    "workflow",
    "agent loop",
    "worker",
    "policy",
    "reference monitor",
    "auth",
    "transport",
    "tui",
    "merge",
    "revert",
)


@dataclass
class Commit:
    sha: str
    short: str
    subject: str
    author: str
    date: str
    side: str
    parents: int
    files: list[str]
    insertions: int
    deletions: int
    risk_score: int
    risk_label: str
    risk_reasons: list[str]


def git(args: list[str]) -> str:
    return subprocess.check_output(["git", *args], text=True, stderr=subprocess.PIPE)


def git_lines(args: list[str]) -> list[str]:
    out = git(args).strip()
    return [] if not out else out.splitlines()


def commit_shas(base: str, target: str, side: str, limit: int | None, cherry_pick: bool) -> list[str]:
    range_arg = f"{base}...{target}"
    side_flag = "--right-only" if side == "nightly-only" else "--left-only"
    args = ["log", side_flag, "--format=%H", range_arg]
    if cherry_pick:
        args.insert(1, "--cherry-pick")
    if limit:
        args.insert(1, f"-{limit}")
    return git_lines(args)


def parse_numstat(sha: str) -> tuple[list[str], int, int]:
    lines = git_lines(["show", "--format=", "--numstat", "--find-renames", sha])
    files: list[str] = []
    insertions = 0
    deletions = 0
    for line in lines:
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        add, delete, path = parts[0], parts[1], parts[2]
        files.append(path)
        if add != "-":
            insertions += int(add)
        if delete != "-":
            deletions += int(delete)
    return files, insertions, deletions


def risk_for(subject: str, parents: int, files: list[str], insertions: int, deletions: int) -> tuple[int, str, list[str]]:
    score = 0
    reasons: list[str] = []
    lowered = subject.lower()

    if parents > 1:
        score += 5
        reasons.append("merge commit")
    if any(h in lowered for h in RISKY_SUBJECT_HINTS):
        score += 2
        reasons.append("risky subject keyword")
    if any(h in lowered for h in SAFE_SUBJECT_HINTS):
        score -= 1
        reasons.append("low-risk subject keyword")

    churn = insertions + deletions
    if churn > 2500:
        score += 5
        reasons.append(f"very high churn ({churn} lines)")
    elif churn > 500:
        score += 3
        reasons.append(f"high churn ({churn} lines)")
    elif churn > 150:
        score += 1
        reasons.append(f"moderate churn ({churn} lines)")

    for path in files:
        for hint, weight in RISKY_PATH_HINTS.items():
            if path.startswith(hint) or path == hint:
                score += weight
                reasons.append(f"touches {hint}")
                break
        for hint, weight in LOW_RISK_PATH_HINTS.items():
            if path.startswith(hint) or path.startswith(hint.rstrip("/")):
                score += weight
                reasons.append(f"mostly {hint}")
                break

    if score <= 1:
        label = "low"
    elif score <= 5:
        label = "medium"
    else:
        label = "high"
    return max(score, 0), label, sorted(set(reasons))


def load_commit(sha: str, side: str) -> Commit:
    meta = git(["show", "-s", "--format=%H%x00%h%x00%s%x00%an%x00%aI%x00%P", sha]).rstrip("\n")
    full, short, subject, author, date, parents_raw = meta.split("\0")
    parents = len(parents_raw.split()) if parents_raw else 0
    files, insertions, deletions = parse_numstat(sha)
    score, label, reasons = risk_for(subject, parents, files, insertions, deletions)
    return Commit(full, short, subject, author, date, side, parents, files, insertions, deletions, score, label, reasons)


def render(commits: list[Commit], base: str, target: str) -> str:
    data = json.dumps([asdict(c) for c in commits], ensure_ascii=False)
    generated = datetime.now(timezone.utc).isoformat(timespec="seconds")
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>imp release commit board</title>
<style>
:root {{ color-scheme: dark; --bg:#10100f; --panel:#191816; --muted:#a9a29a; --text:#f4efe7; --line:#302d29; --accent:#e7b75f; --low:#82c891; --med:#e7b75f; --high:#e06c75; }}
* {{ box-sizing:border-box; }}
body {{ margin:0; font:14px/1.45 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background:var(--bg); color:var(--text); }}
header {{ position:sticky; top:0; z-index:5; padding:18px 22px; background:rgba(16,16,15,.96); border-bottom:1px solid var(--line); backdrop-filter: blur(10px); }}
h1 {{ margin:0 0 6px; font-size:22px; }}
.sub {{ color:var(--muted); }}
.controls {{ display:flex; flex-wrap:wrap; gap:10px; margin-top:14px; align-items:center; }}
input, select, button {{ background:#24211d; color:var(--text); border:1px solid var(--line); border-radius:8px; padding:8px 10px; }}
input {{ min-width:260px; flex:1; }}
button {{ cursor:pointer; }}
main {{ display:grid; grid-template-columns: 1fr 340px; gap:18px; padding:18px 22px; }}
.card {{ background:var(--panel); border:1px solid var(--line); border-radius:12px; padding:14px; margin-bottom:12px; }}
.commit {{ display:grid; gap:10px; }}
.row {{ display:flex; flex-wrap:wrap; gap:8px; align-items:center; }}
.sha {{ font-family:ui-monospace, SFMono-Regular, Menlo, monospace; color:var(--accent); }}
.subject {{ font-size:16px; font-weight:650; }}
.badge {{ border:1px solid var(--line); border-radius:999px; padding:2px 8px; color:var(--muted); }}
.badge.low {{ color:var(--low); border-color:rgba(130,200,145,.45); }}
.badge.medium {{ color:var(--med); border-color:rgba(231,183,95,.45); }}
.badge.high {{ color:var(--high); border-color:rgba(224,108,117,.45); }}
.files {{ max-height:120px; overflow:auto; color:var(--muted); font-family:ui-monospace, SFMono-Regular, Menlo, monospace; font-size:12px; padding-left:18px; }}
.notes {{ width:100%; min-height:56px; resize:vertical; background:#111; color:var(--text); border:1px solid var(--line); border-radius:8px; padding:8px; }}
aside {{ position:sticky; top:116px; align-self:start; }}
.summary strong {{ color:var(--accent); }}
.hidden {{ display:none; }}
@media (max-width: 900px) {{ main {{ grid-template-columns:1fr; }} aside {{ position:static; }} }}
</style>
</head>
<body>
<header>
  <h1>imp release commit board</h1>
  <div class="sub">Generated {html.escape(generated)} · comparing <code>{html.escape(base)}</code> ↔ <code>{html.escape(target)}</code>. Classifications are stored in this browser's localStorage.</div>
  <div class="controls">
    <input id="q" placeholder="Filter commits, files, reasons…">
    <select id="side"><option value="all">both sides</option><option value="nightly-only">nightly-only candidates</option><option value="release-only">release-only drift</option></select>
    <select id="risk"><option value="all">all risk</option><option value="low">low</option><option value="medium">medium</option><option value="high">high</option></select>
    <select id="status"><option value="all">all statuses</option><option value="unreviewed">unreviewed</option><option value="promote">promote</option><option value="defer">defer</option><option value="reject">reject</option><option value="release-only">release-only</option><option value="needs-split">needs split</option></select>
    <button id="export">Export decisions JSON</button>
  </div>
</header>
<main>
<section id="commits"></section>
<aside class="card summary">
  <h2>How to use</h2>
  <p><strong>nightly-only</strong> commits are candidates to cherry-pick into stable release.</p>
  <p><strong>release-only</strong> commits are drift already on release; classify as intentional release-only, keep/promoted, or reject.</p>
  <p>Prefer <code>git cherry-pick -x &lt;sha&gt;</code> for promotions so release keeps provenance.</p>
  <h2>Counts</h2>
  <div id="counts"></div>
</aside>
</main>
<script id="commit-data" type="application/json">{html.escape(data)}</script>
<script>
const commits = JSON.parse(document.getElementById('commit-data').textContent);
const key = 'imp-release-board-decisions-v1';
let decisions = JSON.parse(localStorage.getItem(key) || '{{}}');
const $ = id => document.getElementById(id);
function save() {{ localStorage.setItem(key, JSON.stringify(decisions, null, 2)); }}
function decision(sha) {{ return decisions[sha] || {{status:'unreviewed', notes:''}}; }}
function matches(c) {{
  const q = $('q').value.toLowerCase();
  const d = decision(c.sha);
  if ($('side').value !== 'all' && c.side !== $('side').value) return false;
  if ($('risk').value !== 'all' && c.risk_label !== $('risk').value) return false;
  if ($('status').value !== 'all' && d.status !== $('status').value) return false;
  const hay = [c.sha,c.short,c.subject,c.author,c.side,c.risk_label,...c.files,...c.risk_reasons,d.status,d.notes].join('\n').toLowerCase();
  return hay.includes(q);
}}
function render() {{
  const root = $('commits'); root.innerHTML = '';
  const visible = commits.filter(matches);
  const counts = {{}};
  commits.forEach(c => {{ const s = decision(c.sha).status; counts[s] = (counts[s]||0)+1; }});
  $('counts').innerHTML = Object.entries(counts).sort().map(([k,v]) => `<div>${{k}}: <strong>${{v}}</strong></div>`).join('') + `<hr><div>visible: <strong>${{visible.length}}</strong> / ${{commits.length}}</div>`;
  for (const c of visible) {{
    const d = decision(c.sha);
    const el = document.createElement('article'); el.className = 'card commit';
    el.innerHTML = `
      <div class="row"><span class="sha">${{c.short}}</span><span class="badge">${{c.side}}</span><span class="badge ${{c.risk_label}}">${{c.risk_label}} risk · ${{c.risk_score}}</span><span class="badge">+${{c.insertions}}/-${{c.deletions}}</span><span class="badge">${{c.files.length}} files</span></div>
      <div class="subject">${{escapeHtml(c.subject)}}</div>
      <div class="sub">${{escapeHtml(c.author)}} · ${{c.date}} · parents: ${{c.parents}}</div>
      <div class="sub">${{c.risk_reasons.map(escapeHtml).join(' · ') || 'no obvious risk flags'}}</div>
      <div class="row"><label>Status <select data-sha="${{c.sha}}" class="status-select"><option>unreviewed</option><option>promote</option><option>defer</option><option>reject</option><option>release-only</option><option>needs-split</option></select></label><button data-copy="${{c.sha}}">copy sha</button></div>
      <textarea class="notes" data-notes="${{c.sha}}" placeholder="Reason / verification / required gate…">${{escapeHtml(d.notes || '')}}</textarea>
      <details><summary>Files</summary><ul class="files">${{c.files.map(f => `<li>${{escapeHtml(f)}}</li>`).join('')}}</ul></details>`;
    root.appendChild(el);
    el.querySelector('.status-select').value = d.status;
  }}
}}
function escapeHtml(s) {{ return String(s).replace(/[&<>"']/g, ch => ({{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}}[ch])); }}
document.addEventListener('input', e => {{
  if (e.target.classList.contains('status-select')) {{ decisions[e.target.dataset.sha] = {{...decision(e.target.dataset.sha), status:e.target.value}}; save(); render(); return; }}
  if (e.target.classList.contains('notes')) {{ decisions[e.target.dataset.notes] = {{...decision(e.target.dataset.notes), notes:e.target.value}}; save(); return; }}
  if (['q','side','risk','status'].includes(e.target.id)) render();
}});
document.addEventListener('click', e => {{
  if (e.target.dataset.copy) navigator.clipboard.writeText(e.target.dataset.copy);
  if (e.target.id === 'export') {{
    const blob = new Blob([JSON.stringify(decisions, null, 2)], {{type:'application/json'}});
    const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = 'imp-release-decisions.json'; a.click(); URL.revokeObjectURL(a.href);
  }}
}});
render();
</script>
</body>
</html>
"""


def branch_exists(ref: str) -> bool:
    try:
        subprocess.check_call(["git", "rev-parse", "--verify", "--quiet", ref], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return True
    except subprocess.CalledProcessError:
        return False


def unique_shas(items: list[tuple[str, str]]) -> list[tuple[str, str]]:
    seen: set[str] = set()
    out: list[tuple[str, str]] = []
    for sha, side in items:
        if sha in seen:
            continue
        seen.add(sha)
        out.append((sha, side))
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="release", help="Stable branch/ref, default: release")
    parser.add_argument("--target", default="nightly", help="Integration branch/ref, default: nightly")
    parser.add_argument("--output", default="docs/release-promotions/commit-board.html")
    parser.add_argument("--limit", type=int, default=None, help="Optional per-side commit limit")
    parser.add_argument(
        "--raw",
        action="store_true",
        help="Show raw branch differences instead of hiding cherry-equivalent commits",
    )
    parser.add_argument(
        "--include-workflow",
        action="store_true",
        help="Also include commits unique to workflow when a workflow ref exists",
    )
    args = parser.parse_args()

    shas: list[tuple[str, str]] = []
    cherry_pick = not args.raw
    shas += [(sha, "nightly-only") for sha in commit_shas(args.base, args.target, "nightly-only", args.limit, cherry_pick)]
    shas += [(sha, "release-only") for sha in commit_shas(args.base, args.target, "release-only", args.limit, cherry_pick)]
    if args.include_workflow and branch_exists("workflow"):
        workflow_shas = git_lines(["log", *( [f"-{args.limit}"] if args.limit else [] ), "--format=%H", f"{args.target}..workflow"])
        shas += [(sha, "workflow-only") for sha in workflow_shas]
    commits = [load_commit(sha, side) for sha, side in unique_shas(shas)]

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(render(commits, args.base, args.target), encoding="utf-8")
    print(f"wrote {out} with {len(commits)} commits")


if __name__ == "__main__":
    main()
