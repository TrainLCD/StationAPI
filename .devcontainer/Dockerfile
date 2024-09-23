FROM rust:1
WORKDIR /workspace
RUN apt update && \
    apt install -y --quiet protobuf-compiler mariadb-client && \
    rm -rf /var/lib/apt/lists/*
RUN groupadd vscode && \
    useradd -g vscode -m -s /bin/bash vscode
USER vscode
RUN rustup update && \
    rustup component add clippy rustfmt
RUN cargo install cargo-watch
COPY . .
RUN mkdir -p $HOME/.config/git
RUN echo '.DS_Store\n.envrc\n.vscode/settings.json' >> $HOME/.config/git/ignore
ENV PORT 50051
EXPOSE $PORT
