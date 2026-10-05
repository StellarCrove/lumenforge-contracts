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

/// SEP-41-shaped token that credits the recipient `amount - fee`.
/// Used only to prove `deposit` refuses a short credit.
#[contract]
struct FeeToken;

#[contracttype]
enum FeeKey {
    Admin,
    FeeBps,
    Bal(Address),
}

#[contractimpl]
impl FeeToken {
    pub fn __constructor(env: Env, admin: Address, fee_bps: i128) {
        env.storage().instance().set(&FeeKey::Admin, &admin);
        env.storage().instance().set(&FeeKey::FeeBps, &fee_bps);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let admin: Address = env.storage().instance().get(&FeeKey::Admin).unwrap();
        admin.require_auth();
        let next = Self::balance(env.clone(), to.clone()) + amount;
        env.storage().persistent().set(&FeeKey::Bal(to), &next);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&FeeKey::Bal(id))
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: MuxedAddress, amount: i128) {
        from.require_auth();
        let fee_bps: i128 = env.storage().instance().get(&FeeKey::FeeBps).unwrap();
        let fee = amount.saturating_mul(fee_bps) / 10_000;
        let credited = amount - fee;
        let to_addr = to.address();
        let from_next = Self::balance(env.clone(), from.clone()) - amount;
        env.storage()
            .persistent()
            .set(&FeeKey::Bal(from), &from_next);
        let to_next = Self::balance(env.clone(), to_addr.clone()) + credited;
        env.storage()
            .persistent()
            .set(&FeeKey::Bal(to_addr), &to_next);
    }
}

fn deploy_fee_vault(env: &Env, fee_bps: i128) -> (LumenVaultClient<'static>, Address, Address) {
    let admin = Address::generate(env);
    let token_id = env.register(FeeToken, (admin, fee_bps));
    let owner = Address::generate(env);
    let client = deploy(env, &owner, &token_id, 0, None);
    (client, token_id, owner)
}

#[test]
fn deposit_rejects_fee_on_transfer_and_keeps_balance_in_sync() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, token_id, owner) = deploy_fee_vault(&env, 1_000);
    let token = TokenClient::new(&env, &token_id);
    FeeTokenClient::new(&env, &token_id).mint(&owner, &1_000);

    let result = client.try_deposit(&owner, &1_000);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    assert_eq!(client.balance(), 0);
    assert_eq!(token.balance(&owner), 1_000);
    assert_eq!(token.balance(&client.address), 0);
}

#[test]
fn batch_deposit_credits_each_from_and_is_atomic() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 10, Some(1_000));
    s.token_admin.mint(&a, &500);
    s.token_admin.mint(&b, &500);

    let ok = soroban_sdk::vec![&env, (a.clone(), 100i128), (b.clone(), 250i128)];
    assert_eq!(client.batch_deposit(&ok), 350);
    assert_eq!(s.token.balance(&client.address), 350);
    assert_eq!(s.token.balance(&a), 400);
    assert_eq!(s.token.balance(&b), 250);

    let mixed = soroban_sdk::vec![&env, (a.clone(), 50i128), (b.clone(), 0i128)];
    assert_eq!(
        client.try_batch_deposit(&mixed),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(client.balance(), 350);
    assert_eq!(s.token.balance(&a), 400);

    let empty: soroban_sdk::Vec<(Address, i128)> = soroban_sdk::vec![&env];
    assert_eq!(
        client.try_batch_deposit(&empty),
        Err(Ok(Error::InvalidAmount))
    );
}

// Ignored in the default `cargo test` run: 10,000 host calls take a few
// minutes in debug. `make fuzz` and CI run it explicitly.
#[test]
#[ignore]
fn invariant_ten_thousand_transitions_keep_balance_equal_to_holdings() {
    let env = Env::default();
    env.mock_all_auths();

    let s = setup(&env);
    let owner = Address::generate(&env);
    let client = deploy(&env, &owner, &s.token_id, 1, Some(50_000));
    s.token_admin.mint(&owner, &1_000_000);

    let mut expected = 0i128;
    let mut min_deposit = 1i128;
    let mut max_balance = Some(50_000i128);
    let mut paused = false;
    let mut state = 0x5EED_u64;

    for _ in 0..10_000 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        match state % 5 {
            0 | 1 => {
                let room = max_balance.unwrap_or(i128::MAX).saturating_sub(expected);
                let amount = if room < min_deposit {
                    min_deposit
                } else {
                    min_deposit + (state as i128 % (room - min_deposit + 1).max(1))
                };
                let result = client.try_deposit(&owner, &amount);
                if paused || amount < min_deposit || room < amount || amount <= 0 {
                    assert!(result.is_err());
                } else {
                    assert_eq!(result, Ok(Ok(expected + amount)));
                    expected += amount;
                }
            }
            2 => {
                if expected == 0 {
                    assert!(client.try_withdraw(&1).is_err());
                } else {
                    let amount = 1 + (state as i128 % expected);
                    assert_eq!(client.try_withdraw(&amount), Ok(Ok(expected - amount)));
                    expected -= amount;
                }
            }
            3 => {
                if paused {
                    client.unpause();
                    paused = false;
                } else {
                    client.pause();
                    paused = true;
                }
            }
            _ => {
                let next_min = 1 + (state as i128 % 20);
                client.set_min_deposit(&next_min);
                min_deposit = next_min;
                if state.is_multiple_of(2) {
                    let cap = expected.max(next_min) + (state as i128 % 100);
                    client.set_max_balance(&Some(cap));
                    max_balance = Some(cap);
                }
            }
        }
        assert_eq!(client.balance(), expected);
        assert_eq!(s.token.balance(&client.address), expected);
        // A successful deposit cannot finish above the cap. Lowering the cap
        // afterwards is allowed, so `expected` may sit above `max_balance`.
        let _ = (max_balance, paused);
        assert!(expected >= 0);
    }
}

#[cfg(test)]
mod invariant_proptest {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]
        #[test]
        fn invariant_proptest_deposits_match_holdings(seed in any::<u64>()) {
            let env = Env::default();
            env.mock_all_auths();
            let s = setup(&env);
            let owner = Address::generate(&env);
            let client = deploy(&env, &owner, &s.token_id, 1, Some(10_000));
            s.token_admin.mint(&owner, &100_000);

            let mut expected = 0i128;
            let mut state = seed;
            for _ in 0..40 {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                let room = 10_000 - expected;
                if room < 1 {
                    let amount = 1 + (state as i128 % expected.max(1));
                    prop_assert_eq!(client.try_withdraw(&amount), Ok(Ok(expected - amount)));
                    expected -= amount;
                } else {
                    let amount = 1 + (state as i128 % room);
                    prop_assert_eq!(client.try_deposit(&owner, &amount), Ok(Ok(expected + amount)));
                    expected += amount;
                }
                prop_assert_eq!(client.balance(), expected);
                prop_assert_eq!(s.token.balance(&client.address), expected);
            }
        }
    }
}
