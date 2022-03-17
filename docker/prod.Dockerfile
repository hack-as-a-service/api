FROM rust:1.56 AS builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install --no-install-recommends -y \
    libssl1.1 \
    libpq5 \
    ca-certificates \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release

ENV RUST_LOG info

CMD ["./haas_api"]
