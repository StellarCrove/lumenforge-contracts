# lumenforge-contracts

Soroban smart contracts for the LumenForge project on Stellar.

## Contracts

- **lumen_vault** — a minimal deposit/withdraw vault contract demonstrating
  owner-gated withdrawals and per-account authorization via
  `Address::require_auth`.

## Requirements

- Rust (stable) with the `wasm32-unknown-unknown` target
- [`soroban-cli`](https://developers.stellar.org/docs/tools/developer-tools#soroban-cli)

```bash
rustup target add wasm32-unknown-unknown
```

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Test

```bash
cargo test --workspace
```

## Deploy (testnet)

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/lumen_vault.wasm \
  --source <your-identity> \
  --network testnet
```

## Related

- [`lumenforge-sdk`](https://github.com/lumenforge/lumenforge-sdk) — TypeScript
  client for interacting with these contracts.
