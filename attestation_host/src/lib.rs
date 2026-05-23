// Host-side proof generation. Builds the zkVM environment, runs the
// prover, returns the serialized receipt + decoded journal. The witness
// is consumed by value so callers don't keep secret material alive.

#![deny(unsafe_code)]
#![deny(unused_must_use)]

use std::time::Instant;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use attestation_core::{
    commitment::{compute_commitment, AccountFields},
    journal::JournalFields,
    GuestWitness, Hash32,
};

// Pull ATTESTATION_GUEST_ELF + ATTESTATION_GUEST_ID from the methods
// crate. Re-export the id so callers (CLI, off-chain verifier wrappers)
// can pass it into `verify_attestation` without each binary needing its
// own `methods` dep.
pub use attestation_methods::{ATTESTATION_GUEST_ELF, ATTESTATION_GUEST_ID};

// Private witness. Drop the value as soon as proving completes; it
// holds secret balance and nonce material.
#[derive(Debug, Clone)]
pub struct Witness {
    pub npk: Hash32,
    pub program_owner: Hash32,
    pub balance: u64,
    pub nonce: Hash32,
    pub data: Vec<u8>,
    pub merkle_siblings: Vec<Hash32>,
    pub merkle_indices: Vec<bool>,
    pub spending_pk: Hash32,
    pub spending_pk_offset: u64,
}

impl Witness {
    /// Sanity-check the witness shape and the embedded `spending_pk` offset
    /// before sending it to the prover. Lets the host return a typed error
    /// instead of having the guest panic deep in the zkVM.
    pub fn validate(&self) -> Result<(), ProveError> {
        if self.merkle_siblings.len() != self.merkle_indices.len() {
            return Err(ProveError::WitnessShape(
                "merkle_siblings and merkle_indices have different lengths",
            ));
        }
        if self.merkle_siblings.is_empty() {
            return Err(ProveError::WitnessShape("merkle_siblings is empty"));
        }
        let end = (self.spending_pk_offset as usize)
            .checked_add(32)
            .ok_or(ProveError::WitnessShape("spending_pk_offset overflow"))?;
        if end > self.data.len() {
            return Err(ProveError::WitnessShape(
                "spending_pk_offset out of range of data",
            ));
        }
        let mut slice = [0u8; 32];
        slice.copy_from_slice(
            &self.data[self.spending_pk_offset as usize..(self.spending_pk_offset as usize + 32)],
        );
        if slice != self.spending_pk {
            return Err(ProveError::WitnessShape(
                "spending_pk does not match the bytes embedded in data",
            ));
        }
        Ok(())
    }
}

// Result of prove(). `receipt` is the serialized RISC0 receipt;
// `journal_bytes` is the borsh-encoded JournalFields for convenience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProveOutput {
    pub receipt: Vec<u8>,
    pub journal_bytes: Vec<u8>,
    pub journal: JournalFields,
    /// Wall-clock time spent proving, in milliseconds.
    pub prove_ms: u64,
}

