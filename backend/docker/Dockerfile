FROM rust:1-slim

RUN apt-get update && apt-get install -y build-essential libssl-dev pkg-config && \
    cargo install cargo-watch && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

CMD ["cargo", "watch", "-x", "run -p gestor"]

EXPOSE 3000

#FROM rust:1.75-slim: Todo Dockerfile empieza con un FROM. Le dice a Docker cuál es tu punto de partida. Usamos la imagen oficial de Rust, y la etiqueta slim significa que viene recortada
