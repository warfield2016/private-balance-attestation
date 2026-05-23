// LP-0005 RISC0 guest. Proves, for a private LEZ account:
//
//   1. The account's commitment is in a Merkle tree with the public root.
//   2. The account's balance is >= the public threshold N.
//   3. The account's `program_owner` equals the public expected owner.
//   4. The account's `data` field embeds the presenter's ed25519 public
//      key at the witnessed offset (equality-bind for forwarding
//      resistance — see docs/presenter-binding.md, ADR-003).
//   5. The committed `JournalFields::presenter_pk` equals the embedded
//      key (so the off-chain ed25519 signature check binds to the same
//      identity the circuit checked).
//
// Inputs:
//   env::read() #1 -> attestation_core::GuestWitness  (private)
//   env::read() #2 -> attestation_core::JournalFields (public)
//
// Output (committed):
//   env::commit() -> JournalFields built from the public inputs and the
//                    pinned CIRCUIT_VERSION. The witness is never
//                    committed.

#![no_main]

use risc0_zkvm::guest::env;

use attestation_core::{
    commitment::{compute_commitment, AccountFields},
    journal::{JournalFields, CIRCUIT_VERSION},
    merkle::{verify_merkle_path, MerklePath},
    GuestWitness,
};

risc0_zkvm::guest::entry!(main);

fn main() {
    // Private witness.
    let w: GuestWitness = env::read();
    // Public inputs. The verifier sees these in the journal; the prover
    // cannot lie about them because we commit them back unchanged.
    let public: JournalFields = env::read();

    // 1. Recompute the LEZ commitment from the witness exactly the way
    //    the on-chain token program built it. If any byte of the witness
    //    is wrong, this leaf will not appear in the tree.
    let leaf = compute_commitment(&AccountFields {
        npk: &w.npk,
        program_owner: &w.program_owner,
        balance: w.balance,
        nonce: &w.nonce,
        data: &w.data,
    });

    // 2. Merkle membership against the public root. We borrow the
    //    siblings/indices vectors directly; `verify_merkle_path` does
    //    not mutate them.
    let path = MerklePath {
        siblings: w.merkle_siblings.clone(),
        indices: w.merkle_indices.clone(),
    };
    verify_merkle_path(&leaf, &path, &public.merkle_root)
        .expect("merkle membership proof failed: leaf not under public root");

    // 3. Threshold predicate. The bounty asks for >= N, not > N.
    assert!(
        w.balance >= public.threshold,
        "balance below public threshold"
    );

    // 4. Bind the proof to the LEZ token program that owns the account.
    //    Without this, a commitment from a different token program could
    //    satisfy the gate if its root was anchored.
    assert_eq!(
        w.program_owner, public.program_owner,
        "witness program_owner does not match public program_owner"
    );

    // 5. Presenter binding (equality-bind). The account's `data` field
    //    must embed `spending_pk` at the witnessed offset, and that key
    //    must equal `public.presenter_pk`. Forwarding-resistance: only a
    //    party holding the secret half of `presenter_pk` can later sign
    //    the verifier's challenge. See ADR-003 and
    //    docs/presenter-binding.md.
    let off = w.spending_pk_offset as usize;
    assert!(
        off.checked_add(32)
            .map(|e| e <= w.data.len())
            .unwrap_or(false),
        "spending_pk_offset out of range of data"
    );
    let mut embedded = [0u8; 32];
    embedded.copy_from_slice(&w.data[off..off + 32]);
    assert_eq!(
        embedded, w.spending_pk,
        "data does not embed witness.spending_pk at the claimed offset"
    );
    assert_eq!(
        w.spending_pk, public.presenter_pk,
        "witness.spending_pk does not match public.presenter_pk"
    );

    // 6. Commit only the public inputs we promised, with the pinned
    //    circuit version. The verifier checks `circuit_version` is in
    //    its allow-list. The witness is dropped here.
    let journal = JournalFields {
        merkle_root: public.merkle_root,
        threshold: public.threshold,
        context_id: public.context_id,
        presenter_pk: public.presenter_pk,
        program_owner: public.program_owner,
        circuit_version: CIRCUIT_VERSION,
    };
    env::commit(&journal);
}
