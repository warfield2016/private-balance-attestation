#!/usr/bin/env bash
# Cast a gated governance vote end-to-end.
#
# Requires a running local LEZ sequencer + the verifier program + the
# example governance program deployed. See ../README.md for setup.

set -euo pipefail

if [[ "${RISC0_DEV_MODE:-}" != "0" ]]; then
  echo "[demo] WARNING: RISC0_DEV_MODE != 0 — proofs are unsound." >&2
fi

VOTER=""
PROPOSAL=""
CHOICE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --voter)    VOTER="$2"; shift 2 ;;
    --proposal) PROPOSAL="$2"; shift 2 ;;
    --vote)     CHOICE="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
done

[[ -n "$VOTER"   ]] || { echo "missing --voter"; exit 2; }
[[ -n "$PROPOSAL" ]] || { echo "missing --proposal"; exit 2; }
[[ -n "$CHOICE"  ]] || { echo "missing --vote"; exit 2; }

KEYS_DIR="keys/${VOTER}"
[[ -d "$KEYS_DIR" ]] || { echo "no keys for ${VOTER} at ${KEYS_DIR}"; exit 3; }

PROGRAM_ID="$(cat artifacts/governance.program_id)"
GATE_SEED="$(xxd -p artifacts/gate_seed-${PROPOSAL}.bin | tr -d '\n')"

echo "[1/4] deriving context-id"
CONTEXT_ID="$(attestation-cli context-id program "$PROGRAM_ID" "0x${GATE_SEED}")"

echo "[2/4] proving (this will take ~60s with RISC0_DEV_MODE=0)"
attestation-cli prove \
  --commitment "$(cat ${KEYS_DIR}/commitment.hex)" \
  --threshold 1000 \
  --context-id "$CONTEXT_ID" \
  --presenter-pk "$(attestation-cli pubkey --keypath ${KEYS_DIR}/spending.key)" \
  --witness "${KEYS_DIR}/witness.json" \
  --out "out/${VOTER}-prop-${PROPOSAL}.proof"

echo "[3/4] submitting vote"
attestation-cli submit \
  --program "$PROGRAM_ID" \
  --proof "out/${VOTER}-prop-${PROPOSAL}.proof" \
  --action-tag "prop#${PROPOSAL}-${CHOICE}"

echo "[4/4] reading proposal state"
attestation-cli inspect-state --program "$PROGRAM_ID"
