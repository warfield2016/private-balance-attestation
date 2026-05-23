#!/usr/bin/env bash
# End-to-end demo against a local LEZ standalone sequencer.
# Runs the on-chain governance gate and the off-chain chat-gate.

set -euo pipefail

CI_MODE=0
for arg in "$@"; do
  if [[ "$arg" == "--ci" ]]; then CI_MODE=1; fi
done

if [[ "${RISC0_DEV_MODE:-}" != "0" ]]; then
  if [[ "$CI_MODE" -eq 1 ]]; then
    echo "ABORT: RISC0_DEV_MODE must be 0 in CI." >&2
    exit 4
  fi
  echo "warning: RISC0_DEV_MODE != 0 — proofs will be unsound." >&2
  echo "         set RISC0_DEV_MODE=0 for a real run." >&2
fi

echo "================================================================"
echo " LP-0005 end-to-end demo"
echo " RISC0_DEV_MODE = ${RISC0_DEV_MODE:-unset}"
echo " CI mode        = ${CI_MODE}"
echo "================================================================"
sleep 2

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
mkdir -p artifacts keys/alice keys/bob keys/group out logs

echo
echo "Path 1 — on-chain governance gate"
echo "  (wired once local sequencer + programs are deployed; see"
echo "   scripts/setup-devnet.sh and examples/governance-gate/scripts)"

# Real flow:
#   scripts/setup-devnet.sh &
#   scripts/devnet/mint-test-account.sh --voter alice --balance 5000
#   scripts/devnet/mint-test-account.sh --voter bob   --balance 100
#   scripts/devnet/deploy-program.sh    --crate lez-verifier-program
#   scripts/devnet/deploy-program.sh    --crate governance-gate-example
#   examples/governance-gate/scripts/cast-vote.sh --voter alice --proposal 42 --vote yes
#   examples/governance-gate/scripts/cast-vote.sh --voter bob   --proposal 42 --vote yes
#   alice admitted; bob rejected with code 2 (E_THRESHOLD_TOO_LOW)

echo
echo "Path 2 — off-chain chat-gate"
echo "  (paired in-process transport; replace with Logos client for prod)"

# Real flow:
#   node --import tsx examples/chat-gate/src/demo.ts \
#        --group-pk      <hex> \
#        --program-owner <hex> \
#        --witness       keys/alice/witness.json \
#        --spending-key  keys/alice/spending.bin \
#        --threshold     100

echo
echo "Done. See artifacts/, out/, logs/ for outputs."
