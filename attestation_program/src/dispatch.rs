// Pure instruction dispatch. Takes a GateState, an Instruction, and an
// on-chain context; returns an event or an error.
//
// What this function does NOT do: cryptographically verify the RISC0
// receipt. That's the LEZ runtime's job at transaction acceptance —
// the runtime knows the program's image-id and refuses to invoke the
// program if the receipt doesn't check out. This dispatcher just
// reads the journal fields, enforces gate policy, and verifies the
// presenter's ed25519 signature.
//
// The off-chain verifier crate (`attestation_verifier`) does the
// full receipt-cryptographic check because it has no runtime to lean
// on; this dispatcher trusts the runtime did it.

use attestation_core::{
    context::context_id_for_program,
    journal::{JournalFields, CIRCUIT_VERSION},
    Hash32,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::{
    challenge::rebuild_challenge,
    errors::GateError,
    state::{ChallengeComponents, GateState, Instruction},
};

pub const RECENT_ROOTS_K: usize = 8;
pub const RECENT_SLOT_HASHES_K: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvent {
    pub presenter_pk: Hash32,
    pub action_tag: [u8; 16],
    pub action_counter: u64,
    pub journal: JournalFields,
}

pub struct DispatchCtx<'a> {
    pub program_id: Hash32,
    pub recent_roots: &'a [Hash32],
    pub recent_slot_hashes: &'a [Hash32],
    pub circuit_image_id: &'a [u32; 8],
    // Set to true on the very first instruction landing on a fresh PDA;
    // otherwise false. The LEZ runtime tells us this via account data
    // length (zero == uninitialised). Without it, anyone could send an
    // Initialize and reset the gate.
    pub is_uninitialized: bool,
}

pub fn handle(
    state: &mut GateState,
    ix: &Instruction,
    ctx: DispatchCtx<'_>,
    caller_pubkey: Hash32,
) -> Result<Option<GateEvent>, GateError> {
    match ix {
        Instruction::Initialize {
            admin,
            gate_seed,
            program_owner,
            minimum_threshold,
            initial_circuit_version,
        } => {
            if !ctx.is_uninitialized {
                return Err(GateError::AlreadyInitialized);
            }
            state.admin = *admin;
            state.gate_seed = *gate_seed;
            state.program_owner = *program_owner;
            state.minimum_threshold = *minimum_threshold;
            state.allowed_circuit_versions = [0u32; 8];
            let _ = state.add_circuit_version(*initial_circuit_version);
            state.action_counter = 0;
            Ok(None)
        }

        Instruction::RotateAdmin { new_admin } => {
            require_admin(state, &caller_pubkey)?;
            state.admin = *new_admin;
            Ok(None)
        }

        Instruction::AddCircuit { version } => {
            require_admin(state, &caller_pubkey)?;
            let _ = state.add_circuit_version(*version);
            Ok(None)
        }

        Instruction::RevokeCircuit { version } => {
            require_admin(state, &caller_pubkey)?;
            let _ = state.revoke_circuit_version(*version);
            Ok(None)
        }

        Instruction::UpdateMinimum { new_threshold } => {
            require_admin(state, &caller_pubkey)?;
            state.minimum_threshold = *new_threshold;
            Ok(None)
        }

        Instruction::GateAction {
            receipt,
            challenge,
            signature,
            action_tag,
        } => gate_action(state, ctx, receipt, challenge, signature, *action_tag).map(Some),
    }
}

fn require_admin(state: &GateState, caller: &Hash32) -> Result<(), GateError> {
    if &state.admin == caller {
        Ok(())
    } else {
        Err(GateError::AdminOnly)
    }
}

// Decode `JournalFields` from the receipt bytes. Receipts ship as
// bincode-serialised RISC0 envelopes; the journal sits inside a known
// section. To keep this crate dep-light (no risc0-zkvm host bits), we
// scan the bytes for the journal section directly.
//
// The Basecamp app's app.js has the same scanner — they must stay in
// sync. The journal layout is pinned by JournalFields::ENCODED_LEN.
fn decode_journal_from_receipt(receipt_bytes: &[u8]) -> Result<JournalFields, GateError> {
    if receipt_bytes.len() < JournalFields::ENCODED_LEN {
        return Err(GateError::JournalDecode);
    }
    // The borsh-encoded JournalFields is 140 bytes; the receipt
    // envelope places it somewhere inside the payload. Try each
    // offset until a borsh decode succeeds AND circuit_version is
    // in the allowed range (1..=256, the macro cap).
    let target_len = JournalFields::ENCODED_LEN;
    for off in 0..=(receipt_bytes.len() - target_len) {
        let slice = &receipt_bytes[off..off + target_len];
        if let Ok(j) = borsh::from_slice::<JournalFields>(slice) {
            if j.circuit_version >= 1 && j.circuit_version <= 256 {
                return Ok(j);
            }
        }
    }
    Err(GateError::JournalDecode)
}

