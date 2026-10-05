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
- `withdraw`, `pause`, `unpause`, `propose_owner`,
  `cancel_pending_owner`, `set_min_deposit`, `set_max_balance`, and
  `rescue` all require `owner.require_auth()`, where `owner` is read from
  storage rather than taken as a caller-supplied argument — a caller
  cannot claim ownership by simply passing their own address.
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
so this is not treated as a primary risk. Since a Soroban transaction is
all-or-nothing (a panic anywhere in the call tree reverts every storage
write in it), there is no partial-application window regardless of
ordering — but `deposit` and `withdraw` both still update `Balance`
*before* invoking the token's `transfer`, consistently following
checks-effects-interactions as a defensive default rather than relying
solely on transaction atomicity.

## Arithmetic

- `i128` is used for the balance to avoid the overflow ranges of smaller
  integer types at realistic token amounts.
- `deposit` and `withdraw` both reject non-positive amounts.
- `withdraw` rejects amounts greater than the current balance, then
  computes the new balance with `checked_sub` (`Error::Overflow`). The
  guard already rules out underflow; the checked op is defense in depth,
  kept symmetric with `deposit`.
- `deposit`'s balance increment uses `checked_add`, returning
  `Error::Overflow` instead of panicking or wrapping if it would exceed
  `i128::MAX`.
- `LumenVaultFactory::deploy_vault` increments its `u32` vault counter
  with `checked_add` (`Error::CountOverflow`) rather than a wrapping `+`.

## Non-Standard Tokens

`LumenVault` assumes `token` is a well-behaved SEP-41 implementation
where `transfer(from, to, amount)` moves exactly `amount` and either
succeeds or aborts the transaction — nothing else. Two classes of token
would desynchronize the vault's `Balance` from its actual holdings:

- **Fee-on-transfer tokens**: `deposit` and `batch_deposit` read the
  vault's token balance before and after `transfer`. If the credit is
  not exactly `amount`, the call returns `Error::InvalidAmount` and the
  transfer reverts with it, so `Balance` cannot record funds the vault
  did not receive. These tokens are unusable with the vault, which is
  the intended outcome. Vetting still belongs to the integrator: a
  rejected deposit is safer than a desynced one, and it is not a
  substitute for reading the token before choosing it.
- **Rebasing tokens**: if the token's own accounting changes balances
  outside of `transfer` calls, `Balance` (which only moves on
  deposit/withdraw) would drift from the vault's actual token balance.
  The before/after check does not see that drift, because it only runs
  inside `transfer`.

The fee-on-transfer case is now rejected at deposit time. Rebasing is
not, and is not checked on deployment — vetting `token` before
deploying a vault for it is still an integrator responsibility.

## Known Limitations

### 1. TTL extension is a keeper's job (the schedule is now specified)

Both contracts expose `extend_ttl` (and the factory additionally exposes
`extend_vaults_by_owner_ttl` for its per-owner persistent entries).
Soroban contracts cannot wake themselves, so a keeper process has to
call these. The schedule below is the one integrators should run.
`lumenforge-sdk` implements it as `keepAlive` / `extendTtl` /
`keepOwnerVaultsAlive`, and those helpers already default to these
numbers.

Stellar targets about one ledger every 5 seconds. The counts below use
that rate. Ledger time is not a protocol constant, so treat the
durations as planning numbers.

| Argument | Ledgers | About | Why |
|---|---|---|---|
| `threshold` | 17,280 | 1 day | Extend only once the remaining TTL is inside one day. A daily keeper then has a full missed run of margin. |
| `extend_to` | 518,400 | 30 days | After an extension, the entry lives about 30 days with no further call. |

Call both of these on that schedule for every vault you care about, and
for the factory:

- `extend_ttl(17_280, 518_400)` on each vault (instance storage is the
  whole vault) and on the factory (instance storage: wasm hash and
  vault count).
- `extend_vaults_by_owner_ttl(owner, 17_280, 518_400)` for each owner
  whose index you still read. That persistent entry has its own TTL.

A keeper that runs weekly must raise `threshold` above the gap between
runs (about 120,960 ledgers is a week; 129,600 is a safer weekly
threshold). Leaving the daily threshold in place with a weekly cron
misses the window, and the network can archive the entry.

