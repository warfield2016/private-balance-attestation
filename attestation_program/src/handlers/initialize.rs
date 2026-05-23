use nssa_core::account::{Account, AccountWithMetadata};
use spel_framework::prelude::SpelError;

use crate::errors::GateError;
use crate::state::GateState;

/// Build a fresh `GateState`, serialise it into the PDA payload, and
/// return the post-state account list.
pub fn handle(
    accounts: &[AccountWithMetadata],
    gate_seed: [u8; 32],
    program_owner: [u8; 32],
    minimum_threshold: u64,
    initial_circuit_version: u32,
) -> Result<Vec<Account>, SpelError> {
    if accounts.len() < 2 {
        return Err(err(
            GateError::AlreadyInitialized,
            "initialize: need gate_state + admin",
        ));
    }
    let gate_state = &accounts[0];
    let admin = &accounts[1];

    let admin_pk: [u8; 32] = *admin.account_id.value();

    let mut state = GateState {
        admin: admin_pk,
        gate_seed,
        program_owner,
        minimum_threshold,
        allowed_circuit_versions: [0u32; 8],
        action_counter: 0,
        bump: 0,
    };
    let _ = state.add_circuit_version(initial_circuit_version);

    let payload =
        borsh::to_vec(&state).map_err(|e| err(GateError::JournalDecode, &e.to_string()))?;
    let mut gate_post = gate_state.account.clone();
    gate_post.data = payload.try_into().map_err(|_| {
        err(
            GateError::JournalDecode,
            "payload exceeds account data limit",
        )
    })?;

    Ok(vec![gate_post, admin.account.clone()])
}

fn err(code: GateError, msg: &str) -> SpelError {
    SpelError::custom(code.code(), msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nssa_core::account::{Account, AccountId, AccountWithMetadata};

    fn empty_awm(id: [u8; 32]) -> AccountWithMetadata {
        AccountWithMetadata {
            account_id: AccountId::new(id),
            account: Account::default(),
            is_authorized: true,
        }
    }

    #[test]
    fn initialize_writes_state_and_lists_admin() {
        let admin_pk = [42u8; 32];
        let out = handle(
            &[empty_awm([0u8; 32]), empty_awm(admin_pk)],
            [7u8; 32],
            [8u8; 32],
            1000,
            2,
        )
        .unwrap();
        assert_eq!(out.len(), 2, "returns gate_state + admin accounts");

        let payload: Vec<u8> = out[0].data.clone().into();
        let state: GateState = borsh::from_slice(&payload).unwrap();
        assert_eq!(state.admin, admin_pk);
        assert_eq!(state.gate_seed, [7u8; 32]);
        assert_eq!(state.program_owner, [8u8; 32]);
        assert_eq!(state.minimum_threshold, 1000);
        assert!(state.allowed_versions_slice().contains(&2));
        assert_eq!(state.action_counter, 0);
    }

    #[test]
    fn initialize_rejects_too_few_accounts() {
        let r = handle(&[empty_awm([0u8; 32])], [0u8; 32], [0u8; 32], 0, 1);
        assert!(r.is_err());
    }
}
