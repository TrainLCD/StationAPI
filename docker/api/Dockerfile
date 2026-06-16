FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
# jpreprocess の naist-jdic 辞書(約77MB)が include_bytes! でバイナリに埋め込まれるため、
# 最終クレートのコンパイル/リンク時のピークメモリが大きい。release 既定の
# codegen-units=16 だと 77MB の定数を含むモジュールと他ユニットが並列展開されメモリが
# 跳ね上がり、メモリ制約のある self-hosted ランナーで OOM (exit 137) する。
# codegen-units=1 で並列度を落としピークメモリを抑える (ビルドは遅くなるが安定する)。
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
RUN cargo build -p stationapi --release

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/stationapi /usr/local/bin/stationapi
COPY ./data ./data

RUN apt-get update && apt-get install -y \
    libssl-dev && \
    rm -rf /var/lib/apt/lists/*

ENV HOST=0.0.0.0
ENV PORT=50051
EXPOSE $PORT

CMD ["stationapi"]