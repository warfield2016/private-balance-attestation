# On-chain verifier program

`lez-verifier-program` is a thin wrapper around
`attestation-verifier::verify_attestation`. It does not re-derive
trust; it sources on-chain state, calls the verifier, and gates an
action.

## Instructions

```
0x00 Initialize     admin, gate_seed, program_owner, minimum_threshold,
                    initial_circuit_version
0x01 GateAction     receipt, challenge_components, signature, action_tag
0x02 RotateAdmin    new_admin
0x03 AddCircuit     version
0x04 RevokeCircuit  version
0x05 UpdateMinimum  new_threshold
```

`ChallengeComponents` is what the program rebuilds the challenge from:

```
struct ChallengeComponents {
    slot_hash:    [u8; 32],   // recent on-chain randomness anchor
    presenter_pk: [u8; 32],
    action_tag:   [u8; 16],   // bounded action discriminator
}
```

The challenge itself is

```
SHA256( "lp-0005-challenge-v1" || program_id || slot_hash || presenter_pk || action_tag )
```

The program rebuilds it independently of the presenter and verifies a
signature over the rebuilt value.

## Initialize protection

`Initialize` is only valid when the gate's `GateState` PDA is fresh.
The LEZ runtime owns the "is this PDA uninitialised?" check; the
dispatcher mirrors it via `ctx.is_uninitialized`. Without the mirror,
a misconfigured runtime could let anyone overwrite the admin.

## Error codes

```
 1  context_id mismatch                  E_CONTEXT_MISMATCH
 2  threshold below minimum              E_THRESHOLD_TOO_LOW
 3  merkle_root not in recent anchored   E_ROOT_STALE
 4  circuit version rejected             E_CIRCUIT_VERSION_REJECTED
 5  RISC0 receipt invalid                E_RECEIPT_INVALID
 6  ed25519 signature invalid            E_SIGNATURE_INVALID
 7  challenge slot_hash too old          E_CHALLENGE_STALE
 8  admin-only instruction by non-admin  E_ADMIN_ONLY
 9  challenge reused (one-shot gates)    E_CHALLENGE_REUSED
10  journal decode failed                E_JOURNAL_DECODE
11  presenter_pk not valid ed25519       E_INVALID_PRESENTER_KEY
12  program_owner mismatch               E_PROGRAM_OWNER_MISMATCH
13  gate already initialised             E_ALREADY_INITIALIZED
```

Values are stable across versions; integrator code can branch on them.

## Account layout

```
GateState {
    admin:                    [u8; 32],
    gate_seed:                [u8; 32],
    program_owner:            [u8; 32],
    minimum_threshold:        u64,
    allowed_circuit_versions: [u32; 8],
    action_counter:           u64,
    bump:                     u8,
}
```

`action_counter` increments on every accepted GateAction; it can be
used downstream as a one-shot challenge salt. Dispatchers should
dedupe before invoking the gate so duplicates don't burn counter
slots.

## Compute budget

A RISC0 STARK receipt verifier dominates the per-tx CU cost. A
Groth16 wrap brings it down to roughly a single CPI's worth of CU,
which is the path we recommend for production gates. The off-chain
verifier does not care about CU.

Real numbers go in `docs/benchmarks.md` after the first devnet
deploy.
