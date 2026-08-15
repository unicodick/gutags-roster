FROM rust:1.97-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release --bin gytags-roster --bin discord-scraper

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY config ./config
RUN mkdir -p /app/data

FROM runtime AS api
COPY --from=builder /app/target/release/gytags-roster /usr/local/bin/gytags-roster
ENTRYPOINT ["/usr/local/bin/gytags-roster"]

FROM runtime AS scraper
COPY --from=builder /app/target/release/discord-scraper /usr/local/bin/discord-scraper
ENTRYPOINT ["/usr/local/bin/discord-scraper"]
