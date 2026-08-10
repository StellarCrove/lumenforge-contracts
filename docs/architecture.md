# Architecture

## Overview

LumenForge is a two-contract vault protocol on Soroban:

- **`LumenVault`** custodies a real SEP-41 token (fixed per instance,
  set at deployment) and holds a single running balance plus an owner
  address. Deposits are open to anyone (subject to `require_auth`);
  withdrawals are restricted to the owner. `deposit`/`withdraw` move
  actual token balances via the token's own `transfer` — the vault's
  `Balance` figure always reflects tokens the contract genuinely holds,
  not just an internal counter.
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
| `DataKey::Token` | `Address` | Set atomically at deployment; the SEP-41 token this vault custodies. Immutable — a vault holds exactly one asset for its lifetime. |
| `DataKey::Balance` | `i128` | Aggregate deposited balance, net of withdrawals. Always equal to the vault's actual token balance. |
| `DataKey::Paused` | `bool` | When `true`, `deposit` is rejected. |
| `DataKey::MinDeposit` | `i128` | Minimum amount accepted by `deposit`. Owner-adjustable. |
| `DataKey::MaxBalance` | `i128` (optional) | If present, `deposit` is rejected once it would push `Balance` above this. Owner-adjustable; absent means uncapped. |

**Public functions:**

```rust
fn __constructor(env: Env, owner: Address, token: Address, min_deposit: i128, max_balance: Option<i128>) -> Result<(), Error>
fn deposit(env: Env, from: Address, amount: i128) -> Result<i128, Error>
fn withdraw(env: Env, amount: i128) -> Result<i128, Error>
fn pause(env: Env) -> Result<(), Error>
fn unpause(env: Env) -> Result<(), Error>
fn set_min_deposit(env: Env, min_deposit: i128) -> Result<(), Error>
fn set_max_balance(env: Env, max_balance: Option<i128>) -> Result<(), Error>
fn rescue(env: Env, token: Address, to: Address, amount: i128) -> Result<(), Error>
fn propose_owner(env: Env, new_owner: Address) -> Result<(), Error>
fn accept_owner(env: Env) -> Result<(), Error>
fn balance(env: Env) -> i128
fn owner(env: Env) -> Result<Address, Error>
fn pending_owner(env: Env) -> Option<Address>
fn token(env: Env) -> Result<Address, Error>
fn min_deposit(env: Env) -> i128
fn max_balance(env: Env) -> Option<i128>
fn paused(env: Env) -> bool
fn extend_ttl(env: Env, threshold: u32, extend_to: u32)
```

**Events:** `Deposit`, `Withdraw`, `Paused`, `Resumed`, `OwnerProposed`,
`OwnerTransferred`, `MinDepositUpdated`, `MaxBalanceUpdated`, `Rescued` —
all defined with `#[contractevent]` so they're part of the contract's
published interface spec.

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
fn deploy_vault(env: Env, owner: Address, token: Address, min_deposit: i128, max_balance: Option<i128>, salt: BytesN<32>) -> Result<Address, Error>
fn vault_count(env: Env) -> u32
fn vaults_by_owner(env: Env, owner: Address, offset: u32, limit: u32) -> Vec<Address>
fn vault_wasm_hash(env: Env) -> Result<BytesN<32>, Error>
fn extend_ttl(env: Env, threshold: u32, extend_to: u32)
fn extend_vaults_by_owner_ttl(env: Env, owner: Address, threshold: u32, extend_to: u32) -> Result<(), Error>
```

`vaults_by_owner` is paginated (`offset`/`limit`) rather than returning
the full list — see the "unbounded vector" note in
[`docs/security.md`](security.md#known-limitations). `extend_vaults_by_owner_ttl`
returns `Error::NoVaultsForOwner` if the given owner has never deployed a
vault through this factory, instead of failing at the host level.

`deploy_vault` requires `owner.require_auth()` — only the address that
will own the new vault can trigger its deployment, so the factory cannot
be used to spam vaults on someone else's behalf. The factory itself
never touches any token balances — it only deploys vaults and forwards
each one's `token`/`min_deposit`/`max_balance` constructor arguments.

## Storage Model

`Balance` is a single instance-storage value, not a per-depositor ledger.
`deposit` increments it and pulls `amount` of `Token` from the caller
into the vault via the token's own `transfer`; `withdraw` decrements it
and pushes `amount` back out to the owner. The contract does not track
*who* contributed which portion of the balance — see
[ADR-001](adr/001-single-balance-vault.md) for the rationale and its
consequences. It also does not manage more than one token per instance —
see [ADR-005](adr/005-single-token-per-vault.md).

## Deployment Flow

### Direct deployment

1. Deployer uploads the `lumen_vault` Wasm.
2. Deployer instantiates it with the owner's address, the SEP-41 token
   address it will custody, a minimum deposit, and an optional max
   balance, all as constructor arguments. Ownership and the token are set
   in the same host operation as contract creation — see
   [ADR-002](adr/002-constructor-based-initialization.md).

### Via the factory

1. Someone uploads `lumen_vault`'s Wasm once and deploys
   `LumenVaultFactory` with that Wasm hash as its constructor argument.
2. Any address can call `deploy_vault(owner, token, min_deposit,
   max_balance, salt)`, authorizing as `owner`, to get its own vault for
   the token of its choice. `salt` must be unique per owner — see
   [ADR-004](adr/004-permissionless-factory.md).

## Single-Owner Rationale

Each vault intentionally has one owner and one pooled balance rather than
per-user sub-accounts. This keeps the contract small and auditable, at
the cost of not being suitable for pooled, multi-party custody without an
accompanying off-chain or SDK-level ledger of individual contributions.
Multi-party custody is out of scope — the factory exists so that each
party who wants a vault gets their *own* instance instead.

## Input Validation

`min_deposit` and `max_balance` are validated wherever they're set — at
construction, and again in `set_min_deposit`/`set_max_balance` — via a
shared `validate_deposit_bounds` check that rejects negative values with
`Error::InvalidConfiguration`. Deliberately *not* rejected:
`max_balance < min_deposit`, which an owner can use as a stronger
"no new deposits will ever fit" gate than `pause` (e.g.
`max_balance = Some(0)`).

Every read of internal storage that can plausibly be missing (`Owner`,
`Token`) goes through a `Result<_, Error::NotInitialized>` accessor —
`token()` was the one holdout still using `.expect("not initialized")`
(a raw panic with no error code) until this was made consistent with
`owner()`.

## Recovering Stray Assets

Because `LumenVault`'s address is a real account that can receive any
SEP-41 token (not just the one it's configured for), a wrong-asset
transfer sent directly to it — bypassing `deposit` — would otherwise be
stuck forever. `rescue` lets the owner recover *other* tokens, but is
explicitly blocked from moving the vault's own configured token, so it
can never be used to bypass `withdraw`'s accounting.
