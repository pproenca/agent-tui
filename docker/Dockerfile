# E2E Test Container for agent-tui
# Tests agent-tui against htop in a reproducible environment
# Uses multi-stage build to compile for the correct target architecture

# Stage 1: Build the binary
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary with BuildKit cache mounts for faster rebuilds
# Note: Binary must be copied out of cache-mounted target directory before stage ends
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p agent-tui && \
    cp /build/target/release/agent-tui /build/agent-tui

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install htop, terminal dependencies, and locale support
RUN apt-get update && apt-get install -y --no-install-recommends \
    procps \
    htop \
    libncurses6 \
    libncursesw6 \
    locales \
    less \
    && rm -rf /var/lib/apt/lists/* \
    && sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen \
    && locale-gen

# Create non-root user for running tests
RUN groupadd -g 1001 testuser && \
    useradd -u 1001 -g testuser -m -s /bin/bash testuser

# Set locale and terminal environment
ENV LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8 \
    TERM=xterm-256color \
    AGENT_TUI_SOCKET=/tmp/agent-tui.sock \
    COLUMNS=120 \
    LINES=40

WORKDIR /app

# Copy binary from builder stage (from non-cache location)
COPY --from=builder /build/agent-tui /usr/local/bin/agent-tui

# Copy test script with correct ownership
COPY --chown=testuser:testuser docker/e2e-tests.sh /app/e2e-tests.sh
RUN chmod +x /app/e2e-tests.sh

# Switch to non-root user for running tests
USER testuser

ENTRYPOINT ["/app/e2e-tests.sh"]
