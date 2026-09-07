#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::token::{StellarAssetClient, TokenClient};

struct Setup<'a> {
    token: TokenClient<'a>,
    token_admin: StellarAssetClient<'a>,
    token_id: Address,
}

fn setup(env: &Env) -> Setup<'static> {
    let admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(admin);
    let token_id = sac.address();
    Setup {
        token: TokenClient::new(env, &token_id),
        token_admin: StellarAssetClient::new(env, &token_id),
        token_id,
    }
}

fn deploy(
    env: &Env,
    owner: &Address,
    token_id: &Address,
    min_deposit: i128,
    max_balance: Option<i128>,
) -> LumenVaultClient<'static> {
    let contract_id = env.register(LumenVault, (owner, token_id, min_deposit, max_balance));
    LumenVaultClient::new(env, &contract_id)
}

#[test]
fn deposit_and_withdraw_move_real_token_balances() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    s.token_admin.mint(&owner, &1_000);
    assert_eq!(s.token.balance(&owner), 1_000);

    assert_eq!(client.deposit(&owner, &500), 500);
    assert_eq!(client.balance(), 500);
    assert_eq!(s.token.balance(&owner), 500);
    assert_eq!(s.token.balance(&client.address), 500);

    assert_eq!(client.withdraw(&200), 300);
    assert_eq!(s.token.balance(&owner), 700);
    assert_eq!(s.token.balance(&client.address), 300);
}

#[test]
fn withdraw_more_than_balance_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);
    s.token_admin.mint(&owner, &1_000);
    client.deposit(&owner, &100);

    let result = client.try_withdraw(&1000);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn deposit_non_positive_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let depositor = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    let result = client.try_deposit(&depositor, &0);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn deposit_below_minimum_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let depositor = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 100, None);
    s.token_admin.mint(&depositor, &1_000);

    let result = client.try_deposit(&depositor, &50);
    assert_eq!(result, Err(Ok(Error::BelowMinimumDeposit)));

    assert_eq!(client.deposit(&depositor, &100), 100);
}

#[test]
fn deposit_exceeding_max_balance_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let depositor = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, Some(300));
    s.token_admin.mint(&depositor, &1_000);

    client.deposit(&depositor, &300);
    let result = client.try_deposit(&depositor, &1);
    assert_eq!(result, Err(Ok(Error::ExceedsMaxBalance)));
}

#[test]
fn set_min_deposit_and_max_balance_requires_owner() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    client.set_min_deposit(&50);
    assert_eq!(client.min_deposit(), 50);

    client.set_max_balance(&Some(1_000));
    assert_eq!(client.max_balance(), Some(1_000));

    client.set_max_balance(&None);
    assert_eq!(client.max_balance(), None);
}

#[test]
fn rescue_moves_a_different_token_but_not_the_vault_token() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    let other_admin = Address::generate(&env);
    let other_sac = env.register_stellar_asset_contract_v2(other_admin);
    let other_token = TokenClient::new(&env, &other_sac.address());
    let other_token_admin = StellarAssetClient::new(&env, &other_sac.address());
    other_token_admin.mint(&client.address, &777);

    let recipient = Address::generate(&env);
    client.rescue(&other_sac.address(), &recipient, &777);
    assert_eq!(other_token.balance(&recipient), 777);

    let result = client.try_rescue(&s.token_id, &recipient, &1);
    assert_eq!(result, Err(Ok(Error::CannotRescueVaultToken)));
}

#[test]
fn paused_vault_rejects_deposits() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let depositor = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);
    s.token_admin.mint(&depositor, &1_000);

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

    let s = setup(&env);
    let owner = Address::generate(&env);
    let successor = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);
    s.token_admin.mint(&owner, &1_000);

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

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    let result = client.try_accept_owner();
    assert_eq!(result, Err(Ok(Error::NoPendingOwner)));
}

#[test]
fn cancel_pending_owner_withdraws_the_proposal() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let successor = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    client.propose_owner(&successor);
    assert_eq!(client.pending_owner(), Some(successor.clone()));

    client.cancel_pending_owner();
    assert_eq!(client.pending_owner(), None);

    // The formerly-proposed successor can no longer accept.
    assert_eq!(client.try_accept_owner(), Err(Ok(Error::NoPendingOwner)));
    // Ownership is unchanged.
    assert_eq!(client.owner(), owner);
}

#[test]
fn cancel_pending_owner_without_proposal_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    assert_eq!(
        client.try_cancel_pending_owner(),
        Err(Ok(Error::NoPendingOwner))
    );
}

#[test]
fn deposit_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);
    s.token_admin.mint(&owner, &1_000);

    client.deposit(&owner, &42);

    let events = env.events().all();
    assert_eq!(events.events().len(), 2); // token's transfer event + our Deposit event
}

#[test]
fn deposit_and_withdraw_events_carry_the_running_balance() {
    use soroban_sdk::{xdr, Map, Symbol, TryFromVal, Val};

    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);
    s.token_admin.mint(&owner, &1_000);

    // The `#[contractevent]` macro serializes the non-topic fields as a
    // `{ field_name: value }` map. `new_balance` is the post-op balance,
    // so an indexer never has to replay every prior deposit/withdraw.
    // Read it back from the vault's own last event (the token emits a
    // Transfer event on the same call).
    let last_vault_event_data = || -> Map<Symbol, i128> {
        let events = env.events().all().filter_by_contract(&client.address);
        let raw = events.events();
        let last = raw.last().expect("vault emitted an event");
        let xdr::ContractEventBody::V0(body) = &last.body;
        let val = Val::try_from_val(&env, &body.data).unwrap();
        Map::<Symbol, i128>::try_from_val(&env, &val).unwrap()
    };

    client.deposit(&owner, &300);
    let d = last_vault_event_data();
    assert_eq!(d.get_unchecked(Symbol::new(&env, "amount")), 300);
    assert_eq!(d.get_unchecked(Symbol::new(&env, "new_balance")), 300);

    client.deposit(&owner, &200);
    assert_eq!(
        last_vault_event_data().get_unchecked(Symbol::new(&env, "new_balance")),
        500
    );

    client.withdraw(&150);
    let w = last_vault_event_data();
    assert_eq!(w.get_unchecked(Symbol::new(&env, "amount")), 150);
    assert_eq!(w.get_unchecked(Symbol::new(&env, "new_balance")), 350);
}

#[test]
fn extend_ttl_does_not_panic() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    client.extend_ttl(&100, &1000);
}

#[test]
fn constructor_rejects_negative_min_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        deploy(&env, &owner, &s.token_id, -1, None)
    }));
    assert!(result.is_err());
}

#[test]
fn constructor_rejects_negative_max_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        deploy(&env, &owner, &s.token_id, 0, Some(-1))
    }));
    assert!(result.is_err());
}

#[test]
fn set_min_deposit_rejects_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    let result = client.try_set_min_deposit(&-1);
    assert_eq!(result, Err(Ok(Error::InvalidConfiguration)));
}

#[test]
fn set_max_balance_rejects_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    let result = client.try_set_max_balance(&Some(-1));
    assert_eq!(result, Err(Ok(Error::InvalidConfiguration)));
}

#[test]
fn rescue_rejects_non_positive_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 0, None);

    let other_admin = Address::generate(&env);
    let other_token = env
        .register_stellar_asset_contract_v2(other_admin)
        .address();
    let recipient = Address::generate(&env);

    let result = client.try_rescue(&other_token, &recipient, &0);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}
