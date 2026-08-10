#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    InvalidAmount = 2,
    InsufficientBalance = 3,
    Paused = 4,
    NoPendingOwner = 5,
    Overflow = 6,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
    PendingOwner,
    Balance,
    Paused,
}

#[contractevent]
pub struct Deposit {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent]
pub struct Withdraw {
    #[topic]
    pub owner: Address,
    pub amount: i128,
}

#[contractevent]
pub struct Paused {
    #[topic]
    pub owner: Address,
}

#[contractevent]
pub struct Resumed {
    #[topic]
    pub owner: Address,
}

#[contractevent]
pub struct OwnerProposed {
    #[topic]
    pub new_owner: Address,
}

#[contractevent]
pub struct OwnerTransferred {
    #[topic]
    pub new_owner: Address,
}

#[contract]
pub struct LumenVault;

#[contractimpl]
impl LumenVault {
    /// Runs atomically with contract creation, so there is no window in
    /// which an unrelated caller could front-run `initialize` and claim
    /// ownership of the vault.
    pub fn __constructor(env: Env, owner: Address) {
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::Balance, &0i128);
        env.storage().instance().set(&DataKey::Paused, &false);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<i128, Error> {
        from.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if Self::is_paused(&env) {
            return Err(Error::Paused);
        }

        let balance = Self::read_balance(&env);
        let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
        env.storage()
            .instance()
            .set(&DataKey::Balance, &new_balance);
        Deposit { from, amount }.publish(&env);
        Ok(new_balance)
    }

    pub fn withdraw(env: Env, amount: i128) -> Result<i128, Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let balance = Self::read_balance(&env);
        if amount > balance {
            return Err(Error::InsufficientBalance);
        }
        let new_balance = balance - amount;
        env.storage()
            .instance()
            .set(&DataKey::Balance, &new_balance);
        Withdraw { owner, amount }.publish(&env);
        Ok(new_balance)
    }

    pub fn pause(env: Env) -> Result<(), Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        env.storage().instance().set(&DataKey::Paused, &true);
        Paused { owner }.publish(&env);
        Ok(())
    }

    pub fn unpause(env: Env) -> Result<(), Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        env.storage().instance().set(&DataKey::Paused, &false);
        Resumed { owner }.publish(&env);
        Ok(())
    }

    /// Step 1 of a two-step ownership transfer: the current owner names a
    /// successor. Nothing changes until that successor calls
    /// `accept_owner`, which avoids permanently locking the vault behind
    /// a mistyped or unreachable address.
    pub fn propose_owner(env: Env, new_owner: Address) -> Result<(), Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::PendingOwner, &new_owner);
        OwnerProposed { new_owner }.publish(&env);
        Ok(())
    }

    /// Step 2: the proposed successor claims ownership by authorizing
    /// this call themselves, proving they control that address.
    pub fn accept_owner(env: Env) -> Result<(), Error> {
        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingOwner)
            .ok_or(Error::NoPendingOwner)?;
        pending.require_auth();
        env.storage().instance().set(&DataKey::Owner, &pending);
        env.storage().instance().remove(&DataKey::PendingOwner);
        OwnerTransferred { new_owner: pending }.publish(&env);
        Ok(())
    }

    pub fn balance(env: Env) -> i128 {
        Self::read_balance(&env)
    }

    pub fn owner(env: Env) -> Result<Address, Error> {
        Self::read_owner(&env)
    }

    pub fn pending_owner(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::PendingOwner)
    }

    pub fn paused(env: Env) -> bool {
        Self::is_paused(&env)
    }

    fn read_balance(env: &Env) -> i128 {
        env.storage().instance().get(&DataKey::Balance).unwrap_or(0)
    }

    fn read_owner(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Owner)
            .ok_or(Error::NotInitialized)
    }

    fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }
}

mod test;
