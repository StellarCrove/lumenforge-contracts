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

fn test_token(env: &Env) -> Address {
    let admin = Address::generate(env);
    env.register_stellar_asset_contract_v2(admin).address()
}

#[test]
fn deploy_vault_and_track_ownership() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let token = test_token(&env);
    let owner = Address::generate(&env);
    let salt = BytesN::from_array(&env, &[0u8; 32]);

    let vault_address = factory.deploy_vault(&owner, &token, &0, &None, &salt);

    assert_eq!(factory.vault_count(), 1);
    assert_eq!(
        factory.vaults_by_owner(&owner, &0, &10),
        vec![&env, vault_address.clone()]
    );

    let vault_client = vault_wasm::Client::new(&env, &vault_address);
    assert_eq!(vault_client.owner(), owner);
    assert_eq!(vault_client.token(), token);
    assert_eq!(vault_client.balance(), 0);
}

#[test]
fn multiple_owners_get_independent_vaults() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let token = test_token(&env);
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    factory.deploy_vault(
        &owner_a,
        &token,
        &0,
        &None,
        &BytesN::from_array(&env, &[1u8; 32]),
    );
    factory.deploy_vault(
        &owner_b,
        &token,
        &0,
        &None,
        &BytesN::from_array(&env, &[2u8; 32]),
    );

    assert_eq!(factory.vault_count(), 2);
    assert_eq!(factory.vaults_by_owner(&owner_a, &0, &10).len(), 1);
    assert_eq!(factory.vaults_by_owner(&owner_b, &0, &10).len(), 1);
}

#[test]
fn vaults_by_owner_paginates() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let token = test_token(&env);
    let owner = Address::generate(&env);

    for i in 0..5u8 {
        let mut salt_bytes = [0u8; 32];
        salt_bytes[0] = i;
        factory.deploy_vault(
            &owner,
            &token,
            &0,
            &None,
            &BytesN::from_array(&env, &salt_bytes),
        );
    }

    assert_eq!(factory.vaults_by_owner(&owner, &0, &2).len(), 2);
    assert_eq!(factory.vaults_by_owner(&owner, &2, &2).len(), 2);
    assert_eq!(factory.vaults_by_owner(&owner, &4, &2).len(), 1);
    assert_eq!(factory.vaults_by_owner(&owner, &10, &2).len(), 0);
    assert_eq!(factory.vaults_by_owner(&owner, &0, &100).len(), 5);
}

#[test]
fn reusing_a_salt_for_the_same_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let token = test_token(&env);
    let owner = Address::generate(&env);
    let salt = BytesN::from_array(&env, &[7u8; 32]);

    factory.deploy_vault(&owner, &token, &0, &None, &salt);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        factory.deploy_vault(&owner, &token, &0, &None, &salt);
    }));
    assert!(result.is_err());
}

#[test]
fn extend_ttl_functions_do_not_panic() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let token = test_token(&env);
    let owner = Address::generate(&env);
    factory.deploy_vault(
        &owner,
        &token,
        &0,
        &None,
        &BytesN::from_array(&env, &[9u8; 32]),
    );

    factory.extend_ttl(&100, &1000);
    factory.extend_vaults_by_owner_ttl(&owner, &100, &1000);
}

#[test]
fn extend_vaults_by_owner_ttl_without_vaults_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let factory = deploy_factory(&env);
    let owner = Address::generate(&env);

    let result = factory.try_extend_vaults_by_owner_ttl(&owner, &100, &1000);
    assert_eq!(result, Err(Ok(Error::NoVaultsForOwner)));
}
