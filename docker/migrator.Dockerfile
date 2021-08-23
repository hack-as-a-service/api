# This Docker image performs migrations on the database

FROM rust:1.54

WORKDIR /usr/src/app

RUN cargo install diesel_cli --no-default-features --features postgres --vers 1.4.1

COPY ./migrations ./migrations

CMD ["diesel", "migration", "run"]