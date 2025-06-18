FROM rust:slim-bookworm AS rust-build

WORKDIR /app_build

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    libssl3 \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY . ./
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /tw

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates && \
    update-ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=rust-build /app_build/emoji.txt /tw/emoji.txt
COPY --from=rust-build /app_build/target/release/bridge /tw/bridge

CMD ["/tw/bridge", "econ"]
