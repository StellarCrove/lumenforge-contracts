# Changelog

All notable changes to this project are documented in this file.

## [0.2.0] - 2026-08-10

### Added

- `lumen_vault_factory` contract: permissionless, owner-authorized
  deployment and indexing of `lumen_vault` instances
  (`deploy_vault`, `vaults_by_owner`, `vault_count`).
- `pause`/`unpause` on `lumen_vault`, blocking new deposits.
- Two-step ownership transfer (`propose_owner`/`accept_owner`) on
  `lumen_vault`.
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
