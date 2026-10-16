# syntax=docker/dockerfile:1.7

# ---- Builder ----
FROM rust:1.88-slim-bookworm AS builder
WORKDIR /app

# openssl-sys (pulled in by lettre's native-tls backend) needs pkg-config and
# libssl-dev to build. utoipa-swagger-ui's build script shells out to curl at
# compile time to fetch the Swagger UI assets, so curl + ca-certificates must
# be present too.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
# sqlx::migrate! embeds these at compile time, so they must be present in the
# build context even though the runtime image never reads them from disk.
COPY migrations ./migrations

# Cache mounts keep the cargo registry and incremental build artifacts across
# builds (on the same builder) without baking them into an image layer — the
# binary is copied out to a non-cached path before the mounts disappear.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release \
    && strip target/release/nexuscare_backend \
    && cp target/release/nexuscare_backend /app/nexuscare_backend

# ---- Runtime ----
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 1001 appuser

COPY --from=builder /app/nexuscare_backend ./nexuscare_backend
USER appuser

EXPOSE 8080
CMD ["./nexuscare_backend"]
