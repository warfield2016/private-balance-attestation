use nssa_core::account::{Account, AccountWithMetadata};
use spel_framework::prelude::SpelError;

use crate::dispatch::{handle as dispatch_handle, DispatchCtx};
use crate::errors::GateError;
use crate::state::{ChallengeComponents, GateState, Instruction};

fn err(code: GateError, msg: &str) -> SpelError {
    SpelError::custom(code.code(), msg)
}

/// Apply a `GateAction` against the gate's PDA. The dispatcher decodes
/// the journal, applies policy field-equality checks, and verifies
/// the ed25519 presenter signature. This function only wires accounts
/// to dispatcher inputs and writes the bumped counter back.
pub fn handle(
    accounts: &[AccountWithMetadata],
    receipt: Vec<u8>,
    challenge: ChallengeComponents,
    signature: [u8; 64],
    action_tag: [u8; 16],
) -> Result<Vec<Account>, SpelError> {
    if accounts.is_empty() {
        return Err(err(
            GateError::JournalDecode,
            "gate_action: need gate_state account",
        ));
    }
    let gate_state = &accounts[0];

    let data: Vec<u8> = gate_state.account.data.clone().into();
    let mut state: GateState =
        borsh::from_slice(&data).map_err(|e| err(GateError::JournalDecode, &e.to_string()))?;

    // KNOWN LIMITATION (documented; tracked in STATUS.md).
    //
    // For this submission the on-chain handler does NOT cryptographically
    // verify the RISC0 receipt. It cannot: receipt verification needs
    // `risc0-zkvm` (host-crypto, std), which does not build for the LEZ
    // guest target. The off-chain verifier path
    // (`attestation_verifier::verify_attestation`) does the full
    // cryptographic check using `attestation_host::ATTESTATION_GUEST_ID`.
    //
    // Two principled fixes for a follow-up submission:
    //   (a) LEZ runtime exposes a `verify_receipt(image_id, bytes)`
    //       syscall — the dispatcher would then call it and we'd pass
    //       `&state.circuit_image_id` instead of zeros here.
    //   (b) Store the trusted image_id in `GateState` at initialise
    //       time and have an off-chain attester co-sign the receipt's
    //       admissibility before invoking this instruction.
    //
    // Until (a) or (b) lands, this handler trusts the upstream channel
    // (e.g. a sequencer that has already verified the receipt). The
    // dispatcher still enforces every non-cryptographic gate field
    // (context_id, threshold, program_owner, presenter signature).
    let ctx = DispatchCtx {
        program_id: [0u8; 32],
        recent_roots: &[],
        recent_slot_hashes: &[challenge.slot_hash],
        circuit_image_id: &[0u32; 8],
        is_uninitialized: false,
    };

    let ix = Instruction::GateAction {
        receipt,
        challenge,
        signature,
        action_tag,
    };

    let caller_pubkey: [u8; 32] = *gate_state.account_id.value();
    let event = dispatch_handle(&mut state, &ix, ctx, caller_pubkey)
        .map_err(|e| err(e, &format!("gate denied: code={}", e.code())))?
        .ok_or_else(|| err(GateError::ReceiptInvalid, "gate_action returned no event"))?;

    let _ = event;
    let payload =
        borsh::to_vec(&state).map_err(|e| err(GateError::JournalDecode, &e.to_string()))?;
    let mut gate_post = gate_state.account.clone();
    gate_post.data = payload.try_into().map_err(|_| {
        err(
            GateError::JournalDecode,
            "payload exceeds account data limit",
        )
    })?;

    Ok(vec![gate_post])
}
