// Public inputs committed to the RISC0 journal. Everything here is
// visible to anyone holding the receipt; private witness data does not
// belong here.

use crate::Hash32;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

// Bump on any breaking change to the witness format, the circuit logic,
// or the public-input layout. Verifiers keep an allow-list of versions.
pub const CIRCUIT_VERSION: u32 = 2;

// Public inputs. Field order is the wire format; reordering forces a
// version bump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct JournalFields {
    pub merkle_root: Hash32,
    pub threshold: u64,
    pub context_id: Hash32,
    pub presenter_pk: Hash32,
    // The LEZ token program that owns the account. Made public in v2:
    // without it the circuit accepts any account from any token program
    // whose commitment lands on a trusted root.
    pub program_owner: Hash32,
    pub circuit_version: u32,
}

impl JournalFields {
    pub const ENCODED_LEN: usize = 32 + 8 + 32 + 32 + 32 + 4;

    pub fn new(
        merkle_root: Hash32,
        threshold: u64,
        context_id: Hash32,
        presenter_pk: Hash32,
        program_owner: Hash32,
    ) -> Self {
        Self {
            merkle_root,
            threshold,
            context_id,
            presenter_pk,
            program_owner,
            circuit_version: CIRCUIT_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_len_matches_constant() {
        let j = JournalFields::new([0u8; 32], 100, [1u8; 32], [2u8; 32], [3u8; 32]);
        let bytes = borsh::to_vec(&j).unwrap();
        assert_eq!(bytes.len(), JournalFields::ENCODED_LEN);
    }

    #[test]
    fn round_trip_borsh() {
        let j = JournalFields::new([3u8; 32], 9999, [4u8; 32], [5u8; 32], [6u8; 32]);
        let bytes = borsh::to_vec(&j).unwrap();
        let back: JournalFields = borsh::from_slice(&bytes).unwrap();
        assert_eq!(j, back);
    }
}
