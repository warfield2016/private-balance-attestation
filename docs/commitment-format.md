# LEZ commitment format

The LEZ token program commits each private account as

```
C = SHA256( npk || program_owner || balance_le || nonce || SHA256(data) )
```

with field types

```
npk             [u8; 32]
program_owner   [u8; 32]
balance         u64 little-endian (8 bytes)
nonce           [u8; 32]
SHA256(data)    [u8; 32]
```

Total preimage: 136 bytes.

The circuit recomputes `C` from the witness exactly this way. If LEZ
ever changes the layout, there is one function to update —
`attestation_core::commitment::compute_commitment`. Every other
consumer routes through it.

## Why this format, as-is

The bounty puts changes to the token program out of scope. SHA256 is
fine for a RISC0 circuit; Poseidon would be faster inside the proof
but is not what LEZ uses. We pay the SHA cost.

## Reading a real commitment

The sequencer exposes `get_proof_for_commitment(C)`. The CLI calls

```
GET <sequencer>/v1/account/by_commitment?c=<hex>
```

and gets

```
{
  "leaf": "...",
  "merkle_root": "...",
  "merkle_siblings": ["...", ...],
  "merkle_indices": [false, true, ...],
  "anchor_slot": 123
}
```

The CLI does not ask the sequencer for the witness — that's the
user's. The sequencer only returns the Merkle path for a commitment
the user declares, which is already a public value.
