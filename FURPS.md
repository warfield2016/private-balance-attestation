# FURPS+

## Functionality

1. Generate a client-side RISC0 proof that a shielded LEZ account's
   balance is ≥ a public threshold N.
2. Bind every proof to a specific gate context-id and to the presenter's
   ed25519 verification key.
3. Same proof verifies on-chain (via `attestation_program`, the
   `#[lez_program]`-annotated module) and off-chain (via
   `attestation_verifier`, called directly from a Rust client or via
   the TypeScript SDK).
4. The verifier accepts proofs only for an allow-listed set of circuit
   versions; rejects mismatched `program_owner`, `context_id`,
   `merkle_root`, `threshold`, or signature.
5. The on-chain program supports six instructions: `Initialize`,
   `GateAction`, `RotateAdmin`, `AddCircuit`, `RevokeCircuit`,
   `UpdateMinimum`. Discriminants are stable.

## Usability

1. One `make demo` from a clean clone runs the full flow (build, IDL,
   deploy, prove, submit, off-chain verify) provided the LEZ sequencer
   is up via `make setup`.
2. The CLI accepts hex args with or without a `0x` prefix.
3. `attestation-cli prove` prints prove time and receipt size; the
   `--json` shape is stable and pipe-able to `jq`.
4. `docs/` has one file per topic — `architecture.md`, `circuit.md`,
   `commitment-format.md`, `presenter-binding.md`, `onchain-program.md`,
   `integrations.md`, `benchmarks.md` — readable in any order.

## Reliability

1. The host validates the witness shape before invoking the prover so
   the user gets a typed error rather than waiting a minute for the
   guest to panic.
2. Verification errors map onto a documented numeric code set; the
   on-chain and off-chain codes overlap for the shared range.
3. The off-chain verifier never echoes any witness or journal field
   on rejection; only the numeric code surfaces.
4. The Logos Messaging transport in the SDK is behind a trait; tests
   run against an in-process pair and never touch the network.

## Performance

1. Receipt verification: target one CPI-equivalent of CU on-chain
   (Groth16 wrap path; STARK direct is documented as fallback).
2. Proof generation: ~60s on a single CPU with `RISC0_DEV_MODE=0`,
   ~6s on a CUDA GPU.
3. Receipt size: ~200 KB STARK / ~1 KB Groth16-wrapped.

Real numbers land in `docs/benchmarks.md` once `make demo` runs
against a deployed program on LEZ testnet.

## Supportability

1. Public repo, MIT or Apache-2.0.
2. CI on `main` runs Rust fmt + clippy + tests and TypeScript build +
   typecheck on every push. E2E job builds the guest under
   `RISC0_DEV_MODE=0`.
3. `Makefile` and `scaffold.toml` follow the lez-multisig and
   whisper-wall conventions; any Logos dev with `logos-scaffold` on
   PATH can clone and run.
4. Pins are explicit: `nssa_core` and `spel-framework` reference
   `v0.2.0-rc3` and `v0.3.0` tags respectively; `risc0-zkvm` is
   `=3.0.5` for receipt determinism.
5. ADRs in `ADR.md` record structural decisions; this file (FURPS.md)
   records requirements.
