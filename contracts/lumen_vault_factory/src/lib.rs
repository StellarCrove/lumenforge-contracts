#![no_std]
#[cfg(test)]
extern crate std;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    VaultWasmHash,
    VaultCount,
    VaultsByOwner(Address),
}

#[contractevent]
pub struct VaultDeployed {
    #[topic]
    pub owner: Address,
    pub vault: Address,
}

#[contract]
pub struct LumenVaultFactory;

#[contractimpl]
impl LumenVaultFactory {
    pub fn __constructor(env: Env, vault_wasm_hash: BytesN<32>) {
        env.storage()
            .instance()
            .set(&DataKey::VaultWasmHash, &vault_wasm_hash);
        env.storage().instance().set(&DataKey::VaultCount, &0u32);
    }

    /// Deploys a new `LumenVault` owned by `owner`. `salt` must be unique
    /// per deployment (e.g. a per-owner nonce) since Soroban derives the
    /// deployed contract's address deterministically from the deployer,
    /// salt, and Wasm hash — reusing a salt for the same owner would try
    /// to redeploy to an address that already exists and fail.
    pub fn deploy_vault(env: Env, owner: Address, salt: BytesN<32>) -> Result<Address, Error> {
        owner.require_auth();

        let wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::VaultWasmHash)
            .ok_or(Error::NotInitialized)?;

        let deployed = env
            .deployer()
            .with_current_contract(salt)
            .deploy_v2(wasm_hash, (owner.clone(),));

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::VaultCount, &(count + 1));

        let key = DataKey::VaultsByOwner(owner.clone());
        let mut owned: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));
        owned.push_back(deployed.clone());
        env.storage().persistent().set(&key, &owned);

        VaultDeployed {
            owner,
            vault: deployed.clone(),
        }
        .publish(&env);
        Ok(deployed)
    }

    pub fn vault_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::VaultCount)
            .unwrap_or(0)
    }

    pub fn vaults_by_owner(env: Env, owner: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::VaultsByOwner(owner))
            .unwrap_or(Vec::new(&env))
    }

    pub fn vault_wasm_hash(env: Env) -> Result<BytesN<32>, Error> {
        env.storage()
            .instance()
            .get(&DataKey::VaultWasmHash)
            .ok_or(Error::NotInitialized)
    }
}

mod test;
