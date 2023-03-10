# Setup.
# FROM rust:slim AS rust
FROM rustlang/rust:nightly-bullseye-slim AS rust

# Update OS.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt update && apt upgrade -y
# && apt install -y --no-install-recommends \
# curl pkg-config


# Compile.
FROM rust AS builder

# Use sparse registry.
ARG CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Create a new empty shell project.
RUN cargo new --bin app

WORKDIR /app

# Copy manifests.
# COPY ./.cargo ./.cargo
COPY ./Cargo.lock ./Cargo.toml ./

# Build only the dependencies to cache them.
RUN --mount=type=cache,target=~/.cargo,sharing=locked \
    cargo build --release

# Remove default code from deps build.
RUN rm ./src/*.rs && rm ./target/release/deps/riveting_bot*

# Copy the source code.
COPY ./src ./src

# Build for release.
RUN --mount=type=cache,target=~/.cargo,sharing=locked \
    cargo build --release && strip --strip-all ./target/release/riveting-bot


# Final image.
FROM gcr.io/distroless/cc-debian11 AS final
# FROM debian:testing-slim
# RUN --mount=type=cache,target=/var/cache/apt,sharing=private
#     apt update && apt upgrade -y && apt install ca-certificates -y && update-ca-certificates

# Copy the build artifact from the build stage.
COPY --from=builder /app/target/release/riveting-bot /app/riveting-bot

# Run as non-root.
# USER 1000:1000

# Set the startup command.
ENTRYPOINT ["/app/riveting-bot"]
