# Changelog

All notable changes to this project are documented in this file.

## lumen_vault 0.3.2 - 2026-08-10

### Changed

- **Breaking**: `token()` now returns `Result<Address, Error>` instead
  of panicking with a raw `.expect("not initialized")` message —
  consistent with `owner()`. Internally, `read_token` now returns
  `Result<Address, Error>` and every caller (`deposit`, `withdraw`,
  `rescue`, `token`) propagates it with `?` instead of panicking.
- Audited both contracts for unbounded/non-terminating loops: neither
  contains a manual loop over an unbounded collection (`vaults_by_owner`
  uses `Vec::slice`, not a loop). No change needed on the contract side;
  see the SDK entry below for where an actual (correctly bounded) loop
  was added.

## lumen_vault 0.3.1 / lumen_vault_factory 0.3.0 - 2026-08-10

### Added

- `contractmeta!` name/description on both contracts.
- Input validation: `min_deposit`/`max_balance` reject negative values
  (`Error::InvalidConfiguration`) at construction and in
  `set_min_deposit`/`set_max_balance`; `rescue` rejects non-positive
  amounts (`Error::InvalidAmount`), matching `deposit`/`withdraw`.
- `vaults_by_owner(owner, offset, limit)` — paginated reads instead of
  returning the whole (potentially large) vault list at once.
- `extend_vaults_by_owner_ttl` now returns `Error::NoVaultsForOwner`
  instead of panicking at the host level when the owner has no vaults.
- Tests for every new validation path and for pagination.

### Changed

- **Breaking**: `LumenVaultFactory::vaults_by_owner` and
  `extend_vaults_by_owner_ttl` signatures changed (pagination args;
  `Result` return).
- `deposit` now updates `Balance` *before* calling the token's
  `transfer`, matching `withdraw`'s ordering and consistently following
  checks-effects-interactions (previously the only function that did the
  external call first).
- `LumenVault::__constructor` now returns `Result<(), Error>` so invalid
  `min_deposit`/`max_balance` at deploy time fails cleanly instead of
  deploying a vault whose bounds were silently accepted.

## [0.3.0] - 2026-08-10

### Added

- **Real SEP-41 token custody.** `lumen_vault` now takes a `token`
  address at construction and `deposit`/`withdraw` transfer actual token
  balances via `token::TokenClient`, instead of only moving an internal
  `Balance` counter. See [ADR-005](docs/adr/005-single-token-per-vault.md).
- `min_deposit`/`max_balance` on `lumen_vault`, both owner-adjustable via
  `set_min_deposit`/`set_max_balance`.
- `rescue(token, to, amount)` on `lumen_vault` to recover a *different*
  token accidentally sent to the vault directly; explicitly barred from
  moving the vault's own configured token.
- `MinDepositUpdated`, `MaxBalanceUpdated`, `Rescued` events.
- Tests that assert on real `TokenClient::balance` (not just the vault's
  own accounting) to verify deposits/withdrawals actually move tokens.

### Changed

- `LumenVault::__constructor` and `LumenVaultFactory::deploy_vault` both
  gained `token`, `min_deposit`, `max_balance` parameters — a **breaking
  change** to both contracts' deployment interfaces.
- `docs/architecture.md` and `docs/security.md` updated for real token
  custody, deposit caps, and the `rescue` threat surface (non-standard
  token behavior, fee-on-transfer/rebasing risk).

## [0.2.0] - 2026-08-10

### Added

- `lumen_vault_factory` contract: permissionless, owner-authorized
  deployment and indexing of `lumen_vault` instances
  (`deploy_vault`, `vaults_by_owner`, `vault_count`).
- `pause`/`unpause` on `lumen_vault`, blocking new deposits.
- Two-step ownership transfer (`propose_owner`/`accept_owner`) on
  `lumen_vault`.
- `extend_ttl` on both contracts, plus `extend_vaults_by_owner_ttl` on
  the factory, so storage TTLs can be bumped before network archival.
- Typed contract events (`#[contractevent]`) for deposit, withdraw,
  pause/resume, and ownership-transfer flows.
- ADR-002 (constructor-based initialization), ADR-003 (two-step
  ownership transfer), ADR-004 (permissionless factory).
- `Makefile` with `build`/`test`/`fmt`/`clippy`/`check` targets.
- `Cargo.lock` now committed for reproducible builds.

### Changed

- Ownership is now set via a constructor at deployment (`__constructor`)
  instead of a separate, front-runnable `initialize` call.
- `deposit`'s balance increment now uses checked arithmetic
  (`Error::Overflow` instead of a possible panic/wrap).
- Bumped `soroban-sdk` to 27.0.5; build target changed from
  `wasm32-unknown-unknown` to `wasm32v1-none` (required by soroban-sdk 27
  on Rust 1.84+).
- CI now runs `cargo fmt --check`, `clippy -D warnings`, a wasm build,
  and the test suite, in that order (build before test, since
  `lumen_vault_factory`'s tests import the compiled vault Wasm).
- `docs/architecture.md` and `docs/security.md` updated for the
  two-contract layout; Known Limitations re-audited against what's now
  actually fixed vs. still open.

### Added (docs, 0.1.x follow-up)

- `docs/architecture.md`, `docs/security.md`, and ADR-001 documenting the
  single-pooled-balance design decision.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE` (MIT).

## [0.1.0] - 2026-08-10

### Added

- Initial `lumen_vault` contract: `initialize`, `deposit`, `withdraw`,
  `balance`.
- Unit tests covering deposit/withdraw flow and overdraw rejection.
- CI workflow (build + test on push/PR to `main`).
