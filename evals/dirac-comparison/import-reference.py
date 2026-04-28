#!/usr/bin/env python3
"""Import Dirac public eval reference patches and metadata.

Downloads raw patch files from github.com/dirac-run/dirac/evals/<agent>/ and
writes per-patch metadata used by the imp comparison harness.
"""

from __future__ import annotations

import argparse
import json
import re
import urllib.request
from pathlib import Path

GITHUB_API = "https://api.github.com/repos/dirac-run/dirac/contents/evals/{agent}?ref=master"
TASK_RE = re.compile(r"(?:^|_)(?:refactor|code_refactor)_(.+?)(?:_(?:FAILURE|WRONG|2missing))?$")
DIFF_RE = re.compile(r"^diff --git a/(.*?) b/(.*?)$", re.MULTILINE)
INDEX_RE = re.compile(r"^index ([0-9a-f]+)\.\.([0-9a-f]+)(?:\s+\d+)?$", re.MULTILINE)


def read_json(url: str) -> object:
    with urllib.request.urlopen(url) as response:
        return json.loads(response.read().decode("utf-8"))


def read_text(url: str) -> str:
    with urllib.request.urlopen(url) as response:
        return response.read().decode("utf-8")


def task_name(filename: str) -> str:
    match = TASK_RE.search(filename)
    return match.group(1) if match else filename


def patch_metadata(agent: str, name: str, text: str, download_url: str) -> dict[str, object]:
    files = [{"old_path": old, "new_path": new} for old, new in DIFF_RE.findall(text)]
    indexes = [
        {"old_blob": old, "new_blob": new}
        for old, new in INDEX_RE.findall(text)
    ]
    added = sum(1 for line in text.splitlines() if line.startswith("+") and not line.startswith("+++"))
    removed = sum(1 for line in text.splitlines() if line.startswith("-") and not line.startswith("---"))
    return {
        "agent": agent,
        "task": task_name(name),
        "name": name,
        "source_url": download_url,
        "changed_files": files,
        "file_count": len(files),
        "indexes": indexes,
        "added_lines": added,
        "removed_lines": removed,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--agent", default="dirac", help="eval agent directory to import")
    parser.add_argument(
        "--out",
        default="evals/dirac-comparison/reference",
        help="reference output root",
    )
    args = parser.parse_args()

    out_root = Path(args.out)
    patch_dir = out_root / args.agent
    meta_dir = out_root / "metadata" / args.agent
    patch_dir.mkdir(parents=True, exist_ok=True)
    meta_dir.mkdir(parents=True, exist_ok=True)

    items = read_json(GITHUB_API.format(agent=args.agent))
    if not isinstance(items, list):
        raise RuntimeError("unexpected GitHub API response")

    manifest = []
    for item in items:
        if not isinstance(item, dict) or item.get("type") != "file":
            continue
        name = str(item["name"])
        download_url = str(item["download_url"])
        text = read_text(download_url)
        patch_path = patch_dir / name
        patch_path.write_text(text)
        metadata = patch_metadata(args.agent, name, text, download_url)
        metadata["patch_path"] = str(patch_path)
        meta_path = meta_dir / f"{name}.json"
        meta_path.write_text(json.dumps(metadata, indent=2) + "\n")
        manifest.append(metadata)

    manifest_path = out_root / "manifest.json"
    existing = []
    if manifest_path.exists():
        existing = json.loads(manifest_path.read_text())
        existing = [entry for entry in existing if entry.get("agent") != args.agent]
    manifest_path.write_text(json.dumps(existing + manifest, indent=2) + "\n")
    print(f"imported {len(manifest)} {args.agent} patches into {patch_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
