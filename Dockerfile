# Build stage
FROM rust:1.80 as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    libpq-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 app

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/rust_backend /app/rust_backend

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Change ownership
RUN chown -R app:app /app

# Switch to app user
USER app

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Run the application
CMD ["/app/rust_backend"]

