// Example: vote-counter gated on a verified LP-0005 attestation.
// Real governance programs replace the counter with proposal state;
// the gating pattern is the same.

#![deny(unsafe_code)]

use attestation_core::Hash32;
use borsh::{BorshDeserialize, BorshSerialize};
use attestation_program::{
    dispatch::{handle, DispatchCtx, GateEvent},
    errors::GateError,
    state::{ChallengeComponents, GateState, Instruction as GateInstruction},
};

#[derive(Debug, Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct ProposalState {
    pub yes: u64,
    pub no: u64,
    // Voters already counted. Dedupe is by presenter_pk; a voter who
    // rotates to a fresh presenter_pk can vote again, which is the
    // intended behaviour (presenter binding is an identity check;
    // sybil resistance is a separate concern).
    pub voters: Vec<Hash32>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum ProposalInstruction {
    Vote {
        choice: bool,
        receipt: Vec<u8>,
        challenge: ChallengeComponents,
        signature: [u8; 64],
        action_tag: [u8; 16],
    },
}

pub fn handle_proposal_ix(
    proposal: &mut ProposalState,
    gate: &mut GateState,
    ix: &ProposalInstruction,
    ctx: DispatchCtx<'_>,
    caller_pubkey: Hash32,
) -> Result<(), GateError> {
    let ProposalInstruction::Vote {
        choice,
        receipt,
        challenge,
        signature,
        action_tag,
    } = ix;

    // Dedupe BEFORE touching the gate. A duplicate vote should not
    // consume an action_counter slot; that counter is used downstream
    // as a one-shot challenge salt and bumping it on rejects would
    // corrupt that.
    if proposal.voters.iter().any(|p| p == &challenge.presenter_pk) {
        return Err(GateError::SignatureInvalid);
    }

    let gate_ix = GateInstruction::GateAction {
        receipt: receipt.clone(),
        challenge: challenge.clone(),
        signature: *signature,
        action_tag: *action_tag,
    };

    let event: GateEvent = match handle(gate, &gate_ix, ctx, caller_pubkey)? {
        Some(e) => e,
        None => return Err(GateError::ReceiptInvalid),
    };

    // The dispatcher binds challenge.presenter_pk == journal.presenter_pk;
    // record the journal value to match.
    proposal.voters.push(event.presenter_pk);

    if *choice {
        proposal.yes = proposal.yes.saturating_add(1);
    } else {
        proposal.no = proposal.no.saturating_add(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_gate() -> GateState {
        GateState {
            admin: [1u8; 32],
            gate_seed: [2u8; 32],
            program_owner: [3u8; 32],
            minimum_threshold: 100,
            allowed_circuit_versions: [2, 0, 0, 0, 0, 0, 0, 0],
            action_counter: 0,
            bump: 255,
        }
    }

    #[test]
    fn vote_rejected_when_challenge_stale() {
        let mut p = ProposalState::default();
        let mut g = fresh_gate();
        let ctx = DispatchCtx {
            program_id: [0u8; 32],
            recent_roots: &[],
            recent_slot_hashes: &[],
            circuit_image_id: &[0u32; 8],
            is_uninitialized: false,
        };
        let ix = ProposalInstruction::Vote {
            choice: true,
            receipt: vec![],
            challenge: ChallengeComponents {
                slot_hash: [42u8; 32],
                presenter_pk: [9u8; 32],
                action_tag: *b"prop#42-yes     ",
            },
            signature: [0u8; 64],
            action_tag: *b"prop#42-yes     ",
        };
        let r = handle_proposal_ix(&mut p, &mut g, &ix, ctx, [99u8; 32]);
        assert_eq!(r, Err(GateError::ChallengeStale));
        assert_eq!(p.yes, 0);
        // Counter must not have moved since the gate rejected.
        assert_eq!(g.action_counter, 0);
    }

    #[test]
    fn duplicate_vote_rejected_before_counter_bump() {
        let mut p = ProposalState::default();
        let mut g = fresh_gate();
        p.voters.push([7u8; 32]);
        let ctx = DispatchCtx {
            program_id: [0u8; 32],
            recent_roots: &[],
            recent_slot_hashes: &[[5u8; 32]],
            circuit_image_id: &[0u32; 8],
            is_uninitialized: false,
        };
        let ix = ProposalInstruction::Vote {
            choice: true,
            receipt: vec![],
            challenge: ChallengeComponents {
                slot_hash: [5u8; 32],
                presenter_pk: [7u8; 32], // already voted
                action_tag: *b"prop#42-yes     ",
            },
            signature: [0u8; 64],
            action_tag: *b"prop#42-yes     ",
        };
        let r = handle_proposal_ix(&mut p, &mut g, &ix, ctx, [99u8; 32]);
        assert_eq!(r, Err(GateError::SignatureInvalid));
        assert_eq!(g.action_counter, 0);
    }
}
