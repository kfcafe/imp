# imp benchmarking + memory checks

This directory contains local developer scripts for benchmarking, leak checks,
and stronger memory-safety validation around `imp-core`.

## Scripts

- `run-benchmarks.sh` — runs the existing grep benchmark plus the added hot-path benchmark
- `run-leaks.sh` — runs focused macOS `leaks` checks against selected `imp-core` tests
- `run-miri.sh` — runs selected `imp-core` tests under Miri on nightly
- `run-asan.sh` — runs focused tests with AddressSanitizer on nightly
- `run-tsan.sh` — runs focused tests with ThreadSanitizer on nightly
- `run-stress.sh` — repeats benchmarks and focused tests for retention/regression spotting

## Notes

- These scripts assume a local macOS development environment.
- `run-leaks.sh` requires `/usr/bin/leaks`.
- Miri and sanitizer scripts require a nightly Rust toolchain.
- Sanitizer support can vary by host/toolchain; treat these as developer diagnostics rather than CI guarantees.
- `IMP_SHELL=sh` is forced in several scripts to keep bash-tool behavior deterministic in tests.

## Typical workflow

```bash
cd /Users/asher/tower/imp
bash tools/run-benchmarks.sh
bash tools/run-leaks.sh
bash tools/run-miri.sh
bash tools/run-asan.sh
bash tools/run-tsan.sh
bash tools/run-stress.sh
```
