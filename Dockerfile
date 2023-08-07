FROM rust:1 AS builder 

WORKDIR /app

RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev && \
    rm -rf /var/lib/apt/lists/*
COPY . .
RUN SQLX_OFFLINE=true cargo build --release --quiet

FROM rust:1 as migration
WORKDIR /app
COPY ./migrations /app/migrations
COPY ./scripts /app/scripts
COPY ./sqlgen /app/sqlgen
RUN cd /app/sqlgen && cargo run /app/migrations /app/tmp.sql

FROM debian:bullseye-slim as runtime
WORKDIR /app
RUN mkdir /app/scripts
RUN mkdir /app/migrations
COPY --from=migration /app/tmp.sql .
COPY --from=migration /app/scripts/start.sh ./scripts
COPY --from=migration /app/scripts/migration.sh ./scripts
COPY --from=migration /app/migrations/create_table.sql ./migrations
COPY --from=builder /app/target/release/stationapi /usr/local/bin/stationapi
RUN apt-get update && \
    apt-get install -y --quiet default-mysql-client && \
    rm -rf /var/lib/apt/lists/*

ENV PORT 50051

EXPOSE $PORT

CMD ["sh", "./scripts/start.sh"]