Rent is not a fixed stroop figure in this repo. The fee is whatever
`simulateTransaction` reports for that `extend_ttl` call: inclusion fee
always, plus rent for the ledgers actually added when the remaining TTL
is at or under `threshold`. A call that finds the TTL already above
`threshold` does not write a new expiration. Read the simulated
`minResourceFee` from the SDK keeper rather than hard-coding a price
that the network can change. The ledger arithmetic and the worked
example live in
[lumenforge-docs `data-model.md`](https://github.com/StellarCrove/lumenforge-docs/blob/main/docs/data-model.md#ttl-mechanics-worked-with-real-numbers).

### 2. No per-depositor accounting

Each vault's `Balance` is a single pooled value; the contract has no
on-chain record of who contributed what — only the owner can withdraw,
and only in aggregate. This is an intentional design choice
([ADR-001](adr/001-single-balance-vault.md)), not an oversight.

### 3. No emergency stop beyond `pause`

`pause` blocks new deposits but does not block `withdraw` — by design,
the owner should always be able to retrieve funds, including while
paused. There is no contract-level mechanism to freeze withdrawals if the
*owner's* key is what's compromised; the owner is the trust root.

### 4. Salt management is the caller's responsibility

`LumenVaultFactory::deploy_vault` does not generate or track salts for
callers — a naive integration that always passes the same salt for the
same owner will only succeed once. See
[ADR-004](adr/004-permissionless-factory.md). `lumenforge-sdk` provides
`randomSalt`/`ownerNonceSalt`/`deployVaultViaFactory` so most
integrators never handle a salt directly — an ergonomic answer, not a
change to who's responsible for it.

### 5. `rescue` trusts the owner not to grief depositors indirectly

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
- ~~No input validation on `min_deposit`/`max_balance`~~ — negative
  values now rejected with `Error::InvalidConfiguration`, checked at
  construction and in both setters via a shared
  `validate_deposit_bounds` helper.
- ~~No validation on `rescue`'s amount~~ — non-positive amounts now
  rejected with `Error::InvalidAmount`, same as `deposit`/`withdraw`.
- ~~Reading a large `VaultsByOwner` list was all-or-nothing~~ —
  `vaults_by_owner` now takes `offset`/`limit`.
- ~~Paginating `vaults_by_owner` had no total to page against~~ —
  `vaults_by_owner_count(owner)` added, so a caller stops at a known
  count instead of only on a short page.
- ~~`VaultsByOwner`'s write path (`deploy_vault`'s `push_back`) grew
  unbounded per owner~~ — capped at `MAX_VAULTS_PER_OWNER` (100);
  `deploy_vault` now returns `Error::TooManyVaultsForOwner` past that,
  checked before the Wasm deploy runs so a doomed call fails cheaply.
- ~~`extend_vaults_by_owner_ttl` panicked at the host level for an owner
  with no vaults~~ — now returns `Error::NoVaultsForOwner` instead.
- ~~`deploy_vault`'s vault-counter increment could wrap past
  `u32::MAX`~~ — now `checked_add` / `Error::CountOverflow`.
- ~~`withdraw`'s balance decrement used a bare `-`~~ — now `checked_sub`,
  matching `deposit`.
- ~~Fee-on-transfer tokens could make `Balance` greater than the tokens
  the vault holds~~ — `deposit` and `batch_deposit` now compare the
  vault's token balance before and after `transfer` and return
  `Error::InvalidAmount` unless the credit is exactly `amount`.

## Disclosure

This project has not yet had an external audit. If you find a
vulnerability, please open a private security advisory on this repository
rather than a public issue.

## Audit Checklist (pre-mainnet)

- [x] TTL/rent extension policy for `LumenVault` instance storage and
      `LumenVaultFactory`'s instance and per-owner persistent storage.
      Daily keeper, `threshold` 17,280 ledgers, `extend_to` 518,400
      ledgers. See "TTL extension is a keeper's job" above. The fee is
      taken from simulation, not a hard-coded stroop rate.
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
      integrators — drafted in
      [`lumenforge-docs`](https://github.com/StellarCrove/lumenforge-docs/blob/main/docs/token-vetting-checklist.md);
      leaving unchecked pending maintainer review
