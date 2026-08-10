#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Events as _};

fn deploy(env: &Env, owner: &Address) -> LumenVaultClient<'static> {
    let contract_id = env.register(LumenVault, (owner,));
    LumenVaultClient::new(env, &contract_id)
}

#[test]
fn deposit_and_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let client = deploy(&env, &owner);

    assert_eq!(client.deposit(&owner, &500), 500);
    assert_eq!(client.balance(), 500);

    assert_eq!(client.withdraw(&200), 300);
    assert_eq!(client.balance(), 300);
}

#[test]
fn withdraw_more_than_balance_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let client = deploy(&env, &owner);
    client.deposit(&owner, &100);

    let result = client.try_withdraw(&1000);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn deposit_non_positive_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let depositor = Address::generate(&env);
    let client = deploy(&env, &owner);

    let result = client.try_deposit(&depositor, &0);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn paused_vault_rejects_deposits() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let depositor = Address::generate(&env);
    let client = deploy(&env, &owner);

    client.pause();
    assert!(client.paused());

    let result = client.try_deposit(&depositor, &100);
    assert_eq!(result, Err(Ok(Error::Paused)));

    client.unpause();
    assert!(!client.paused());
    assert_eq!(client.deposit(&depositor, &100), 100);
}

#[test]
fn two_step_ownership_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let successor = Address::generate(&env);
    let client = deploy(&env, &owner);

    client.deposit(&owner, &50);

    client.propose_owner(&successor);
    assert_eq!(client.pending_owner(), Some(successor.clone()));

    // Old owner can no longer withdraw once the successor accepts.
    client.accept_owner();
    assert_eq!(client.owner(), successor);
    assert_eq!(client.pending_owner(), None);

    assert_eq!(client.withdraw(&50), 0);
}

#[test]
fn accept_owner_without_proposal_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let client = deploy(&env, &owner);

    let result = client.try_accept_owner();
    assert_eq!(result, Err(Ok(Error::NoPendingOwner)));
}

#[test]
fn deposit_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let client = deploy(&env, &owner);

    client.deposit(&owner, &42);

    let events = env.events().all();
    assert_eq!(events.events().len(), 1);
}

#[test]
fn extend_ttl_does_not_panic() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let client = deploy(&env, &owner);

    client.extend_ttl(&100, &1000);
}
