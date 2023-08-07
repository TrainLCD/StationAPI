FROM rust:1 AS builder 
WORKDIR /app
RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev && \
    rm -rf /var/lib/apt/lists/*
COPY . .
RUN cd ./sqlgen && cargo run ../migrations ./tmp.sql
RUN SQLX_OFFLINE=true cargo build --release

FROM debian:bullseye-slim as runtime
WORKDIR /app
RUN mkdir /app/scripts
RUN mkdir /app/migrations
COPY --from=builder /app/tmp.sql .
COPY --from=builder /app/scripts/start.sh ./scripts
COPY --from=builder /app/scripts/migration.sh ./scripts
COPY --from=builder /app/migrations/create_table.sql ./migrations
COPY --from=builder /app/target/release/stationapi /usr/local/bin/stationapi
RUN apt-get update && \
    apt-get install -y --quiet default-mysql-client && \
    rm -rf /var/lib/apt/lists/*

ENV PORT 50051

EXPOSE $PORT

CMD ["sh", "./scripts/start.sh"]