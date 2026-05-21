# Terminal Bench 2 Harbor adapter scaffold

This directory contains minimal scaffolding for running imp through a Harbor-style Terminal Bench 2 adapter.

## Files

- `harbor_imp_agent.py` — Python adapter wrapper that resolves the imp binary and executes a prompt command.
- `run-termbench2.sh` — local runner wrapper for invoking the adapter with environment configuration.

## Usage

Set `IMP_BINARY` to a local imp binary, or leave it unset to resolve `imp` from `PATH`:

```sh
IMP_BINARY=/path/to/imp ./evals/terminal-bench-2/run-termbench2.sh "Solve the task"
```

Optional environment:

- `IMP_RELEASE_CHANNEL=edge` documents that an external runner should prefer an edge binary.
- `IMP_BINARY_URL` may be consumed by a Harbor job before invoking this scaffold; this script does not download binaries.

## Limitations

This is scaffolding only. It verifies local adapter syntax and command construction, but does not claim an end-to-end Harbor or Terminal Bench 2 run has passed in this repository environment.
