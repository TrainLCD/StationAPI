FROM rust:1.69 as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust:1.69 as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.69 as builder
WORKDIR /app
COPY . .
RUN apt-get update
RUN apt-get install -y protobuf-compiler libprotobuf-dev
RUN rm -rf /var/lib/apt/lists/*
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release

FROM node:18-slim as migration
WORKDIR /app
COPY ./migrations /app/migrations
COPY ./scripts /app/scripts
RUN cd ./scripts && npm install
RUN node ./scripts/sqlgen.js

FROM debian:bullseye-slim
WORKDIR /app
RUN mkdir /app/scripts
RUN mkdir /app/migrations
COPY --from=migration /app/tmp.sql .
COPY --from=migration /app/scripts/start.sh ./scripts
COPY --from=migration /app/scripts/migration.sh ./scripts
COPY --from=migration /app/migrations/create_table.sql ./migrations
COPY --from=builder /app/target/release/stationapi /usr/local/bin/stationapi
RUN apt-get update
RUN apt-get install -y default-mysql-client
RUN rm -rf /var/lib/apt/lists/*

ENV PORT 50051

EXPOSE $PORT

CMD ["sh", "./scripts/start.sh"]