FROM rust:1-slim-bookworm AS builder

WORKDIR /source

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /source/target/release/memocp /usr/local/bin/memocp

ENTRYPOINT ["memocp"]
