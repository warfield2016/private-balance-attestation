// PDA derivation helpers for the LP-0005 gate.
//
// The `#[lez_program]` macro derives PDAs on-chain via
// `#[account(init, pda = arg("gate_seed"))]`. Off-chain callers
// (CLI, TypeScript SDK, CPI clients) need the same value to address
// the gate's account before submitting any instruction. This
// module is the single source of truth — the on-chain and off-chain
// derivations must agree byte-for-byte.

use crate::{sha256_concat, Hash32};

/// Domain-separation prefix for the gate PDA. Different prefix from
/// every `context_id_*` helper so a (program_id, gate_seed) pair
/// can never collide with a context-id from any other domain.
pub const GATE_PDA_DOMAIN: &[u8] = b"lp-0005:gate-pda:";

/// Derive the 32-byte account id for a gate's `GateState` PDA.
///
/// `program_id` is the on-chain id of the LEZ program; `gate_seed`
/// is the per-gate discriminator the admin chose at `Initialize`.
/// Two gates with the same program_id but different gate_seeds get
/// distinct account ids.
pub fn derive_gate_account_id(program_id: &Hash32, gate_seed: &Hash32) -> Hash32 {
    sha256_concat(&[GATE_PDA_DOMAIN, program_id, gate_seed])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let pid = [1u8; 32];
        let seed = [2u8; 32];
        assert_eq!(
            derive_gate_account_id(&pid, &seed),
            derive_gate_account_id(&pid, &seed)
        );
    }

    #[test]
    fn distinct_seeds_produce_distinct_ids() {
        let pid = [1u8; 32];
        assert_ne!(
            derive_gate_account_id(&pid, &[2u8; 32]),
            derive_gate_account_id(&pid, &[3u8; 32])
        );
    }

    #[test]
    fn distinct_programs_produce_distinct_ids() {
        let seed = [2u8; 32];
        assert_ne!(
            derive_gate_account_id(&[1u8; 32], &seed),
            derive_gate_account_id(&[4u8; 32], &seed)
        );
    }

    #[test]
    fn domain_separation_from_context_id() {
        // A gate PDA must never collide with a context_id for the
        // same (program_id, gate_seed) pair. The trailing-colon
        // domain on both prevents this by construction; pin it.
        use crate::context::context_id_for_program;
        let pid = [9u8; 32];
        let seed = [7u8; 32];
        assert_ne!(
            derive_gate_account_id(&pid, &seed),
            context_id_for_program(&pid, &seed)
        );
    }
}
