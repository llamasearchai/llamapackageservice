# Multi-stage build for the LlamaPackageService
# Stage 1: Build environment
FROM rust:1.75-bookworm as builder

# Set the author
LABEL maintainer="Nik Jois <nikjois@llamasearch.ai>"

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libcurl4-openssl-dev \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src/ ./src/
COPY assets/ ./assets/
COPY templates/ ./templates/
COPY test_data/ ./test_data/

# Build the application in release mode
RUN cargo build --release --bin llamapackageservice
RUN cargo build --release --bin server

# Stage 2: Runtime environment
FROM debian:bookworm-slim

# Set the author
LABEL maintainer="Nik Jois <nikjois@llamasearch.ai>"
LABEL description="LlamaPackageService - Transform code repositories into structured text"
LABEL version="1.0.0"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    git \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -r -s /bin/false -m -d /app llamaservice

# Set working directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /app/target/release/llamapackageservice /usr/local/bin/
COPY --from=builder /app/target/release/server /usr/local/bin/

# Copy necessary assets and templates
COPY --from=builder /app/assets/ ./assets/
COPY --from=builder /app/templates/ ./templates/

# Create directories for data and output
RUN mkdir -p /app/data /app/output /app/logs && \
    chown -R llamaservice:llamaservice /app

# Switch to non-root user
USER llamaservice

# Expose port for web server
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command runs the CLI version
CMD ["llamapackageservice", "--help"] 