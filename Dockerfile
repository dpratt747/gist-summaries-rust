FROM rust:1.85-slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
COPY src-tauri/src ./src
COPY src-tauri/build.rs ./build.rs
COPY src-tauri/tauri.conf.json ./tauri.conf.json
COPY src-tauri/icons ./icons

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/gist-summary /gist-summary

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /gist-summary /usr/local/bin/gist-summary
CMD ["gist-summary"]

