FROM rust:1.54 AS builder

WORKDIR /usr/src/app

RUN cargo init

COPY Cargo.toml Cargo.lock ./

# Dummy build to cache dependencies where possible
RUN cargo build --release

COPY . .

RUN cargo build --release

FROM debian:buster AS runner

WORKDIR /usr/src/app

RUN apt-get update -y && apt-get install -y libssl1.1 libpq5 ca-certificates

COPY --from=builder /usr/src/app/target/release/haas_api .
COPY --from=builder /usr/src/app/Rocket.toml .
COPY --from=builder /usr/src/app/openapi/openapi.yaml ./openapi/openapi.yaml

CMD ["./haas_api"]
