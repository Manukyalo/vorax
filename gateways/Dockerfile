# VORAX Gateway — multi-stage Docker build
# Produces a ~10 MB stripped binary suitable for Raspberry Pi 4, industrial NUC, or any Linux x86-64 edge host.

# ── Stage 1: Builder ──────────────────────────────────────────────────────────
FROM rust:1.80-slim AS builder

WORKDIR /build

# Cache dependencies before copying source (Docker layer caching)
COPY gateways/Cargo.toml gateways/Cargo.lock* ./gateways/
COPY Cargo.toml ./

# Dummy source to pre-compile dependencies
RUN mkdir -p gateways/src && echo 'fn main(){}' > gateways/src/main.rs
RUN cargo build --release --manifest-path gateways/Cargo.toml 2>/dev/null || true

# Copy real source and rebuild (only changed code is recompiled)
COPY gateways/src ./gateways/src
RUN touch gateways/src/main.rs
RUN cargo build --release --manifest-path gateways/Cargo.toml

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/gateways/target/release/vorax-gateway /app/vorax-gateway

# Audit logs are written to /data — mount a persistent volume here in production
RUN mkdir -p /data
VOLUME ["/data"]

# Non-root user for least-privilege operation
RUN useradd -r -u 1001 -g root vorax
USER vorax

EXPOSE 8765

ENTRYPOINT ["/app/vorax-gateway"]
CMD ["--audit-log", "/data/vorax-gateway-audit.jsonl"]
