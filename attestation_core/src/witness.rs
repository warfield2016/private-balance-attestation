// Shared wire format for the RISC0 guest's witness.
//
// Lives in attestation_core so the host and the guest share one
// definition. If the host's struct and the guest's struct ever
// diverge, the prover writes bytes the guest decodes as garbage and
// the proof fails opaquely deep in the zkVM.
//
// Carries `Serialize`/`Deserialize` for risc0_zkvm's `ExecutorEnv::write`
// and `ExecutorEnv::read`. Borsh derives stay for any tooling that
// wants the deterministic borsh layout instead.

use crate::Hash32;
use alloc::vec::Vec;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct GuestWitness {
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
