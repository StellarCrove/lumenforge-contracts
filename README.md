# lumenforge-contracts

Soroban smart contracts for the LumenForge project on Stellar.

## Contracts

- **`lumen_vault`** — an owner-gated deposit/withdraw vault. Deposits are
  open to any authorized address; only the vault's owner can withdraw.
  Supports pausing, and a two-step ownership transfer
  (`propose_owner` / `accept_owner`) so ownership can't be lost to a typo.
  Ownership is set atomically at deployment via a constructor, so there is
  no window for a third party to front-run initialization.
- **`lumen_vault_factory`** — a permissionless factory that deploys
  `lumen_vault` instances on demand and keeps an on-chain index of which
  vaults belong to which owner (`deploy_vault`, `vaults_by_owner`,
  `vault_count`).

See [`docs/architecture.md`](docs/architecture.md) for how the two fit
together and [`docs/security.md`](docs/security.md) for the threat model
and known limitations.

## Requirements

- Rust (stable) with the `wasm32v1-none` target — soroban-sdk 27 requires
  this target rather than `wasm32-unknown-unknown` on Rust 1.84+
- [`soroban-cli`](https://developers.stellar.org/docs/tools/developer-tools#soroban-cli)

```bash
rustup target add wasm32v1-none
```

## Build

```bash
make build
# equivalent to:
cargo build --target wasm32v1-none --release --workspace
```

## Test

```bash
make test
```

`lumen_vault_factory`'s tests deploy a real `lumen_vault` instance inside
the test environment, importing the compiled Wasm via `contractimport!` —
so `lumen_vault` must be built first. `make test` (and CI) handles the
ordering; running `cargo test --workspace` directly requires having run
`make build` (or the `cargo build` command above) at least once beforehand.

## Deploy (testnet)

```bash
# 1. Install the vault Wasm on-chain and capture its hash
soroban contract install \
  --wasm target/wasm32v1-none/release/lumen_vault.wasm \
  --source <your-identity> \
  --network testnet

# 2. Deploy the factory, passing that hash as its constructor argument
soroban contract deploy \
  --wasm target/wasm32v1-none/release/lumen_vault_factory.wasm \
  --source <your-identity> \
  --network testnet \
  -- --vault_wasm_hash <hash-from-step-1>

# 3. Anyone can now self-serve a vault
soroban contract invoke --id <factory-id> --source <your-identity> --network testnet \
  -- deploy_vault --owner <your-address> --salt <32-byte-hex-salt>
```

A vault can also be deployed directly without the factory:

```bash
soroban contract deploy \
  --wasm target/wasm32v1-none/release/lumen_vault.wasm \
  --source <your-identity> \
  --network testnet \
  -- --owner <owner-address>
```

## Related

- [`lumenforge-sdk`](https://github.com/StellarCrove/lumenforge-sdk) —
  TypeScript client for interacting with these contracts.
