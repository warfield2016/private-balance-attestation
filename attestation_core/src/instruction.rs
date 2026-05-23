// LEZ verifier-program instruction wire types.
//
// Lives in attestation_core (the shared no_std crate) so off-chain
// callers — CLI, TypeScript SDK, future CPI clients — can construct
// and serialise instructions without depending on the on-chain
// `attestation_program` crate (which pulls spel-framework and
// nssa_core). This matches the convention `lez-multisig` uses with
// its `multisig_core::Instruction`.

use crate::Hash32;
use borsh::{BorshDeserialize, BorshSerialize};

/// On-chain instruction set for the LP-0005 verifier program.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    /// Create or initialise a gate PDA. Caller becomes admin.
    Initialize {
        admin: Hash32,
        gate_seed: Hash32,
        program_owner: Hash32,
        minimum_threshold: u64,
        initial_circuit_version: u32,
    },
    /// Verify a receipt + signature, gate the named action.
    GateAction {
        receipt: alloc::vec::Vec<u8>,
        challenge: ChallengeComponents,
        signature: [u8; 64],
        action_tag: [u8; 16],
    },
    /// Admin: rotate the admin key.
    RotateAdmin { new_admin: Hash32 },
    /// Admin: append a circuit version to the allow-list.
    AddCircuit { version: u32 },
    /// Admin: revoke a circuit version.
    RevokeCircuit { version: u32 },
    /// Admin: bump the minimum threshold.
    UpdateMinimum { new_threshold: u64 },
}

/// Components a presenter signs over. The program rebuilds the
/// challenge deterministically; the presenter cannot smuggle in an
/// arbitrary value.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ChallengeComponents {
    pub slot_hash: Hash32,
    pub presenter_pk: Hash32,
    pub action_tag: [u8; 16],
}