#[derive(Debug, Error)]
pub enum ProveError {
    #[error("witness shape: {0}")]
    WitnessShape(&'static str),

    #[error("commitment mismatch: recomputed commitment did not match the leaf used in the Merkle path. Did you assemble the path against the right account?")]
    CommitmentMismatch,

    #[error("balance below threshold (would produce no valid proof)")]
    BalanceBelowThreshold,

    #[error("zkVM execution failed: {0}")]
    Risc0Failure(String),

    #[error("journal decode failed: {0}")]
    JournalDecode(String),
}

// Run the RISC0 prover. Witness is consumed by value so callers
// can't keep secrets alive across the call.
pub fn prove(witness: Witness, public: JournalFields) -> Result<ProveOutput, ProveError> {
    witness.validate()?;

    // Host-side sanity check: recompute the commitment and verify the leaf
    // matches what the first level of the Merkle path expects.
    let leaf = compute_commitment(&AccountFields {
        npk: &witness.npk,
        program_owner: &witness.program_owner,
        balance: witness.balance,
        nonce: &witness.nonce,
        data: &witness.data,
    });

    if witness.balance < public.threshold {
        // We could let the guest panic, but we save the user a 60-second
        // proving session for an unsatisfiable predicate.
        return Err(ProveError::BalanceBelowThreshold);
    }

    // Verify the path host-side too. The guest checks this anyway, but if
    // the host catches it we surface a friendlier error.
    {
        use attestation_core::merkle::{verify_merkle_path, MerklePath};
        let path = MerklePath {
            siblings: witness.merkle_siblings.clone(),
            indices: witness.merkle_indices.clone(),
        };
        verify_merkle_path(&leaf, &path, &public.merkle_root)
            .map_err(|_| ProveError::CommitmentMismatch)?;
    }

    // Build the guest's witness from the host's. Both refer to the same
    // type now (re-exported from attestation_core) so the wire format
    // can't drift between prover and circuit.
    let guest_witness = GuestWitness {
        npk: witness.npk,
        program_owner: witness.program_owner,
        balance: witness.balance,
        nonce: witness.nonce,
        data: witness.data,
        merkle_siblings: witness.merkle_siblings,
        merkle_indices: witness.merkle_indices,
        spending_pk: witness.spending_pk,
        spending_pk_offset: witness.spending_pk_offset,
    };

    let env = ExecutorEnv::builder()
        .write(&guest_witness)
        .map_err(|e| ProveError::Risc0Failure(e.to_string()))?
        .write(&public)
        .map_err(|e| ProveError::Risc0Failure(e.to_string()))?
        .build()
        .map_err(|e| ProveError::Risc0Failure(e.to_string()))?;

    let prover = default_prover();
    let start = Instant::now();
    let prove_info = prover
        .prove(env, ATTESTATION_GUEST_ELF)
        .map_err(|e| ProveError::Risc0Failure(e.to_string()))?;
    let prove_ms = start.elapsed().as_millis() as u64;

    let receipt: Receipt = prove_info.receipt;
    let receipt_bytes =
        bincode::serialize(&receipt).map_err(|e| ProveError::Risc0Failure(e.to_string()))?;

    let journal: JournalFields = receipt
        .journal
        .decode()
        .map_err(|e| ProveError::JournalDecode(e.to_string()))?;
    let journal_bytes =
        borsh::to_vec(&journal).map_err(|e| ProveError::JournalDecode(e.to_string()))?;

    Ok(ProveOutput {
        receipt: receipt_bytes,
        journal_bytes,
        journal,
        prove_ms,
    })
}

// (The host previously redeclared a `GuestWitness` mirror here so it
// didn't pull in the guest crate. The single definition now lives in
// `attestation_core::witness` and is shared via the workspace.)

// Fresh ed25519 keypair for a new presenter identity. The caller
// persists the secret key and embeds the public half in the
// account's `data` (or mints a fresh account with it embedded).
pub fn generate_presenter_keypair() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

// 32 bytes of system randomness, suitable as a verifier challenge.
pub fn fresh_challenge() -> Hash32 {
    use rand_core::RngCore;
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    buf
}

// Sign a verifier-issued challenge with the spending secret key.
pub fn sign_challenge(sk: &SigningKey, challenge: &Hash32) -> [u8; 64] {
    let sig: Signature = sk.sign(challenge);
    sig.to_bytes()
}

// Raw 32-byte ed25519 verifying key for a given signing key.
pub fn verifying_key_bytes(sk: &SigningKey) -> Hash32 {
    let vk: VerifyingKey = sk.verifying_key();
    vk.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn witness_validate_catches_shape_errors() {
        let mut w = Witness {
            npk: [0u8; 32],
            program_owner: [0u8; 32],
            balance: 100,
            nonce: [0u8; 32],
            data: vec![0u8; 64],
            merkle_siblings: vec![[0u8; 32]; 3],
            merkle_indices: vec![true, false], // mismatched length
            spending_pk: [0u8; 32],
            spending_pk_offset: 0,
        };
        assert!(matches!(w.validate(), Err(ProveError::WitnessShape(_))));

        w.merkle_indices.push(true);

        // spending_pk_offset out of range
        w.spending_pk_offset = 100;
        assert!(matches!(w.validate(), Err(ProveError::WitnessShape(_))));

        // good shape, but spending_pk not embedded
        w.spending_pk_offset = 0;
        w.spending_pk = [9u8; 32];
        assert!(matches!(w.validate(), Err(ProveError::WitnessShape(_))));

        // embed it correctly
        w.spending_pk = [0u8; 32];
        assert!(w.validate().is_ok());
    }

    #[test]
    fn challenge_is_random() {
        let a = fresh_challenge();
        let b = fresh_challenge();
        assert_ne!(a, b);
    }

    #[test]
    fn sign_and_verify_round_trip() {
        use ed25519_dalek::{Verifier, VerifyingKey};
        let sk = generate_presenter_keypair();
        let challenge = fresh_challenge();
        let sig_bytes = sign_challenge(&sk, &challenge);
        let sig = Signature::from_bytes(&sig_bytes);
        let vk = VerifyingKey::from_bytes(&verifying_key_bytes(&sk)).unwrap();
        assert!(vk.verify(&challenge, &sig).is_ok());
    }
}
