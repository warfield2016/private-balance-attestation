# Example: fee-tier gate (off-chain)

A small library that picks a fee-rate tier for a user based on a
verified balance attestation. No transaction; the verifier runs
locally and applies a tier mapping.

This is the third reference integration. It's the smallest of the
three and is meant to be easy to extend.

## Files

```
src/lib.ts   verifyTierAttestation + tierForBalance
src/cli.ts   tiny driver that takes a proof file + tier table
```

## Run the scaffold

```
node --import tsx ./src/cli.ts \
     --proof       ./fixtures/sample.proof \
     --tier-table  ./fixtures/tiers.json \
     --challenge   ./fixtures/challenge.bin \
     --signature   ./fixtures/signature.bin
```

Output: `Tier awarded: 2 (threshold satisfied: 1000)`.

## Extending

The most common changes:

- Replace the in-memory tier table with whatever your service uses
  (config, on-chain account, REST endpoint).
- Swap the context-id helper (`contextIdForFeeTier`) for a generic
  helper if the gate isn't a fee tier.
- Wire the awarded tier into your pricing or trade path.

The verification call stays the same in all cases.
