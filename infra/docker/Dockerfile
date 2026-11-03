FROM rust:1.93.0-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps ./apps
COPY crates ./crates
COPY tests ./tests
COPY database ./database
RUN cargo build --release --locked --workspace

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --no-create-home ninja
COPY --from=build /src/target/release/game-server /usr/local/bin/game-server
COPY --from=build /src/target/release/chat-server /usr/local/bin/chat-server
COPY --from=build /src/target/release/pvp-server /usr/local/bin/pvp-server
USER 10001
ENV BIND_ADDR=0.0.0.0:8080
ENTRYPOINT ["/usr/local/bin/game-server"]
