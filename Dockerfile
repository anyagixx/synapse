# MODULE_CONTRACT
# MODULE_ID: M-BUILD
# PURPOSE: Build the Synapse release container with explicit cargo release semantics and minimal required system packages
# SCOPE: builder/runtime image stages, cargo release build command, runtime certificate/user setup, and syn entrypoint healthcheck
# DEPENDS: M-PROXY-FILTER, M-HOOKS
# LINKS: Cargo.toml, docs/modules/M-BUILD.xml, docs/phases/Phase-63.xml

# START_MODULE_MAP
# builder-stage — Compiles the syn binary in release mode
# runtime-stage — Packages the syn binary with runtime certificates and a non-root user
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v3.1.0 — Removed no-op cargo feature flag and unused build packages]
# END_CHANGE_SUMMARY

# START_builder-stage
FROM rust:1.81-slim AS builder

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ ./src/
RUN cargo build --release
# END_builder-stage

# START_runtime-stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1001 synapse

COPY --from=builder /app/target/release/syn /usr/local/bin/syn

USER synapse
HEALTHCHECK --interval=30s --timeout=3s CMD syn --help || exit 1
ENTRYPOINT ["syn"]
CMD ["--help"]
# END_runtime-stage
