FROM rust:bullseye
COPY Cargo.toml Cargo.lock /build/
COPY .cargo /build/.cargo
COPY src /build/src
COPY resources /build/resources
WORKDIR /build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates&& apt-get clean
COPY --from=0 /build/target/release/binary-blog /binary-blog
ENTRYPOINT ["/binary-blog"]
