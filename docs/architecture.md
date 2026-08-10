# Architecture

## Overview

LumenForge is a single-contract vault protocol on Soroban. One `LumenVault`
instance is deployed per owner. It accepts deposits from any authorized
address and releases funds only to the address that initialized the
contract.

There is no factory and no registry: each deployment is independent, and
discovery of a given vault's contract ID happens off-chain (via the SDK
caller's own records).

## Contract Responsibilities

### LumenVault

The sole contract. Holds a single running balance and an owner address.

**Storage:**

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::Owner` | `Address` | Set once at `initialize`. Only this address can withdraw. |
| `BALANCE` | `i128` | Aggregate deposited balance, net of withdrawals. |

**Public functions:**

```rust
fn initialize(env: Env, owner: Address)
fn deposit(env: Env, from: Address, amount: i128) -> i128
fn withdraw(env: Env, amount: i128) -> i128
fn balance(env: Env) -> i128
```

## Storage Model

`BALANCE` is a single instance-storage value, not a per-depositor ledger.
`deposit` increments it; `withdraw` decrements it. The contract does not
track *who* contributed which portion of the balance — see
[ADR-001](adr/001-single-balance-vault.md) for the rationale and its
consequences.

## Deployment Flow

1. Deployer uploads the `lumen_vault` Wasm and instantiates a contract
   instance.
2. Deployer (or any address, per the current implementation) calls
   `initialize(owner)` exactly once. Re-initialization panics.
3. Depositors call `deposit(from, amount)`, authorizing as `from`.
4. Only `owner` can call `withdraw(amount)`; `owner.require_auth()` is
   enforced on every call.

## Single-Owner Rationale

The vault intentionally has one owner and one balance rather than per-user
sub-accounts. This keeps the contract small and auditable, at the cost of
not being suitable for pooled, multi-party custody without an accompanying
off-chain or SDK-level ledger of individual contributions. Multi-party
custody is out of scope for `v0.1` — see the SDK repo for how contribution
tracking is handled at the client layer instead.
