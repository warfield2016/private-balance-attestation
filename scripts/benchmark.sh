#!/usr/bin/env bash
# Run a proof-gen benchmark. Writes results to docs/benchmarks.md.
# Requires attestation-cli on PATH.

set -euo pipefail

if [[ "${RISC0_DEV_MODE:-}" != "0" ]]; then
  echo "warning: RISC0_DEV_MODE != 0 — numbers will be invalid." >&2
fi

RUNS="${1:-5}"
THRESHOLD="${2:-100}"

OUT="$(mktemp -d)/benchmarks.json"
attestation-cli benchmark --runs "$RUNS" --threshold "$THRESHOLD" --json > "$OUT"
echo "Results: $OUT"
echo "(parse + update docs/benchmarks.md once the benchmark subcommand is wired)"
