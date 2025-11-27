# Multi-stage build for Windjammer
FROM rust:1.75 AS builder

WORKDIR /app

# Copy workspace and dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/windjammer-lsp/Cargo.toml ./crates/windjammer-lsp/
COPY crates/windjammer-mcp/Cargo.toml ./crates/windjammer-mcp/
COPY crates/windjammer-runtime/Cargo.toml ./crates/windjammer-runtime/

# Create dummy source files to build dependencies
RUN mkdir -p src benches crates/windjammer-lsp/src crates/windjammer-lsp/benches crates/windjammer-mcp/src crates/windjammer-mcp/benches crates/windjammer-runtime/src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    echo "fn main() {}" > benches/compilation.rs && \
    echo "fn main() {}" > benches/runtime.rs && \
    echo "fn main() {}" > benches/defer_drop_bench.rs && \
    echo "fn main() {}" > benches/smallvec_bench.rs && \
    echo "fn main() {}" > benches/cow_bench.rs && \
    echo "fn main() {}" > benches/defer_drop_latency.rs && \
    echo "fn main() {}" > benches/incremental_compilation.rs && \
    echo "fn main() {}" > benches/regression_benchmarks.rs && \
    echo "pub fn dummy() {}" > crates/windjammer-lsp/src/lib.rs && \
    echo "fn main() {}" > crates/windjammer-lsp/benches/salsa_performance.rs && \
    echo "pub fn dummy() {}" > crates/windjammer-mcp/src/lib.rs && \
    echo "fn main() {}" > crates/windjammer-mcp/benches/mcp_tools_benchmarks.rs && \
    echo "pub fn dummy() {}" > crates/windjammer-runtime/src/lib.rs && \
    cargo build --release && \
    rm -rf src benches crates/*/src crates/*/benches

# Copy the actual source code
COPY src ./src
COPY crates ./crates
COPY std ./std
COPY examples ./examples

# Build the actual binary
RUN cargo build --release --bin wj

# Runtime stage - use slim Debian image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libssl3 \
        && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/wj /usr/local/bin/wj

# Create symlink for backwards compatibility
RUN ln -s /usr/local/bin/wj /usr/local/bin/windjammer

# Copy the standard library
COPY --from=builder /app/std /usr/local/lib/windjammer/std

# Set environment variable for stdlib location
ENV WINDJAMMER_STDLIB=/usr/local/lib/windjammer/std

# Create workspace directory
WORKDIR /workspace

# Default command
ENTRYPOINT ["wj"]
CMD ["--help"]

# Metadata
LABEL org.opencontainers.image.title="Windjammer"
LABEL org.opencontainers.image.description="A simple language that transpiles to Rust"
LABEL org.opencontainers.image.url="https://github.com/jeffreyfriedman/windjammer"
LABEL org.opencontainers.image.source="https://github.com/jeffreyfriedman/windjammer"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"
