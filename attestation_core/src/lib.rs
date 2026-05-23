// Shared types for LP-0005. no_std so the RISC0 guest can import it.
// The on-chain program and the off-chain verifier depend on this too,
// so there is exactly one definition of: the commitment format, the
// Merkle hashing rule, the journal layout, and the context-id
// helpers. If any of these diverge between prover and verifier, proofs
// verify successfully but mean nothing.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod commitment;
pub mod context;
pub mod instruction;
pub mod journal;
pub mod merkle;
pub mod pda;
pub mod witness;

pub use commitment::{compute_commitment, AccountFields};
pub use context::{
    context_id_for_chat, context_id_for_fee_tier, context_id_for_program, context_id_generic,
    DOMAIN_CHAT, DOMAIN_FEE, DOMAIN_GENERIC, DOMAIN_PROGRAM,
};
pub use instruction::{ChallengeComponents, Instruction};
pub use journal::{JournalFields, CIRCUIT_VERSION};
pub use merkle::{verify_merkle_path, MerklePath};
pub use pda::{derive_gate_account_id, GATE_PDA_DOMAIN};
pub use witness::GuestWitness;

pub type Hash32 = [u8; 32];
pub type Signature64 = [u8; 64];

#[inline]
pub fn sha256_concat(parts: &[&[u8]]) -> Hash32 {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    let out = hasher.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    MerklePathShapeMismatch,
    SpendingPkOffsetOutOfRange,
    EmptyMerklePath,
    RootMismatch,
}

#[cfg(feature = "std")]
impl core::fmt::Display for CoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            CoreError::MerklePathShapeMismatch => "merkle path/indices length mismatch",
            CoreError::SpendingPkOffsetOutOfRange => "spending_pk_offset out of range",
            CoreError::EmptyMerklePath => "empty merkle path not supported",
            CoreError::RootMismatch => "reconstructed root != expected root",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CoreError {}
