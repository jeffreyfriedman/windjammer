# Multi-stage build for Windjammer
FROM rust:1.75 as builder

WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src ./src
COPY std ./std
COPY examples ./examples

# Build the actual binary
RUN cargo build --release

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
COPY --from=builder /app/target/release/windjammer /usr/local/bin/windjammer

# Copy the standard library
COPY --from=builder /app/std /usr/local/lib/windjammer/std

# Set environment variable for stdlib location
ENV WINDJAMMER_STDLIB=/usr/local/lib/windjammer/std

# Create workspace directory
WORKDIR /workspace

# Default command
ENTRYPOINT ["windjammer"]
CMD ["--help"]

# Metadata
LABEL org.opencontainers.image.title="Windjammer"
LABEL org.opencontainers.image.description="A simple language that transpiles to Rust"
LABEL org.opencontainers.image.url="https://github.com/jeffreyfriedman/windjammer"
LABEL org.opencontainers.image.source="https://github.com/jeffreyfriedman/windjammer"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"
