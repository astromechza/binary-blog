FROM rust:bullseye
COPY ./ /build
RUN cd /build &&\
    cargo test --verbose --release &&\
    cargo build --release

FROM debian:bullseye-slim
COPY --from=0 /build/target/release/binary-blog /binary-blog
ENTRYPOINT ["/binary-blog"]
