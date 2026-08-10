# ADR-005: Real SEP-41 Token Custody, One Token Per Vault

## Status

Accepted

## Context

The original `v0.1`/`v0.2` `LumenVault` tracked `Balance` as a bare
`i128` counter, incremented on `deposit` and decremented on `withdraw`,
with no actual asset transfer. That's not a vault — it's an unbacked
ledger; a depositor's "deposit" call never moved any real value, and an
"withdraw" would pay out from nothing. Any real Stellar vault needs to
actually custody a [SEP-41](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
token.

The remaining question was scope: one token per vault instance, or a
single vault instance supporting multiple tokens (like a multi-asset
pool)?

## Decision

Each `LumenVault` instance is constructed with exactly one `token`
address, set atomically at deployment and immutable afterward.
`deposit`/`withdraw` call that token's `transfer` directly. A vault for a
different asset is a *different deployment* — cheap and easy via
`LumenVaultFactory`, which takes `token` as a `deploy_vault` argument.

## Consequences

- `Balance` is now meaningful: it's kept in lockstep with the vault's
  actual token balance (verified directly in
  `deposit_and_withdraw_move_real_token_balances`, which asserts on
  `TokenClient::balance`, not just the vault's own accounting).
- No multi-asset bookkeeping, no per-token maps, no risk of one token's
  accounting leaking into another's — the entire contract only ever
  reasons about a single balance of a single asset.
- A user who wants vaults for N different assets deploys N vaults (cheap
  via the factory) rather than one vault juggling N token balances.
- `rescue` exists specifically to handle the one gap this leaves: a
  *different* token sent to the vault's address by mistake, outside of
  `deposit`. It can never move the vault's own configured token.

## Alternatives Considered

- **Multi-asset vault** (a `Map<Address, i128>` of per-token balances):
  rejected — significantly larger attack surface (cross-token accounting
  bugs, per-token pause/cap state) for a use case ("one owner wants
  several different assets pooled together") that's already well served
  by deploying several single-asset vaults through the factory.
