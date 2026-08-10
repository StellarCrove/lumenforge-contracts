# ADR-001: Single Pooled Balance Instead of Per-Depositor Accounting

## Status

Accepted

## Context

`LumenVault` needed to decide, at the storage-layout level, whether
deposits from multiple addresses would be tracked individually (a map of
`Address -> i128`) or pooled into a single running total that only the
owner can withdraw against.

## Decision

Use a single pooled `BALANCE` value. `deposit` increments it and
authorizes the depositor; `withdraw` decrements it and is restricted to
the vault's `owner`.

## Consequences

- The contract stays small: no map iteration, no per-key storage growth
  as the number of depositors increases.
- The contract cannot answer "how much did address X contribute" on-chain.
  Any use case that needs that (e.g., proportional payout, refunds to
  individual depositors) must track contributions off-chain or in the SDK
  layer, and treat the contract's `BALANCE` purely as an aggregate escrow.
- This makes `LumenVault` a poor fit, as-is, for trustless multi-party
  pooling where depositors need independent withdrawal rights. That would
  require a different contract (or a `DataKey::Balance(Address)`-keyed
  storage variant), which is out of scope for `v0.1`.

## Alternatives Considered

- **Per-depositor map**: rejected for v0.1 to keep the initial contract
  surface minimal and easier to audit; revisit once a concrete multi-party
  use case is defined.

## See Also

- [ADR-004](004-permissionless-factory.md): rather than making a single
  vault multi-party, `LumenVaultFactory` lets each party deploy their own
  single-owner vault instead.
