# Plumed remote microphone relay. Build context: this directory.
# TODO: pin both images by digest (rust:1-bookworm@sha256:..., distroless
# cc-debian12:nonroot@sha256:...) so a rebuild cannot pull a changed image.
FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY protocol protocol
COPY server server
RUN cargo build --release --locked -p plumed-relay

# distroless: no shell, no package manager, runs as UID 65532
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/plumed-relay /plumed-relay
EXPOSE 5174
ENTRYPOINT ["/plumed-relay"]
