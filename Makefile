.PHONY: build test run clean install lint fmt check ci token-economy coverage coverage-watch coverage-open bench bench-search bench-graph bench-cascade telemetry-up telemetry-down telemetry-ui

BIN_NAME = syn
COVERAGE_THRESHOLD ?= 65

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

coverage:
	cargo tarpaulin --out Html --out Json --output-dir coverage --exclude-files 'tests/*' --exclude-files 'benches/*' --fail-under $(COVERAGE_THRESHOLD)

coverage-watch:
	cargo tarpaulin --out Html --output-dir coverage --exclude-files 'tests/*' --exclude-files 'benches/*'

coverage-open:
	@if command -v xdg-open >/dev/null 2>&1; then xdg-open coverage/tarpaulin-report.html; elif command -v open >/dev/null 2>&1; then open coverage/tarpaulin-report.html; else echo coverage/tarpaulin-report.html; fi

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

telemetry-up:
	docker compose -f docker-compose.telemetry.yml up -d

telemetry-down:
	docker compose -f docker-compose.telemetry.yml down

telemetry-ui:
	@if command -v xdg-open >/dev/null 2>&1; then xdg-open http://localhost:16686; elif command -v open >/dev/null 2>&1; then open http://localhost:16686; else echo http://localhost:16686; fi

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
