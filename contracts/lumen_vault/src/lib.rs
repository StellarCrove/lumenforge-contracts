#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

const BALANCE: Symbol = symbol_short!("BALANCE");

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
}

#[contract]
pub struct LumenVault;

#[contractimpl]
impl LumenVault {
    pub fn initialize(env: Env, owner: Address) {
        if env.storage().instance().has(&DataKey::Owner) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&BALANCE, &0i128);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) -> i128 {
        from.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }
        let balance: i128 = env.storage().instance().get(&BALANCE).unwrap_or(0);
        let new_balance = balance + amount;
        env.storage().instance().set(&BALANCE, &new_balance);
        new_balance
    }

    pub fn withdraw(env: Env, amount: i128) -> i128 {
        let owner: Address = env
            .storage()
            .instance()
            .get(&DataKey::Owner)
            .expect("not initialized");
        owner.require_auth();

        let balance: i128 = env.storage().instance().get(&BALANCE).unwrap_or(0);
        if amount <= 0 || amount > balance {
            panic!("invalid withdrawal amount");
        }
        let new_balance = balance - amount;
        env.storage().instance().set(&BALANCE, &new_balance);
        new_balance
    }

    pub fn balance(env: Env) -> i128 {
        env.storage().instance().get(&BALANCE).unwrap_or(0)
    }
}

mod test;
