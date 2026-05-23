// Deterministic challenge rebuild for the on-chain path. The presenter
// signs SHA256( "lp-0005-challenge-v1" || program_id || slot_hash ||
// presenter_pk || action_tag ); the program rebuilds it independently
// so the presenter can't smuggle in an attacker-chosen value.

use attestation_core::{sha256_concat, Hash32};

use crate::state::ChallengeComponents;

pub const CHALLENGE_DOMAIN: &[u8] = b"lp-0005-challenge-v1";

pub fn rebuild_challenge(c: &ChallengeComponents, program_id: &Hash32) -> Hash32 {
    sha256_concat(&[
        CHALLENGE_DOMAIN,
        program_id,
        &c.slot_hash,
        &c.presenter_pk,
        &c.action_tag,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn components() -> ChallengeComponents {
        ChallengeComponents {
            slot_hash: [7u8; 32],
            presenter_pk: [11u8; 32],
            action_tag: *b"vote#42         ",
        }
    }

    #[test]
    fn deterministic() {
        let c = components();
        let pid = [3u8; 32];
        assert_eq!(rebuild_challenge(&c, &pid), rebuild_challenge(&c, &pid));
    }

    #[test]
    fn program_id_matters() {
        let c = components();
        assert_ne!(
            rebuild_challenge(&c, &[3u8; 32]),
            rebuild_challenge(&c, &[4u8; 32])
        );
    }

    #[test]
    fn action_tag_matters() {
        let pid = [3u8; 32];
        let a = components();
        let mut b = components();
        b.action_tag = *b"vote#43         ";
        assert_ne!(rebuild_challenge(&a, &pid), rebuild_challenge(&b, &pid));
    }
}
