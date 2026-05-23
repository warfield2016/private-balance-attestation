# Architecture Decision Records

A running log of structural decisions and the alternatives that were
rejected. New ADRs go to the bottom; entries are append-only — if a
decision is reversed, write a new ADR.

## ADR-001 — SPEL framework for the on-chain program

The on-chain verifier is annotated with `#[lez_program]` from
`spel-framework` (tag v0.3.0 against `logos-co/spel`). The macro emits
the LEZ entrypoint, dispatch, account loading, and the IDL JSON.

Considered alternatives:
- **Naked Rust + manual dispatch.** An earlier draft built a
  hand-rolled `Instruction` enum, a `handle()` function, and a stub
  for the runtime entrypoint behind a feature flag. Rejected because
  the program could not actually deploy on LEZ — there was no
  `entrypoint!` symbol and account loading was hand-rolled.
- **Direct `nssa_core` without the framework.** Would compile and
  deploy but reproduces the same boilerplate the macro generates.
  Rejected on maintainability.

The pure-Rust `dispatch::handle` function still exists for off-chain
testing; the macro mod is a thin shell around it.

## ADR-002 — `program_owner` as a public input (CIRCUIT_VERSION = 2)

The LEZ private-account commitment is
`SHA256(npk || program_owner || balance_le || nonce || SHA256(data))`.
The first version of the circuit kept `program_owner` private. An
audit pass noticed that without anchoring `program_owner` to a public
value, a prover with an account in token program A could satisfy a
gate that expects accounts from token program B as long as both share
a root. v2 promotes `program_owner` to the public journal and asserts
`witness.program_owner == public.program_owner` inside the circuit.

## ADR-003 — Equality-bind presenter scheme (out-of-circuit signature)

The forwarding-resistance scheme embeds an ed25519 verification key
(`spending_pk`) at a known offset in the account's `data` blob. The
circuit asserts `data[offset..offset+32] == witness.spending_pk` and
`witness.spending_pk == public.presenter_pk`. Verifiers then issue a
fresh challenge that the presenter signs out-of-circuit.

Considered: signature-of-knowledge (SoK) — the ed25519 verify inside
the guest. Rejected for now because it adds ~100k cycles per proof and
forces the verifier to know the challenge ahead of proof generation
(no fresh challenge per session). Documented in
`docs/presenter-binding.md` as future work if reviewers want it.

## ADR-004 — Trailing-colon domain separators

The four `context_id` domain strings — `lp-0005:onchain:`,
`lp-0005:chat:`, `lp-0005:fee:`, `lp-0005:generic:` — all end with a
colon so no string is a prefix of another. An earlier draft used
`lp-0005:` as the generic prefix, which let
`context_id_generic("chat", group || epoch)` collide with
`context_id_for_chat(group, epoch)`. The trailing colon kills the
prefix relationship; a test pins the property.

## ADR-005 — Single `verify_attestation` function

Both verifiers (on-chain LEZ program, off-chain client) call the same
`attestation_verifier::verify_attestation`. The on-chain program is a
thin shell; the off-chain verifier calls it directly. This is the
mechanism that prevents the two paths from drifting over time.

## ADR-006 — Bincode size cap before deserialisation

`verify_attestation` enforces `args.receipt_bytes.len() <= 1 MiB`
before bincode touches the bytes. Without this, a malformed receipt
claiming a Vec with 2³² elements could allocate gigabytes before
failing.

## ADR-007 — `program_owner` and challenge order in verify

Inside `verify_attestation` the order is:
1. size cap
2. bincode deserialize
3. `receipt.verify(image_id)` (cryptographic proof check)
4. journal field equality checks (`circuit_version`, `context_id`,
   `program_owner`, `threshold`, `merkle_root`)
5. ed25519 signature over the challenge

The receipt verification runs **before** any journal field is read.
An earlier draft read journal fields first and used them for
fast-path policy checks; this leaked a policy oracle to attackers
sending forged receipts.

## ADR-009 — CI skips the riscv32 guest build

CI runs Rust fmt/clippy/test on host and TypeScript build on host,
then generates the SPEL IDL via the `attestation-idl-gen` binary.
It does NOT compile the `methods/guest` riscv32 target. Two reasons:

1. **`risc0-zkvm`'s default features pull `bonsai-sdk` → `reqwest`
   → `hyper-rustls` → `ring`.** Ring's build script tries to compile
   C/asm with `-m64`, which the riscv cross-compiler rejects.
   Cargo's feature unification means even disabling default-features
   on our direct dep doesn't help — `nssa_core` (upstream) pulls
   risc0-zkvm with full features.
2. **`lez-multisig` (the canonical reference LEZ project) does the
   same.** Their CI sets `RISC0_SKIP_BUILD=1` and pulls a pre-built
   circuits cache for any test that needs the guest binary. The
   guest is compiled locally by developers with the full
   `rzup install`-managed toolchain via `make build`.

The IDL generator binary runs on host and parses the program source
via `spel_framework::generate_idl!`, so the macro surface IS
validated by CI even though the guest isn't compiled there.

## ADR-008 — On-chain program does not link `attestation_verifier`

The on-chain `attestation_program` does not depend on the
`attestation_verifier` crate. The dispatcher decodes the journal
directly from the receipt bytes, runs the policy-field equality
checks, and verifies the presenter's ed25519 signature inline. The
RISC0 receipt's cryptographic verification is done by the LEZ
runtime at transaction acceptance — the runtime knows the program's
image-id and refuses to invoke the program if the receipt is
invalid.

Two reasons:

1. **Build cleanliness.** `attestation_verifier` depends on
   `risc0-zkvm`'s host machinery, which transitively pulls
   `ring`. Ring's build script fails on the `riscv32im-risc0-zkvm-elf`
   target (it calls the riscv cross-compiler with `-m64`). The
   guest build for `methods/guest` would otherwise fail.
2. **Correctness.** A LEZ program is itself a RISC0 guest. Nested
   receipt verification inside the guest would be a circular call.
   The standard pattern across `lez-multisig` and `whisper-wall` is
   to trust the runtime's receipt-verification syscall and only
   apply program-specific policy in the guest.

The off-chain path (`attestation_verifier::verify_attestation`)
still does the full cryptographic check because there is no runtime
to lean on. Both paths converge on the same `JournalFields` policy
checks — the security argument is in those.
