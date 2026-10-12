//! Owner-managed deposit allowances.
//!
//! Off by default: anyone who can authorize `from` may deposit, which is
//! the original vault behavior. When the owner turns allowances on, a
//! deposit is also limited by the remaining amount stored for that
//! address. The member list is capped so instance storage cannot grow
//! with every address an owner has ever named.

use soroban_sdk::{contractevent, Address, Env, Vec};

use super::{DataKey, Error};

/// Maximum addresses that can hold an allowance at once. Instance storage
/// holds the whole member list in one entry, so this is also the bound on
/// that entry's size. Clearing an allowance frees a slot.
pub const MAX_ALLOWANCES: u32 = 32;

#[contractevent]
pub struct AllowancesToggled {
    pub enabled: bool,
}

#[contractevent]
pub struct AllowanceUpdated {
    #[topic]
    pub depositor: Address,
    /// Remaining amount this address may still deposit. `0` means the
    /// address was removed from the member list.
    pub remaining: i128,
}

pub fn enabled(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::AllowancesEnabled)
        .unwrap_or(false)
}

pub fn set_enabled(env: &Env, enabled: bool) {
    env.storage()
        .instance()
        .set(&DataKey::AllowancesEnabled, &enabled);
    AllowancesToggled { enabled }.publish(env);
}

pub fn members(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::AllowanceMembers)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn read(env: &Env, depositor: &Address) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::Allowance(depositor.clone()))
        .unwrap_or(0)
}

/// `amount == 0` removes `depositor` from the list. A positive amount
/// creates a slot if they are not already a member, or replaces the
/// remaining amount if they are. Negative amounts are `InvalidAmount`.
/// A new member past [`MAX_ALLOWANCES`] is `AllowanceListFull` and
/// changes nothing.
pub fn set(env: &Env, depositor: Address, amount: i128) -> Result<(), Error> {
    if amount < 0 {
        return Err(Error::InvalidAmount);
    }
    let mut listed = members(env);
    let index = position(&listed, &depositor);
    if amount == 0 {
        if let Some(index) = index {
            listed.remove(index);
            env.storage()
                .instance()
                .remove(&DataKey::Allowance(depositor.clone()));
            env.storage()
                .instance()
                .set(&DataKey::AllowanceMembers, &listed);
        }
        AllowanceUpdated {
            depositor,
            remaining: 0,
        }
        .publish(env);
        return Ok(());
    }
    if index.is_none() {
        if listed.len() >= MAX_ALLOWANCES {
            return Err(Error::AllowanceListFull);
        }
        listed.push_back(depositor.clone());
        env.storage()
            .instance()
            .set(&DataKey::AllowanceMembers, &listed);
    }
    env.storage()
        .instance()
        .set(&DataKey::Allowance(depositor.clone()), &amount);
    AllowanceUpdated {
        depositor,
        remaining: amount,
    }
    .publish(env);
    Ok(())
}

/// When allowances are off, this is a no-op. When they are on, `amount`
/// must be within the stored remaining value, which is then reduced.
/// Call this before the token transfer: a later failure reverts the
/// write with the rest of the invocation.
pub fn consume(env: &Env, from: &Address, amount: i128) -> Result<(), Error> {
    if !enabled(env) {
        return Ok(());
    }
    let remaining = read(env, from);
    if amount > remaining {
        return Err(Error::AllowanceExceeded);
    }
    let next = remaining.checked_sub(amount).ok_or(Error::Overflow)?;
    env.storage()
        .instance()
        .set(&DataKey::Allowance(from.clone()), &next);
    AllowanceUpdated {
        depositor: from.clone(),
        remaining: next,
    }
    .publish(env);
    Ok(())
}

fn position(listed: &Vec<Address>, depositor: &Address) -> Option<u32> {
    for (index, member) in listed.iter().enumerate() {
        if &member == depositor {
            return u32::try_from(index).ok();
        }
    }
    None
}
