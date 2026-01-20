# agent-tui Docker image
#
# Build:
#   docker build -t agent-tui .
#
# Run daemon (TCP mode for cross-container communication):
#   docker run -d --name agent-tui-daemon \
#     -e AGENT_TUI_TRANSPORT=tcp \
#     -e AGENT_TUI_TCP_PORT=19847 \
#     -p 19847:19847 \
#     agent-tui daemon
#
# Run CLI (connect to daemon):
#   docker run --rm -it \
#     -e AGENT_TUI_TRANSPORT=tcp \
#     -e AGENT_TUI_TCP_PORT=19847 \
#     --network host \
#     agent-tui health
#
# Or with Docker Compose (see docker-compose.yml)

# Stage 1: Build CLI
FROM rust:1.75-slim as cli-builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy CLI source
COPY cli/Cargo.toml cli/Cargo.lock ./cli/
COPY cli/src ./cli/src

# Build CLI
WORKDIR /build/cli
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    tini \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash agent-tui

WORKDIR /app

# Copy built CLI binary
COPY --from=cli-builder /build/cli/target/release/agent-tui /usr/local/bin/

# Set environment defaults
ENV AGENT_TUI_TRANSPORT=tcp
ENV AGENT_TUI_TCP_PORT=19847
ENV AGENT_TUI_LOG_LEVEL=info

# Switch to non-root user
USER agent-tui

# Expose TCP port
EXPOSE 19847

# Use tini as init system
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default: run daemon
CMD ["agent-tui", "daemon"]
