# Security

## Threat Model

### Assets at risk

- The real SEP-41 token balance each vault holds (transferred in via
  `deposit`, out via `withdraw`/`rescue`) — this is actual custodied
  value, not just an internal counter.
- Availability of `withdraw` for the legitimate owner.
- Correctness of the factory's `VaultsByOwner` index (an integrity, not a
  funds, risk — the index is informational, not authoritative; a vault's
  own `owner()` is always the source of truth for who controls it).

### Trusted parties

- The address set as `owner` at deployment is fully trusted with the
  entire balance. There is no recovery mechanism if that key is lost or
  compromised, beyond a successful `propose_owner`/`accept_owner` run
  *before* the key is lost.
- The `token` address set at deployment is trusted to behave like a
  conforming [SEP-41](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
  token. See "Non-standard tokens" below — a malicious or broken token
  contract can break this vault's accounting.

## Authentication

- `deposit` requires `from.require_auth()` — a caller can only deposit on
  behalf of an address that has authorized the invocation. The
  underlying token transfer additionally requires its own `from`
  authorization internally; both must be present in the signed
  transaction's auth tree.
- `withdraw`, `pause`, `unpause`, `propose_owner`, `set_min_deposit`,
  `set_max_balance`, and `rescue` all require `owner.require_auth()`,
  where `owner` is read from storage rather than taken as a
  caller-supplied argument — a caller cannot claim ownership by simply
  passing their own address.
- `accept_owner` requires `pending_owner.require_auth()`, proving control
  of the proposed address before ownership actually moves.
- `LumenVaultFactory::deploy_vault` requires `owner.require_auth()`: a
  caller can only deploy a vault *for themselves*, not on behalf of an
  arbitrary address.
- `extend_ttl` (both contracts) and `extend_vaults_by_owner_ttl`
  (factory) require no authorization at all, intentionally: extending a
  TTL only costs the caller fees and benefits whoever's storage it is,
  so there is nothing to gate.

## Reentrancy

Soroban's execution model does not permit the classic EVM-style
reentrancy pattern (no fallback-triggered external calls mid-execution),
so this is not treated as a primary risk. `deposit`/`withdraw` update
`Balance` and then invoke the token's `transfer` — since a Soroban
transaction is all-or-nothing (a panic anywhere in the call tree reverts
every storage write in it), there is no partial-application window
regardless of ordering, but state is still kept in its natural
checks-effects-interactions order for clarity.

## Arithmetic

- `i128` is used for the balance to avoid the overflow ranges of smaller
  integer types at realistic token amounts.
- `deposit` and `withdraw` both reject non-positive amounts.
- `withdraw` rejects amounts greater than the current balance, preventing
  underflow.
- `deposit`'s balance increment uses `checked_add`, returning
  `Error::Overflow` instead of panicking or wrapping if it would exceed
  `i128::MAX`.

## Non-Standard Tokens

`LumenVault` assumes `token` is a well-behaved SEP-41 implementation
where `transfer(from, to, amount)` moves exactly `amount` and either
succeeds or aborts the transaction — nothing else. Two classes of token
would desynchronize the vault's `Balance` from its actual holdings:

- **Fee-on-transfer tokens**: if the token deducts a fee so the vault
  receives less than `amount`, `Balance` would over-state real holdings.
- **Rebasing tokens**: if the token's own accounting changes balances
  outside of `transfer` calls, `Balance` (which only moves on
  deposit/withdraw) would drift from the vault's actual token balance.

Neither is checked for on deployment — vetting `token` before deploying
a vault for it is an integrator responsibility.

## Known Limitations

### 1. No TTL/rent *policy* (mechanism exists)

Both contracts expose `extend_ttl` (and the factory additionally exposes
`extend_vaults_by_owner_ttl` for its per-owner persistent entries), so
instance/persistent storage TTLs *can* be bumped by anyone before they
expire and the network archives that storage. What's still missing is a
policy for *who actually calls these on a schedule* — there's no
self-triggering keeper on-chain (Soroban contracts can't wake themselves
up). This needs an off-chain cron/keeper before mainnet use, or
integrators must be told to call it themselves periodically.

### 2. `VaultsByOwner` is an unbounded vector

An owner who deploys a very large number of vaults through the factory
grows their `VaultsByOwner` entry without bound, increasing the cost of
reading/writing it. There is no pagination on `vaults_by_owner`. Fine for
the expected use case (a handful of vaults per owner); would need
revisiting for a use case with unbounded per-owner vault counts.

### 3. No per-depositor accounting

Each vault's `Balance` is a single pooled value; the contract has no
on-chain record of who contributed what — only the owner can withdraw,
and only in aggregate. This is an intentional design choice
([ADR-001](adr/001-single-balance-vault.md)), not an oversight.

### 4. No emergency stop beyond `pause`

`pause` blocks new deposits but does not block `withdraw` — by design,
the owner should always be able to retrieve funds, including while
paused. There is no contract-level mechanism to freeze withdrawals if the
*owner's* key is what's compromised; the owner is the trust root.

### 5. Salt management is the caller's responsibility

`LumenVaultFactory::deploy_vault` does not generate or track salts for
callers — a naive integration that always passes the same salt for the
same owner will only succeed once. See
[ADR-004](adr/004-permissionless-factory.md).

### 6. `rescue` trusts the owner not to grief depositors indirectly

`rescue` cannot move the vault's own configured `token`, but it *can*
move any other token the vault happens to hold — including, in principle,
LP or receipt tokens some future integration might expect to stay put.
Not a fund-loss risk for the vault's own depositors, but integrators
building on top of a vault (rather than depositing into it directly)
should be aware the owner has this reach.

## Resolved

- ~~Front-runnable `initialize`~~ — replaced with constructor-based
  initialization; see [ADR-002](adr/002-constructor-based-initialization.md).
- ~~Unchecked overflow on deposit accumulation~~ — `deposit` now uses
  `checked_add` and returns `Error::Overflow`.
- ~~No pause/emergency-stop mechanism~~ — `pause`/`unpause` added,
  gating new deposits.
- ~~`Balance` didn't correspond to any real asset~~ — `deposit`/`withdraw`
  now transfer a real SEP-41 `token` in and out; see
  [ADR-005](adr/005-single-token-per-vault.md).
- ~~No way to recover a wrong-asset transfer sent directly to a vault~~
  — `rescue` added, explicitly barred from moving the vault's own token.
- ~~No deposit-size controls~~ — `min_deposit`/`max_balance`, both
  owner-adjustable via `set_min_deposit`/`set_max_balance`.

## Disclosure

This project has not yet had an external audit. If you find a
vulnerability, please open a private security advisory on this repository
rather than a public issue.

## Audit Checklist (pre-mainnet)

- [ ] TTL/rent extension policy for `LumenVaultFactory`'s persistent
      storage
- [ ] Decide on and document per-depositor accounting requirements (if
      any consumer needs them)
- [ ] Third-party audit of `contracts/lumen_vault` and
      `contracts/lumen_vault_factory`
- [ ] Testnet soak period with monitored deposits/withdrawals and at
      least one full ownership-transfer cycle
- [ ] Confirm salt-management story in the SDK before advertising the
      factory as the primary integration path
- [ ] Document a token vetting checklist (fee-on-transfer / rebasing /
      pausable-by-issuer) before recommending a `token` address to
      integrators
