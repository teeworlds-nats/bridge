FROM rust:alpine AS rust-build

ADD . ./app_build

RUN apk --update add git build-base && \
    cd /app_build ; cargo build --release

FROM alpine:3.17

COPY config_example.yaml ./config_example.yaml
COPY --from=rust-build /app_build/target/release/bridge_bridge-rs /tw/bridge

CMD ["/tw/bridge"]