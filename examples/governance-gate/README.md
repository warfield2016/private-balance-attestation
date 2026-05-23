# Example: on-chain governance gate

A LEZ program that lets a user cast a vote on a proposal only if they
hold at least N tokens of a configured kind. The token-balance check
runs as a verified LP-0005 attestation.

## Files

```
program/src/lib.rs      vote-counter program; CPI-calls the verifier
scripts/cast-vote.sh    deploy + prove + submit + observe
```

## Run

```
# Local sequencer + program deployed (see ../../scripts/setup-devnet.sh
# and the deploy step in cast-vote.sh).

./scripts/cast-vote.sh --voter alice --proposal 42 --vote yes
```

Alice (above threshold) admitted; Bob (below threshold) rejected with
error code 2 (`E_THRESHOLD_TOO_LOW`).

## Integration pattern

In your own LEZ program, build a CPI context and call

```rust
let event = lez_verifier_program::handle(
    &mut gate_state, &gate_instruction, ctx, caller_pubkey,
)?;
```

On `Ok(Some(event))` the gate is open; you have a verified
`presenter_pk` to deduplicate against. Any error variant maps to a
documented error code.

## Limitations

Single circuit version, single threshold. Production gates likely
want per-proposal thresholds and admin rotation — both are first-class
instructions on the verifier program (`UpdateMinimum`, `RotateAdmin`).
