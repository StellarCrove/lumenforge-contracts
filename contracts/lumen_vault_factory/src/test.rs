#![cfg(test)]
use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::vec;

mod vault_wasm {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/lumen_vault.wasm");
}

fn deploy_factory(env: &Env) -> LumenVaultFactoryClient<'static> {
    let wasm_hash = env.deployer().upload_contract_wasm(vault_wasm::WASM);
    let factory_id = env.register(LumenVaultFactory, (wasm_hash,));
    LumenVaultFactoryClient::new(env, &factory_id)
}

#[test]
fn deploy_vault_and_track_ownership() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let owner = Address::generate(&env);
    let salt = BytesN::from_array(&env, &[0u8; 32]);

    let vault_address = factory.deploy_vault(&owner, &salt);

    assert_eq!(factory.vault_count(), 1);
    assert_eq!(
        factory.vaults_by_owner(&owner),
        vec![&env, vault_address.clone()]
    );

    let vault_client = vault_wasm::Client::new(&env, &vault_address);
    assert_eq!(vault_client.owner(), owner);
    assert_eq!(vault_client.balance(), 0);
}

#[test]
fn multiple_owners_get_independent_vaults() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    factory.deploy_vault(&owner_a, &BytesN::from_array(&env, &[1u8; 32]));
    factory.deploy_vault(&owner_b, &BytesN::from_array(&env, &[2u8; 32]));

    assert_eq!(factory.vault_count(), 2);
    assert_eq!(factory.vaults_by_owner(&owner_a).len(), 1);
    assert_eq!(factory.vaults_by_owner(&owner_b).len(), 1);
}

#[test]
fn reusing_a_salt_for_the_same_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let owner = Address::generate(&env);
    let salt = BytesN::from_array(&env, &[7u8; 32]);

    factory.deploy_vault(&owner, &salt);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        factory.deploy_vault(&owner, &salt);
    }));
    assert!(result.is_err());
}

#[test]
fn extend_ttl_functions_do_not_panic() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let owner = Address::generate(&env);
    factory.deploy_vault(&owner, &BytesN::from_array(&env, &[9u8; 32]));

    factory.extend_ttl(&100, &1000);
    factory.extend_vaults_by_owner_ttl(&owner, &100, &1000);
}
