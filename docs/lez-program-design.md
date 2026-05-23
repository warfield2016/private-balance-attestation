# On-chain program — design

This is the piece an evaluator usually digs into first: the SPEL
program. The shape might look unusual until the constraints behind
it are visible.

## Two surfaces over one rule-set

The on-chain program in `attestation_program/` has two faces:

1. **The `#[lez_program]` macro mod** in `lib.rs`, gated on
   `target_os = "zkvm"`. This is what `methods/guest` compiles
   into the riscv32 binary the LEZ runtime executes. It exists
   only as a guest binary.
2. **The pure-Rust `dispatch::handle`** in `dispatch.rs`. Same
   logic, no macro. Tested on the host (`cargo test`) without
   needing `rzup`, the riscv toolchain, or a sequencer.

Each macro `#[instruction]` handler is a four-line shim that
unpacks accounts, calls a function in `handlers/`, and wraps the
result in `SpelOutput::execute(...)`. The actual rule-set lives in
`dispatch::handle` and is shared between the two surfaces. The
security argument has one place to live.

This dual-surface pattern is borrowed from `lez-multisig`'s
approach: the macro mod is thin; per-instruction work lives in
plain Rust modules that can be unit-tested in isolation.

## Why `attestation_program` does not link `attestation_verifier`

The off-chain `attestation_verifier` crate calls
`risc0_zkvm::Receipt::verify(image_id)` to do the cryptographic
receipt check. That call pulls a lot:

```
attestation_verifier
└── risc0-zkvm (with default features)
    └── bonsai-sdk
        └── reqwest
            └── hyper-rustls
                └── rustls
                    └── ring   ← cross-compiles via cc with -m64
```

Cargo's feature unification means that even when our direct
dependency declares `default-features = false`, an upstream crate
(`nssa_core` in this case) requesting full features wins. The
unified graph pulls `ring`, and `ring`'s build script then passes
`-m64` to the riscv32 cross-compiler — which doesn't recognise it.
The guest build fails before any of our code runs.

The fix is structural: the on-chain program does not need to
verify the receipt cryptographically. The LEZ runtime knows the
program's image-id and refuses to invoke the program if the
receipt is invalid at transaction-acceptance time. The program
only needs to:

1. Decode the journal from the receipt bytes (cheap, no crypto)
2. Apply policy field-equality checks
3. Verify the ed25519 signature over the deterministically rebuilt
   challenge (pure-Rust via `ed25519-dalek`)

`attestation_program/Cargo.toml` therefore avoids `attestation_verifier`
entirely and pulls only `attestation_core`, `nssa_core`,
`spel-framework`, `borsh`, `ed25519-dalek`, `thiserror`. The
result is a guest build that does not see `ring`.

`lez-multisig` and `whisper-wall` ship the same shape: the on-chain
crate depends on the core types crate + framework + nssa_core +
borsh + serde + risc0-zkvm — no off-chain verifier crate. We
match.

## Instruction enum lives in `attestation_core`

The macro attribute is
`#[lez_program(instruction = "attestation_core::Instruction")]` —
the wire type is in the shared `_core` crate, not in
`attestation_program`. Two consequences:

1. Off-chain callers (CLI, TypeScript SDK, future CPI clients) can
   construct and serialise instructions without depending on the
   on-chain program crate (which would force them to compile
   `spel-framework` and `nssa_core`).
2. An evaluator reading the macro attribute sees a path into a
   first-class shared crate, not into a `crate::` internal
   module. This is the same shape `lez-multisig` uses with
   `multisig_core::Instruction`.

## PDA derivation, shared

`attestation_core::pda::derive_gate_account_id(program_id, gate_seed)`
is the off-chain equivalent of the macro's
`#[account(init, pda = arg("gate_seed"))]`. Both produce the same
32-byte account id. Pinned by a domain-separation test that
verifies the gate PDA can never collide with any `context_id_*`
value for the same `(program_id, gate_seed)`.

## Error model

All `GateError` variants carry a numeric code (1–13). On-chain
they surface through `SpelError::custom(code, message)`, so the
on-wire response carries the numeric identity that the CLI and
SDK can branch on. Off-chain `VerifyError` (1–9) uses the
overlapping range; on-chain-only codes are 10–13.

A `codes_match_documentation_table` test pins the numeric values
against the docs. Reordering the enum produces a compile-time
failure in the test, not a silent change of the wire format.

## What the LEZ runtime does that we don't

The LEZ runtime (the part of the sequencer that accepts and
applies transactions) is responsible for:

- Verifying the RISC0 receipt cryptographically against the
  program's pinned image-id at transaction acceptance.
- Loading PDAs from the persistent storage indexed by the
  derivation rule.
- Enforcing PDA initialise-once semantics (only one Initialize
  per `gate_seed`).
- Providing the recent-roots and recent-slot-hashes windows the
  program reads.
- Routing chained calls if a handler returns a `ChainedCall`.

The program assumes all of these. The unit tests use a stub
`DispatchCtx` that simulates the runtime's inputs; the live e2e
tests in `e2e_tests/` exercise the real runtime contract once a
sequencer is up.

## Where the macro touches what

```
attestation_program/src/lib.rs        → #[lez_program] mod attestation_gate
                                          ├── pub fn initialize    → handlers/initialize::handle
                                          ├── pub fn gate_action   → handlers/gate_action::handle
                                          ├── pub fn rotate_admin  → handlers/admin::rotate_admin
                                          ├── pub fn add_circuit   → handlers/admin::add_circuit
                                          ├── pub fn revoke_circuit→ handlers/admin::revoke_circuit
                                          └── pub fn update_minimum→ handlers/admin::update_minimum
attestation_program/src/dispatch.rs   → pure-Rust handle() called by gate_action handler
attestation_program/src/handlers/     → per-instruction logic (host-testable)
attestation_program/src/state.rs      → GateState, re-exports Instruction + ChallengeComponents from core
attestation_program/src/challenge.rs  → rebuild_challenge() shared host/guest
attestation_program/src/errors.rs     → GateError codes 1–13, pinned by test
methods/guest/src/bin/attestation.rs  → one-liner: entry!(attestation_program::main)
methods/guest/src/bin/generate_idl.rs → one-liner: generate_idl!("../../attestation_program/src/lib.rs")
```

## CI does not build the guest

`.github/workflows/ci.yml` runs three jobs: Rust fmt + clippy +
test, TypeScript build + typecheck, and IDL generation. None of
them compiles the riscv32 guest. This matches `lez-multisig`'s CI
exactly — they set `RISC0_SKIP_BUILD=1` and run host-only checks
plus IDL generation. The guest is compiled locally with the full
RISC0 toolchain via `make build`. The macro surface is validated
in CI by the IDL generator, which macro-expands the program
source even though no riscv binary is produced.

## What to read after this

- [`ADR.md`](../ADR.md) — every structural decision with its
  alternatives and rejection reason
- [`SPEC.md`](../SPEC.md) — the formal interface
- [`FURPS.md`](../FURPS.md) — functional + non-functional
  requirements
- [presenter-binding.md](presenter-binding.md) — the forwarding-
  resistance scheme + its limits
