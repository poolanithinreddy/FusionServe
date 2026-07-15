# Multi-stage build for the FusionServe gateway.
# Build context is gateway-rs/ (see docker-compose.yml).
FROM rust:1.83-slim AS build
WORKDIR /app
# Cache dependencies first.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && cargo build --release || true
# Now the real sources.
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*
# Run as non-root.
RUN useradd -r -u 10001 fusionserve
COPY --from=build /app/target/release/fusionserve-gateway /usr/local/bin/fusionserve-gateway
USER fusionserve
EXPOSE 8080
ENV FUSIONSERVE_CONFIG=/etc/fusionserve/config.yaml
ENTRYPOINT ["/usr/local/bin/fusionserve-gateway"]
