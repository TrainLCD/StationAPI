FROM rust:1
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --quiet protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
COPY . .
RUN SQLX_OFFLINE=true cargo build -p stationapi --release
ENV DATABASE_URL=file:memdb?mode=memory&cache=shared
ENV HOST=0.0.0.0
ENV PORT=50051
EXPOSE $PORT
CMD ["./target/release/stationapi"]
