FROM node:22-bookworm-slim AS web
WORKDIR /web
COPY apps/web-client/package.json apps/web-client/package-lock.json ./
RUN npm ci --ignore-scripts --no-audit --no-fund
COPY apps/web-client ./
RUN npm run build

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
COPY --from=build /src/target/release/import-reference /usr/local/bin/import-reference
COPY --from=build /src/target/release/migrate /usr/local/bin/migrate
COPY --from=web /web/dist /srv/web-client
USER 10001
ENV BIND_ADDR=0.0.0.0:8080
ENV WEB_CLIENT_ROOT=/srv/web-client
ENTRYPOINT ["/usr/local/bin/game-server"]
