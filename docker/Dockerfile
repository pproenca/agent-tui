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

# Build release binary
RUN cargo build --release -p agent-tui

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

# Set locale and terminal environment
ENV LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8 \
    TERM=xterm-256color \
    AGENT_TUI_SOCKET=/tmp/agent-tui.sock \
    COLUMNS=120 \
    LINES=40

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /build/target/release/agent-tui /usr/local/bin/agent-tui

# Copy test script
COPY docker/e2e-tests.sh /app/e2e-tests.sh
RUN chmod +x /app/e2e-tests.sh

ENTRYPOINT ["/app/e2e-tests.sh"]
