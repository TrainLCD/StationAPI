FROM rust:1 as stationapi-builder
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --quiet protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --bin stationapi --release

FROM rust:1 as migration-builder
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --quiet protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --bin migration --release

FROM gcr.io/distroless/cc-debian12 as stationapi
WORKDIR /app
COPY --from=stationapi-builder /app/target/release/stationapi .
ENV PORT 50051
EXPOSE $PORT
CMD ["stationapi"]

FROM debian:bookworm-slim as migration
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --quiet default-mysql-client && \
    rm -rf /var/lib/apt/lists/*
COPY --from=migration-builder /app/data .
COPY --from=migration-builder /app/target/release/migration .
CMD ["migration"]
