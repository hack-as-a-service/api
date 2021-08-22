FROM rust:1.54 AS builder

ARG DATABASE_URL

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release

RUN cargo install diesel_cli --no-default-features --features postgres
RUN diesel migration run

FROM debian:buster AS runner

WORKDIR /usr/src/app

RUN apt-get update -y && apt-get install -y libssl1.1 libpq5

COPY --from=builder /usr/src/app/target/release/haas_api .
COPY --from=builder /usr/src/app/Rocket.toml .
COPY --from=builder /usr/src/app/openapi/openapi.yaml ./openapi/openapi.yaml

CMD ["./haas_api"]