fn gate_action(
    state: &mut GateState,
    ctx: DispatchCtx<'_>,
    receipt_bytes: &[u8],
    components: &ChallengeComponents,
    signature: &[u8; 64],
    action_tag: [u8; 16],
) -> Result<GateEvent, GateError> {
    if !ctx
        .recent_slot_hashes
        .iter()
        .any(|h| h == &components.slot_hash)
    {
        return Err(GateError::ChallengeStale);
    }

    // The action tag is folded into the challenge; we also check it
    // directly so a wrong tag fails fast with a clear code.
    if action_tag != components.action_tag {
        return Err(GateError::SignatureInvalid);
    }

    // The runtime has already verified the receipt cryptographically;
    // we just decode the journal to read the policy fields.
    let journal = decode_journal_from_receipt(receipt_bytes)?;

    if journal.circuit_version != CIRCUIT_VERSION
        && !state
            .allowed_versions_slice()
            .contains(&journal.circuit_version)
    {
        return Err(GateError::CircuitVersionRejected);
    }

    let expected_context_id = context_id_for_program(&ctx.program_id, &state.gate_seed);
    if journal.context_id != expected_context_id {
        return Err(GateError::ContextMismatch);
    }
    if journal.program_owner != state.program_owner {
        return Err(GateError::ProgramOwnerMismatch);
    }
    if journal.threshold < state.minimum_threshold {
        return Err(GateError::ThresholdTooLow);
    }
    if !ctx.recent_roots.iter().any(|r| r == &journal.merkle_root) {
        return Err(GateError::RootStale);
    }

    // Presenter binding: the journal's presenter_pk must equal the
    // pk inside the challenge components, AND it must have signed the
    // rebuilt challenge.
    if components.presenter_pk != journal.presenter_pk {
        return Err(GateError::SignatureInvalid);
    }
    let challenge = rebuild_challenge(components, &ctx.program_id);
    let vk = VerifyingKey::from_bytes(&journal.presenter_pk)
        .map_err(|_| GateError::InvalidPresenterKey)?;
    let sig = Signature::from_bytes(signature);
    vk.verify(&challenge, &sig)
        .map_err(|_| GateError::SignatureInvalid)?;

    state.action_counter = state.action_counter.saturating_add(1);

    Ok(GateEvent {
        presenter_pk: journal.presenter_pk,
        action_tag,
        action_counter: state.action_counter,
        journal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> GateState {
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

    fn empty_ctx() -> DispatchCtx<'static> {
        DispatchCtx {
            program_id: [0u8; 32],
            recent_roots: &[],
            recent_slot_hashes: &[],
            circuit_image_id: &[0u32; 8],
            is_uninitialized: false,
        }
    }

    #[test]
    fn initialize_rejects_when_already_init() {
        let mut s = fresh_state();
        let r = handle(
            &mut s,
            &Instruction::Initialize {
                admin: [42u8; 32],
                gate_seed: [42u8; 32],
                program_owner: [42u8; 32],
                minimum_threshold: 1,
                initial_circuit_version: 2,
            },
            empty_ctx(),
            [99u8; 32],
        );
        assert_eq!(r, Err(GateError::AlreadyInitialized));
        assert_eq!(s.admin, [1u8; 32]);
    }

    #[test]
    fn rotate_admin_requires_admin() {
        let mut s = fresh_state();
        let r = handle(
            &mut s,
            &Instruction::RotateAdmin {
                new_admin: [9u8; 32],
            },
            empty_ctx(),
            [99u8; 32],
        );
        assert_eq!(r, Err(GateError::AdminOnly));
        assert_eq!(s.admin, [1u8; 32]);

        let r = handle(
            &mut s,
            &Instruction::RotateAdmin {
                new_admin: [9u8; 32],
            },
            empty_ctx(),
            [1u8; 32],
        );
        assert_eq!(r, Ok(None));
        assert_eq!(s.admin, [9u8; 32]);
    }

    #[test]
    fn gate_action_rejects_stale_slot_hash() {
        let mut s = fresh_state();
        let ctx = DispatchCtx {
            program_id: [0u8; 32],
            recent_roots: &[],
            recent_slot_hashes: &[[1u8; 32], [2u8; 32]],
            circuit_image_id: &[0u32; 8],
            is_uninitialized: false,
        };
        let components = ChallengeComponents {
            slot_hash: [99u8; 32],
            presenter_pk: [0u8; 32],
            action_tag: *b"vote#42         ",
        };
        let r = handle(
            &mut s,
            &Instruction::GateAction {
                receipt: vec![],
                challenge: components.clone(),
                signature: [0u8; 64],
                action_tag: *b"vote#42         ",
            },
            ctx,
            [99u8; 32],
        );
        assert_eq!(r, Err(GateError::ChallengeStale));
    }

    #[test]
    fn gate_action_rejects_action_tag_mismatch() {
        let mut s = fresh_state();
        let slot = [5u8; 32];
        let ctx = DispatchCtx {
            program_id: [0u8; 32],
            recent_roots: &[],
            recent_slot_hashes: &[slot],
            circuit_image_id: &[0u32; 8],
            is_uninitialized: false,
        };
        let components = ChallengeComponents {
            slot_hash: slot,
            presenter_pk: [0u8; 32],
            action_tag: *b"vote#42         ",
        };
        let r = handle(
            &mut s,
            &Instruction::GateAction {
                receipt: vec![],
                challenge: components,
                signature: [0u8; 64],
                action_tag: *b"vote#43         ",
            },
            ctx,
            [99u8; 32],
        );
        assert_eq!(r, Err(GateError::SignatureInvalid));
    }
}
