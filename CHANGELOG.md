# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Added

- `docs/architecture.md`, `docs/security.md`, and ADR-001 documenting the
  single-pooled-balance design decision.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE` (MIT).

## [0.1.0] - 2026-08-10

### Added

- Initial `lumen_vault` contract: `initialize`, `deposit`, `withdraw`,
  `balance`.
- Unit tests covering deposit/withdraw flow and overdraw rejection.
- CI workflow (build + test on push/PR to `main`).
