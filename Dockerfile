FROM rust:1-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /source

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /source/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin memocp

FROM debian:bookworm-slim
COPY --from=builder /source/target/release/memocp /usr/local/bin/memocp
ENTRYPOINT ["memocp"]
