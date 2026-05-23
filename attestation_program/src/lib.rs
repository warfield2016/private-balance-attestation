// LP-0005 on-chain attestation gate.
//
// Two surfaces share the same pure-Rust dispatch logic:
//
//   1. `attestation_gate` (the macro mod below) — annotated with
//      `#[lez_program]` from spel-framework. Generates the LEZ
//      entrypoint, the instruction dispatch, the IDL JSON, and the
//      account-management boilerplate. Each handler is a thin shell
//      that hands off to the pure-Rust modules.
//
//   2. `dispatch::handle` (this file's siblings) — pure-Rust callable
//      directly. Used by the off-chain verifier path, unit tests, and
//      example programs that wrap the gate via CPI in the future.
//
// The macro mod's responsibility is wiring `AccountWithMetadata` to
// the pure-Rust handler's `(state, instruction, dispatch_ctx)`
// signature, then writing the post-state back. The security argument
// lives entirely in `dispatch::handle` and
// `attestation_verifier::verify_attestation`.

#![deny(unsafe_code)]

pub mod challenge;
pub mod dispatch;
pub mod errors;
pub mod handlers;
pub mod state;

pub use dispatch::{handle, DispatchCtx, GateEvent, RECENT_ROOTS_K, RECENT_SLOT_HASHES_K};
pub use errors::GateError;
pub use state::{ChallengeComponents, GateState, Instruction};

// -----------------------------------------------------------------------
// LEZ program surface — compiled only as a riscv32im guest.
// -----------------------------------------------------------------------

#[cfg(target_os = "zkvm")]
use spel_framework::prelude::*;

#[cfg(target_os = "zkvm")]
#[lez_program(instruction = "attestation_core::Instruction")]
mod attestation_gate {
    use super::*;

    /// Create a fresh gate. PDA derives from `gate_seed`.
    #[instruction]
    pub fn initialize(
        #[account(init, pda = arg("gate_seed"))] gate_state: AccountWithMetadata,
        #[account(signer)] admin: AccountWithMetadata,
        gate_seed: [u8; 32],
        program_owner: [u8; 32],
        minimum_threshold: u64,
        initial_circuit_version: u32,
    ) -> SpelResult {
        let accounts = vec![gate_state, admin];
        let accounts_out = crate::handlers::initialize::handle(
            &accounts,
            gate_seed,
            program_owner,
            minimum_threshold,
            initial_circuit_version,
        )?;
        Ok(SpelOutput::execute(accounts_out, vec![]))
    }

    /// Primary admission path. The presenter has produced a RISC0 receipt
    /// off-chain; the handler runs `verify_attestation` against the
    /// journal + signature challenge.
    #[instruction]
    pub fn gate_action(
        #[account(mut, pda = arg("gate_seed"))] gate_state: AccountWithMetadata,
        gate_seed: [u8; 32],
        receipt: Vec<u8>,
        challenge: ChallengeComponents,
        signature: [u8; 64],
        action_tag: [u8; 16],
    ) -> SpelResult {
        let _ = gate_seed; // pinned via the PDA derivation
        let accounts = vec![gate_state];
        let accounts_out = crate::handlers::gate_action::handle(
            &accounts, receipt, challenge, signature, action_tag,
        )?;
        Ok(SpelOutput::execute(accounts_out, vec![]))
    }

    /// Admin-only: rotate the admin key.
    #[instruction]
    pub fn rotate_admin(
        #[account(mut, pda = arg("gate_seed"))] gate_state: AccountWithMetadata,
        #[account(signer)] caller: AccountWithMetadata,
        gate_seed: [u8; 32],
        new_admin: [u8; 32],
    ) -> SpelResult {
        let _ = gate_seed;
        let accounts = vec![gate_state, caller];
        let accounts_out = crate::handlers::admin::rotate_admin(&accounts, new_admin)?;
        Ok(SpelOutput::execute(accounts_out, vec![]))
    }

    /// Admin-only: append an allowed circuit version.
    #[instruction]
    pub fn add_circuit(
        #[account(mut, pda = arg("gate_seed"))] gate_state: AccountWithMetadata,
        #[account(signer)] caller: AccountWithMetadata,
        gate_seed: [u8; 32],
        version: u32,
    ) -> SpelResult {
        let _ = gate_seed;
        let accounts = vec![gate_state, caller];
        let accounts_out = crate::handlers::admin::add_circuit(&accounts, version)?;
        Ok(SpelOutput::execute(accounts_out, vec![]))
    }

    /// Admin-only: revoke an allowed circuit version.
    #[instruction]
    pub fn revoke_circuit(
        #[account(mut, pda = arg("gate_seed"))] gate_state: AccountWithMetadata,
        #[account(signer)] caller: AccountWithMetadata,
        gate_seed: [u8; 32],
        version: u32,
    ) -> SpelResult {
        let _ = gate_seed;
        let accounts = vec![gate_state, caller];
        let accounts_out = crate::handlers::admin::revoke_circuit(&accounts, version)?;
        Ok(SpelOutput::execute(accounts_out, vec![]))
    }

    /// Admin-only: change the minimum threshold a proof must clear.
    #[instruction]
    pub fn update_minimum(
        #[account(mut, pda = arg("gate_seed"))] gate_state: AccountWithMetadata,
        #[account(signer)] caller: AccountWithMetadata,
        gate_seed: [u8; 32],
        new_threshold: u64,
    ) -> SpelResult {
        let _ = gate_seed;
        let accounts = vec![gate_state, caller];
        let accounts_out = crate::handlers::admin::update_minimum(&accounts, new_threshold)?;
        Ok(SpelOutput::execute(accounts_out, vec![]))
    }
}
