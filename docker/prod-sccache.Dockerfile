FROM rust:1.56 AS builder

WORKDIR /usr/src/app

COPY . .

# Note that we add wget here
RUN apt-get update && apt-get install --yes --no-install-recommends wget

# Install sccache to greatly speedup builds in the CI
RUN wget --progress=dot:giga https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
    && tar xzf sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
    && mv sccache-v0.2.15-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
    && chmod +x /usr/local/bin/sccache

ENV RUSTC_WRAPPER=/usr/local/bin/sccache

RUN --mount=type=secret,id=sccache_redis_uri \
    SCCACHE_REDIS="$(cat /run/secrets/sccache_redis_uri)" cargo build --release

FROM debian:buster AS runner

WORKDIR /usr/src/app

RUN apt-get update && apt-get install --no-install-recommends -y \
    libssl1.1 \
    libpq5 \
    ca-certificates \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/haas_api .
COPY --from=builder /usr/src/app/Rocket.toml .

ENV RUST_LOG info

CMD ["./haas_api"]
