# Architecture

## Overview

LumenForge is a two-contract vault protocol on Soroban:

- **`LumenVault`** holds a single running balance and an owner address.
  Deposits are open to anyone (subject to `require_auth`); withdrawals are
  restricted to the owner.
- **`LumenVaultFactory`** is an optional, permissionless entry point that
  deploys new `LumenVault` instances and indexes them by owner. Using the
  factory is not required — a vault can be deployed directly — but it
  gives callers on-chain discoverability (`vaults_by_owner`) instead of
  needing to track contract IDs off-chain themselves.

## Contract Responsibilities

### LumenVault

**Storage:**

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::Owner` | `Address` | Set atomically at deployment. Only this address can withdraw, pause, or propose a new owner. |
| `DataKey::PendingOwner` | `Address` | Set by `propose_owner`; cleared once `accept_owner` succeeds. |
| `DataKey::Balance` | `i128` | Aggregate deposited balance, net of withdrawals. |
| `DataKey::Paused` | `bool` | When `true`, `deposit` is rejected. |

**Public functions:**

```rust
fn __constructor(env: Env, owner: Address)
fn deposit(env: Env, from: Address, amount: i128) -> Result<i128, Error>
fn withdraw(env: Env, amount: i128) -> Result<i128, Error>
fn pause(env: Env) -> Result<(), Error>
fn unpause(env: Env) -> Result<(), Error>
fn propose_owner(env: Env, new_owner: Address) -> Result<(), Error>
fn accept_owner(env: Env) -> Result<(), Error>
fn balance(env: Env) -> i128
fn owner(env: Env) -> Result<Address, Error>
fn pending_owner(env: Env) -> Option<Address>
fn paused(env: Env) -> bool
fn extend_ttl(env: Env, threshold: u32, extend_to: u32)
```

**Events:** `Deposit`, `Withdraw`, `Paused`, `Resumed`, `OwnerProposed`,
`OwnerTransferred` — all defined with `#[contractevent]` so they're part
of the contract's published interface spec.

### LumenVaultFactory

**Storage:**

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::VaultWasmHash` | `BytesN<32>` | Set atomically at deployment; the Wasm hash new vaults are deployed from. |
| `DataKey::VaultCount` | `u32` | Total vaults deployed by this factory. |
| `DataKey::VaultsByOwner(Address)` | `Vec<Address>` | Vaults deployed for a given owner, in deployment order. |

**Public functions:**

```rust
fn __constructor(env: Env, vault_wasm_hash: BytesN<32>)
fn deploy_vault(env: Env, owner: Address, salt: BytesN<32>) -> Result<Address, Error>
fn vault_count(env: Env) -> u32
fn vaults_by_owner(env: Env, owner: Address) -> Vec<Address>
fn vault_wasm_hash(env: Env) -> Result<BytesN<32>, Error>
fn extend_ttl(env: Env, threshold: u32, extend_to: u32)
fn extend_vaults_by_owner_ttl(env: Env, owner: Address, threshold: u32, extend_to: u32)
```

`deploy_vault` requires `owner.require_auth()` — only the address that
will own the new vault can trigger its deployment, so the factory cannot
be used to spam vaults on someone else's behalf.

## Storage Model

`Balance` is a single instance-storage value, not a per-depositor ledger.
`deposit` increments it; `withdraw` decrements it. The contract does not
track *who* contributed which portion of the balance — see
[ADR-001](adr/001-single-balance-vault.md) for the rationale and its
consequences.

## Deployment Flow

### Direct deployment

1. Deployer uploads the `lumen_vault` Wasm.
2. Deployer instantiates it with the owner's address as a constructor
   argument. Ownership is set in the same host operation as contract
   creation — see [ADR-002](adr/002-constructor-based-initialization.md).

### Via the factory

1. Someone uploads `lumen_vault`'s Wasm once and deploys
   `LumenVaultFactory` with that Wasm hash as its constructor argument.
2. Any address can call `deploy_vault(owner, salt)`, authorizing as
   `owner`, to get its own vault. `salt` must be unique per owner — see
   [ADR-004](adr/004-permissionless-factory.md).

## Single-Owner Rationale

Each vault intentionally has one owner and one pooled balance rather than
per-user sub-accounts. This keeps the contract small and auditable, at
the cost of not being suitable for pooled, multi-party custody without an
accompanying off-chain or SDK-level ledger of individual contributions.
Multi-party custody is out of scope — the factory exists so that each
party who wants a vault gets their *own* instance instead.
