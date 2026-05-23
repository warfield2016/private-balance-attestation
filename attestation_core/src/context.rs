// Context-id helpers. Each verifier declares a 32-byte gate id; the
// prover commits it to the journal; the verifier rejects proofs whose
// context_id doesn't match.
//
// Each domain string ends with a colon so no prefix relationship can
// hold between them. Earlier drafts had "lp-0005:" as the generic
// prefix and "lp-0005:chat" as a typed one, which let
// context_id_generic("chat", group_pk || epoch) collide with
// context_id_for_chat(group_pk, epoch). The trailing colon kills that.

use crate::{sha256_concat, Hash32};

pub const DOMAIN_PROGRAM: &[u8] = b"lp-0005:onchain:";
pub const DOMAIN_CHAT: &[u8] = b"lp-0005:chat:";
pub const DOMAIN_FEE: &[u8] = b"lp-0005:fee:";
pub const DOMAIN_GENERIC: &[u8] = b"lp-0005:generic:";

pub fn context_id_for_program(program_pubkey: &Hash32, gate_seed: &Hash32) -> Hash32 {
    sha256_concat(&[DOMAIN_PROGRAM, program_pubkey, gate_seed])
}

pub fn context_id_for_chat(group_pubkey: &Hash32, epoch: u64) -> Hash32 {
    sha256_concat(&[DOMAIN_CHAT, group_pubkey, &epoch.to_le_bytes()])
}

pub fn context_id_for_fee_tier(tier: u32, group_pubkey: &Hash32) -> Hash32 {
    // Little-endian for consistency with the rest of the crate.
    sha256_concat(&[DOMAIN_FEE, &tier.to_le_bytes(), group_pubkey])
}

pub fn context_id_generic(integration_id: &str, extra: &[u8]) -> Hash32 {
    sha256_concat(&[DOMAIN_GENERIC, integration_id.as_bytes(), extra])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_domains_produce_distinct_ids() {
        let program = [1u8; 32];
        let seed = [2u8; 32];
        let group = [3u8; 32];

        let a = context_id_for_program(&program, &seed);
        let b = context_id_for_chat(&group, 1);
        let c = context_id_for_fee_tier(1, &group);
        let d = context_id_generic("acme-allowlist-v1", b"");

        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(b, c);
        assert_ne!(b, d);
        assert_ne!(c, d);
    }

    #[test]
    fn epoch_rotation_changes_id() {
        let group = [3u8; 32];
        assert_ne!(
            context_id_for_chat(&group, 1),
            context_id_for_chat(&group, 2)
        );
    }

    #[test]
    fn generic_helper_is_stable() {
        let a = context_id_generic("acme", b"v1");
        let b = context_id_generic("acme", b"v1");
        assert_eq!(a, b);
    }

    // Earlier draft had DOMAIN_GENERIC = b"lp-0005:" which collides with
    // any other domain string. The trailing colon makes prefix collisions
    // impossible; this test pins the property.
    #[test]
    fn generic_cannot_collide_with_typed_domain() {
        let group = [3u8; 32];
        let chat = context_id_for_chat(&group, 1);
        // Reconstruct: typed call hashes b"lp-0005:chat:" || group || epoch_le.
        // Generic hashes b"lp-0005:generic:" || id || extra. The byte
        // streams must differ even if the caller picks adversarial inputs.
        let mut adversarial = Vec::new();
        adversarial.extend_from_slice(&group);
        adversarial.extend_from_slice(&1u64.to_le_bytes());
        let g = context_id_generic("chat:", &adversarial);
        assert_ne!(chat, g);
    }
}
