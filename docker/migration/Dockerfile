FROM rust:1 as builder
WORKDIR /app
COPY Cargo.toml .
COPY Cargo.lock .
COPY migration .
RUN cargo build -p migration --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --quiet default-mysql-client && \
    rm -rf /var/lib/apt/lists/*
COPY data /app/data
COPY --from=builder /app/target/release/migration .
CMD ["./migration"]