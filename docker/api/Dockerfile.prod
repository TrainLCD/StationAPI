FROM rust:1 as builder
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --quiet protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build -p stationapi --release

FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/stationapi .
ENV PORT 50051
EXPOSE $PORT
CMD ["./stationapi"]
