FROM rust:1 AS chef 
RUN cargo install cargo-chef 
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as build-recipe
RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev && \
    rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM chef as builder
RUN apt-get update && \
    apt-get install -y protobuf-compiler libprotobuf-dev && \
    rm -rf /var/lib/apt/lists/*
COPY --from=build-recipe . .
RUN SQLX_OFFLINE=true cargo build --release

FROM node:18-slim as migration
WORKDIR /app
COPY ./migrations /app/migrations
COPY ./scripts /app/scripts
RUN cd ./scripts && npm install
RUN node ./scripts/sqlgen.js

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