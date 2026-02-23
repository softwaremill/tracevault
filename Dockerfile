FROM rust:1.84-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release -p tracevault-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/tracevault-server /usr/local/bin/
COPY crates/tracevault-server/migrations/ /app/migrations/
EXPOSE 3000
CMD ["tracevault-server"]
