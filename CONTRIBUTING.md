# Contributing

## Setup

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cargo test --workspace
```

## Workflow

1. Fork and branch from `main`.
2. Keep changes scoped to a single contract or concern per PR.
3. Add or update tests in `contracts/lumen_vault/src/test.rs` for any
   behavior change.
4. Run `cargo test --workspace` and `cargo build --target
   wasm32-unknown-unknown --release` before opening a PR.
5. If the change affects storage layout, authorization, or arithmetic,
   update `docs/security.md` and add an ADR under `docs/adr/` if it's a
   real design decision (not just an implementation detail).

## Commit style

Small, single-purpose commits. Describe *why* in the body if the *what*
isn't obvious from the diff.

## Code of Conduct

This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).
