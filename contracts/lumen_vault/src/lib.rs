#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct LumenVault;

#[contractimpl]
impl LumenVault {
    pub fn balance(_env: Env) -> i128 {
        0
    }
}
