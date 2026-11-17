# Plumed remote microphone relay. Build context: this directory.
# Both images are pinned by digest so a rebuild cannot pull a changed image.
# To update: docker pull the tag, then copy its RepoDigests from docker image inspect.
FROM rust:1-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY protocol protocol
COPY server server
RUN cargo build --release --locked -p plumed-relay

# distroless: no shell, no package manager, runs as UID 65532
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=build /src/target/release/plumed-relay /plumed-relay
EXPOSE 5174
ENTRYPOINT ["/plumed-relay"]
