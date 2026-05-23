use nssa_core::account::{Account, AccountWithMetadata};
use spel_framework::prelude::SpelError;

use crate::errors::GateError;
use crate::state::GateState;

fn err(code: GateError, msg: &str) -> SpelError {
    SpelError::custom(code.code(), msg)
}

fn load(account: &AccountWithMetadata) -> Result<GateState, SpelError> {
    let data: Vec<u8> = account.account.data.clone().into();
    borsh::from_slice(&data).map_err(|e| err(GateError::JournalDecode, &e.to_string()))
}

fn commit(
    gate_state: &AccountWithMetadata,
    caller: &AccountWithMetadata,
    state: GateState,
) -> Result<Vec<Account>, SpelError> {
    let payload =
        borsh::to_vec(&state).map_err(|e| err(GateError::JournalDecode, &e.to_string()))?;
    let mut gate_post = gate_state.account.clone();
    gate_post.data = payload.try_into().map_err(|_| {
        err(
            GateError::JournalDecode,
            "payload exceeds account data limit",
        )
    })?;
    Ok(vec![gate_post, caller.account.clone()])
}

fn require_admin(state: &GateState, caller: &AccountWithMetadata) -> Result<(), SpelError> {
    let pk: [u8; 32] = *caller.account_id.value();
    if state.admin != pk {
        return Err(err(GateError::AdminOnly, "admin only"));
    }
    Ok(())
}

pub fn rotate_admin(
    accounts: &[AccountWithMetadata],
    new_admin: [u8; 32],
) -> Result<Vec<Account>, SpelError> {
    if accounts.len() < 2 {
        return Err(err(
            GateError::AdminOnly,
            "rotate_admin: need gate_state + caller",
        ));
    }
    let mut state = load(&accounts[0])?;
    require_admin(&state, &accounts[1])?;
    state.admin = new_admin;
    commit(&accounts[0], &accounts[1], state)
}

pub fn add_circuit(
    accounts: &[AccountWithMetadata],
    version: u32,
) -> Result<Vec<Account>, SpelError> {
    if accounts.len() < 2 {
        return Err(err(
            GateError::AdminOnly,
            "add_circuit: need gate_state + caller",
        ));
    }
    let mut state = load(&accounts[0])?;
    require_admin(&state, &accounts[1])?;
    let _ = state.add_circuit_version(version);
    commit(&accounts[0], &accounts[1], state)
}

pub fn revoke_circuit(
    accounts: &[AccountWithMetadata],
    version: u32,
) -> Result<Vec<Account>, SpelError> {
    if accounts.len() < 2 {
        return Err(err(
            GateError::AdminOnly,
            "revoke_circuit: need gate_state + caller",
        ));
    }
    let mut state = load(&accounts[0])?;
    require_admin(&state, &accounts[1])?;
    let _ = state.revoke_circuit_version(version);
    commit(&accounts[0], &accounts[1], state)
}

pub fn update_minimum(
    accounts: &[AccountWithMetadata],
    new_threshold: u64,
) -> Result<Vec<Account>, SpelError> {
    if accounts.len() < 2 {
        return Err(err(
            GateError::AdminOnly,
            "update_minimum: need gate_state + caller",
        ));
    }
    let mut state = load(&accounts[0])?;
    require_admin(&state, &accounts[1])?;
    state.minimum_threshold = new_threshold;
    commit(&accounts[0], &accounts[1], state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nssa_core::account::{Account, AccountId, AccountWithMetadata};

    fn fresh_state(admin_pk: [u8; 32]) -> GateState {
        GateState {
            admin: admin_pk,
            gate_seed: [2u8; 32],
            program_owner: [3u8; 32],
            minimum_threshold: 100,
            allowed_circuit_versions: [2, 0, 0, 0, 0, 0, 0, 0],
            action_counter: 0,
            bump: 0,
        }
    }

    // Build an AccountWithMetadata whose .account.data carries the
    // borsh-encoded GateState. Used by the admin tests to simulate
    // what the macro would route in on-chain.
    #[allow(clippy::field_reassign_with_default)]
    fn awm_with_state(id: [u8; 32], state: &GateState) -> AccountWithMetadata {
        let mut account = Account::default();
        account.data = borsh::to_vec(state).unwrap().try_into().unwrap();
        AccountWithMetadata {
            account_id: AccountId::new(id),
            account,
            is_authorized: true,
        }
    }

    fn awm_signer(id: [u8; 32]) -> AccountWithMetadata {
        AccountWithMetadata {
            account_id: AccountId::new(id),
            account: Account::default(),
            is_authorized: true,
        }
    }

    #[test]
    fn rotate_admin_happy_path() {
        let admin = [1u8; 32];
        let new_admin = [9u8; 32];
        let gate = awm_with_state([0u8; 32], &fresh_state(admin));
        let caller = awm_signer(admin);
        let out = rotate_admin(&[gate.clone(), caller], new_admin).unwrap();
        // out[0] is the gate post-state; decode and check.
        let payload: Vec<u8> = out[0].data.clone().into();
        let updated: GateState = borsh::from_slice(&payload).unwrap();
        assert_eq!(updated.admin, new_admin);
    }

    #[test]
    fn rotate_admin_rejects_non_admin() {
        let admin = [1u8; 32];
        let gate = awm_with_state([0u8; 32], &fresh_state(admin));
        let caller = awm_signer([99u8; 32]); // not the admin
        let result = rotate_admin(&[gate, caller], [9u8; 32]);
        let err = result.unwrap_err();
        assert!(format!("{err:?}").contains("admin only"));
    }

    #[test]
    fn update_minimum_persists() {
        let admin = [1u8; 32];
        let gate = awm_with_state([0u8; 32], &fresh_state(admin));
        let caller = awm_signer(admin);
        let out = update_minimum(&[gate, caller], 5000).unwrap();
        let payload: Vec<u8> = out[0].data.clone().into();
        let updated: GateState = borsh::from_slice(&payload).unwrap();
        assert_eq!(updated.minimum_threshold, 5000);
    }

    #[test]
    fn add_then_revoke_circuit() {
        let admin = [1u8; 32];
        let gate = awm_with_state([0u8; 32], &fresh_state(admin));
        let caller = awm_signer(admin);

        let out = add_circuit(&[gate, caller.clone()], 3).unwrap();
        let payload: Vec<u8> = out[0].data.clone().into();
        let mid: GateState = borsh::from_slice(&payload).unwrap();
        assert!(mid.allowed_versions_slice().contains(&3));

        let gate2 = awm_with_state([0u8; 32], &mid);
        let out2 = revoke_circuit(&[gate2, caller], 3).unwrap();
        let payload2: Vec<u8> = out2[0].data.clone().into();
        let after: GateState = borsh::from_slice(&payload2).unwrap();
        assert!(!after.allowed_versions_slice().contains(&3));
    }

    #[test]
    fn insufficient_accounts_rejected() {
        assert!(rotate_admin(&[], [9u8; 32]).is_err());
        assert!(add_circuit(&[], 5).is_err());
        assert!(revoke_circuit(&[], 5).is_err());
        assert!(update_minimum(&[], 100).is_err());
    }
}
