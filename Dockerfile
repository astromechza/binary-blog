FROM rust:bullseye
COPY ./ /build
RUN cd /build &&\
    cargo test --verbose --release &&\
    cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates&& apt-get clean
COPY --from=0 /build/target/release/binary-blog /binary-blog
ENTRYPOINT ["/binary-blog"]
