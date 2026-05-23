// On-chain account state for the LP-0005 verifier program.
//
// The `Instruction` and `ChallengeComponents` wire types live in
// `attestation_core::instruction` so off-chain callers can use them
// without depending on `attestation_program`. Re-exported here for
// back-compat with existing internal call sites.

use attestation_core::Hash32;
use borsh::{BorshDeserialize, BorshSerialize};

pub use attestation_core::{ChallengeComponents, Instruction};

/// Persistent state held by a single `GateState` PDA per gate.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct GateState {
    pub admin: Hash32,
    pub gate_seed: Hash32,
    pub program_owner: Hash32,
    pub minimum_threshold: u64,
    pub allowed_circuit_versions: [u32; 8],
    pub action_counter: u64,
    pub bump: u8,
}

impl GateState {
    pub fn allowed_versions_slice(&self) -> Vec<u32> {
        self.allowed_circuit_versions
            .iter()
            .copied()
            .filter(|v| *v != 0)
            .collect()
    }

    pub fn add_circuit_version(&mut self, v: u32) -> bool {
        if v == 0 {
            return false;
        }
        for slot in self.allowed_circuit_versions.iter_mut() {
            if *slot == v {
                return true;
            }
            if *slot == 0 {
                *slot = v;
                return true;
            }
        }
        false
    }

    pub fn revoke_circuit_version(&mut self, v: u32) -> bool {
        for slot in self.allowed_circuit_versions.iter_mut() {
            if *slot == v {
                *slot = 0;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> GateState {
        GateState {
            admin: [9u8; 32],
            gate_seed: [1u8; 32],
            program_owner: [2u8; 32],
            minimum_threshold: 100,
            allowed_circuit_versions: [0u32; 8],
            action_counter: 0,
            bump: 0,
        }
    }

    #[test]
    fn version_allowlist_add_and_revoke() {
        let mut s = empty_state();
        assert!(s.add_circuit_version(1));
        assert!(s.add_circuit_version(2));
        assert_eq!(s.allowed_versions_slice(), vec![1, 2]);
        assert!(s.revoke_circuit_version(1));
        assert_eq!(s.allowed_versions_slice(), vec![2]);
    }

    #[test]
    fn version_zero_is_rejected() {
        let mut s = empty_state();
        assert!(!s.add_circuit_version(0));
        assert!(s.allowed_versions_slice().is_empty());
    }
}
