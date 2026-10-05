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

# The 10k transition test is `#[ignore]`d so a normal `make test` stays
# short. This target runs it, plus the proptest cases.
fuzz: build
	cargo test -p lumen-vault invariant_ -- --ignored --test-threads=1
	cargo test -p lumen-vault invariant_proptest -- --test-threads=1
