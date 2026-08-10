# ADR-002: Constructor-Based Initialization

## Status

Accepted

## Context

The original `lumen_vault` design used a separate `initialize(owner)`
call, invoked after contract creation. Any address could call it before
the legitimate deployer did, becoming the owner — a front-running risk
noted in an earlier revision of `docs/security.md`.

## Decision

Use Soroban's constructor support (`__constructor`, available from
Protocol 21 / soroban-sdk 21+) instead. The owner is passed as a
constructor argument and set in the same host operation that creates the
contract, so there is no transaction ordering in which another caller
could claim ownership first.

## Consequences

- Removes the front-running window entirely; no separate `initialize`
  call, and no `AlreadyInitialized` error path is needed.
- Deploying a vault now requires passing the owner at deploy time (via
  `soroban contract deploy -- --owner <address>`, or via the factory's
  `deploy_vault`), rather than deploying "blank" and initializing later.
- `LumenVaultFactory` uses the same pattern for its own `vault_wasm_hash`
  setup.
