# ADR-004: Permissionless, Owner-Authorized Factory

## Status

Accepted

## Context

Without a factory, every integrator has to deploy and track `lumen_vault`
contract IDs themselves, off-chain. A shared factory can deploy vaults on
demand and index them, but that raises two design questions: who can
trigger a deployment, and how are deployed addresses kept collision-free.

## Decision

- `deploy_vault(owner, salt)` requires `owner.require_auth()`. Anyone can
  call it, but only *as* the address that will own the resulting vault —
  the factory cannot be used to deploy vaults on someone else's behalf
  without their authorization.
- Deployed addresses are derived from `(factory, salt)` via
  `env.deployer().with_current_contract(salt).deploy_v2(...)`, which is
  Soroban's standard deterministic-address deployment path. `salt`
  uniqueness is the caller's responsibility; reusing a salt for a second
  deployment fails at the host level because that address already exists.

## Consequences

- No admin/allowlist on the factory — it's a pure convenience/indexing
  layer, not a gatekeeper. Direct deployment (bypassing the factory
  entirely) remains fully supported and is not privileged in any way.
- Callers must generate and manage their own salts (e.g. a per-owner
  nonce, or `owner`'s address itself if one vault per owner is
  sufficient). The SDK is expected to provide a helper for this rather
  than pushing salt management onto every integrator by hand.
- `vaults_by_owner` storage grows unbounded with an owner's vault count,
  same caveat as any unbounded on-chain vector — see
  [`docs/security.md`](../security.md#known-limitations).
