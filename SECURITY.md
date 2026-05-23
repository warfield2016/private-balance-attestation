# Security

## Reporting

Open a private security advisory on GitHub:
https://github.com/warfield2016/private-balance-attestation/security/advisories/new

Please include reproduction steps and the affected commit hash.

## Threat model

The primitive aims to provide:

- **Balance secrecy** — the verifier cannot read the exact balance.
- **Account unlinkability** — the verifier cannot map a proof to the
  on-chain LEZ account (the `npk` and the nullifier derivation).
- **Cross-gate isolation** — a proof for gate A cannot be replayed
  against gate B (context binding).
- **Forwarding resistance** — a proof handed to a third party does
  not let them pass verification (presenter binding via ed25519
  challenge signature).

## Out of scope (documented in `docs/presenter-binding.md`)

- Voluntary spending-key disclosure by the prover.
- Real-time co-operative signing (Alice signs Bob's challenge live).
- Linkability across gates when a stable `presenter_pk` is reused.
- Pre-signed challenges if verifiers reuse challenges across sessions.

## Cryptographic dependencies

- RISC0 STARK proving system, `risc0-zkvm` pinned to `=3.0.5`.
- ed25519 via `ed25519-dalek` 2.x — `Signature::from_bytes` is
  infallible on `&[u8; 64]` in 2.x; if anyone downgrades to 1.x the
  verifier code in `attestation_verifier::verify_attestation` must be
  updated to handle the new `Result`.

## Build-time pins

- `nssa_core` and `spel-framework` are pinned to tagged releases, not
  to mutable branches. `cargo audit` should be run before any
  production deploy.
