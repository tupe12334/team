# ─── Build Stage ──────────────────────────────────────────────────────────────
# Use Alpine + musl for a fully static binary with no glibc version concerns.
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev protobuf-dev git

WORKDIR /build

# Copy proto files (required by daemon/build.rs)
COPY proto/ ./proto/

# Copy manifests first to cache cargo dependency downloads
COPY daemon/Cargo.toml daemon/Cargo.lock ./daemon/
COPY daemon/build.rs ./daemon/

# Build deps with a stub main — this layer is cached as long as Cargo.toml/lock don't change
RUN mkdir -p daemon/src && echo 'fn main() {}' > daemon/src/main.rs
WORKDIR /build/daemon
RUN cargo build --release
# Remove stub artifact so cargo rebuilds the binary from real source
RUN rm -f target/release/deps/daemon* target/release/daemon

# Build with real source
COPY daemon/src/ ./src/
RUN cargo build --release

# Build worktree-io so workers can execute tasks
# worktree-io lives at github.com/worktree-io/runner (not on crates.io)
RUN cargo install --git https://github.com/worktree-io/runner --root /build/tools

# ─── Runtime Stage ────────────────────────────────────────────────────────────
FROM alpine:3

# centy: auto-poller for issues marked "in queue"
# nodejs/npm are required to run the centy CLI (it's a Node.js package)
RUN apk add --no-cache ca-certificates nodejs npm git && \
    npm install -g centy@0.8.1 && \
    npm cache clean --force

COPY --from=builder /build/daemon/target/release/daemon /usr/local/bin/daemon
COPY --from=builder /build/tools/bin/worktree /usr/local/bin/worktree

# DAEMON_HOST=[::] binds to all interfaces so other containers can reach the daemon.
# Override to [::1] if you want loopback-only outside Docker.
ENV DAEMON_PORT=50051
ENV DAEMON_HOST=[::]
ENV HOME=/root

# Set working dir to /root so centy's startup hook finds the .centy stub and
# doesn't abort with "No .centy folder found" when running --global queries.
WORKDIR /root
RUN mkdir -p /root/.centy

EXPOSE 50051

CMD ["daemon"]
