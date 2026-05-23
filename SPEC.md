# Spec

## Goal

A reusable private balance attestation primitive for the Logos
Execution Zone. Holders of shielded LEZ token accounts produce a
RISC0 proof of `balance >= N`. The same proof is verified on-chain
(by a LEZ program) or off-chain (over Logos Messaging) without
revealing the balance, the account's nullifier key, the nonce, or the
opaque account payload.

## LEZ commitment format (out of scope to change)

```
C = SHA256( npk || program_owner || balance_le || nonce || SHA256(data) )
```

The circuit recomputes this byte-for-byte. See
`attestation_core::commitment::compute_commitment`.

## Public inputs (committed to journal)

```
JournalFields {
    merkle_root:     [u8; 32],
    threshold:       u64,
    context_id:      [u8; 32],
    presenter_pk:    [u8; 32],
    program_owner:   [u8; 32],
    circuit_version: u32,
}
```

`CIRCUIT_VERSION = 2`.

## Circuit predicate

Given the witness `(npk, program_owner, balance, nonce, data,
merkle_path, merkle_indices, spending_pk, spending_pk_offset)`:

1. `compute_commitment(witness) ∈ merkle_path → merkle_root`
2. `witness.program_owner == public.program_owner`
3. `data[offset .. offset+32] == witness.spending_pk == public.presenter_pk`
4. `balance >= threshold`

Only `JournalFields` is committed.

## On-chain instructions

```
Initialize(admin, gate_seed, program_owner, minimum_threshold,
           initial_circuit_version)
GateAction(receipt, challenge_components, signature, action_tag)
RotateAdmin(new_admin)
AddCircuit(version)
RevokeCircuit(version)
UpdateMinimum(new_threshold)
```

Each gate is a PDA derived from `gate_seed`. The same `attestation_program`
binary serves any number of gates.

## Verification (both paths)

```
verify_attestation(
    receipt_bytes,
    image_id,
    expected_context_id,
    expected_program_owner,
    trusted_roots,
    minimum_threshold,
    allowed_versions,
    challenge,
    signature,
) -> Result<JournalFields, VerifyError>
```

Order:
1. size cap (1 MiB)
2. bincode deserialise receipt
3. cryptographic receipt verify
4. journal field equality checks
5. ed25519 signature over challenge

## Error codes

```
 1  context_id mismatch                  (on/off-chain)
 2  threshold below minimum
 3  merkle_root not in trusted set
 4  circuit version not allowed
 5  receipt invalid
 6  signature invalid
 7  journal decode failed
 8  invalid presenter_pk
 9  program_owner mismatch
10  challenge slot stale                 (on-chain only)
11  presenter key did not sign challenge (on-chain only)
12  challenge reused                     (on-chain only)
13  gate already initialised             (on-chain only)
```

## Pinned dependencies

```
nssa_core       v0.2.0-rc3   (logos-blockchain/logos-execution-zone)
spel-framework  v0.3.0       (logos-co/spel)
risc0-zkvm      =3.0.5       (risc0/risc0)
```
