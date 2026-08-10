#![no_std]
#[cfg(test)]
extern crate std;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contractmeta, contracttype, token,
    Address, Env, MuxedAddress,
};

contractmeta!(key = "Name", val = "lumen_vault");
contractmeta!(
    key = "Description",
    val = "Owner-gated SEP-41 deposit/withdraw vault"
);

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
    BelowMinimumDeposit = 7,
    ExceedsMaxBalance = 8,
    CannotRescueVaultToken = 9,
    InvalidConfiguration = 10,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
    PendingOwner,
    Token,
    Balance,
    Paused,
    MinDeposit,
    MaxBalance,
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

#[contractevent]
pub struct MinDepositUpdated {
    pub min_deposit: i128,
}

#[contractevent]
pub struct MaxBalanceUpdated {
    pub max_balance: Option<i128>,
}

#[contractevent]
pub struct Rescued {
    #[topic]
    pub token: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contract]
pub struct LumenVault;

#[contractimpl]
impl LumenVault {
    /// Runs atomically with contract creation, so there is no window in
    /// which an unrelated caller could front-run `initialize` and claim
    /// ownership of the vault. `token` is the SEP-41 asset this vault
    /// actually custodies — deposits and withdrawals move real balances
    /// of it, they don't just increment an internal counter.
    pub fn __constructor(
        env: Env,
        owner: Address,
        token: Address,
        min_deposit: i128,
        max_balance: Option<i128>,
    ) -> Result<(), Error> {
        Self::validate_deposit_bounds(min_deposit, max_balance)?;

        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Balance, &0i128);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::MinDeposit, &min_deposit);
        if let Some(max) = max_balance {
            env.storage().instance().set(&DataKey::MaxBalance, &max);
        }
        Ok(())
    }

    pub fn deposit(env: Env, from: Address, amount: i128) -> Result<i128, Error> {
        from.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if Self::is_paused(&env) {
            return Err(Error::Paused);
        }
        if amount < Self::read_min_deposit(&env) {
            return Err(Error::BelowMinimumDeposit);
        }

        let balance = Self::read_balance(&env);
        let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
        if let Some(max) = Self::read_max_balance(&env) {
            if new_balance > max {
                return Err(Error::ExceedsMaxBalance);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Balance, &new_balance);

        let token_client = token::TokenClient::new(&env, &Self::read_token(&env));
        token_client.transfer(
            &from,
            MuxedAddress::from(env.current_contract_address()),
            &amount,
        );

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

        let token_client = token::TokenClient::new(&env, &Self::read_token(&env));
        token_client.transfer(
            &env.current_contract_address(),
            MuxedAddress::from(owner.clone()),
            &amount,
        );

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

    pub fn set_min_deposit(env: Env, min_deposit: i128) -> Result<(), Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        Self::validate_deposit_bounds(min_deposit, Self::read_max_balance(&env))?;
        env.storage()
            .instance()
            .set(&DataKey::MinDeposit, &min_deposit);
        MinDepositUpdated { min_deposit }.publish(&env);
        Ok(())
    }

    /// Pass `None` to remove the cap entirely.
    pub fn set_max_balance(env: Env, max_balance: Option<i128>) -> Result<(), Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        Self::validate_deposit_bounds(Self::read_min_deposit(&env), max_balance)?;
        match max_balance {
            Some(max) => env.storage().instance().set(&DataKey::MaxBalance, &max),
            None => env.storage().instance().remove(&DataKey::MaxBalance),
        }
        MaxBalanceUpdated { max_balance }.publish(&env);
        Ok(())
    }

    /// Recovers a token *other than* this vault's own accounted asset,
    /// sent to the vault's address by mistake (e.g. a wrong-asset
    /// transfer that bypassed `deposit`). Cannot touch the vault's own
    /// token — that balance is depositors' funds, not stray tokens.
    pub fn rescue(env: Env, token: Address, to: Address, amount: i128) -> Result<(), Error> {
        let owner = Self::read_owner(&env)?;
        owner.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if token == Self::read_token(&env) {
            return Err(Error::CannotRescueVaultToken);
        }
        let token_client = token::TokenClient::new(&env, &token);
        token_client.transfer(
            &env.current_contract_address(),
            MuxedAddress::from(to.clone()),
            &amount,
        );
        Rescued { token, to, amount }.publish(&env);
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

    pub fn token(env: Env) -> Address {
        Self::read_token(&env)
    }

    pub fn min_deposit(env: Env) -> i128 {
        Self::read_min_deposit(&env)
    }

    pub fn max_balance(env: Env) -> Option<i128> {
        Self::read_max_balance(&env)
    }

    pub fn paused(env: Env) -> bool {
        Self::is_paused(&env)
    }

    /// Bumps this vault's instance storage TTL. Callable by anyone (they
    /// pay the fee) since keeping the vault alive benefits its owner, not
    /// the caller — there's nothing to gate. Without periodic calls to
    /// this, the network can archive the contract's storage once its TTL
    /// expires.
    pub fn extend_ttl(env: Env, threshold: u32, extend_to: u32) {
        env.storage().instance().extend_ttl(threshold, extend_to);
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

    fn read_token(env: &Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .expect("not initialized")
    }

    fn read_min_deposit(env: &Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::MinDeposit)
            .unwrap_or(0)
    }

    fn read_max_balance(env: &Env) -> Option<i128> {
        env.storage().instance().get(&DataKey::MaxBalance)
    }

    /// `max_balance < min_deposit` is deliberately allowed — an owner can
    /// use it as a stronger "no new deposits will ever fit" gate than
    /// `pause` (e.g. `max_balance = Some(0)`), without that being an
    /// input-validation error. Only genuinely nonsensical values
    /// (negative amounts) are rejected.
    fn validate_deposit_bounds(min_deposit: i128, max_balance: Option<i128>) -> Result<(), Error> {
        if min_deposit < 0 {
            return Err(Error::InvalidConfiguration);
        }
        if let Some(max) = max_balance {
            if max < 0 {
                return Err(Error::InvalidConfiguration);
            }
        }
        Ok(())
    }

    fn is_paused(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }
}

mod test;
