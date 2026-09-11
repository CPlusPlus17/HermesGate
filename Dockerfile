# ==============================================================================
# Build Stage
# ==============================================================================
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/

# Cache dependencies build
RUN mkdir -p server/src server/src/ui/static && \
    echo 'fn main() { println!("dummy"); }' > server/src/main.rs && \
    echo 'pub fn dummy() {}' > server/src/lib.rs && \
    echo '<html></html>' > server/src/ui/static/index.html && \
    cargo build --release --bin hermesgate && \
    rm -rf server/src

# Copy actual source code
COPY server/src server/src/

# Build final release binary
RUN touch server/src/main.rs server/src/lib.rs && \
    cargo build --release --bin hermesgate

# ==============================================================================
# Runtime Stage
# ==============================================================================
FROM debian:bookworm-slim AS runner

# Install ca-certificates and curl for healthchecks
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user and group
RUN groupadd -g 10001 hermesgate && \
    useradd -u 10001 -g hermesgate -s /bin/sh -m hermesgate

# Setup data directory for SQLite database
RUN mkdir -p /data && chown -R hermesgate:hermesgate /data

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/hermesgate /usr/local/bin/hermesgate

USER hermesgate:hermesgate
WORKDIR /data

ENV HOST=0.0.0.0 \
    PORT=8080 \
    DATABASE_PATH=/data/hermesgate.db \
    RUST_LOG=hermesgate=info,tower_http=info

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["/usr/local/bin/hermesgate"]
