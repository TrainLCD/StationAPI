FROM rust:1 AS builder

RUN apt-get update && apt-get install -y \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN SQLX_OFFLINE=true cargo build -p stationapi --release

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/stationapi /usr/local/bin/stationapi
COPY ./data ./data

ENV DATABASE_URL=file:memdb?mode=memory&cache=shared
ENV HOST=0.0.0.0
ENV PORT=50051
EXPOSE $PORT

CMD ["stationapi"]