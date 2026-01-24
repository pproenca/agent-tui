FROM rust:1.83-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release -p agent-tui && \
    cp /build/target/release/agent-tui /build/agent-tui

FROM debian:bookworm-slim

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

RUN groupadd -g 1001 testuser && \
    useradd -u 1001 -g testuser -m -s /bin/bash testuser

ENV LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8 \
    TERM=xterm-256color \
    AGENT_TUI_SOCKET=/tmp/agent-tui.sock \
    COLUMNS=120 \
    LINES=40

WORKDIR /app

COPY --from=builder /build/agent-tui /usr/local/bin/agent-tui

COPY --chown=testuser:testuser docker/e2e-tests.sh /app/e2e-tests.sh
RUN chmod +x /app/e2e-tests.sh

USER testuser

ENTRYPOINT ["/app/e2e-tests.sh"]
