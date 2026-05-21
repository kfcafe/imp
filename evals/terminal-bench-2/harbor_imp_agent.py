#!/usr/bin/env python3
"""Minimal Harbor/Terminal Bench 2 adapter scaffold for imp.

The adapter intentionally stays small: it resolves an imp binary and executes a
single prompt in print/headless style. External Harbor job configuration remains
responsible for task checkout, container setup, and benchmark result collection.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ImpAgentConfig:
    binary: Path
    model: str | None = None
    release_channel: str | None = None


def resolve_imp_binary() -> Path:
    configured = os.environ.get("IMP_BINARY")
    if configured:
        return Path(configured)

    discovered = shutil.which("imp")
    if discovered:
        return Path(discovered)

    raise FileNotFoundError("set IMP_BINARY or put `imp` on PATH")


def build_command(config: ImpAgentConfig, prompt: str) -> list[str]:
    command = [str(config.binary), "print", prompt]
    if config.model:
        command[2:2] = ["--model", config.model]
    return command


def run_prompt(config: ImpAgentConfig, prompt: str) -> int:
    command = build_command(config, prompt)
    completed = subprocess.run(command, check=False)
    return completed.returncode


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run imp as a Terminal Bench 2 Harbor agent")
    parser.add_argument("prompt", nargs="?", default=os.environ.get("TB_PROMPT", ""))
    parser.add_argument("--model", default=os.environ.get("IMP_MODEL"))
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if not args.prompt:
        print("prompt is required as an argument or TB_PROMPT", file=sys.stderr)
        return 2

    try:
        config = ImpAgentConfig(
            binary=resolve_imp_binary(),
            model=args.model,
            release_channel=os.environ.get("IMP_RELEASE_CHANNEL"),
        )
    except FileNotFoundError as exc:
        print(str(exc), file=sys.stderr)
        return 127

    return run_prompt(config, args.prompt)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
