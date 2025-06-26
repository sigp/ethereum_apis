FROM rust:1.85-slim as builder

WORKDIR /usr/src/workspace

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    cmake \
    libclang-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Build only the rustic-builder binary from the workspace
RUN cargo build --release --bin rustic-builder

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the specific binary from the workspace target directory
COPY --from=builder /usr/src/workspace/target/release/rustic-builder /usr/local/bin/

EXPOSE 8560

ENTRYPOINT ["rustic-builder"]