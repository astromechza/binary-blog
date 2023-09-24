FROM rust:bullseye
COPY ./ /build
RUN cd /build &&\
    cargo test --verbose --release --offline &&\
    cargo build --release --offline

FROM debian:bullseye-slim
COPY --from=0 /build/target/release/binary-blog /binary-blog
ENTRYPOINT ["/binary-blog"]
