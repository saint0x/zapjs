# ZapRS Production Dockerfile
# Multi-stage build for minimal final image

# Stage 1: Build Rust binary
FROM rust:1.75-slim as rust-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY packages/core/Cargo.toml packages/core/
COPY packages/server/Cargo.toml packages/server/
COPY packages/macros/Cargo.toml packages/macros/
COPY packages/codegen/Cargo.toml packages/codegen/

# Create dummy src files for dependency build
RUN mkdir -p packages/core/src packages/server/src packages/macros/src packages/codegen/src \
    && echo "fn main() {}" > packages/server/src/main.rs \
    && echo "pub fn dummy() {}" > packages/core/src/lib.rs \
    && echo "pub fn dummy() {}" > packages/server/src/lib.rs \
    && echo "pub fn dummy() {}" > packages/macros/src/lib.rs \
    && echo "pub fn dummy() {}" > packages/codegen/src/lib.rs \
    && mkdir -p packages/server/src/bin \
    && echo "fn main() {}" > packages/server/src/bin/zap.rs

# Build dependencies (cached layer)
RUN cargo build --release --workspace 2>/dev/null || true

# Copy actual source code
COPY packages/core/src packages/core/src
COPY packages/server/src packages/server/src
COPY packages/macros/src packages/macros/src
COPY packages/codegen/src packages/codegen/src

# Build release binary
RUN cargo build --release --bin zap

# Stage 2: Build frontend (optional)
FROM node:20-slim as frontend-builder

WORKDIR /app

# Copy package files
COPY package.json pnpm-workspace.yaml ./
COPY packages/runtime/package.json packages/runtime/
COPY packages/cli/package.json packages/cli/
COPY packages/dev-server/package.json packages/dev-server/

# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install --frozen-lockfile || pnpm install

# Copy source and build
COPY packages/runtime packages/runtime
COPY packages/cli packages/cli
COPY packages/dev-server packages/dev-server
COPY tsconfig.base.json ./

# Build TypeScript packages
RUN pnpm -r build || true

# Stage 3: Final minimal image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=rust-builder /app/target/release/zap /app/bin/zap

# Copy static files if they exist (from frontend build)
# COPY --from=frontend-builder /app/dist /app/static

# Create non-root user
RUN useradd -r -s /bin/false zap && \
    chown -R zap:zap /app

USER zap

# Default environment
ENV PORT=3000
ENV HOST=0.0.0.0
ENV RUST_LOG=info
ENV ZAP_ENV=production

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["/app/bin/zap"]
