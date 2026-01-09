FROM rust:1 AS builder

RUN apt-get update && apt-get install -y \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build -p stationapi --release

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/stationapi /usr/local/bin/stationapi
COPY ./data ./data

RUN apt-get update && apt-get install -y \
    libssl-dev && \
    rm -rf /var/lib/apt/lists/*

ENV HOST=0.0.0.0
# PORT is configured via docker-compose environment
EXPOSE 50051 50052

CMD ["stationapi"]