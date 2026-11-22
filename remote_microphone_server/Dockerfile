# Plumed remote microphone relay. Build context: this directory.
# Both images are pinned by digest so a rebuild cannot pull a changed image.
# To update: docker pull the tag, then copy its RepoDigests from docker image inspect.
FROM rust:1-trixie@sha256:223cc32ddca8f3ee90c436dbc94b6b5f052ea7f9653c5201e9612f08ad7d21b8 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY protocol protocol
COPY server server
RUN cargo build --release --locked -p plumed-relay

# distroless: no shell, no package manager, runs as UID 65532
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:cb87acc574abe719b6e5c547384defa2f7b1f24af9605d3f618cf852495d8be2
COPY --from=build /src/target/release/plumed-relay /plumed-relay
EXPOSE 5174
ENTRYPOINT ["/plumed-relay"]
