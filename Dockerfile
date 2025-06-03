FROM rust:slim-bookworm AS rust-build

WORKDIR /app_build

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install pkg-config libssl-dev libssl3 ca-certificates -y && \
    rm -rf /var/lib/apt/lists/* && \

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN cargo build --release && \
    strip target/release/bridge


FROM debian:bookworm-slim

WORKDIR /tw

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates && \
    update-ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=rust-build /app_build/target/release/bridge /tw/bridge

CMD ["/tw/bridge", "econ"]
