# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Stage 1: Build
# ---------------------------------------------------------------------------
FROM rust:1.91-bookworm AS builder

# Install build deps (needed by reth's C/C++ transitive deps)
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    cmake \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache cargo registry separately from source
COPY Cargo.toml Cargo.lock ./
COPY crates/primitives/Cargo.toml crates/primitives/Cargo.toml
COPY crates/evm/Cargo.toml         crates/evm/Cargo.toml
COPY bin/mi-reth/Cargo.toml        bin/mi-reth/Cargo.toml

# Stub out lib/main so dependency compilation can be cached
RUN mkdir -p crates/primitives/src crates/evm/src bin/mi-reth/src && \
    echo 'pub fn placeholder() {}' > crates/primitives/src/lib.rs && \
    echo 'pub fn placeholder() {}' > crates/evm/src/lib.rs && \
    echo 'fn main() {}' > bin/mi-reth/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release --bin mi-reth 2>/dev/null || true

# Now copy real source and build
COPY crates/ crates/
COPY bin/    bin/

# Touch to force rebuild of our crates (not deps)
RUN touch crates/primitives/src/lib.rs crates/evm/src/lib.rs bin/mi-reth/src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --locked --release --bin mi-reth && \
    cp target/release/mi-reth /usr/local/bin/mi-reth

# ---------------------------------------------------------------------------
# Stage 2: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/mi-reth /usr/local/bin/mi-reth

# Also expose as 'reth' for drop-in compatibility
RUN ln -s /usr/local/bin/mi-reth /usr/local/bin/reth

EXPOSE 30303 30303/udp 8545 8546 8551

ENTRYPOINT ["/usr/local/bin/mi-reth"]
