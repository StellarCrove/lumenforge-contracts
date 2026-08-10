.PHONY: build test fmt fmt-check clippy check

# Build must run before test: the factory contract's tests import the
# compiled lumen_vault Wasm via `contractimport!`.
build:
	cargo build --target wasm32v1-none --release --workspace

test: build
	cargo test --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

check: fmt-check clippy test
