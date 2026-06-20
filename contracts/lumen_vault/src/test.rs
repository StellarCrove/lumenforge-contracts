#![cfg(test)]
use super::*;
use soroban_sdk::testutils::Address as _;

#[test]
fn deposit_and_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, LumenVault);
    let client = LumenVaultClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    client.initialize(&owner);

    assert_eq!(client.deposit(&owner, &500), 500);
    assert_eq!(client.balance(), 500);

    assert_eq!(client.withdraw(&200), 300);
    assert_eq!(client.balance(), 300);
}

#[test]
#[should_panic(expected = "invalid withdrawal amount")]
fn withdraw_more_than_balance_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, LumenVault);
    let client = LumenVaultClient::new(&env, &contract_id);

    let owner = Address::generate(&env);
    client.initialize(&owner);
    client.deposit(&owner, &100);
    client.withdraw(&1000);
}
