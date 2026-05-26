.PHONY: build test run clean install lint fmt check ci token-economy bench bench-search bench-graph bench-cascade

BIN_NAME = syn

build:
	cargo build

build-release:
	cargo build --release

test:
	cargo test

test-verbose:
	cargo test -- --nocapture

lint:
	cargo clippy --all-targets --all-features -- -D warnings

lint-fix:
	cargo clippy --fix --all-targets --all-features

fmt:
	cargo fmt

fmt-check:
	cargo fmt --all -- --check

check: fmt-check lint test

ci:
	bash scripts/ci.sh

token-economy:
	bash scripts/token_economy_gate.sh

clean:
	cargo clean

install: build-release
	cargo install --path .

uninstall:
	cargo uninstall $(BIN_NAME)

doc:
	cargo doc --no-deps --open

bench:
	cargo bench --bench search
	cargo bench --bench graph
	cargo bench --bench cascade

bench-search:
	cargo bench --bench search

bench-graph:
	cargo bench --bench graph

bench-cascade:
	cargo bench --bench cascade

audit:
	cargo audit --deny warnings --ignore RUSTSEC-2024-0436

outdated:
	cargo outdated

size:
	@du -sh target/release/$(BIN_NAME)

setup:
	rustup component add clippy rustfmt
	cargo install cargo-audit cargo-outdated

release-dry:
	cargo package

git-tag:
	@echo "Usage: make git-tag VERSION=x.y.z"
	git tag v$(VERSION) && git push origin v$(VERSION)
