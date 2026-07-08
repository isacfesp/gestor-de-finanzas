FROM rust:1-slim

# pkg-config/build-essential: dependencias nativas de compilar crates.
# rustup target add: trunk compila el frontend a wasm32, no al target nativo.
# cargo install trunk: la herramienta que build/watch/sirve el SPA.
RUN apt-get update && apt-get install -y build-essential pkg-config && \
    rustup target add wasm32-unknown-unknown && \
    cargo install trunk --locked && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app/frontend

# Trunk.docker.toml (no Trunk.toml) porque dentro de la red de compose el
# backend se llama "app", no "localhost", y hay que escuchar en 0.0.0.0
# para que el mapeo de puertos del host funcione.
CMD ["trunk", "serve", "--config", "Trunk.docker.toml"]

EXPOSE 8080
