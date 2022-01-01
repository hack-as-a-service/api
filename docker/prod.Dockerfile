FROM rust:1.56 AS builder

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release

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
