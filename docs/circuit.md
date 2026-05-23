# Circuit

A RISC0 guest program (`attestation-circuit`) that proves, given a
witness, the following:

- the LEZ commitment recomputed from the witness sits on the supplied
  Merkle path under a public root
- the witness's `program_owner` equals the public `program_owner`
- the witness `spending_pk` is embedded in `data` at the declared
  offset and equals the public `presenter_pk`
- `balance >= threshold`

The journal commits only `JournalFields` — the witness fields stay
private.

## Public inputs (journal)

```
struct JournalFields {
    merkle_root:     [u8; 32],
    threshold:       u64,
    context_id:      [u8; 32],
    presenter_pk:    [u8; 32],
    program_owner:   [u8; 32],
    circuit_version: u32,
}
```

`program_owner` is public because without it the verifier can't tell
which token program the account belongs to. A prover with an account
in token program A can otherwise satisfy a gate that expects accounts
from program B as long as both share a root.

## Witness

```
struct Witness {
    npk:                [u8; 32],
    program_owner:      [u8; 32],
    balance:            u64,
    nonce:              [u8; 32],
    data:               Vec<u8>,
    merkle_siblings:    Vec<[u8; 32]>,
    merkle_indices:     Vec<bool>,
    spending_pk:        [u8; 32],
    spending_pk_offset: u64,
}
```

`spending_pk` is the ed25519 verification key used for presenter
binding. It must be embedded in `data` at `spending_pk_offset` and
must equal `public.presenter_pk`. The verifier later issues a fresh
challenge and the prover signs it with the secret half — see
[presenter-binding.md](presenter-binding.md).

## Logic

```
let leaf = compute_commitment(&witness);
verify_merkle_path(&leaf, &path, &public.merkle_root)?;

assert witness.program_owner == public.program_owner;

let embedded = &witness.data[offset .. offset + 32];
assert embedded == witness.spending_pk;
assert witness.spending_pk == public.presenter_pk;

assert witness.balance >= public.threshold;

commit(public);
```

Without any of these asserts a malicious prover can pass a predicate
they shouldn't satisfy. The asserts are deliberately separate so the
guest's failure message tells you which one failed.

## Circuit versioning

`CIRCUIT_VERSION` lives in `attestation-core::journal`. The verifier
keeps an allow-list and rejects unknown versions. Bumping the version
is breaking — any deployed gate needs to add the new version to its
allow-list before users can submit proofs against it.

Current version: 2. v2 added `program_owner` to the public inputs.
