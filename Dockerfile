FROM rust:1.81-slim AS builder

RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ ./src/
RUN cargo build --release --no-default-features

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1001 synapse

COPY --from=builder /app/target/release/synapse /usr/local/bin/syn

USER synapse
HEALTHCHECK --interval=30s --timeout=3s CMD syn --help || exit 1
ENTRYPOINT ["syn"]
CMD ["--help"]
