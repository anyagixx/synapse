# MODULE_CONTRACT
# MODULE_ID: M-BUILD
# PURPOSE: Build the Synapse release container with the project Rust toolchain, explicit locked cargo release semantics, and minimal required system packages
# SCOPE: Rust 1.95.0-compatible builder stage, embedded OpenCode assets, GLIBC-compatible runtime image stage, locked cargo release build command, runtime certificate/user setup, and syn entrypoint healthcheck
# DEPENDS: M-PROXY-FILTER, M-HOOKS
# LINKS: Cargo.toml, docs/modules/M-BUILD.xml, docs/phases/Phase-63.xml

# START_MODULE_MAP
# builder-stage — Compiles the syn binary in release mode
# runtime-stage — Packages the syn binary with runtime certificates and a non-root user
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v2.7.0 — Workspace split: copy all member crate sources]
# END_CHANGE_SUMMARY

# START_builder-stage
FROM rust:1.95.0-slim AS builder

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY syn-core/Cargo.toml ./syn-core/
COPY syn-engine/Cargo.toml ./syn-engine/
COPY syn-proxy/Cargo.toml ./syn-proxy/
COPY syn-run/Cargo.toml ./syn-run/
COPY syn-skills/Cargo.toml ./syn-skills/
COPY syn-mcp/Cargo.toml ./syn-mcp/
COPY syn-cli/Cargo.toml ./syn-cli/
COPY syn-core/src/ ./syn-core/src/
COPY syn-engine/src/ ./syn-engine/src/
COPY syn-proxy/src/ ./syn-proxy/src/
COPY syn-run/src/ ./syn-run/src/
COPY syn-skills/src/ ./syn-skills/src/
COPY syn-mcp/src/ ./syn-mcp/src/
COPY syn-cli/src/ ./syn-cli/src/
COPY .opencode/rules/ ./.opencode/rules/
COPY .opencode/plugins/ ./.opencode/plugins/
RUN cargo build --release --locked
# END_builder-stage

# START_runtime-stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1001 synapse

COPY --from=builder /app/target/release/syn /usr/local/bin/syn

USER synapse
HEALTHCHECK --interval=30s --timeout=3s CMD syn --help || exit 1
ENTRYPOINT ["syn"]
CMD ["--help"]
# END_runtime-stage
