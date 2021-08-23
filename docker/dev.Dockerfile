FROM rust:1.54

WORKDIR /usr/src/app

COPY . .

RUN cargo install diesel_cli --no-default-features --features postgres && \
    cargo install cargo-watch

# Dummy build to fetch/compile dependencies
RUN cargo build

CMD ["cargo", "watch", "-x", "run"]