# Integrations

Three reference apps live under `examples/`. Each shows a different
way to call into the verifier.

## Governance gate (on-chain)

`examples/governance-gate/`. A vote-counter program that gates each
vote on a verified attestation. Demonstrates the on-chain CPI pattern:
a parent program calls into `lez-verifier-program::handle()` and only
proceeds if it returns Ok.

Context-id derivation: `context_id_for_program(program_id, gate_seed)`
where `gate_seed` is whatever per-proposal bytes the program picks.

## Chat-gate (off-chain)

`examples/chat-gate/`. A demo of the four-message Logos Messaging flow
running over an in-process transport (`InProcessTransport`). Real
Logos Messaging deployments swap that one type for a Logos client; the
rest of the flow is unchanged.

Context-id derivation: `context_id_for_chat(group_pk, epoch)`. Bump
`epoch` to invalidate older proofs.

## Fee-tier (slot for an external integrator)

`examples/fee-tier-gate/`. A scaffold for the third integration. Small
enough to extend in an evening. The bounty requires at least one
integration by a party outside the submitting team — this is where it
lands.

The scaffold verifies an attestation against each tier's context-id
(highest first) and awards the first one that passes. Each tier
declares its own context-id, so the prover commits to a specific tier
at proof-gen time and over-awarding is not possible.

## Patterns

Every integration ends up calling

```rust
let journal = verify_attestation(VerifyArgs { .. })?;
```

with `expected_context_id` and `expected_program_owner` set to
whatever the gate is configured for. The verifier returns the journal
on success; the integration logs `journal.presenter_pk` if it needs to
deduplicate or rate-limit, then runs whatever its gated action is.
