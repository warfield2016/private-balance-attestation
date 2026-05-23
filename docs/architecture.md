# Architecture

The system has six pieces: a shared types crate, a RISC0 guest circuit,
a host wrapper that drives the prover, a verifier library, an on-chain
LEZ program, and a CLI. A TypeScript SDK wraps the host and verifier
for browser and Node integrators.

```
                       sequencer (Merkle path)
                                 │
                                 ▼
                          attestation-host
   user wallet ──────► (witness + public inputs)
                          │
                          │   receipt + journal
            ┌─────────────┴─────────────┐
            │                           │
            ▼                           ▼
   LEZ verifier program        attestation-verifier (off-chain)
   (governance, allowlists)    (chat-gate, fee tiers, ...)
            │                           │
            └────────── shared ─────────┘
              verify_attestation()
```

Both verification paths call the same `verify_attestation` function in
`attestation-verifier`. The on-chain program is a thin wrapper that
sources the trusted roots from anchored on-chain state; the off-chain
clients source them from a recent snapshot they trust. Keeping the
verifier in one place is what stops the two paths from drifting.

## Data flow — on-chain

1. CLI runs `attestation-cli prove`. It pulls the Merkle path for the
   user's commitment from the sequencer, assembles the witness, and
   invokes the RISC0 prover.
2. CLI submits the receipt + a ChallengeComponents struct + an ed25519
   signature to the LEZ verifier program.
3. The program rebuilds the challenge from on-chain state and the
   components, verifies the receipt against the pinned image-ID,
   compares the journal fields against gate config, and verifies the
   signature. On success it dispatches the gated action.

## Data flow — off-chain

1. Prover side runs the same `attestation-cli prove`.
2. Sends an `AttestationOffer` over Logos Messaging to the verifier.
3. Verifier issues a fresh challenge.
4. Prover signs and sends a `ChallengeResponse`.
5. Verifier runs `verify_attestation`, admits or denies, sends an
   `AdmissionResult`.

## Why a single verifier function

A pure function that takes (receipt, expectations, challenge,
signature) and returns Result<JournalFields, VerifyError> is what
makes the two paths interchangeable. The on-chain program does not
re-derive trust; it sources `trusted_roots` from the LEZ runtime and
calls in. Anything else would let on-chain and off-chain semantics
diverge over time.
