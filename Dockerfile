# Multi-stage build for OCM Implementation
FROM rust:1.70 as builder

# Install wasm-pack for WASM builds
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Build the application
RUN cargo build --release --bin web-server --bin migrate
RUN cd ocm-wasm && wasm-pack build --target web --out-dir pkg

# Runtime stage
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false ocmuser

# Set working directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /app/target/release/web-server /app/
COPY --from=builder /app/target/release/migrate /app/
COPY --from=builder /app/ocm-core/migrations /app/migrations/
COPY --from=builder /app/ocm-wasm /app/ocm-wasm/

# Create data directory
RUN mkdir -p /app/data && chown ocmuser:ocmuser /app/data

# Switch to app user
USER ocmuser

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8000/ || exit 1

# Default command - run migrations then start server
CMD ["/bin/bash", "-c", "./migrate && ./web-server"]