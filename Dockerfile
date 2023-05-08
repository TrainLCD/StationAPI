FROM rust:1.69 as builder
WORKDIR /usr/src/stationapi
COPY . .
RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev && rm -rf /var/lib/apt/lists/*
RUN cargo install --path .

FROM node:18-alpine as migration
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
COPY --from=builder /usr/local/cargo/bin/stationapi /usr/local/bin/stationapi
RUN apt-get update && apt-get install -y default-mysql-client && rm -rf /var/lib/apt/lists/*
RUN GRPC_HEALTH_PROBE_VERSION=v0.4.13 && \
    wget -qO/bin/grpc_health_probe https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64 && \
    chmod +x /bin/grpc_health_probe

EXPOSE 50051

CMD ["sh", "./scripts/start.sh"]
