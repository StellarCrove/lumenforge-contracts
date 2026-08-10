# Security

## Threat Model

### Assets at risk

- Tokens/native balance represented by each vault's `Balance` ledger
  value.
- Availability of `withdraw` for the legitimate owner.
- Correctness of the factory's `VaultsByOwner` index (an integrity, not a
  funds, risk — the index is informational, not authoritative; a vault's
  own `owner()` is always the source of truth for who controls it).

### Trusted parties

- The address set as `owner` at deployment is fully trusted with the
  entire balance. There is no recovery mechanism if that key is lost or
  compromised, beyond a successful `propose_owner`/`accept_owner` run
  *before* the key is lost.

## Authentication

- `deposit` requires `from.require_auth()` — a caller can only deposit on
  behalf of an address that has authorized the invocation.
- `withdraw`, `pause`, `unpause`, and `propose_owner` all require
  `owner.require_auth()`, where `owner` is read from storage rather than
  taken as a caller-supplied argument — a caller cannot claim ownership by
  simply passing their own address.
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
so this is not treated as a primary risk. State is updated before events
are published in every state-changing function, consistent with
checks-effects-interactions as a defensive default.

## Arithmetic

- `i128` is used for the balance to avoid the overflow ranges of smaller
  integer types at realistic token amounts.
- `deposit` and `withdraw` both reject non-positive amounts.
- `withdraw` rejects amounts greater than the current balance, preventing
  underflow.
- `deposit`'s balance increment uses `checked_add`, returning
  `Error::Overflow` instead of panicking or wrapping if it would exceed
  `i128::MAX`.

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

## Resolved

- ~~Front-runnable `initialize`~~ — replaced with constructor-based
  initialization; see [ADR-002](adr/002-constructor-based-initialization.md).
- ~~Unchecked overflow on deposit accumulation~~ — `deposit` now uses
  `checked_add` and returns `Error::Overflow`.
- ~~No pause/emergency-stop mechanism~~ — `pause`/`unpause` added,
  gating new deposits.

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
