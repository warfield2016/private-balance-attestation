#!/usr/bin/env bash
# Bring up a local LEZ standalone sequencer. Placeholder until the LEZ
# sequencer release we pin against is published.

set -euo pipefail

WORKDIR="${LEZ_WORKDIR:-./.lez-devnet}"
mkdir -p "$WORKDIR"

echo "[devnet] would start sequencer in $WORKDIR (not yet wired)"
echo
echo "To run against the upstream LEZ sequencer in the meantime:"
echo "  git clone https://github.com/logos-co/lez && cd lez && cargo build --release"
echo "  lez-sequencer --mode standalone --rpc-port 8899 --data-dir $WORKDIR"
echo "  export LEZ_SEQUENCER_URL=http://127.0.0.1:8899"
echo "  scripts/demo.sh"
